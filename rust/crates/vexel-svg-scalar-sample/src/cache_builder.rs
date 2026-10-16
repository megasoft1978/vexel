//! Caller-owned publication metadata. No global descriptor registry.
use vexel_geometry::Edge;
use vexel_scene::*;

/// Keep this companion with its scratch descriptor for the job's lifetime.
/// Copies retain token history; replacement makes those copies stale.
#[derive(Debug, Clone, Copy)]
pub struct CacheState {
    scratch: JobRasterScratch,
    generation: Option<CacheGeneration>,
    /// Actual caller-ledger statistics immediately before successful publication.
    pub staged_stats: RenderMemoryStats,
}
fn invalid() -> Diagnostic {
    Diagnostic { kind: ErrorKind::InvalidArgument, offset: 0 }
}
impl CacheState {
    /// Create and own the initial empty scratch descriptor in this job.
    /// Callers cannot substitute another descriptor or publication token.
    pub fn new(job: &mut RasterJob, c: &mut dyn WorkControl) -> Result<Self> {
        let scratch = JobRasterScratch::new(job, c)?;
        Ok(Self { scratch, generation: None, staged_stats: job.stats() })
    }
    /// Copy-only handles for lower readers; descriptor mutation cannot affect this owner.
    pub fn scratch(&self) -> JobRasterScratch { self.scratch }
    pub fn generation(&self) -> Option<CacheGeneration> { self.generation }
}
/// Production builder: independent logical generation, physical caller ledger.
/// Every failure leaves state, old payload and scratch descriptor usable.
pub fn rebuild_dash_cache(
    view: SceneView<'_>, local_edges: &[Edge], seams: &[bool], job: &mut RasterJob,
    state: &mut CacheState, c: &mut dyn WorkControl,
) -> Result<JobDashCache> {
    job.validate_source(view.source_owner())?;
    if let Some(old) = state.generation {
        job.cache_descriptor(old)?;
    } else {
        // Recheck liveness and emptiness before staging. This does not retire it.
        let v = job.dash_view(state.scratch.dash, c)?;
        if !v.shapes.is_empty() || !v.segments.is_empty() || !v.joins.is_empty() {
            return Err(invalid());
        }
    }
    let mut tx = job.begin_cache(view.source_owner(), state.generation, c)?;
    let cache = crate::job_dash::build_lower(view, local_edges, seams, &mut tx, c)?;
    let stats = tx.stats();
    let generation = tx.commit(cache, c)?;
    let descriptor = job.cache_descriptor(generation)?;
    if state.generation.is_none() {
        // Only the validated initial empty unscoped handles need manual cleanup;
        // replacement of published generations belongs exclusively to commit.
        crate::job_dash::release_cache(job, state.scratch.dash)?;
    }
    state.scratch.dash = descriptor;
    state.generation = Some(generation);
    state.staged_stats = stats;
    Ok(descriptor)
}

/// Explicit legacy compatibility entrypoint using original unscoped pool policy.
/// Production callers must use rebuild_dash_cache and retain CacheState.
pub fn build_dash_cache(
    view: SceneView<'_>, local_edges: &[Edge], seams: &[bool], job: &mut RasterJob,
    scratch: &mut JobRasterScratch, c: &mut dyn WorkControl,
) -> Result<JobDashCache> {
    job.validate_source(view.source_owner())?;
    let cache = crate::job_dash::build_lower(view, local_edges, seams, job, c)?;
    crate::job_dash::release_cache(job, scratch.dash)?;
    scratch.dash = cache;
    Ok(cache)
}
