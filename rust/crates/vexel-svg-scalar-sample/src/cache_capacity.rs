//! Logical retained cache payload, separate from the job ledger's physical
//! old/new overlap and metadata history. One instance per staged cache.
use crate::DASH_CACHE_BYTE_LIMIT;
use vexel_scene::{
    Diagnostic, ErrorKind, JobBuffer, JobElement, LimitKind, Result, WorkControl,
};

pub(crate) fn limit() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(LimitKind::DashCache),
        offset: 0,
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct CacheCapacity {
    used: usize,
}
impl CacheCapacity {
    fn replacing<T>(&self, old: usize, new: usize) -> Result<usize> {
        let size = std::mem::size_of::<T>();
        self.used
            .checked_sub(old.checked_mul(size).ok_or_else(limit)?)
            .and_then(|n| n.checked_add(new.checked_mul(size)?))
            .filter(|&n| n <= DASH_CACHE_BYTE_LIMIT)
            .ok_or_else(limit)
    }
    pub(crate) fn admits<T>(&self, count: usize) -> bool {
        self.replacing::<T>(0, count).is_ok()
    }
    pub(crate) fn reserve<T: JobElement>(
        &mut self,
        job: &mut impl crate::cache_storage::Storage,
        handle: JobBuffer<T>,
        requested: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let old = job.capacity(handle)?;
        if requested <= old {
            return Ok(());
        }
        // Predict before allocation; then verify the allocator's actual capacity.
        self.replacing::<T>(old, requested)?;
        job.reserve(handle, requested, control)?;
        self.used = self.replacing::<T>(old, job.capacity(handle)?)?;
        Ok(())
    }
}
