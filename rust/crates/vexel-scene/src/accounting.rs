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
    payload_history: [usize; 5],
    entries: Vec<Entry>,
    free: Option<usize>,
    stats: RenderMemoryStats,
    pub work: RenderWorkStats,
}
impl ChargeLedger {
    pub(crate) fn composition_parts(&mut self) -> (&SourceOwner, u64, &mut RenderWorkStats) {
        (&self.source, self.owner, &mut self.work)
    }
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
            payload_history: [0; 5],
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
    /// Cumulative admitted caller payload bytes by tag plus actual retained
    /// lease-registry capacity growth under `None`. Neither history decreases
    /// on release or registry-slot reuse.
    pub fn allocation_history(&self) -> [usize; 5] {
        self.payload_history
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
        let payload_history = add(self.payload_history[tag as usize], amount)?;
        let total = add(self.stats.live_bytes, amount)?;
        let overlap = self
            .reserved_bytes
            .checked_sub(total)
            .ok_or_else(|| limit(LimitKind::ReservedBytes))?;
        if self.free.is_none() {
            count.checked_add(1).ok_or_else(arithmetic)?;
            let metadata_headroom = (usize::MAX - self.stats.total_allocated_bytes).min(
                usize::MAX
                    - if tag == AllocationTag::None {
                        tagged
                    } else {
                        self.per_tag[0]
                    },
            );
            let overlap = overlap.min(usize::MAX - history).min(metadata_headroom);
            let required = add(self.entries.len(), 1)?;
            if required > self.max_leases {
                return Err(limit(LimitKind::Nodes));
            }
            if required > self.entries.capacity() {
                let entry_size = std::mem::size_of::<Entry>();
                let requested = required.checked_mul(entry_size).ok_or_else(arithmetic)?;
                if requested > overlap {
                    return Err(limit(LimitKind::ReservedBytes));
                }
                let old = bytes(&self.entries)?;
                // reserve rejects any actual replacement exceeding overlap before
                // moving entries. Vec may overallocate, so prove history fits for
                // EVERY capacity it can commit, not merely its requested capacity.
                // Capacity is an integral number of Entries; old is smaller than
                // requested. Keep this arithmetic gate separate from budget errors.
                let max_delta = (overlap / entry_size * entry_size)
                    .checked_sub(old)
                    .ok_or_else(arithmetic)?;
                let none_history = if tag == AllocationTag::None {
                    payload_history
                } else {
                    self.payload_history[0]
                };
                add(none_history, max_delta)?;
            }
            let (new, old) = reserve(&mut self.entries, 1, self.max_leases, overlap)?;
            if new != 0 {
                let current = add(total, new)?;
                let new_history = add(history, new)?;
                let new_count = count.checked_add(1).ok_or_else(arithmetic)?;
                self.stats.peak_live_bytes = self.stats.peak_live_bytes.max(current);
                self.stats.live_bytes = add(self.stats.live_bytes - old, new)?;
                self.per_tag[0] = add(self.per_tag[0] - old, new)?;
                let delta = new.checked_sub(old).ok_or_else(arithmetic)?;
                self.payload_history[0] = add(self.payload_history[0], delta)?;
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
        self.payload_history[tag as usize] = if tag == AllocationTag::None {
            add(self.payload_history[0], amount)?
        } else {
            payload_history
        };
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

impl ChargeLedger {
    pub(crate) fn check_lease(&self, lease: AllocationLease) -> Result<()> {
        if lease.owner != self.owner
            || !self
                .entries
                .get(lease.slot)
                .is_some_and(|e| e.active && e.serial == lease.serial)
        {
            return Err(invalid(0));
        }
        Ok(())
    }
    pub(crate) fn identity(&self) -> u64 {
        self.owner
    }

    /// A zero-copy replacement allocator used only by the owning job. The
    /// active entry reserves requested bytes before Vec::try_reserve_exact;
    /// exclusive ledger access also holds the remaining overlap budget in escrow
    /// until the actual capacity has been checked and the lease finalized.
    pub(crate) fn allocate<T>(
        &mut self,
        tag: AllocationTag,
        capacity: usize,
        control: &mut dyn WorkControl,
    ) -> Result<(Vec<T>, AllocationLease)> {
        control.checkpoint().map_err(crate::types::stopped)?;
        let requested = match capacity.checked_mul(std::mem::size_of::<T>()) {
            Some(requested) => requested,
            None => {
                self.stats.overflow = true;
                return Err(arithmetic());
            }
        };
        // Mint a zero lease first: registry growth is admitted before payload
        // allocation, and retained even when payload allocation subsequently fails.
        let lease = self.admit(tag, 0)?;
        let result = (|| {
            if requested > self.available() {
                return Err(limit(LimitKind::ReservedBytes));
            }
            add(self.stats.total_allocated_bytes, requested)?;
            self.stats
                .allocation_count
                .checked_add(usize::from(requested != 0))
                .ok_or_else(arithmetic)?;
            let tagged_reservation = add(self.per_tag[tag as usize], requested)?;
            let live_reservation = add(self.stats.live_bytes, requested)?;
            // The active registry entry is a real pre-allocation reservation.
            // Failure releases it; only successful physical reserves enter history.
            self.entries[lease.slot].bytes = requested;
            self.per_tag[tag as usize] = tagged_reservation;
            self.stats.live_bytes = live_reservation;
            let mut values = Vec::new();
            values.try_reserve_exact(capacity).map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset: 0,
            })?;
            let actual = bytes(&values)?;
            // Vec may report excess capacity. No values are transferred or
            // published until actual capacity and history pass the SAME budget.
            let history = add(self.stats.total_allocated_bytes, actual)?;
            let payload_history = add(self.payload_history[tag as usize], actual)?;
            let count = self
                .stats
                .allocation_count
                .checked_add(usize::from(actual != 0))
                .ok_or_else(arithmetic)?;
            self.stats.total_allocated_bytes = history;
            self.payload_history[tag as usize] = payload_history;
            self.stats.allocation_count = count;
            let escrow = add(requested, self.available())?;
            if actual > escrow {
                return Err(limit(LimitKind::ReservedBytes));
            }
            let tagged = add(self.per_tag[tag as usize] - requested, actual)?;
            let live = add(self.stats.live_bytes - requested, actual)?;
            self.entries[lease.slot].bytes = actual;
            self.per_tag[tag as usize] = tagged;
            self.stats.live_bytes = live;
            self.stats.peak_live_bytes = self.stats.peak_live_bytes.max(live);
            control.checkpoint().map_err(crate::types::stopped)?;
            Ok(values)
        })();
        match result {
            Ok(values) => Ok((values, lease)),
            Err(e) => {
                self.release(lease)?;
                if e.kind == ErrorKind::Limit(LimitKind::Arithmetic) {
                    self.stats.overflow = true;
                }
                Err(e)
            }
        }
    }
    pub(crate) fn empty_lease(&mut self, lease: AllocationLease) -> Result<()> {
        if lease.owner != self.owner {
            return Err(invalid(0));
        }
        let entry = self.entries.get_mut(lease.slot).ok_or_else(|| invalid(0))?;
        if !entry.active || entry.serial != lease.serial {
            return Err(invalid(0));
        }
        self.stats.live_bytes -= entry.bytes;
        self.per_tag[entry.tag as usize] -= entry.bytes;
        entry.bytes = 0;
        Ok(())
    }
}
impl ChargeLedger {
    pub(crate) fn require_no_leases(&self) -> Result<()> {
        if self.entries.iter().any(|e| e.active) {
            Err(invalid(0))
        } else {
            Ok(())
        }
    }
    pub(crate) fn finish_owned(self) -> RenderMemoryStats {
        let mut stats = self.stats();
        drop(self);
        stats.live_bytes = 0;
        stats.token_array_bytes = 0;
        stats.reference_index_bytes = 0;
        stats.clip_copy_bytes = 0;
        stats.output_bytes = 0;
        stats
    }
}

#[cfg(test)]
mod raster_reservation_arithmetic {
    use super::*;
    struct Run;
    impl WorkControl for Run {
        fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
            Ok(())
        }
        fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
            Ok(())
        }
    }
    #[test]
    fn exhausted_history_count_and_lease_serial_never_allocate_or_change_live_storage() {
        let source = std::sync::Arc::new(Vec::new());
        let mut ledger = ChargeLedger::new(&source, 1 << 20, 64).unwrap();
        let lease = ledger.admit(AllocationTag::Output, 0).unwrap();
        ledger.release(lease).unwrap();
        let live = ledger.stats.live_bytes;
        ledger.stats.total_allocated_bytes = usize::MAX;
        assert_eq!(
            ledger
                .allocate::<u8>(AllocationTag::Output, 1, &mut Run)
                .unwrap_err()
                .kind,
            ErrorKind::Limit(LimitKind::Arithmetic)
        );
        assert_eq!(ledger.stats.live_bytes, live);
        assert_eq!(ledger.stats.total_allocated_bytes, usize::MAX);
        assert_eq!(ledger.stats().output_bytes, 0);
        ledger.stats.total_allocated_bytes = 0;
        ledger.stats.allocation_count = usize::MAX;
        assert!(
            ledger
                .allocate::<u8>(AllocationTag::Output, 1, &mut Run)
                .is_err()
        );
        assert_eq!(ledger.stats.live_bytes, live);
        assert_eq!(ledger.stats.total_allocated_bytes, 0);
        ledger.stats.allocation_count = 0;
        ledger.next = u64::MAX;
        assert!(
            ledger
                .allocate::<u8>(AllocationTag::Output, 1, &mut Run)
                .is_err()
        );
        assert_eq!(ledger.stats.live_bytes, live);
        assert!(ledger.entries.iter().all(|entry| !entry.active));
    }

