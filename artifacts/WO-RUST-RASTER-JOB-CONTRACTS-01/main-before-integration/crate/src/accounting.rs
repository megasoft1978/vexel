//! Per-job capacity admission. Reservations are explicit leases; consumers
//! release them when the corresponding owned buffers leave the job. This is
//! accounting over caller-owned allocations, not a global allocator hook.
use crate::memory::{add, arithmetic, bytes, reserve};
use crate::types::{invalid, limit};
use crate::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_LEDGER: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum AllocationTag {
    None,
    TokenArray,
    ReferenceIndex,
    ClipCopy,
    Output,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderWorkStats {
    pub shape_pixels: u64,
    pub shape_samples: u64,
    pub fill_edge_tests: u64,
    pub fill_interval_tests: u64,
    pub stroke_tests: u64,
    pub scanline_edge_entries: u64,
    pub shape_row_visits: u64,
    pub boundary_mark_work: u64,
    pub total_work: u64,
}
impl RenderWorkStats {
    pub fn charge(&mut self, kind: WorkKind, units: u64) -> Result<()> {
        let total = self.total_work.checked_add(units).ok_or_else(arithmetic)?;
        let target = match kind {
            WorkKind::ShapePixel => Some(&mut self.shape_pixels),
            WorkKind::ShapeSample => Some(&mut self.shape_samples),
            WorkKind::FillEdgeTest => Some(&mut self.fill_edge_tests),
            WorkKind::FillIntervalTest => Some(&mut self.fill_interval_tests),
            WorkKind::StrokeTest => Some(&mut self.stroke_tests),
            WorkKind::ScanlineEdgeEntry => Some(&mut self.scanline_edge_entries),
            WorkKind::ShapeRowVisit => Some(&mut self.shape_row_visits),
            WorkKind::BoundaryMark => Some(&mut self.boundary_mark_work),
            _ => None,
        };
        if let Some(value) = target {
            *value = value.checked_add(units).ok_or_else(arithmetic)?;
        }
        self.total_work = total;
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderMemoryStats {
    pub peak_live_bytes: usize,
    pub live_bytes: usize,
    pub total_allocated_bytes: usize,
    pub allocation_count: usize,
    pub source_bytes: usize,
    pub token_array_bytes: usize,
    pub reference_index_bytes: usize,
    pub clip_copy_bytes: usize,
    pub output_bytes: usize,
    pub overflow: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationLease {
    owner: u64,
    slot: usize,
    serial: u64,
}
#[derive(Debug, Clone, Copy)]
struct Entry {
    serial: u64,
    tag: AllocationTag,
    bytes: usize,
    active: bool,
    next_free: Option<usize>,
}
#[derive(Debug)]
pub struct ChargeLedger {
    source: SourceOwner,
    owner: u64,
    next: u64,
    max_leases: usize,
    reserved_bytes: usize,
    per_tag: [usize; 5],
    entries: Vec<Entry>,
    free: Option<usize>,
    stats: RenderMemoryStats,
    pub work: RenderWorkStats,
}
impl ChargeLedger {
    pub fn new(source: &SourceOwner, reserved_bytes: usize, max_leases: usize) -> Result<Self> {
        if source.len() > 1 << 30 {
            return Err(limit(LimitKind::SourceBytes));
        }
        if source.capacity() > reserved_bytes {
            return Err(limit(LimitKind::ReservedBytes));
        }
        let owner = NEXT_LEDGER
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| arithmetic())?;
        let source_bytes = source.capacity();
        Ok(Self {
            source: source.clone(),
            owner,
            next: 1,
            max_leases,
            reserved_bytes,
            per_tag: [0; 5],
            entries: Vec::new(),
            free: None,
            stats: RenderMemoryStats {
                live_bytes: source_bytes,
                peak_live_bytes: source_bytes,
                source_bytes,
                ..Default::default()
            },
            work: Default::default(),
        })
    }
    pub fn source_owner(&self) -> &SourceOwner {
        &self.source
    }
    pub fn available(&self) -> usize {
        self.reserved_bytes - self.stats.live_bytes
    }
    pub fn stats(&self) -> RenderMemoryStats {
        let mut s = self.stats;
        s.token_array_bytes = self.per_tag[1];
        s.reference_index_bytes = self.per_tag[2];
        s.clip_copy_bytes = self.per_tag[3];
        s.output_bytes = self.per_tag[4];
        s
    }
    /// Capacity supplied by an actual Vec, not logical element count.
    pub fn admit_vec<T>(&mut self, tag: AllocationTag, values: &Vec<T>) -> Result<AllocationLease> {
        self.admit(tag, bytes(values)?)
    }
    /// Aggregate nested capacity computed by the full owning module (e.g. scratch).
    /// Callers retain the buffer and release its lease at the same lifetime boundary.
    pub fn admit(&mut self, tag: AllocationTag, amount: usize) -> Result<AllocationLease> {
        match self.admit_inner(tag, amount) {
            Err(error) if error.kind == ErrorKind::Limit(LimitKind::Arithmetic) => {
                self.stats.overflow = true;
                Err(error)
            }
            result => result,
        }
    }
    fn admit_inner(&mut self, tag: AllocationTag, amount: usize) -> Result<AllocationLease> {
        let next = self.next.checked_add(1).ok_or_else(arithmetic)?;
        let count = self
            .stats
            .allocation_count
            .checked_add(usize::from(amount != 0))
            .ok_or_else(arithmetic)?;
        let tagged = add(self.per_tag[tag as usize], amount)?;
        let history = add(self.stats.total_allocated_bytes, amount)?;
        let total = add(self.stats.live_bytes, amount)?;
        let overlap = self
            .reserved_bytes
            .checked_sub(total)
            .ok_or_else(|| limit(LimitKind::ReservedBytes))?;
        if self.free.is_none() {
            count.checked_add(1).ok_or_else(arithmetic)?;
            let (new, old) = reserve(
                &mut self.entries,
                1,
                self.max_leases,
                overlap.min(usize::MAX - history),
            )?;
            if new != 0 {
                let current = add(total, new)?;
                let new_history = add(history, new)?;
                let new_count = count.checked_add(1).ok_or_else(arithmetic)?;
                self.stats.peak_live_bytes = self.stats.peak_live_bytes.max(current);
                self.stats.live_bytes = add(self.stats.live_bytes - old, new)?;
                self.per_tag[0] = add(self.per_tag[0] - old, new)?;
                self.stats.total_allocated_bytes = new_history - amount;
                self.stats.allocation_count = new_count - usize::from(amount != 0);
            }
        }
        // Recalculate after registry growth. Metadata is itself retained and charged.
        let current = add(self.stats.live_bytes, amount)?;
        if current > self.reserved_bytes {
            return Err(limit(LimitKind::ReservedBytes));
        }
        let serial = self.next;
        self.next = next;
        let slot = if let Some(slot) = self.free {
            self.free = self.entries[slot].next_free;
            slot
        } else {
            self.entries.len()
        };
        let entry = Entry {
            serial,
            tag,
            bytes: amount,
            active: true,
            next_free: None,
        };
        if slot == self.entries.len() {
            self.entries.push(entry);
        } else {
            self.entries[slot] = entry;
        }
        self.per_tag[tag as usize] = if tag == AllocationTag::None {
            add(self.per_tag[0], amount)?
        } else {
            tagged
        };
        self.stats.live_bytes = current;
        self.stats.peak_live_bytes = self.stats.peak_live_bytes.max(current);
        self.stats.total_allocated_bytes = add(self.stats.total_allocated_bytes, amount)?;
        self.stats.allocation_count = self
            .stats
            .allocation_count
            .checked_add(usize::from(amount != 0))
            .ok_or_else(arithmetic)?;
        Ok(AllocationLease {
            owner: self.owner,
            slot,
            serial,
        })
    }
    pub fn release(&mut self, lease: AllocationLease) -> Result<()> {
        if lease.owner != self.owner {
            return Err(invalid(0));
        }
        let entry = self.entries.get_mut(lease.slot).ok_or_else(|| invalid(0))?;
        if !entry.active || entry.serial != lease.serial {
            return Err(invalid(0));
        }
        entry.active = false;
        self.per_tag[entry.tag as usize] -= entry.bytes;
        self.stats.live_bytes -= entry.bytes;
        entry.next_free = self.free;
        self.free = Some(lease.slot);
        Ok(())
    }
}
