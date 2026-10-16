//! Safe repeated cache reads with disjoint, already-admitted same-job writes.
//! No temporary cache, metadata allocation, detach/restore or panic cleanup path.
use crate::{
    raster_job::Slot,
    types::{invalid, stopped},
    *,
};
pub(crate) trait CacheRead {
    fn cells(&self, h: JobBuffer<GridCell>) -> Result<&[GridCell]>;
    fn references(&self, h: JobBuffer<u32>) -> Result<&[u32]>;
}
impl CacheRead for RasterJob {
    fn cells(&self, h: JobBuffer<GridCell>) -> Result<&[GridCell]> {
        self.require_pool(h, RasterPool::Grid)?;
        self.slice(h)
    }
    fn references(&self, h: JobBuffer<u32>) -> Result<&[u32]> {
        self.require_pool(h, RasterPool::Grid)?;
        self.slice(h)
    }
}
struct Split<'a> {
    owner: u64,
    head: &'a [Slot],
    middle: &'a [Slot],
    tail: &'a [Slot],
    a: usize,
    b: usize,
}
impl Split<'_> {
    fn slice<T: JobElement>(&self, h: JobBuffer<T>, pool: RasterPool) -> Result<&[T]> {
        let (owner, slot, serial) = h.identity();
        if owner != self.owner || slot == self.a || slot == self.b {
            return Err(invalid(0));
        }
        let s = if slot < self.a {
            self.head.get(slot)
        } else if slot < self.b {
            self.middle.get(slot - self.a - 1)
        } else {
            self.tail.get(slot - self.b - 1)
        }
        .ok_or_else(|| invalid(0))?;
        if !s.active || s.serial != serial || s.pool != pool {
            return Err(invalid(0));
        }
        Ok(T::get(&s.data).ok_or_else(|| invalid(0))?)
    }
}
impl CacheRead for Split<'_> {
    fn cells(&self, h: JobBuffer<GridCell>) -> Result<&[GridCell]> {
        self.slice(h, RasterPool::Grid)
    }
    fn references(&self, h: JobBuffer<u32>) -> Result<&[u32]> {
        self.slice(h, RasterPool::Grid)
    }
}
/// Exclusive job borrow prevents cache mutation, release, finish or teardown.
/// Writes are elements of preallocated scratch/boundary/group/output buffers.
/// Partial writes on engine error/cancel/panic are retained honestly; the owner
/// may clear/reuse/discard them after the scope. No successful output is implied.
pub struct RasterComposition<'a> {
    job: &'a mut RasterJob,
    cache: JobDashCache,
}
impl RasterJob {
    /// Cache release while the scope remains live is rejected by Rust ownership.
    /// ```compile_fail,E0499
    /// use vexel_scene::*;
    /// struct Run;
    /// impl WorkControl for Run {
    ///     fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> { Ok(()) }
    ///     fn checkpoint(&mut self) -> std::result::Result<(), Stop> { Ok(()) }
    /// }
    /// let source = std::sync::Arc::new(Vec::new());
    /// let mut job = RasterJob::new(ChargeLedger::new(&source, 1 << 20, 128).unwrap()).unwrap();
    /// let scratch = JobRasterScratch::new(&mut job, &mut Run).unwrap();
    /// let scope = job.compose(&source, scratch.dash, &mut Run).unwrap();
    /// job.release_all().unwrap();
    /// scope.stats();
    /// ```
    pub fn compose<'a>(
        &'a mut self,
        source: &SourceOwner,
        cache: JobDashCache,
        control: &mut dyn WorkControl,
    ) -> Result<RasterComposition<'a>> {
        self.validate_source(source)?;
        if cache.build_steps > 1_000_000 {
            return Err(crate::types::limit(LimitKind::DashCache));
        }
        let (source, owner, work, slots) = self.composition_parts();
        let reader = Split {
            owner,
            head: slots,
            middle: &[],
            tail: &[],
            a: usize::MAX,
            b: usize::MAX,
        };
        let view = cache_view(&reader, source, cache)?;
        let mut meter = JobWorkMeter {
            work,
            inner: control,
        };
        meter.checkpoint().map_err(stopped)?;
        view.validate(&mut meter)?; // exactly once, on the same job ledger
        Ok(RasterComposition { job: self, cache })
    }
}
impl RasterComposition<'_> {
    pub fn stats(&self) -> RenderMemoryStats {
        self.job.stats()
    }
    pub fn work(&self) -> RenderWorkStats {
        self.job.work()
    }
    pub fn charge_work(
        &mut self,
        kind: WorkKind,
        units: u64,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let (_, _, work, _) = self.job.composition_parts();
        JobWorkMeter {
            work,
            inner: control,
        }
        .charge(kind, units)
        .map_err(stopped)
    }
    /// Supply two distinct mutable element slices plus the validated cache. All
    /// pools/identities/aliases are checked before borrowing or calling the engine.
    /// HRTB callback cannot return a cache/write borrow or unchecked lease/Vec.
    /// Borrowed writes cannot escape the callback.
    /// ```compile_fail
    /// use vexel_scene::*;
    /// fn escape<'a>(scope: &mut RasterComposition<'a>, a: JobBuffer<u8>,
    ///     b: JobBuffer<u32>, c: &mut dyn WorkControl) -> &'a mut [u8] {
    ///     scope.with_buffers(a, b, c, |_, values, _, _| Ok(values)).unwrap()
    /// }
    /// ```
    pub fn with_buffers<A: JobElement, B: JobElement, R>(
        &mut self,
        a: JobBuffer<A>,
        b: JobBuffer<B>,
        control: &mut dyn WorkControl,
        engine: impl FnOnce(
            ValidatedDashView<'_>,
            &mut [A],
            &mut [B],
            &mut dyn WorkControl,
        ) -> Result<R>,
    ) -> Result<R> {
        self.job.slice(a)?;
        self.job.slice(b)?;
        let (_, ai, _) = a.identity();
        let (_, bi, _) = b.identity();
        if ai == bi {
            return Err(invalid(0));
        }
        for slot in [ai, bi] {
            if matches!(
                self.job.slots[slot].pool,
                RasterPool::Dash | RasterPool::Grid
            ) {
                return Err(invalid(0));
            }
        }
        control.checkpoint().map_err(stopped)?;
        let (source, owner, work, slots) = self.job.composition_parts();
        let (low, high) = (ai.min(bi), ai.max(bi));
        let (head, rest) = slots.split_at_mut(low);
        let (left, rest) = rest.split_first_mut().ok_or_else(|| invalid(0))?;
        let (middle, rest) = rest.split_at_mut(high - low - 1);
        let (right, tail) = rest.split_first_mut().ok_or_else(|| invalid(0))?;
        let reader = Split {
            owner,
            head,
            middle,
            tail,
            a: low,
            b: high,
        };
        let view = cache_view(&reader, source, self.cache)?;
        let mut meter = JobWorkMeter {
            work,
            inner: control,
        };
        let (a_slot, b_slot) = if ai < bi {
            (left, right)
        } else {
            (right, left)
        };
        engine(
            ValidatedDashView { inner: view },
            A::get_mut(&mut a_slot.data).ok_or_else(|| invalid(0))?,
            B::get_mut(&mut b_slot.data).ok_or_else(|| invalid(0))?,
            &mut meter,
        )
    }
}

