//! Explicit safe allocation accounting over caller-owned buffers.
use std::cell::Cell;
use std::sync::Arc;
use vexel_scene::{
    AllocationLease, AllocationTag, ChargeLedger, Diagnostic, ErrorKind, LimitKind, Result,
};

/// Historical maximum number of simultaneously tracked allocation records.
pub const SVG_ALLOC_TRACKER_CAPACITY: usize = 65_536;

fn arithmetic() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Arithmetic),
        offset: 0,
    }
}
fn invalid() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    }
}
fn tracker_limit() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Nodes),
        offset: 0,
    }
}
fn oom() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: 0,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MemoryStats {
    pub source_bytes: usize,
    pub peak_live_bytes: usize,
    pub live_bytes: usize,
    pub total_allocated_bytes: usize,
    pub allocation_count: usize,
    pub token_array_bytes: usize,
    pub reference_index_bytes: usize,
    pub clip_copy_bytes: usize,
    pub output_bytes: usize,
    pub overflow: bool,
}

/// Per-job admission scope. All leases, including replacement overlap, share its cap.
#[derive(Debug)]
pub struct AllocationTracking {
    owner: Arc<()>,
    source: Arc<Vec<u8>>,
    ledger_identity: Option<AllocationLease>,
    active: usize,
}
#[derive(Debug)]
struct TrackedAllocationLease {
    owner: Arc<()>,
    lease: AllocationLease,
    released: Cell<bool>,
}

