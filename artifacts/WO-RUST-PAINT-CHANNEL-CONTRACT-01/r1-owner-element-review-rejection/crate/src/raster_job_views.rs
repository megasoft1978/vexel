//! Borrowed engine inputs: scalar math reads the original element types, never
//! copies a cache into unmetered Vecs. Descriptors own no nested allocation.
use crate::types::{invalid, limit, stopped};
use crate::*;

#[derive(Debug, Clone, Copy)]
pub struct JobDashRange {
    pub segments: TableRange,
    pub joins: TableRange,
    pub grid_cells: JobBuffer<GridCell>,
    pub grid_refs: JobBuffer<u32>,
    pub grid_min_x: f64,
    pub grid_min_y: f64,
    pub grid_cell_size: f64,
    pub grid_columns: u16,
    pub grid_rows: u16,
}
#[derive(Debug, Clone, Copy)]
pub struct JobDashCache {
    pub segments: JobBuffer<DashSegment>,
    pub joins: JobBuffer<DashJoin>,
    pub shapes: JobBuffer<JobDashRange>,
    pub build_steps: usize,
}
#[derive(Clone, Copy)]
pub struct JobDashView<'a> {
    pub(crate) reader: &'a dyn crate::raster_composition::CacheRead,
    pub(crate) source: &'a SourceOwner,
    pub segments: &'a [DashSegment],
    pub joins: &'a [DashJoin],
    pub shapes: &'a [JobDashRange],
}
impl RasterJob {
    pub fn dash_view(
        &self,
        cache: JobDashCache,
        control: &mut dyn WorkControl,
    ) -> Result<JobDashView<'_>> {
        control.checkpoint().map_err(stopped)?;
        if cache.build_steps > 1_000_000 {
            return Err(limit(LimitKind::DashCache));
        }
        self.require_pool(cache.segments, RasterPool::Dash)?;
        self.require_pool(cache.joins, RasterPool::Dash)?;
        self.require_pool(cache.shapes, RasterPool::Dash)?;
        let view = JobDashView {
            reader: self,
            source: self.source_owner(),
            segments: self.slice(cache.segments)?,
            joins: self.slice(cache.joins)?,
            shapes: self.slice(cache.shapes)?,
        };
        view.validate(control)?;
        Ok(view)
    }
}
impl<'a> JobDashView<'a> {
    /// Constant-time selection of original validated shape/grid metadata.
    pub fn grid_at(&self, shape: usize) -> Result<(&'a JobDashRange, &'a [GridCell], &'a [u32])> {
        let range = self.shapes.get(shape).ok_or_else(|| invalid(0))?;
        let (cells, refs) = self.grid(range)?;
        Ok((range, cells, refs))
    }
    /// Resolves only same-job, still-live handles. A caller may retain descriptors
    /// across release, but can never dereference them without revalidation.
    pub fn grid(&self, range: &JobDashRange) -> Result<(&'a [GridCell], &'a [u32])> {
        Ok((
            self.reader.cells(range.grid_cells)?,
            self.reader.references(range.grid_refs)?,
        ))
    }
    pub fn validate(&self, control: &mut dyn WorkControl) -> Result<()> {
        if self.shapes.len() > 65536 {
            return Err(limit(LimitKind::Shapes));
        }
        for segment in self.segments {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            if ![segment.edge.a, segment.edge.b]
                .into_iter()
                .all(vexel_geometry::finite_point)
            {
                return Err(invalid(0));
            }
        }
        for join in self.joins {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            if ![join.previous, join.vertex, join.next]
                .into_iter()
                .all(vexel_geometry::finite_point)
            {
                return Err(invalid(0));
            }
        }
        let (mut cell_count, mut ref_count) = (0usize, 0usize);
        for range in self.shapes {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            range.segments.require(self.segments.len(), 0)?;
            range.joins.require(self.joins.len(), 0)?;
            let (cells, refs) = self.grid(range)?;
            cell_count = cell_count
                .checked_add(cells.len())
                .ok_or_else(crate::memory::arithmetic)?;
            ref_count = ref_count
                .checked_add(refs.len())
                .ok_or_else(crate::memory::arithmetic)?;
            if cell_count > 65536
                || ref_count > 1_000_000
                || range.grid_columns > 128
                || range.grid_rows > 128
            {
                return Err(limit(LimitKind::Grid));
            }
            if ![range.grid_min_x, range.grid_min_y, range.grid_cell_size]
                .into_iter()
                .all(f64::is_finite)
                || cells.len() != range.grid_columns as usize * range.grid_rows as usize
                || ((cells.is_empty()) != (range.grid_columns == 0 && range.grid_rows == 0))
                || (!cells.is_empty() && range.grid_cell_size <= 0.0)
            {
                return Err(invalid(0));
            }
            for cell in cells {
                control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                TableRange {
                    first: cell.first,
                    count: cell.count,
                }
                .require(refs.len(), 0)?;
            }
            for reference in refs {
                control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                let index = (*reference & !STROKE_GRID_JOIN_BIT) as usize;
                let allowed = if *reference & STROKE_GRID_JOIN_BIT != 0 {
                    range.joins
                } else {
                    range.segments
                };
                if index < allowed.first
                    || index >= allowed.end().ok_or_else(crate::memory::arithmetic)?
                {
                    return Err(invalid(0));
                }
            }
        }
        control.checkpoint().map_err(stopped)
    }
}