fn cache_view<'a>(
    reader: &'a Split<'a>,
    source: &'a SourceOwner,
    cache: JobDashCache,
) -> Result<JobDashView<'a>> {
    Ok(JobDashView {
        reader,
        source,
        segments: reader.slice(cache.segments, RasterPool::Dash)?,
        joins: reader.slice(cache.joins, RasterPool::Dash)?,
        shapes: reader.slice(cache.shapes, RasterPool::Dash)?,
    })
}
struct JobWorkMeter<'a> {
    work: &'a mut RenderWorkStats,
    inner: &'a mut dyn WorkControl,
}
impl WorkControl for JobWorkMeter<'_> {
    fn charge(&mut self, kind: WorkKind, units: u64) -> std::result::Result<(), Stop> {
        self.work.charge(kind, units).map_err(|_| Stop::Overflow)?;
        self.inner.charge(kind, units)
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        self.inner.checkpoint()
    }
}

/// Immutable validated input for the production composition scope. The legacy
/// JobDashView's replaceable public slice fields remain explicitly compatible.
#[derive(Clone, Copy)]
pub struct ValidatedDashView<'a> {
    inner: JobDashView<'a>,
}
impl<'a> ValidatedDashView<'a> {
    pub fn segments(self) -> &'a [DashSegment] {
        self.inner.segments
    }
    pub fn joins(self) -> &'a [DashJoin] {
        self.inner.joins
    }
    pub fn shapes(self) -> &'a [JobDashRange] {
        self.inner.shapes
    }
    pub fn grid_at(self, index: usize) -> Result<(&'a JobDashRange, &'a [GridCell], &'a [u32])> {
        self.inner.grid_at(index)
    }
}
