//! Shared owned scalar data and interfaces. Concrete scalar operations live in
//! their full port modules; there is no default renderer or fallback backend.
use crate::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PixelBounds {
    pub x_first: u32,
    pub x_last: u32,
    pub y_first: u32,
    pub y_last: u32,
    pub visible: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ShapeBounds {
    pub device_left: f64,
    pub device_top: f64,
    pub device_right: f64,
    pub device_bottom: f64,
    pub pixels: PixelBounds,
}
#[derive(Debug, Default)]
pub struct BoundsSet {
    pub shapes: Vec<ShapeBounds>,
    pub pixel_count: usize,
}
#[derive(Debug, Default)]
pub struct BoundaryMask {
    pub bits: Vec<u8>,
    pub bit_count: usize,
    pub marked_pixel_count: usize,
    pub mark_work: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanlineEvent {
    pub slot: usize,
    pub edge: u32,
    pub add: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeRowEvent {
    pub row: u32,
    pub shape: u32,
    pub add: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanlineNode {
    pub left: i32,
    pub right: i32,
    pub height: i32,
}
#[derive(Debug)]
pub struct ScanlineIndex {
    pub events: Vec<ScanlineEvent>,
    pub nodes: Vec<ScanlineNode>,
    pub roots: Vec<i32>,
    pub shape_events: Vec<ShapeRowEvent>,
    pub shape_nodes: Vec<ScanlineNode>,
    pub shape_root: i32,
    pub event_cursor: usize,
    pub shape_event_cursor: usize,
    pub shape_count: usize,
    pub edge_count: usize,
    pub scanline_count: usize,
    pub listed_edge_count: usize,
}
impl Default for ScanlineIndex {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            nodes: Vec::new(),
            roots: Vec::new(),
            shape_events: Vec::new(),
            shape_nodes: Vec::new(),
            shape_root: -1,
            event_cursor: 0,
            shape_event_cursor: 0,
            shape_count: 0,
            edge_count: 0,
            scanline_count: 0,
            listed_edge_count: 0,
        }
    }
}
#[derive(Debug, Default)]
pub struct RowBuffers {
    pub shape_offsets: Vec<usize>,
    pub active_edges: Vec<u32>,
    pub active_shapes: Vec<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FillInterval {
    pub left: f64,
    pub right: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillCrossing {
    pub x: f64,
    pub winding_delta: i32,
    pub edge_index: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRun {
    pub first_x: u32,
    pub end_x: u32,
    pub center_only: bool,
}
#[derive(Debug, Default)]
pub struct FillCache {
    pub eligible_shapes: Vec<bool>,
    pub max_edge_count: usize,
    pub interval_stride: usize,
    pub crossings: Vec<FillCrossing>,
    pub intervals: Vec<FillInterval>,
    pub interval_counts: [usize; 5],
    pub pixel_runs: Vec<PixelRun>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FillRowWork {
    pub edge_visits: usize,
    pub work_units: usize,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DashSegment {
    pub edge: Edge,
    pub start_cap: Linecap,
    pub end_cap: Linecap,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DashJoin {
    pub previous: Point,
    pub vertex: Point,
    pub next: Point,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridCell {
    pub first: usize,
    pub count: usize,
}
#[derive(Debug)]
pub struct DashRange {
    pub segments: TableRange,
    pub joins: TableRange,
    pub grid_cells: Vec<GridCell>,
    pub grid_refs: Vec<u32>,
    pub grid_min_x: f64,
    pub grid_min_y: f64,
    pub grid_cell_size: f64,
    pub grid_columns: u16,
    pub grid_rows: u16,
}
#[derive(Debug, Default)]
pub struct DashCache {
    pub segments: Vec<DashSegment>,
    pub joins: Vec<DashJoin>,
    pub shapes: Vec<DashRange>,
    pub build_steps: usize,
    pub grid_cell_count: usize,
    pub grid_ref_count: usize,
    pub grid_bytes: usize,
}
pub struct SampleInput<'a> {
    pub resources: &'a dyn KernelResources,
    pub active_edges: &'a [u32],
    pub local_edges: &'a [Edge],
    pub dash_seams: &'a [bool],
    pub dash_cache: &'a DashCache,
    pub px: f64,
    pub py: f64,
    pub paint_x: f64,
    pub paint_y: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaMode {
    SourceOver,
    FrozenOracleOnly,
}
#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub alpha: AlphaMode,
    pub limits: Limits,
}
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            alpha: AlphaMode::SourceOver,
            limits: Limits::default(),
        }
    }
}
#[derive(Debug)]
pub struct RenderResult {
    pub generation: u64,
    pub tile: vexel_raster::Tile,
    pub work: RenderWorkStats,
    pub memory: RenderMemoryStats,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewMapping {
    pub scale_x: f64,
    pub scale_y: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}
#[derive(Debug, Default)]
pub struct RasterScratch {
    pub bounds: BoundsSet,
    pub boundary: BoundaryMask,
    pub scanlines: ScanlineIndex,
    pub rows: RowBuffers,
    pub fill: FillCache,
    pub local_edges: Vec<Edge>,
    pub dash_seams: Vec<bool>,
    pub dash: DashCache,
    pub center_samples: Vec<u8>,
    pub subpixel_samples: Vec<u8>,
    pub stroke_cost: Vec<usize>,
}
/// The coordinator supplies a complete implementation from paint/clip/stroke ports.
/// Methods that visit variable-length data receive the same job work control.
pub trait KernelResources {
    fn paint_sample(&self, paint: &Paint, stops: &[PaintStop], x: f64, y: f64) -> [u8; 4];
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
        cache: &DashCache,
        x: f64,
        y: f64,
        control: &mut dyn WorkControl,
    ) -> Result<(usize, bool, bool)>;
}
pub trait FlatKernel {
    fn render_flat(
        &mut self,
        view: SceneView<'_>,
        range: TableRange,
        options: RenderOptions,
        ledger: &mut ChargeLedger,
        control: &mut dyn WorkControl,
    ) -> Result<RenderResult>;
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selection {
    pub shape: ShapeId,
    pub source: Span,
    pub user_bounds: Rect,
    pub device_bounds: Rect,
    pub transform: Affine,
}