    #[test]
    fn cumulative_payload_history_overflow_is_atomic_except_overflow_flag() {
        let source = std::sync::Arc::new(vec![0]);
        let mut ledger = ChargeLedger::new(&source, usize::MAX, 8).unwrap();
        ledger.payload_history[AllocationTag::Output as usize] = usize::MAX - 3;
        let before_stats = ledger.stats;
        let before_tags = ledger.per_tag;
        let before_history = ledger.allocation_history();
        let error = ledger.admit(AllocationTag::Output, 4).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Arithmetic));
        assert!(ledger.stats.overflow);
        let mut after_stats = ledger.stats;
        after_stats.overflow = before_stats.overflow;
        assert_eq!(after_stats, before_stats);
        assert_eq!(ledger.per_tag, before_tags);
        assert_eq!(ledger.allocation_history(), before_history);
        assert_eq!(ledger.stats.live_bytes, source.capacity());
    }

    #[test]
    fn retained_registry_capacity_and_none_payload_have_exact_separate_deltas() {
        let source = std::sync::Arc::new(Vec::new());
        let mut ledger = ChargeLedger::new(&source, 1 << 20, 64).unwrap();
        let first = ledger.admit(AllocationTag::None, 7).unwrap();
        let old = bytes(&ledger.entries).unwrap();
        assert_eq!(ledger.payload_history[0], old + 7);
        // Fill the actual first capacity, rather than assuming Vec's growth size.
        while ledger.entries.len() < ledger.entries.capacity() {
            ledger.admit(AllocationTag::Output, 0).unwrap();
        }
        let before = ledger.stats;
        let history = ledger.allocation_history();
        let second = ledger.admit(AllocationTag::None, 11).unwrap();
        let new = bytes(&ledger.entries).unwrap();
        assert!(new > old);
        assert_eq!(ledger.payload_history[0] - history[0], new - old + 11);
        assert_eq!(ledger.stats.live_bytes - before.live_bytes, new - old + 11);
        assert_eq!(ledger.per_tag[0], new + 18);
        // Physical allocation history deliberately records the full replacement.
        assert_eq!(
            ledger.stats.total_allocated_bytes - before.total_allocated_bytes,
            new + 11
        );
        assert_eq!(ledger.stats.allocation_count - before.allocation_count, 2);
        let history = ledger.allocation_history();
        ledger.release(first).unwrap();
        ledger.release(second).unwrap();
        assert_eq!(ledger.stats.live_bytes, new);
        assert_eq!(ledger.per_tag[0], new);
        assert_eq!(ledger.allocation_history(), history);
        let before = ledger.stats;
        let reused = ledger.admit(AllocationTag::None, 13).unwrap();
        assert_eq!(reused.slot, second.slot);
        assert_eq!(bytes(&ledger.entries).unwrap(), new);
        assert_eq!(ledger.payload_history[0], history[0] + 13);
        assert_eq!(
            ledger.stats.total_allocated_bytes,
            before.total_allocated_bytes + 13
        );
        ledger.release(reused).unwrap();
        assert_eq!(ledger.stats.live_bytes, new);
        assert_eq!(ledger.payload_history[0], history[0] + 13);
    }

    #[test]
    fn registry_history_exhaustion_is_arithmetic_and_atomic_before_any_growth() {
        for second_growth in [false, true] {
            for tag in [AllocationTag::None, AllocationTag::Output] {
                let source = std::sync::Arc::new(Vec::new());
                let mut ledger = ChargeLedger::new(&source, 1 << 20, 64).unwrap();
                if second_growth {
                    ledger.admit(AllocationTag::Output, 3).unwrap();
                    while ledger.entries.len() < ledger.entries.capacity() {
                        ledger.admit(AllocationTag::Output, 0).unwrap();
                    }
                }
                ledger.payload_history[0] = usize::MAX - 7;
                let before = format!("{ledger:?}");
                let pointer = ledger.entries.as_ptr();
                let error = ledger.admit(tag, 7).unwrap_err();
                assert_eq!(error.kind, ErrorKind::Limit(LimitKind::Arithmetic));
                assert!(ledger.stats.overflow);
                ledger.stats.overflow = false;
                assert_eq!(format!("{ledger:?}"), before);
                assert_eq!(ledger.entries.as_ptr(), pointer);
                // An existing free slot requires no metadata history headroom.
                if second_growth {
                    let entry = ledger.entries[0];
                    ledger
                        .release(AllocationLease {
                            owner: ledger.owner,
                            slot: 0,
                            serial: entry.serial,
                        })
                        .unwrap();
                    ledger.payload_history[0] = usize::MAX;
                    let reused = ledger.admit(AllocationTag::None, 0).unwrap();
                    assert_eq!(reused.slot, 0);
                    assert_eq!(ledger.payload_history[0], usize::MAX);
                    ledger.release(reused).unwrap();
                }
            }
        }
    }

    #[test]
    fn registry_history_bound_admits_exact_headroom_and_preserves_error_priority() {
        let source = std::sync::Arc::new(Vec::new());
        let mut ledger = ChargeLedger::new(&source, 1 << 20, 64).unwrap();
        ledger.admit(AllocationTag::Output, 0).unwrap();
        while ledger.entries.len() < ledger.entries.capacity() {
            ledger.admit(AllocationTag::Output, 0).unwrap();
        }
        let old = bytes(&ledger.entries).unwrap();
        let amount = 7;
        let overlap = ledger.available() - amount;
        let size = std::mem::size_of::<Entry>();
        let max_delta = overlap / size * size - old;
        ledger.payload_history[0] = usize::MAX - max_delta - amount;
        let history = ledger.payload_history[0];
        ledger.admit(AllocationTag::None, amount).unwrap();
        let delta = bytes(&ledger.entries).unwrap() - old;
        assert_eq!(ledger.payload_history[0], history + delta + amount);
        assert!(!ledger.stats.overflow);
        // Existing lease-limit and insufficient physical overlap errors precede
        // the new registry-history proof, and cannot mutate the registry.
        for (budget, cap, expected) in [
            (1 << 20, 0, LimitKind::Nodes),
            (0, 64, LimitKind::ReservedBytes),
        ] {
            let mut ledger = ChargeLedger::new(&source, budget, cap).unwrap();
            ledger.payload_history[0] = usize::MAX;
            let before = format!("{ledger:?}");
            assert_eq!(
                ledger.admit(AllocationTag::Output, 0).unwrap_err().kind,
                ErrorKind::Limit(expected)
            );
            assert_eq!(format!("{ledger:?}"), before);
        }
    }
}
