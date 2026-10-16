//! Allocation-free descriptors for the complete original scratch layout.
use crate::*;

#[derive(Debug, Clone, Copy)]
pub struct JobRasterScratch {
    pub bounds: JobBuffer<ShapeBounds>,
    pub boundary_bits: JobBuffer<u8>,
    pub scanline_events: JobBuffer<ScanlineEvent>,
    pub scanline_nodes: JobBuffer<ScanlineNode>,
    pub scanline_roots: JobBuffer<i32>,
    pub shape_events: JobBuffer<ShapeRowEvent>,
    pub shape_nodes: JobBuffer<ScanlineNode>,
    pub shape_offsets: JobBuffer<usize>,
    pub active_edges: JobBuffer<u32>,
    pub active_shapes: JobBuffer<u32>,
    pub eligible_shapes: JobBuffer<bool>,
    pub crossings: JobBuffer<FillCrossing>,
    pub intervals: JobBuffer<FillInterval>,
    pub pixel_runs: JobBuffer<PixelRun>,
    pub local_edges: JobBuffer<Edge>,
    pub dash_seams: JobBuffer<bool>,
    pub dash: JobDashCache,
    pub center_samples: JobBuffer<u8>,
    pub subpixel_samples: JobBuffer<u8>,
    pub stroke_cost: JobBuffer<usize>,
}
impl JobRasterScratch {
    /// Registers empty buffers only; the engine supplies each exact capacity to
    /// reserve. Every registration/payload/nested grid uses this same job ledger.
    /// Failure invalidates newly registered handles; metadata capacity is retained.
    pub fn new(job: &mut RasterJob, control: &mut dyn WorkControl) -> Result<Self> {
        let mark = job.resource_mark();
        let result = (|| {
            let dash = JobDashCache {
                segments: job.buffer(RasterPool::Dash, usize::MAX, control)?,
                joins: job.buffer(RasterPool::Dash, usize::MAX, control)?,
                shapes: job.buffer(RasterPool::Dash, 65536, control)?,
                build_steps: 0,
            };
            macro_rules! scratch {
                () => {
                    job.buffer(RasterPool::Scratch, usize::MAX, control)?
                };
            }
            Ok(Self {
                bounds: scratch!(),
                boundary_bits: job.buffer(RasterPool::Boundary, usize::MAX, control)?,
                scanline_events: scratch!(),
                scanline_nodes: scratch!(),
                scanline_roots: scratch!(),
                shape_events: scratch!(),
                shape_nodes: scratch!(),
                shape_offsets: scratch!(),
                active_edges: scratch!(),
                active_shapes: scratch!(),
                eligible_shapes: scratch!(),
                crossings: scratch!(),
                intervals: scratch!(),
                pixel_runs: scratch!(),
                local_edges: scratch!(),
                dash_seams: scratch!(),
                dash,
                center_samples: scratch!(),
                subpixel_samples: scratch!(),
                stroke_cost: scratch!(),
            })
        })();
        if result.is_err() {
            job.rollback_resources(mark)?;
        }
        result
    }
    fn require_distinct(&self) -> Result<()> {
        let identities = [
            self.bounds.identity(),
            self.boundary_bits.identity(),
            self.scanline_events.identity(),
            self.scanline_nodes.identity(),
            self.scanline_roots.identity(),
            self.shape_events.identity(),
            self.shape_nodes.identity(),
            self.shape_offsets.identity(),
            self.active_edges.identity(),
            self.active_shapes.identity(),
            self.eligible_shapes.identity(),
            self.crossings.identity(),
            self.intervals.identity(),
            self.pixel_runs.identity(),
            self.local_edges.identity(),
            self.dash_seams.identity(),
            self.dash.segments.identity(),
            self.dash.joins.identity(),
            self.dash.shapes.identity(),
            self.center_samples.identity(),
            self.subpixel_samples.identity(),
            self.stroke_cost.identity(),
        ];
        for (i, id) in identities.iter().enumerate() {
            if identities[..i].contains(id) {
                return Err(crate::types::invalid(0));
            }
        }
        Ok(())
    }
    // Fixed set of descriptor fields; cleanup does not depend on work control.
    fn require_roles(&self, job: &RasterJob) -> Result<()> {
        macro_rules! scratch { ($($field:ident),* $(,)?) => { $(
            job.require_pool(self.$field, RasterPool::Scratch)?;
        )* }; }
        scratch!(
            bounds,
            scanline_events,
            scanline_nodes,
            scanline_roots,
            shape_events,
            shape_nodes,
            shape_offsets,
            active_edges,
            active_shapes,
            eligible_shapes,
            crossings,
            intervals,
            pixel_runs,
            local_edges,
            dash_seams,
            center_samples,
            subpixel_samples,
            stroke_cost
        );
        job.require_pool(self.boundary_bits, RasterPool::Boundary)?;
        job.require_pool(self.dash.segments, RasterPool::Dash)?;
        job.require_pool(self.dash.joins, RasterPool::Dash)?;
        job.require_pool(self.dash.shapes, RasterPool::Dash)?;
        Ok(())
    }
    /// Check handle liveness and allocation roles before lower index/sample use.
    pub fn validate(&self, job: &RasterJob, control: &mut dyn WorkControl) -> Result<()> {
        self.require_distinct()?;
        control.checkpoint().map_err(crate::types::stopped)?;
        self.require_roles(job)?;
        job.dash_view(self.dash, control)?;
        Ok(())
    }
    /// Explicit early release. Job Drop also destroys all resources, including
    /// per-shape grids which can be shared by descriptors and remain job-owned.
    /// Individual releases reject stale/double handles; no recursive ownership.
    pub fn release(self, job: &mut RasterJob) -> Result<()> {
        // Validate every token first, so a mismatched descriptor changes nothing.
        macro_rules! each {
            ($method:ident) => {{
                job.$method(self.bounds)?;
                job.$method(self.boundary_bits)?;
                job.$method(self.scanline_events)?;
                job.$method(self.scanline_nodes)?;
                job.$method(self.scanline_roots)?;
                job.$method(self.shape_events)?;
                job.$method(self.shape_nodes)?;
                job.$method(self.shape_offsets)?;
                job.$method(self.active_edges)?;
                job.$method(self.active_shapes)?;
                job.$method(self.eligible_shapes)?;
                job.$method(self.crossings)?;
                job.$method(self.intervals)?;
                job.$method(self.pixel_runs)?;
                job.$method(self.local_edges)?;
                job.$method(self.dash_seams)?;
                job.$method(self.dash.segments)?;
                job.$method(self.dash.joins)?;
                job.$method(self.dash.shapes)?;
                job.$method(self.center_samples)?;
                job.$method(self.subpixel_samples)?;
                job.$method(self.stroke_cost)?;
            }};
        }
        self.require_distinct()?;
        self.require_roles(job)?;
        each!(capacity);
        each!(release);
        Ok(())
    }
}