/// A buffer cannot be resized outside the tracker and keeps its capacity lease attached.
#[derive(Debug)]
pub struct TrackedBuffer<T> {
    values: Vec<T>,
    allocation: TrackedAllocationLease,
}
impl<T> TrackedBuffer<T> {
    pub fn as_slice(&self) -> &[T] {
        &self.values
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub fn capacity(&self) -> usize {
        self.values.capacity()
    }
    pub fn push(
        &mut self,
        tracking: &mut AllocationTracking,
        job: &mut ChargeLedger,
        tag: AllocationTag,
        value: T,
    ) -> Result<()> {
        if self.values.len() == self.values.capacity() {
            reserve_table(tracking, job, tag, self, 1)?;
        } else {
            tracking.check_job(job)?;
            if !Arc::ptr_eq(&tracking.owner, &self.allocation.owner)
                || self.allocation.released.get()
            {
                return Err(invalid());
            }
        }
        self.values.push(value);
        Ok(())
    }
}

impl AllocationTracking {
    pub fn tracking_start(job: &ChargeLedger) -> Self {
        Self {
            owner: Arc::new(()),
            source: job.source_owner().clone(),
            ledger_identity: None,
            active: 0,
        }
    }
    fn check_job(&mut self, job: &mut ChargeLedger) -> Result<()> {
        if !Arc::ptr_eq(&self.source, job.source_owner()) {
            return Err(invalid());
        }
        // The shared API exposes no ledger identity accessor. Its opaque zero-byte lease
        // provides an ownership probe; release/reissue leaves byte history unchanged.
        if let Some(identity) = self.ledger_identity {
            job.release(identity)?;
            self.ledger_identity = None;
        }
        self.ledger_identity = Some(job.admit(AllocationTag::None, 0)?);
        Ok(())
    }
    fn reserve_record(&mut self) -> Result<()> {
        if self.active >= SVG_ALLOC_TRACKER_CAPACITY {
            return Err(tracker_limit());
        }
        self.active = self.active.checked_add(1).ok_or_else(arithmetic)?;
        Ok(())
    }
    fn cancel_record(&mut self) {
        self.active -= 1;
    }
    pub fn track_buffer<T>(
        &mut self,
        job: &mut ChargeLedger,
        tag: AllocationTag,
        values: Vec<T>,
    ) -> Result<TrackedBuffer<T>> {
        self.check_job(job)?;
        self.reserve_record()?;
        match job.admit_vec(tag, &values) {
            Ok(lease) => Ok(TrackedBuffer {
                values,
                allocation: TrackedAllocationLease {
                    owner: self.owner.clone(),
                    lease,
                    released: Cell::new(false),
                },
            }),
            Err(error) => {
                self.cancel_record();
                Err(error)
            }
        }
    }
    /// Drops physical storage before releasing its lease and tracker record.
    pub fn release_buffer<T>(
        &mut self,
        job: &mut ChargeLedger,
        buffer: &mut TrackedBuffer<T>,
    ) -> Result<()> {
        self.check_job(job)?;
        if !Arc::ptr_eq(&self.owner, &buffer.allocation.owner) || buffer.allocation.released.get() {
            return Err(invalid());
        }
        let active = self.active.checked_sub(1).ok_or_else(arithmetic)?;
        let values = std::mem::take(&mut buffer.values);
        drop(values);
        job.release(buffer.allocation.lease)?;
        buffer.allocation.released.set(true);
        self.active = active;
        Ok(())
    }
    pub fn tracking_stop(&mut self, job: &mut ChargeLedger) -> Result<MemoryStats> {
        if !Arc::ptr_eq(&self.source, job.source_owner()) {
            return Err(invalid());
        }
        if let Some(identity) = self.ledger_identity {
            job.release(identity)?;
            self.ledger_identity = None;
        }
        let ledger = job.stats();
        Ok(MemoryStats {
            source_bytes: ledger.source_bytes,
            peak_live_bytes: ledger.peak_live_bytes,
            live_bytes: ledger.live_bytes,
            total_allocated_bytes: ledger.total_allocated_bytes,
            allocation_count: ledger.allocation_count,
            token_array_bytes: ledger.token_array_bytes,
            reference_index_bytes: ledger.reference_index_bytes,
            clip_copy_bytes: ledger.clip_copy_bytes,
            output_bytes: ledger.output_bytes,
            overflow: ledger.overflow,
        })
    }
}

/// Replaces a tracked buffer while both capacities count against the record and byte caps.
/// `available()` is a failure-safe preflight under the exclusive ledger borrow: if the OS
/// cannot reserve the requested buffer, no lease/history has been admitted or changed.
pub fn reserve_table<T>(
    tracking: &mut AllocationTracking,
    job: &mut ChargeLedger,
    tag: AllocationTag,
    table: &mut TrackedBuffer<T>,
    additional: usize,
) -> Result<()> {
    tracking.check_job(job)?;
    if !Arc::ptr_eq(&tracking.owner, &table.allocation.owner) || table.allocation.released.get() {
        return Err(invalid());
    }
    let required = table
        .values
        .len()
        .checked_add(additional)
        .ok_or_else(arithmetic)?;
    let minimum_bytes = required
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(arithmetic)?;
    if required <= table.values.capacity() {
        return Ok(());
    }
    if tracking.active >= SVG_ALLOC_TRACKER_CAPACITY {
        return Err(tracker_limit());
    }
    if job.available() < minimum_bytes {
        return Err(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::ReservedBytes),
            offset: 0,
        });
    }
    let next_active = tracking.active.checked_add(1).ok_or_else(arithmetic)?;
    tracking.active = next_active; // Count overlap before attempting physical allocation.
    let mut replacement = Vec::new();
    if replacement.try_reserve_exact(required).is_err() {
        tracking.cancel_record();
        return Err(oom());
    }
    let actual_bytes = match replacement.capacity().checked_mul(std::mem::size_of::<T>()) {
        Some(bytes) => bytes,
        None => {
            tracking.cancel_record();
            return Err(arithmetic());
        }
    };
    if actual_bytes > job.available() {
        tracking.cancel_record();
        return Err(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::ReservedBytes),
            offset: 0,
        });
    }
    let lease = match job.admit(tag, actual_bytes) {
        Ok(lease) => lease,
        Err(error) => {
            tracking.cancel_record();
            return Err(error);
        }
    };
    replacement.append(&mut table.values);
    let old = std::mem::replace(&mut table.values, replacement);
    drop(old); // Old capacity is live through the overlap and until physical cleanup.
    job.release(table.allocation.lease)?;
    tracking.cancel_record();
    table.allocation = TrackedAllocationLease {
        owner: tracking.owner.clone(),
        lease,
        released: Cell::new(false),
    };
    Ok(())
}
