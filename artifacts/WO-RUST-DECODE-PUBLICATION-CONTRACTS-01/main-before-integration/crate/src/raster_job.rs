//! Owning job allocations. Handles never own or expose a Vec or a lease.
use crate::memory::{arithmetic, bytes};
use crate::types::{invalid, limit, stopped};
use crate::*;
use std::marker::PhantomData;

/// Copy-only, allocation-free scalar records supported by the job arena.
/// Sealed: nested heap payloads cannot bypass accounting through an element.
pub trait JobElement: Copy + private::Element {}
#[derive(Debug, PartialEq, Eq)]
pub struct JobBuffer<T> {
    owner: u64,
    slot: usize,
    serial: u64,
    marker: PhantomData<T>,
}
impl<T> JobBuffer<T> {
    pub(crate) fn identity(self) -> (u64, usize, u64) {
        (self.owner, self.slot, self.serial)
    }
}
impl<T> Copy for JobBuffer<T> {}
impl<T> Clone for JobBuffer<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Reference caps apply to retained actual capacity, including nested grid data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterPool {
    Scratch,
    Dash,
    Grid,
    Boundary,
    Output,
    GroupScratch,
}
impl RasterPool {
    fn cap(self) -> usize {
        match self {
            Self::Scratch => usize::MAX,
            Self::GroupScratch => 64 * 1024 * 1024,
            Self::Dash => 16 * 1024 * 1024,
            Self::Grid => 16 * 1024 * 1024,
            Self::Boundary => usize::MAX, // original cap is marking WORK, not bits
            Self::Output => usize::MAX,
        }
    }
    fn error(self) -> Diagnostic {
        limit(match self {
            Self::Scratch | Self::GroupScratch => LimitKind::Scratch,
            Self::Dash => LimitKind::DashCache,
            Self::Grid => LimitKind::Grid,
            Self::Boundary => LimitKind::Scratch,
            Self::Output => LimitKind::ReservedBytes,
        })
    }
}
struct Slot {
    serial: u64,
    data: private::Storage,
    lease: AllocationLease,
    pool: RasterPool,
    cap: usize,
    active: bool,
    next_free: Option<usize>,
    boundary_work: usize,
}
/// The sole owner of every buffer. Moving/copying a handle never transfers data.
/// Drop destroys all data and the ledger; no caller release or Drop allocation.
pub struct RasterJob {
    ledger: ChargeLedger,
    slots: Vec<Slot>,
    metadata: Option<AllocationLease>,
    free: Option<usize>,
    next: u64,
    pools: [usize; 6],
    grid_capacities: [usize; 2],
}
/// A failed ownership transfer returns the unchanged ledger to its caller.
#[derive(Debug)]
pub struct JobStartError {
    pub diagnostic: Diagnostic,
    ledger: ChargeLedger,
}
impl JobStartError {
    pub fn into_ledger(self) -> ChargeLedger {
        self.ledger
    }
}
impl RasterJob {
    /// Move the SAME source/job ledger used by the coordinator into its owner.
    /// Rejects existing external leases: every live resource must be job-owned.
    pub fn new(ledger: ChargeLedger) -> std::result::Result<Self, JobStartError> {
        if let Err(diagnostic) = ledger.require_no_leases() {
            return Err(JobStartError { diagnostic, ledger });
        }
        Ok(Self {
            ledger,
            slots: Vec::new(),
            metadata: None,
            free: None,
            next: 1,
            pools: [0; 6],
            grid_capacities: [0; 2],
        })
    }
    pub(crate) fn resource_mark(&self) -> u64 {
        self.next
    }
    pub(crate) fn rollback_resources(&mut self, mark: u64) -> Result<()> {
        for slot in 0..self.slots.len() {
            if self.slots[slot].active && self.slots[slot].serial >= mark {
                self.release_slot(slot)?;
            }
        }
        Ok(())
    }
    fn release_slot(&mut self, slot: usize) -> Result<()> {
        let s = &mut self.slots[slot];
        let old = s.data.bytes()?;
        if s.pool == RasterPool::Grid {
            if let Some((kind, _)) = s.data.grid_kind() {
                self.grid_capacities[kind] -= old / s.data.element_size();
            }
        }
        s.data = private::Storage::Byte(Vec::new());
        self.ledger.release(s.lease)?;
        self.pools[s.pool as usize] -= old;
        s.active = false;
        s.next_free = self.free;
        self.free = Some(slot);
        Ok(())
    }
    /// Reclaim even resources whose descriptor handles the caller discarded.
    /// Invalidates all tokens, retaining metadata capacity/history for reuse.
    pub fn release_all(&mut self) -> Result<()> {
        self.rollback_resources(0)
    }
    /// Destroy every owned payload and both metadata registries. Returned live
    /// bytes are zero: this job's source charge also ends at this boundary.
    pub fn finish(mut self) -> Result<RenderMemoryStats> {
        self.rollback_resources(0)?;
        self.slots = Vec::new();
        if let Some(lease) = self.metadata.take() {
            self.ledger.release(lease)?;
        }
        Ok(self.ledger.finish_owned())
    }
    pub fn source_owner(&self) -> &SourceOwner {
        self.ledger.source_owner()
    }
    pub fn validate_source(&self, source: &SourceOwner) -> Result<()> {
        if std::sync::Arc::ptr_eq(source, self.source_owner()) {
            Ok(())
        } else {
            Err(invalid(0))
        }
    }
    pub fn stats(&self) -> RenderMemoryStats {
        self.ledger.stats()
    }
    pub fn work(&self) -> RenderWorkStats {
        self.ledger.work
    }
    pub fn charge_work(&mut self, kind: WorkKind, units: u64) -> Result<()> {
        self.ledger.work.charge(kind, units)
    }
    fn check_pool(&self, pool: RasterPool, old: usize, new: usize) -> Result<()> {
        if matches!(pool, RasterPool::Dash | RasterPool::Grid) {
            let total = self.pools[RasterPool::Dash as usize]
                .checked_add(self.pools[RasterPool::Grid as usize])
                .and_then(|n| n.checked_sub(old))
                .and_then(|n| n.checked_add(new))
                .ok_or_else(arithmetic)?;
            if total > 16 * 1024 * 1024 {
                return Err(limit(LimitKind::DashCache));
            }
        }
        Ok(())
    }
    fn check_grid(&self, grid: Option<(usize, usize)>, old: usize, new: usize) -> Result<()> {
        if let Some((kind, max)) = grid {
            let total = self.grid_capacities[kind]
                .checked_sub(old)
                .and_then(|n| n.checked_add(new))
                .ok_or_else(arithmetic)?;
            if total > max {
                return Err(limit(LimitKind::Grid));
            }
        }
        Ok(())
    }
    fn checked<T: JobElement>(&self, h: JobBuffer<T>) -> Result<&Slot> {
        if h.owner != self.ledger.identity() {
            return Err(invalid(0));
        }
        let s = self.slots.get(h.slot).ok_or_else(|| invalid(0))?;
        if !s.active || s.serial != h.serial || T::get(&s.data).is_none() {
            return Err(invalid(0));
        }
        Ok(s)
    }
    pub fn slice<T: JobElement>(&self, h: JobBuffer<T>) -> Result<&[T]> {
        Ok(T::get(&self.checked(h)?.data).ok_or_else(|| invalid(0))?)
    }
    /// Mutable elements only; capacity, ownership and logical length stay private.
    pub fn slice_mut<T: JobElement>(&mut self, h: JobBuffer<T>) -> Result<&mut [T]> {
        self.checked(h)?;
        Ok(T::get_mut(&mut self.slots[h.slot].data).ok_or_else(|| invalid(0))?)
    }
    pub fn capacity<T: JobElement>(&self, h: JobBuffer<T>) -> Result<usize> {
        Ok(T::get(&self.checked(h)?.data)
            .ok_or_else(|| invalid(0))?
            .capacity())
    }
    fn metadata(&mut self, control: &mut dyn WorkControl) -> Result<()> {
        if self.free.is_some() || self.slots.len() < self.slots.capacity() {
            return Ok(());
        }
        let target = self.slots.len().checked_add(1).ok_or_else(arithmetic)?;
        let grown = self
            .slots
            .capacity()
            .checked_mul(2)
            .unwrap_or(target)
            .max(target)
            .max(8);
        let result = self.ledger.allocate::<Slot>(AllocationTag::None, grown, control);
        let (mut replacement, lease) = match result {
            Err(e) if e.kind == ErrorKind::Limit(LimitKind::ReservedBytes) && grown != target => self
                .ledger
                .allocate::<Slot>(AllocationTag::None, target, control)?,
            other => other?,
        };
        // All fallible work precedes the indivisible move. No cloned source/payloads.
        replacement.append(&mut self.slots);
        self.slots = replacement;
        if let Some(old) = self.metadata.replace(lease) {
            self.ledger.release(old)?;
        }
        Ok(())
    }
    /// Create a buffer with an explicit engine element cap (no smaller default).
    pub fn buffer<T: JobElement>(
        &mut self,
        pool: RasterPool,
        cap: usize,
        control: &mut dyn WorkControl,
    ) -> Result<JobBuffer<T>> {
        control.checkpoint().map_err(stopped)?;
        let data = T::wrap(Vec::new());
        if !data.accepts(pool) {
            return Err(invalid(0));
        }
        let cap = if pool == RasterPool::Grid {
            cap.min(data.grid_kind().ok_or_else(|| invalid(0))?.1)
        } else {
            cap
        };
        let serial = self.next;
        let next = serial.checked_add(1).ok_or_else(arithmetic)?;
        let cap = if pool == RasterPool::Output {
            cap.min(vexel_raster::MAX_ALLOCATED_BYTES / std::mem::size_of::<T>())
        } else {
            cap
        };
        self.metadata(control)?;
        let lease = self.ledger.admit(AllocationTag::None, 0)?;
        if let Err(e) = control.checkpoint().map_err(stopped) {
            self.ledger.release(lease)?;
            return Err(e);
        }
        let slot = self.free.unwrap_or(self.slots.len());
        let entry = Slot {
            serial,
            data,
            lease,
            pool,
            cap,
            active: true,
            next_free: None,
            boundary_work: 0,
        };
        if slot == self.slots.len() {
            self.slots.push(entry);
        } else {
            self.free = self.slots[slot].next_free;
            self.slots[slot] = entry;
        }
        self.next = next;
        Ok(JobBuffer {
            owner: self.ledger.identity(),
            slot,
            serial,
            marker: PhantomData,
        })
    }
    /// Replacement reserves old+new overlap BEFORE allocating. All copy loops
    /// poll control; publication follows the final checkpoint. Failure preserves
    /// old contents, capacity, token AND lease (physical history is retained).
    pub fn reserve<T: JobElement>(
        &mut self,
        h: JobBuffer<T>,
        capacity: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.write(h, capacity, &[], false, control)
    }
    pub fn extend<T: JobElement>(
        &mut self,
        h: JobBuffer<T>,
        values: &[T],
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let target = self
            .slice(h)?
            .len()
            .checked_add(values.len())
            .ok_or_else(arithmetic)?;
        self.write(h, target, values, false, control)
    }
    /// Physical shrink uses a fallible replacement and accounts old/new overlap.
    /// The caller chooses the retained capacity; values are never truncated.
    pub fn shrink_to<T: JobElement>(
        &mut self,
        h: JobBuffer<T>,
        capacity: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let old = self.capacity(h)?;
        if capacity < self.slice(h)?.len() {
            return Err(invalid(0));
        }
        if capacity >= old {
            return control.checkpoint().map_err(stopped);
        }
        if capacity == 0 {
            return self.clear_capacity(h, control);
        }
        self.write(h, capacity, &[], true, control)
    }
    fn write<T: JobElement>(
        &mut self,
        h: JobBuffer<T>,
        capacity: usize,
        values: &[T],
        shrink: bool,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let s = self.checked(h)?;
        control.checkpoint().map_err(stopped)?;
        let old = T::get(&s.data).ok_or_else(|| invalid(0))?;
        if capacity > s.cap {
            return Err(s.pool.error());
        }
        if capacity <= old.capacity() && !shrink {
            let dst = T::get_mut(&mut self.slots[h.slot].data).ok_or_else(|| invalid(0))?;
            let len = dst.len();
            let result = Self::copy(dst, values, control).and_then(|_| control.checkpoint().map_err(stopped));
            if result.is_err() {
                dst.truncate(len);
            }
            return result;
        }
        let grid = if s.pool == RasterPool::Grid {
            s.data.grid_kind()
        } else {
            None
        };
        let old_elements = old.capacity();
        self.check_grid(grid, old_elements, capacity)?;
        let old_bytes = bytes(old)?;
        let request = capacity
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(arithmetic)?;
        let pool = s.pool;
        let cap = s.cap;
        self.check_pool(pool, old_bytes, request)?;
        let retained = self.pools[pool as usize]
            .checked_sub(old_bytes)
            .and_then(|n| n.checked_add(request))
            .ok_or_else(arithmetic)?;
        if retained > pool.cap() {
            return Err(pool.error());
        }
        let tag = if pool == RasterPool::Output {
            AllocationTag::Output
        } else {
            AllocationTag::ReferenceIndex
        };
        let (mut replacement, lease) = self.ledger.allocate::<T>(tag, capacity, control)?;
        let actual = bytes(&replacement)?;
        let result = (|| {
            if replacement.capacity() > cap {
                return Err(pool.error());
            }
            self.check_grid(grid, old_elements, replacement.capacity())?;
            self.check_pool(pool, old_bytes, actual)?;
            let retained = (self.pools[pool as usize] - old_bytes)
                .checked_add(actual)
                .ok_or_else(arithmetic)?;
            if retained > pool.cap() {
                return Err(pool.error());
            }
            let old = T::get(&self.slots[h.slot].data).ok_or_else(|| invalid(0))?;
            Self::copy(&mut replacement, old, control)?;
            Self::copy(&mut replacement, values, control)?;
            control.checkpoint().map_err(stopped)?;
            Ok(retained)
        })();
        let retained = match result {
            Ok(n) => n,
            Err(e) => {
                drop(replacement);
                self.ledger.release(lease)?;
                return Err(e);
            }
        };
        if let Some((kind, _)) = grid {
            self.grid_capacities[kind] = self.grid_capacities[kind] - old_elements + replacement.capacity();
        }
        let s = &mut self.slots[h.slot];
        s.data = T::wrap(replacement);
        self.ledger.release(std::mem::replace(&mut s.lease, lease))?;
        self.pools[pool as usize] = retained;
        Ok(())
    }
    fn copy<T: JobElement>(dst: &mut Vec<T>, values: &[T], control: &mut dyn WorkControl) -> Result<()> {
        for value in values {
            control.charge(WorkKind::SourceRead, 1).map_err(stopped)?;
            dst.push(*value);
        }
        Ok(())
    }
    /// Original 200M cap belongs to each boundary-mask lifetime. The counter
    /// lives in the private slot, so copying its descriptor cannot reset it.
    pub fn boundary_work(&self, h: JobBuffer<u8>) -> Result<usize> {
        self.require_pool(h, RasterPool::Boundary)?;
        Ok(self.slots[h.slot].boundary_work)
    }
    /// Call inside actual marking loops with their original work amount.
    pub fn charge_boundary(
        &mut self,
        h: JobBuffer<u8>,
        units: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let total = self.boundary_work(h)?.checked_add(units).ok_or_else(arithmetic)?;
        if total > RASTER_BOUNDARY_WORK_LIMIT {
            return Err(limit(LimitKind::Work));
        }
        let amount = u64::try_from(units).map_err(|_| arithmetic())?;
        let mut work = self.ledger.work;
        work.charge(WorkKind::BoundaryMark, amount)?;
        control.charge(WorkKind::BoundaryMark, amount).map_err(stopped)?;
        self.slots[h.slot].boundary_work = total;
        self.ledger.work = work;
        Ok(())
    }
    pub(crate) fn require_pool<T: JobElement>(&self, h: JobBuffer<T>, pool: RasterPool) -> Result<()> {
        if self.checked(h)?.pool == pool {
            Ok(())
        } else {
            Err(invalid(0))
        }
    }
    pub fn truncate<T: JobElement>(&mut self, h: JobBuffer<T>, len: usize) -> Result<()> {
        self.checked(h)?;
        T::get_mut(&mut self.slots[h.slot].data)
            .ok_or_else(|| invalid(0))?
            .truncate(len);
        Ok(())
    }
    /// Release storage but retain the same live handle for reuse.
    pub fn clear_capacity<T: JobElement>(
        &mut self,
        h: JobBuffer<T>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.checked(h)?;
        control.checkpoint().map_err(stopped)?;
        let s = &mut self.slots[h.slot];
        let old = bytes(T::get(&s.data).ok_or_else(|| invalid(0))?)?;
        if s.pool == RasterPool::Grid {
            if let Some((kind, _)) = s.data.grid_kind() {
                self.grid_capacities[kind] -= old / s.data.element_size();
            }
        }
        s.data = T::wrap(Vec::new());
        self.ledger.empty_lease(s.lease)?;
        self.pools[s.pool as usize] -= old;
        Ok(())
    }
    /// Invalidates all copies of the token. Cleanup remains available after stop.
    pub fn release<T: JobElement>(&mut self, h: JobBuffer<T>) -> Result<()> {
        self.checked(h)?;
        self.release_slot(h.slot)
    }
}

