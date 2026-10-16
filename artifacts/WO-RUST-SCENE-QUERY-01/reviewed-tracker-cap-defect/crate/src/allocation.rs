//! Explicit safe allocation accounting over caller-owned buffers.
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

/// Per-job admission scope. The shared ledger remains the only lease registry; this stores
/// just an active-count bound and an unforgeable shared owner token.
#[derive(Debug)]
pub struct AllocationTracking {
    owner: Arc<()>,
    active: usize,
}
#[derive(Debug)]
pub struct TrackedAllocationLease {
    owner: Arc<()>,
    lease: AllocationLease,
}

impl AllocationTracking {
    pub fn tracking_start(job: &ChargeLedger) -> Self {
        let _ = job;
        Self {
            owner: Arc::new(()),
            active: 0,
        }
    }
    pub fn track_capacity<T>(
        &mut self,
        job: &mut ChargeLedger,
        tag: AllocationTag,
        values: &Vec<T>,
    ) -> Result<TrackedAllocationLease> {
        if self.active >= SVG_ALLOC_TRACKER_CAPACITY {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Nodes),
                offset: 0,
            });
        }
        let active = self.active.checked_add(1).ok_or_else(arithmetic)?;
        let lease = job.admit_vec(tag, values)?;
        self.active = active;
        Ok(TrackedAllocationLease {
            owner: self.owner.clone(),
            lease,
        })
    }
    pub fn release(
        &mut self,
        job: &mut ChargeLedger,
        lease: &TrackedAllocationLease,
    ) -> Result<()> {
        if !Arc::ptr_eq(&self.owner, &lease.owner) {
            return Err(Diagnostic {
                kind: ErrorKind::InvalidArgument,
                offset: 0,
            });
        }
        let active = self.active.checked_sub(1).ok_or_else(arithmetic)?;
        job.release(lease.lease)?;
        self.active = active;
        Ok(())
    }
    pub fn tracking_stop(&self, job: &ChargeLedger) -> MemoryStats {
        let ledger = job.stats();
        MemoryStats {
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
        }
    }
}

/// Reserve into a fresh buffer, charge its actual capacity while the old buffer remains
/// live, then transfer values and publish. Before calling, the caller must have an active
/// ledger lease for current Vec capacity; release that old lease after return, when the old
/// allocation is dropped. `None` means the existing charged capacity already suffices.
pub fn reserve_table<T>(
    job: &mut ChargeLedger,
    tag: AllocationTag,
    table: &mut Vec<T>,
    additional: usize,
) -> Result<Option<AllocationLease>> {
    let required = table.len().checked_add(additional).ok_or_else(arithmetic)?;
    required
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(arithmetic)?;
    if required <= table.capacity() {
        return Ok(None);
    }
    let mut replacement = Vec::new();
    replacement
        .try_reserve_exact(required)
        .map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        })?;
    let actual = replacement
        .capacity()
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(arithmetic)?;
    let lease = job.admit(tag, actual)?;
    replacement.append(table);
    std::mem::swap(table, &mut replacement);
    Ok(Some(lease))
}
