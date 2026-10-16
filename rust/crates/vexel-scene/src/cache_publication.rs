//! Logical generations share one physical ledger. No heap transaction registry.
use crate::{
    types::{invalid, stopped},
    *,
};

/// Same-job publication token. Copies become stale together when replaced.
#[derive(Debug, Clone, Copy)]
pub struct CacheGeneration {
    cache: JobDashCache,
    scope: u64,
}
/// Exclusive construction borrow; dropping or aborting retires only staged data.
/// No job/Vec/lease escape. Handles remain bound to their original generation.
pub struct CacheTransaction<'a> {
    job: &'a mut RasterJob,
    scope: u64,
    old: Option<CacheGeneration>,
    committed: bool,
}
impl RasterJob {
    pub(crate) fn cache_scope(&self, cache: JobDashCache, staged: bool) -> Result<u64> {
        self.require_pool(cache.segments, RasterPool::Dash)?;
        self.require_pool(cache.joins, RasterPool::Dash)?;
        self.require_pool(cache.shapes, RasterPool::Dash)?;
        let scope = self.checked(cache.segments)?.scope;
        for s in [
            self.checked(cache.segments)?,
            self.checked(cache.joins)?,
            self.checked(cache.shapes)?,
        ] {
            if s.scope != scope || (!staged && !s.published) {
                return Err(invalid(0));
            }
        }
        Ok(scope)
    }
    fn generation(&self, generation: CacheGeneration) -> Result<()> {
        if generation.scope == 0 || self.cache_scope(generation.cache, false)? != generation.scope {
            return Err(invalid(0));
        }
        Ok(())
    }
    /// Starts an explicit per-cache 16MiB/65536-cell/1M-reference generation.
    /// Physical old/new overlap, registry growth and history use the caller ledger.
    pub fn begin_cache<'a>(
        &'a mut self,
        source: &SourceOwner,
        old: Option<CacheGeneration>,
        control: &mut dyn WorkControl,
    ) -> Result<CacheTransaction<'a>> {
        self.validate_source(source)?;
        if self.cache_transaction.is_some() {
            return Err(invalid(0));
        }
        if let Some(old) = old {
            self.cache_view(old, control)?;
        }
        control.checkpoint().map_err(stopped)?;
        let scope = self.next;
        self.next = scope.checked_add(1).ok_or_else(crate::memory::arithmetic)?;
        self.cache_transaction = Some(scope);
        Ok(CacheTransaction {
            job: self,
            scope,
            old,
            committed: false,
        })
    }
    /// Descriptor compatibility for existing lower readers/composition. Each use
    /// still validates liveness, roles and generation; this transfers no ownership.
    pub fn cache_descriptor(&self, generation: CacheGeneration) -> Result<JobDashCache> {
        self.generation(generation)?;
        Ok(generation.cache)
    }
    pub fn cache_view(
        &self,
        generation: CacheGeneration,
        control: &mut dyn WorkControl,
    ) -> Result<JobDashView<'_>> {
        self.dash_view(self.cache_descriptor(generation)?, control)
    }
    pub(crate) fn release_scope(&mut self, scope: u64) -> Result<()> {
        for i in 0..self.slots.len() {
            if self.slots[i].active && self.slots[i].scope == scope {
                self.release_slot(i)?;
            }
        }
        Ok(())
    }
}
impl CacheTransaction<'_> {
    fn require<T: JobElement>(&self, h: JobBuffer<T>) -> Result<()> {
        if self.job.checked(h)?.scope != self.scope {
            return Err(invalid(0));
        }
        Ok(())
    }
    /// Uses the actual typed job arena; only original Dash/Grid roles are legal.
    pub fn buffer<T: JobElement>(
        &mut self,
        pool: RasterPool,
        cap: usize,
        control: &mut dyn WorkControl,
    ) -> Result<JobBuffer<T>> {
        if !matches!(pool, RasterPool::Dash | RasterPool::Grid) {
            return Err(invalid(0));
        }
        self.job.scoped_buffer(pool, cap, self.scope, control)
    }
    pub fn reserve<T: JobElement>(
        &mut self,
        h: JobBuffer<T>,
        capacity: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.require(h)?;
        self.job.reserve(h, capacity, control)
    }
    pub fn extend<T: JobElement>(
        &mut self,
        h: JobBuffer<T>,
        values: &[T],
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.require(h)?;
        self.job.extend(h, values, control)
    }
    pub fn slice<T: JobElement>(&self, h: JobBuffer<T>) -> Result<&[T]> {
        self.require(h)?;
        self.job.slice(h)
    }
    pub fn slice_mut<T: JobElement>(&mut self, h: JobBuffer<T>) -> Result<&mut [T]> {
        self.require(h)?;
        self.job.slice_mut(h)
    }
    pub fn capacity<T: JobElement>(&self, h: JobBuffer<T>) -> Result<usize> {
        self.require(h)?;
        self.job.capacity(h)
    }
    pub fn truncate<T: JobElement>(&mut self, h: JobBuffer<T>, len: usize) -> Result<()> {
        self.require(h)?;
        self.job.truncate(h, len)
    }
    pub fn shrink_to<T: JobElement>(
        &mut self,
        h: JobBuffer<T>,
        capacity: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.require(h)?;
        self.job.shrink_to(h, capacity, control)
    }
    pub fn clear_capacity<T: JobElement>(
        &mut self,
        h: JobBuffer<T>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.require(h)?;
        self.job.clear_capacity(h, control)
    }
    pub fn release<T: JobElement>(&mut self, h: JobBuffer<T>) -> Result<()> {
        self.require(h)?;
        self.job.release(h)
    }
    pub fn stats(&self) -> RenderMemoryStats {
        self.job.stats()
    }
    /// Validate actual payload and same-scope grids before retiring any old slot.
    /// Consuming self makes duplicate commit impossible; copied descriptors confer
    /// no publication authority. Validation failure invokes unconditional cleanup.
    /// ```compile_fail,E0382
    /// use vexel_scene::*;
    /// fn replay(tx: CacheTransaction<'_>, cache: JobDashCache, c: &mut dyn WorkControl) {
    ///     let _ = tx.commit(cache, c);
    ///     let _ = tx.commit(cache, c); // publication authority was consumed
    /// }
    /// ```
    pub fn commit(
        mut self,
        cache: JobDashCache,
        control: &mut dyn WorkControl,
    ) -> Result<CacheGeneration> {
        if self.job.cache_scope(cache, true)? != self.scope {
            return Err(invalid(0));
        }
        self.job.dash_view_staged(cache, control)?;
        if let Some(old) = self.old {
            self.job.generation(old)?;
        }
        control.checkpoint().map_err(stopped)?;
        // No allocation/cancellation/fallible validation after this boundary.
        if let Some(old) = self.old {
            self.job.release_scope(old.scope)?;
        }
        for s in &mut self.job.slots {
            if s.active && s.scope == self.scope {
                s.published = true;
            }
        }
        self.job.cache_transaction = None;
        self.committed = true;
        Ok(CacheGeneration {
            cache,
            scope: self.scope,
        })
    }
    pub fn abort(self) {
        drop(self);
    }
}
impl Drop for CacheTransaction<'_> {
    fn drop(&mut self) {
        if !self.committed {
            // Private leases cannot be corrupted by callers. If cleanup ever
            // fails, unpublished slots remain and finish refuses success.
            if self.job.release_scope(self.scope).is_ok() {
                self.job.cache_transaction = None;
            }
        }
    }
}
