//! Explicit safe allocation accounting over caller-owned buffers.
use vexel_scene::{
    AllocationLease, AllocationTag, ChargeLedger, Diagnostic, ErrorKind, LimitKind, Result,
};

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

/// Per-job tracker. Every charge is attached to an opaque shared ledger lease.
#[derive(Debug, Default)]
pub struct AllocationTracking {
    leases: Vec<(AllocationTag, AllocationLease, usize)>,
    stats: MemoryStats,
}

impl AllocationTracking {
    pub fn tracking_start(job: &ChargeLedger) -> Self {
        Self {
            leases: Vec::new(),
            stats: MemoryStats {
                source_bytes: job.stats().source_bytes,
                ..MemoryStats::default()
            },
        }
    }
    pub fn track_capacity<T>(
        &mut self,
        job: &mut ChargeLedger,
        tag: AllocationTag,
        values: &Vec<T>,
    ) -> Result<()> {
        let bytes = values
            .capacity()
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(arithmetic)?;
        let next_live = self
            .stats
            .live_bytes
            .checked_add(bytes)
            .ok_or_else(arithmetic)?;
        let next_total = self
            .stats
            .total_allocated_bytes
            .checked_add(bytes)
            .ok_or_else(arithmetic)?;
        let next_count = self
            .stats
            .allocation_count
            .checked_add(usize::from(bytes != 0))
            .ok_or_else(arithmetic)?;
        let next_tag = match tag {
            AllocationTag::TokenArray => Some(
                self.stats
                    .token_array_bytes
                    .checked_add(bytes)
                    .ok_or_else(arithmetic)?,
            ),
            AllocationTag::ReferenceIndex => Some(
                self.stats
                    .reference_index_bytes
                    .checked_add(bytes)
                    .ok_or_else(arithmetic)?,
            ),
            AllocationTag::ClipCopy => Some(
                self.stats
                    .clip_copy_bytes
                    .checked_add(bytes)
                    .ok_or_else(arithmetic)?,
            ),
            AllocationTag::Output => Some(
                self.stats
                    .output_bytes
                    .checked_add(bytes)
                    .ok_or_else(arithmetic)?,
            ),
            AllocationTag::None => None,
        };
        self.leases.try_reserve(1).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        })?;
        let lease = job.admit(tag, bytes)?;
        self.stats.live_bytes = next_live;
        self.stats.total_allocated_bytes = next_total;
        self.stats.allocation_count = next_count;
        self.stats.peak_live_bytes = self.stats.peak_live_bytes.max(next_live);
        match tag {
            AllocationTag::TokenArray => self.stats.token_array_bytes = next_tag.unwrap(),
            AllocationTag::ReferenceIndex => self.stats.reference_index_bytes = next_tag.unwrap(),
            AllocationTag::ClipCopy => self.stats.clip_copy_bytes = next_tag.unwrap(),
            AllocationTag::Output => self.stats.output_bytes = next_tag.unwrap(),
            AllocationTag::None => {}
        }
        self.leases.push((tag, lease, bytes));
        Ok(())
    }
    pub fn release_last(&mut self, job: &mut ChargeLedger) -> Result<()> {
        let (_, lease, bytes) = *self.leases.last().ok_or_else(invalid)?;
        let live = self
            .stats
            .live_bytes
            .checked_sub(bytes)
            .ok_or_else(arithmetic)?;
        job.release(lease)?;
        self.leases.pop();
        self.stats.live_bytes = live;
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
/// live, then transfer values and publish. This models the real old/new overlap.
pub fn reserve_table<T>(
    job: &mut ChargeLedger,
    tag: AllocationTag,
    table: &mut Vec<T>,
    additional: usize,
) -> Result<AllocationLease> {
    let required = table.len().checked_add(additional).ok_or_else(arithmetic)?;
    if required <= table.capacity() {
        return job.admit(tag, 0);
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
    Ok(lease)
}
