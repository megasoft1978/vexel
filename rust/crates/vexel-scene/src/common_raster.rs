//! Common immutable table algorithms for ready frames and request-minted tiles.
use crate::{
    types::{invalid, stopped},
    *,
};
/// Original [min x, min y, max x, max y], including zero extent.
/// Rejects overflowed extents even when each coordinate itself is finite.
pub fn ordered_bounds_extent(bounds: [f64; 4]) -> Option<[f64; 2]> {
    if !bounds.into_iter().all(f64::is_finite) {
        return None;
    }
    let extent = [bounds[2] - bounds[0], bounds[3] - bounds[1]];
    extent
        .into_iter()
        .all(|v| v.is_finite() && v >= 0.)
        .then_some(extent)
}
/// An animation-free read capability. No raw scene/ready conversion or arbitrary
/// scene lookup is exposed. All table slices retain their original owners.
#[derive(Clone, Copy)]
pub struct RasterInput<'a> {
    tables: SceneView<'a, Viewport, ()>,
    scope: Option<crate::static_pattern::StaticScope>,
}
impl<'a> RasterInput<'a> {
    pub fn ready(view: SceneView<'a>) -> Self {
        Self {
            tables: view.raster_tables(view.viewport),
            scope: None,
        }
    }
    pub(crate) fn static_child(
        tables: SceneView<'a, Viewport, ()>,
        scope: crate::static_pattern::StaticScope,
    ) -> Self {
        Self {
            tables,
            scope: Some(scope),
        }
    }
    pub fn validate_job(self, job: &RasterJob, control: &mut dyn WorkControl) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        job.validate_source(self.source_owner())?;
        if self.scope.is_some_and(|s| s.job != job.owner_identity()) {
            return Err(invalid(0));
        }
        Ok(())
    }
    pub fn static_generation(self) -> Option<u64> {
        self.scope.map(|s| s.generation)
    }
    pub fn static_ticket(self) -> Option<PatternTicket> {
        self.scope.map(|s| s.ticket)
    }
    pub fn static_request(self) -> Option<RequestToken> {
        self.scope.map(|s| s.request)
    }
    pub fn static_draft_nonce(self) -> Option<u64> {
        self.scope.map(|s| s.draft)
    }
    pub fn id(self) -> SceneId {
        self.tables.id
    }
    pub fn viewport(self) -> Viewport {
        self.tables.viewport
    }
    pub fn source(self) -> &'a [u8] {
        self.tables.source()
    }
    pub fn source_owner(self) -> &'a SourceOwner {
        self.tables.source_owner()
    }
    pub fn ignored(self, kind: IgnoredKind) -> Ignored {
        self.tables.ignored(kind)
    }
    /// Only a checked mask in this exact scene authorizes a sealed child read.
    pub fn mask_content(self, mask: MaskId) -> Option<Self> {
        let content = self.tables.mask(mask)?.content;
        Some(Self {
            tables: self.tables.scene(content)?,
            scope: self.scope,
        })
    }
    pub fn edges(self) -> &'a [Edge] {
        self.tables.edges
    }
    pub fn edge_colors(self) -> &'a [[u8; 4]] {
        self.tables.edge_colors
    }
    pub fn edge_shape_ids(self) -> &'a [u32] {
        self.tables.edge_shape_ids
    }
    pub fn shapes(self) -> &'a [Shape] {
        self.tables.shapes
    }
    pub fn path_subpaths(self) -> &'a [vexel_svg_path::PathSubpath] {
        self.tables.path_subpaths
    }
    pub fn path_segments(self) -> &'a [ScenePathSegment] {
        self.tables.path_segments
    }
    pub fn path_numbers(self) -> &'a [vexel_svg_path::PathNumber] {
        self.tables.path_numbers
    }
    pub fn shape_geometry_source(self, id: ShapeId) -> Option<ShapeGeometrySource> {
        self.tables.shape_geometry_source(id)
    }
    pub fn transforms(self) -> &'a [TransformFrame] {
        self.tables.transforms
    }
    pub fn images(self) -> &'a [Image] {
        self.tables.images
    }
    pub fn groups(self) -> &'a [OpacityGroup] {
        self.tables.groups
    }
    pub fn filters(self) -> &'a [Filter] {
        self.tables.filters
    }
    pub fn filter_primitives(self) -> &'a [FilterPrimitive] {
        self.tables.filter_primitives
    }
    pub fn masks(self) -> &'a [Mask] {
        self.tables.masks
    }
    pub fn patterns(self) -> &'a [Pattern] {
        self.tables.patterns
    }
    pub fn fill_paints(self) -> &'a [Paint] {
        self.tables.fill_paints
    }
    pub fn stroke_paints(self) -> &'a [Paint] {
        self.tables.stroke_paints
    }
    pub fn dash_values(self) -> &'a [f64] {
        self.tables.dash_values
    }
    pub fn stops(self) -> &'a [PaintStop] {
        self.tables.stops
    }
    pub fn clips(self) -> &'a [Clip] {
        self.tables.clips
    }
    pub fn shape(self, id: ShapeId) -> Option<&'a Shape> {
        self.tables.shape(id)
    }
    pub fn shape_id(self, index: usize) -> Option<ShapeId> {
        self.tables.shape_id(index)
    }
    pub fn transform(self, id: TransformId) -> Option<&'a TransformFrame> {
        self.tables.transform(id)
    }
    pub fn transform_id(self, index: usize) -> Option<TransformId> {
        self.tables.transform_id(index)
    }
    pub fn clip(self, id: ClipId) -> Option<&'a Clip> {
        self.tables.clip(id)
    }
    pub fn clip_id(self, index: usize) -> Option<ClipId> {
        self.tables.clip_id(index)
    }
    pub fn pattern(self, id: PatternId) -> Option<&'a Pattern> {
        self.tables.pattern(id)
    }
    pub fn pattern_id(self, index: usize) -> Option<PatternId> {
        self.tables.pattern_id(index)
    }
    pub fn mask(self, id: MaskId) -> Option<&'a Mask> {
        self.tables.mask(id)
    }
    pub fn mask_id(self, index: usize) -> Option<MaskId> {
        self.tables.mask_id(index)
    }
    pub fn filter(self, id: FilterId) -> Option<&'a Filter> {
        self.tables.filter(id)
    }
    pub fn filter_id(self, index: usize) -> Option<FilterId> {
        self.tables.filter_id(index)
    }
    pub fn group(self, id: GroupId) -> Option<&'a OpacityGroup> {
        self.tables.group(id)
    }
    pub fn group_id(self, index: usize) -> Option<GroupId> {
        self.tables.group_id(index)
    }
    pub fn edge_shape(self, edge: usize) -> Option<ShapeId> {
        self.tables.edge_shape(edge)
    }
    pub fn fill_paint(self, id: ShapeId) -> Option<&'a Paint> {
        self.tables.fill_paint(id)
    }
    pub fn stroke_paint(self, id: ShapeId) -> Option<&'a Paint> {
        self.tables.stroke_paint(id)
    }
    pub fn shape_dashes(self, id: ShapeId) -> Option<&'a [f64]> {
        self.tables.shape_dashes(id)
    }
}