/// Complete controlled capability; deliberately has NO blanket conversion from
/// KernelResources, no default paint, and no production placeholder implementation.
/// The coordinator implements every method from the actual paint/clip/stroke
/// engines. Variable loops must poll this exact control and propagate its error.
pub trait ControlledKernelResources {
    fn paint_sample(
        &self,
        paint: &Paint,
        stops: &[PaintStop],
        x: f64,
        y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<[u8; 4]>;
    fn clip_contains(
        &self,
        view: SceneView<'_>,
        clip: ClipId,
        x: f64,
        y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<bool>;
    fn clip_boundary_edges(
        &self,
        view: SceneView<'_>,
        clip: ClipId,
        visitor: &mut dyn FnMut(Edge) -> bool,
        control: &mut dyn WorkControl,
    ) -> Result<bool>;
    fn clip_work_counts(
        &self,
        view: SceneView<'_>,
        clip: ClipId,
        control: &mut dyn WorkControl,
    ) -> Result<ClipWorkCounts>;
    fn stroke_sample_work(
        &self,
        view: SceneView<'_>,
        shape: ShapeId,
        cache: JobDashView<'_>,
        x: f64,
        y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<(usize, bool, bool)>;
}
/// Same scalar coordinates/lattice/edge data as SampleInput. The lower engine
/// changes its input type and propagates Result; its math and cost stay unchanged.
pub struct ControlledSampleInput<'a> {
    pub resources: &'a dyn ControlledKernelResources,
    pub active_edges: &'a [u32],
    pub local_edges: &'a [Edge],
    pub dash_seams: &'a [bool],
    pub dash_cache: JobDashView<'a>,
    pub px: f64,
    pub py: f64,
    pub paint_x: f64,
    pub paint_y: f64,
}
impl ControlledSampleInput<'_> {
    pub fn validate(&self, view: SceneView<'_>, control: &mut dyn WorkControl) -> Result<()> {
        if !std::sync::Arc::ptr_eq(self.dash_cache.source, view.source_owner()) {
            return Err(invalid(0));
        }
        if ![self.px, self.py, self.paint_x, self.paint_y]
            .into_iter()
            .all(f64::is_finite)
            || self.dash_seams.len() != self.local_edges.len()
        {
            return Err(invalid(0));
        }
        // Legacy fields are replaceable by callers; retain its explicit validation.
        self.dash_cache.validate(control)?;
        for index in self.active_edges {
            control.charge(WorkKind::Edge, 1).map_err(stopped)?;
            if *index as usize >= view.edges.len() {
                return Err(invalid(0));
            }
        }
        Ok(())
    }
    /// No precharge or aftercharge substitute: the actual resource receives the
    /// caller's control, including while traversing stops/pattern samples.
    pub fn paint_sample(
        &self,
        paint: &Paint,
        stops: &[PaintStop],
        control: &mut dyn WorkControl,
    ) -> Result<[u8; 4]> {
        control.checkpoint().map_err(stopped)?;
        self.resources
            .paint_sample(paint, stops, self.paint_x, self.paint_y, control)
    }
}
/// Typed lower seam. Production implementations cannot accidentally accept the
/// legacy uncontrolled paint capability through this entrypoint.
pub trait ControlledSampleKernel {
    fn sample(
        &mut self,
        view: SceneView<'_>,
        shape: ShapeId,
        input: &ControlledSampleInput<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<[u8; 4]>;
}
/// Lower index writers receive the same owning job as child/output/sample work.
pub trait ControlledIndexKernel {
    fn build_index(
        &mut self,
        view: SceneView<'_>,
        job: &mut RasterJob,
        mapping: ViewMapping,
        control: &mut dyn WorkControl,
    ) -> Result<JobRasterScratch>;
}