mod private {
    use super::*;
    pub trait Element: Copy {
        fn get(s: &Storage) -> Option<&Vec<Self>>;
        fn get_mut(s: &mut Storage) -> Option<&mut Vec<Self>>;
        fn wrap(v: Vec<Self>) -> Storage;
    }
    macro_rules! elements {
        ($($variant:ident : $ty:ty),* $(,)?) => {
            pub enum Storage { $($variant(Vec<$ty>)),* }
            impl Storage {
                pub fn accepts(&self, pool: RasterPool) -> bool {
                    match pool {
                        RasterPool::Scratch => true,
                        RasterPool::Dash => matches!(self, Self::Segment(_) | Self::Join(_) | Self::DashRecord(_)),
                        RasterPool::Grid => self.grid_kind().is_some(),
                        _ => matches!(self, Self::Byte(_)),
                    }
                }
                pub fn grid_kind(&self) -> Option<(usize, usize)> {
                    match self { Self::Cell(_) => Some((0, 65536)),
                        Self::EdgeIndex(_) => Some((1, 1_000_000)), _ => None }
                }
                pub fn element_size(&self) -> usize {
                    match self { $(Self::$variant(_) => std::mem::size_of::<$ty>()),* }
                }
                pub fn bytes(&self) -> Result<usize> {
                    match self { $(Self::$variant(v) => bytes(v)),* }
                }
            }
            $(impl Element for $ty {
                fn get(s: &Storage) -> Option<&Vec<Self>> {
                    if let Storage::$variant(v) = s { Some(v) } else { None }
                }
                fn get_mut(s: &mut Storage) -> Option<&mut Vec<Self>> {
                    if let Storage::$variant(v) = s { Some(v) } else { None }
                }
                fn wrap(v: Vec<Self>) -> Storage { Storage::$variant(v) }
            }
            impl JobElement for $ty {})*
        }
    }
    elements! {
        Byte: u8, Flag: bool, Index: usize, EdgeIndex: u32, Root: i32,
        Geometry: Edge, Bounds: ShapeBounds, Event: ScanlineEvent,
        ShapeEvent: ShapeRowEvent, Node: ScanlineNode, Crossing: FillCrossing,
        Interval: FillInterval, Run: PixelRun, Segment: DashSegment,
        Join: DashJoin, Cell: GridCell, DashRecord: JobDashRange,
    }
}

#[cfg(test)]
mod arithmetic_proofs {
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
    fn exhausted_token_serial_and_corrupted_token_reject_without_mutation() {
        let source = std::sync::Arc::new(Vec::new());
        let mut job = RasterJob::new(ChargeLedger::new(&source, 1 << 20, 64).unwrap()).unwrap();
        let h = job.buffer::<u8>(RasterPool::Output, 10, &mut Run).unwrap();
        job.extend(h, &[3], &mut Run).unwrap();
        let before = job.stats();
        job.next = u64::MAX;
        assert_eq!(
            job.buffer::<u8>(RasterPool::Output, 10, &mut Run)
                .unwrap_err()
                .kind,
            ErrorKind::Limit(LimitKind::Arithmetic)
        );
        assert_eq!(job.stats(), before);
        let wrong = JobBuffer {
            slot: usize::MAX,
            ..h
        };
        assert!(job.release(wrong).is_err());
        assert_eq!(job.slice(h).unwrap(), &[3]);
        assert_eq!(job.stats(), before);
    }
}
