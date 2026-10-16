//! Tracked query buffers backed by the accepted owning ledger allocator.
use std::sync::Arc;
use vexel_scene::{
    AllocationTag, ChargeLedger, Diagnostic, ErrorKind, LedgerBuffer, LedgerIdentity, LimitKind,
    Result, SourceOwner, WorkControl,
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

/// Scope bound to the exact accepted job identity and source owner.
#[derive(Debug)]
pub struct AllocationTracking {
    owner: LedgerIdentity,
    source: SourceOwner,
    scope: Arc<()>,
    active: usize,
}

/// Typed capacity remains inside LedgerBuffer; no Vec or lease escapes.
#[derive(Debug)]
pub struct TrackedBuffer<T: Copy> {
    scope: Arc<()>,
    buffer: LedgerBuffer<T>,
}
impl<T: Copy> TrackedBuffer<T> {
    fn require(&self, tracking: &AllocationTracking, job: &ChargeLedger) -> Result<()> {
        tracking.require_job(job)?;
        if !Arc::ptr_eq(&self.scope, &tracking.scope) {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn values<'a>(
        &'a self,
        tracking: &AllocationTracking,
        job: &ChargeLedger,
    ) -> Result<&'a [T]> {
        self.require(tracking, job)?;
        self.buffer.values(job)
    }
    pub fn values_mut<'a>(
        &'a mut self,
        tracking: &AllocationTracking,
        job: &ChargeLedger,
    ) -> Result<&'a mut [T]> {
        self.require(tracking, job)?;
        self.buffer.values_mut(job)
    }
    pub fn len(&self, tracking: &AllocationTracking, job: &ChargeLedger) -> Result<usize> {
        Ok(self.values(tracking, job)?.len())
    }
    pub fn is_empty(&self, tracking: &AllocationTracking, job: &ChargeLedger) -> Result<bool> {
        Ok(self.values(tracking, job)?.is_empty())
    }
    pub fn capacity(&self, tracking: &AllocationTracking, job: &ChargeLedger) -> Result<usize> {
        self.require(tracking, job)?;
        self.buffer.capacity(job)
    }
    pub fn append(
        &mut self,
        tracking: &AllocationTracking,
        job: &ChargeLedger,
        values: &[T],
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.require(tracking, job)?;
        self.buffer.append(job, values, control)
    }
    pub fn truncate(
        &mut self,
        tracking: &AllocationTracking,
        job: &ChargeLedger,
        len: usize,
    ) -> Result<()> {
        self.require(tracking, job)?;
        self.buffer.truncate(job, len)
    }
    pub fn push(
        &mut self,
        tracking: &mut AllocationTracking,
        job: &mut ChargeLedger,
        tag: AllocationTag,
        value: T,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        if self.len(tracking, job)? == self.capacity(tracking, job)? {
            reserve_table(tracking, job, tag, self, 1, control)?;
        }
        self.buffer
            .append(job, std::slice::from_ref(&value), control)
    }
}

impl AllocationTracking {
    pub fn tracking_start(job: &ChargeLedger) -> Self {
        Self {
            owner: job.owner_identity(),
            source: job.source_owner().clone(),
            scope: Arc::new(()),
            active: 0,
        }
    }
    fn require_job(&self, job: &ChargeLedger) -> Result<()> {
        job.require_owner(self.owner, &self.source)
    }
    fn require_buffer<T: Copy>(&self, job: &ChargeLedger, buffer: &TrackedBuffer<T>) -> Result<()> {
        self.require_job(job)?;
        if !Arc::ptr_eq(&self.scope, &buffer.scope) {
            return Err(invalid());
        }
        Ok(())
    }
    fn acquire_record(&mut self) -> Result<()> {
        if self.active >= SVG_ALLOC_TRACKER_CAPACITY {
            return Err(tracker_limit());
        }
        self.active = self.active.checked_add(1).ok_or_else(arithmetic)?;
        Ok(())
    }
    fn release_record(&mut self) -> Result<()> {
        self.active = self.active.checked_sub(1).ok_or_else(arithmetic)?;
        Ok(())
    }
    pub fn track_buffer<T: Copy>(
        &mut self,
        job: &mut ChargeLedger,
        tag: AllocationTag,
        capacity: usize,
        initial: &[T],
        control: &mut dyn WorkControl,
    ) -> Result<TrackedBuffer<T>> {
        if initial.len() > capacity {
            return Err(invalid());
        }
        self.require_job(job)?;
        self.acquire_record()?;
        let mut buffer = match job.allocate_buffer(tag, capacity, control) {
            Ok(buffer) => buffer,
            Err(error) => {
                self.release_record()?;
                return Err(error);
            }
        };
        if let Err(error) = buffer.append(job, initial, control) {
            buffer.release(job)?;
            self.release_record()?;
            return Err(error);
        }
        Ok(TrackedBuffer {
            scope: self.scope.clone(),
            buffer,
        })
    }
    /// Allocates typed storage and initializes every element to the caller's zero value.
    /// No temporary heap buffer is created; work and cancellation are polled per element.
    pub fn track_filled_buffer<T: Copy>(
        &mut self,
        job: &mut ChargeLedger,
        tag: AllocationTag,
        count: usize,
        zero: T,
        control: &mut dyn WorkControl,
    ) -> Result<TrackedBuffer<T>> {
        self.require_job(job)?;
        self.acquire_record()?;
        let mut buffer = match job.allocate_buffer(tag, count, control) {
            Ok(buffer) => buffer,
            Err(error) => {
                self.release_record()?;
                return Err(error);
            }
        };
        for _ in 0..count {
            if let Err(error) = buffer.append(job, std::slice::from_ref(&zero), control) {
                buffer.release(job)?;
                self.release_record()?;
                return Err(error);
            }
        }
        Ok(TrackedBuffer {
            scope: self.scope.clone(),
            buffer,
        })
    }
    pub fn release_buffer<T: Copy>(
        &mut self,
        job: &mut ChargeLedger,
        buffer: &mut TrackedBuffer<T>,
    ) -> Result<()> {
        self.require_buffer(job, buffer)?;
        buffer.buffer.capacity(job)?;
        let remaining = self.active.checked_sub(1).ok_or_else(arithmetic)?;
        buffer.buffer.release(job)?;
        self.active = remaining;
        Ok(())
    }
    pub fn tracking_stop(&self, job: &ChargeLedger) -> Result<MemoryStats> {
        self.require_job(job)?;
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

/// Replaces a tracked capacity through accepted requested-byte preallocation,
/// actual-capacity reconciliation, work polling and conservative history rules.
pub fn reserve_table<T: Copy>(
    tracking: &mut AllocationTracking,
    job: &mut ChargeLedger,
    tag: AllocationTag,
    table: &mut TrackedBuffer<T>,
    additional: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    tracking.require_buffer(job, table)?;
    let required = table
        .len(tracking, job)?
        .checked_add(additional)
        .ok_or_else(arithmetic)?;
    if required <= table.capacity(tracking, job)? {
        return Ok(());
    }
    if tracking.active >= SVG_ALLOC_TRACKER_CAPACITY {
        return Err(tracker_limit());
    }
    tracking.acquire_record()?; // The replacement lease counts alongside the old one.
    let result = table.buffer.replace_capacity(job, tag, required, control);
    tracking.release_record()?; // Replacement success leaves one record; errors retain old.
    result
}