/// Output remains inside the job until explicitly released or job destruction.
/// Taking a snapshot never silently returns an unleased Tile allocation.
#[derive(Debug, Clone, Copy)]
pub struct JobRenderOutput {
    pub generation: u64,
    pub pixels: JobBuffer<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: usize,
}
impl JobRenderOutput {
    pub fn validate(&self, job: &RasterJob) -> Result<()> {
        job.require_pool(self.pixels, RasterPool::Output)?;
        let row = (self.width as usize)
            .checked_mul(4)
            .ok_or_else(crate::memory::arithmetic)?;
        let bytes = self
            .stride
            .checked_mul(self.height as usize)
            .ok_or_else(crate::memory::arithmetic)?;
        if self.width == 0
            || self.height == 0
            || self.width > vexel_raster::MAX_AXIS
            || self.height > vexel_raster::MAX_AXIS
            || self.stride != row
            || job.slice(self.pixels)?.len() != bytes
            || job.capacity(self.pixels)? > vexel_raster::MAX_ALLOCATED_BYTES
        {
            return Err(crate::types::invalid(0));
        }
        Ok(())
    }
}
pub trait ControlledFlatKernel {
    fn render_flat(
        &mut self,
        view: SceneView<'_>,
        range: TableRange,
        options: RenderOptions,
        job: &mut RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<JobRenderOutput>;
}

/// Original shape_bounds.c cap: marking operations, NOT boundary bit count.
pub const RASTER_BOUNDARY_WORK_LIMIT: usize = 200_000_000;