pub trait CommonKernelResources {
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
        view: RasterInput<'_>,
        clip: ClipId,
        x: f64,
        y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<bool>;
    fn clip_boundary_edges(
        &self,
        view: RasterInput<'_>,
        clip: ClipId,
        visitor: &mut dyn FnMut(Edge) -> bool,
        control: &mut dyn WorkControl,
    ) -> Result<bool>;
    fn clip_work_counts(
        &self,
        view: RasterInput<'_>,
        clip: ClipId,
        control: &mut dyn WorkControl,
    ) -> Result<ClipWorkCounts>;
    fn stroke_sample_work(
        &self,
        view: RasterInput<'_>,
        shape: ShapeId,
        cache: ValidatedDashView<'_>,
        x: f64,
        y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<(usize, bool, bool)>;
}
/// Same scalar coordinates/lattice/edge data as SampleInput. The lower engine
/// changes its input type and propagates Result; its math and cost stay unchanged.
pub struct CommonSampleInput<'a> {
    pub resources: &'a dyn CommonKernelResources,
    pub active_edges: &'a [u32],
    pub local_edges: &'a [Edge],
    pub dash_seams: &'a [bool],
    pub dash_cache: ValidatedDashView<'a>,
    pub px: f64,
    pub py: f64,
    pub paint_x: f64,
    pub paint_y: f64,
}
impl CommonSampleInput<'_> {
    pub fn validate(&self, view: RasterInput<'_>, control: &mut dyn WorkControl) -> Result<()> {
        if !self.dash_cache.matches_source(view.source_owner()) {
            return Err(invalid(0));
        }
        if ![self.px, self.py, self.paint_x, self.paint_y]
            .into_iter()
            .all(f64::is_finite)
            || self.dash_seams.len() != self.local_edges.len()
        {
            return Err(invalid(0));
        }
        control.checkpoint().map_err(stopped)?;
        for index in self.active_edges {
            control.charge(WorkKind::Edge, 1).map_err(stopped)?;
            if *index as usize >= view.edges().len() {
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
pub trait CommonSampleKernel {
    fn sample(
        &mut self,
        view: RasterInput<'_>,
        shape: ShapeId,
        input: &CommonSampleInput<'_>,
        control: &mut dyn WorkControl,
    ) -> Result<[u8; 4]>;
}
/// Lower index writers receive the same owning job as child/output/sample work.
pub trait CommonIndexKernel {
    fn build_index(
        &mut self,
        view: RasterInput<'_>,
        job: &mut RasterJob,
        mapping: ViewMapping,
        control: &mut dyn WorkControl,
    ) -> Result<JobRasterScratch>;
}

/// Required production flat boundary for both roles; no inherited ready fallback.
pub trait CommonFlatKernel {
    fn render_flat(
        &mut self,
        input: RasterInput<'_>,
        range: TableRange,
        options: RenderOptions,
        job: &mut RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<JobRenderOutput>;
}
/// All capabilities are explicit obligations on the actual production engine.
pub trait CommonRasterKernel:
    CommonKernelResources + CommonIndexKernel + CommonSampleKernel + CommonFlatKernel
{
}

/// Guard before calling the lower renderer. Its output remains in the same job.
pub fn render_common_pattern(
    kernel: &mut dyn CommonRasterKernel,
    request: &StaticPatternRequest<'_>,
    options: RenderOptions,
    job: &mut RasterJob,
    control: &mut dyn WorkControl,
) -> Result<JobRenderOutput> {
    let input = request.child.input();
    input.validate_job(job, control)?;
    let output = kernel.render_flat(
        input,
        TableRange {
            first: 0,
            count: input.shapes().len(),
        },
        options,
        job,
        control,
    )?;
    let validation = output.validate(job).and_then(|()| {
        if Some(output.generation) != input.static_generation()
            || output.width != request.descriptor.width
            || output.height != request.descriptor.height
        {
            Err(invalid(request.descriptor.error_offset))
        } else {
            Ok(())
        }
    });
    if let Err(error) = validation {
        if job.require_pool(output.pixels, RasterPool::Output).is_ok()
            && job.slice(output.pixels).is_ok()
        {
            job.release(output.pixels)?;
        }
        return Err(error);
    }
    Ok(output)
}
