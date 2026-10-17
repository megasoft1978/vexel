use crate::{
    Affine, ClipId, Edge, ElementId, FilterId, GroupId, ImagePixels, MaskId, PatternId, SceneId,
    ShapeId, TableRange, TransformId,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    SourceBytes,
    TokenBytes,
    Depth,
    Nodes,
    Attributes,
    TextBytes,
    Glyphs,
    TextCommands,
    Images,
    Edges,
    Shapes,
    PathCommands,
    PathNumbers,
    References,
    Expansions,
    Transforms,
    PaintStops,
    PaintVisits,
    Clips,
    Patterns,
    Masks,
    Filters,
    Scratch,
    PixelRuns,
    DashCache,
    Grid,
    Animation,
    Work,
    ReservedBytes,
    Arithmetic,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidArgument,
    Xml(vexel_xml::XmlError),
    Unsupported,
    Number,
    Path,
    Viewport,
    OutOfMemory,
    Canceled,
    Limit(LimitKind),
    UnsupportedFeTurbulence,
    UnsupportedFeDisplacementMap,
    UnsupportedFeConvolveMatrix,
    UnsupportedFeImage,
    UnsupportedFeLighting,
    UnsupportedFeMorphology,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: ErrorKind,
    pub offset: usize,
}
pub type Result<T> = std::result::Result<T, Diagnostic>;
pub(crate) fn invalid(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset,
    }
}
pub(crate) fn limit(kind: LimitKind) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(kind),
        offset: 0,
    }
}
pub(crate) fn stopped(stop: crate::Stop) -> Diagnostic {
    Diagnostic {
        kind: match stop {
            crate::Stop::Canceled => ErrorKind::Canceled,
            crate::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            crate::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ignored {
    pub count: usize,
    pub first_offset: Option<usize>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoredKind {
    UnknownAttribute,
    Meta,
    Doctype,
    WritingMode,
}
/// The single shape-count cap: per scene, per raster-job dash pool, and the bound on
/// `<use>` instances (`MAX_USE_INSTANCES`). Memory proof: artifacts/WO-RUST-SCENE-SHAPES-01/shapes.md.
pub const MAX_SHAPES: usize = 262_144;
/// Decoder-wide `<use>` expansion bound. Equal to `MAX_SHAPES` by construction.
pub const MAX_USE_INSTANCES: usize = MAX_SHAPES;
const _: () = assert!(MAX_SHAPES <= u32::MAX as usize);
const _: () = assert!(MAX_USE_INSTANCES == MAX_SHAPES);
/// Per-scene reference admission bounds. Byte policy is explicitly caller configurable.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub source_bytes: usize,
    pub reserved_bytes: usize,
    pub scenes: usize,
    pub shapes: usize,
    pub edges: usize,
    pub commands: usize,
    pub numbers: usize,
    pub transforms: usize,
    pub clips: usize,
    pub patterns: usize,
    pub masks: usize,
    pub filters: usize,
    pub stops: usize,
    pub groups: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            source_bytes: 1 << 30,
            reserved_bytes: usize::MAX,
            scenes: 65536,
            shapes: MAX_SHAPES,
            edges: 4 * 1024 * 1024,
            commands: 4 * 1024 * 1024,
            numbers: 28 * 1024 * 1024,
            transforms: 4 * 1024 * 1024,
            clips: 65536,
            patterns: 4096,
            masks: 4096,
            filters: 4096,
            stops: 65536,
            groups: 65536,
        }
    }
}
impl Stroke {
    pub fn initial() -> Self {
        Self {
            rgba: [0; 4],
            width: 1.0,
            miter_limit: 4.0,
            dash_offset: 0.0,
            dash_values: TableRange::EMPTY,
            cap: Linecap::Butt,
            join: Linejoin::Miter,
            paint_server: false,
            crisp_edges: false,
            paint_order: 0,
        }
    }
}
impl Shape {
    pub fn new(source: SourceMap) -> Self {
        Self {
            edges: TableRange::EMPTY,
            subpaths: TableRange::EMPTY,
            evenodd: false,
            filled: true,
            stroke_closed: false,
            source,
            stroke_inverse: vexel_geometry::affine_identity(),
            stroke: Stroke::initial(),
            fill_paint: None,
            stroke_paint: None,
            clip: None,
            image: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceMap {
    pub element: vexel_base::Span,
    pub transform: vexel_geometry::Affine,
    pub transform_node: Option<TransformId>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformFrame {
    pub source_offset: usize,
    pub parent: Option<TransformId>,
    pub attribute: Affine,
    pub suffix: Affine,
    pub motion: Affine,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenePathSegment {
    pub path: vexel_svg_path::PathSegment,
    pub shape: ShapeId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linecap {
    Butt,
    Square,
    Round,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linejoin {
    Miter,
    Bevel,
    Round,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub rgba: [u8; 4],
    pub width: f64,
    pub miter_limit: f64,
    pub dash_offset: f64,
    pub dash_values: TableRange,
    pub cap: Linecap,
    pub join: Linejoin,
    pub paint_server: bool,
    pub crisp_edges: bool,
    pub paint_order: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintKind {
    Solid,
    Linear,
    Radial,
    Image,
    Pattern,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spread {
    Pad,
    Reflect,
    Repeat,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaintStop {
    pub offset: f64,
    pub rgba: [u8; 4],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    pub kind: PaintKind,
    pub spread: Spread,
    pub object_bounding_box: bool,
    pub coordinates: [f64; 6],
    pub bounds: [f64; 4],
    pub user_inverse: Affine,
    pub gradient_inverse: Affine,
    pub stops: TableRange,
    pub pattern: Option<PatternId>,
    pub pattern_inverse: Affine,
    pub pattern_opacity: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub edges: TableRange,
    pub subpaths: TableRange,
    pub evenodd: bool,
    pub filled: bool,
    pub stroke_closed: bool,
    pub source: SourceMap,
    pub stroke_inverse: Affine,
    pub stroke: Stroke,
    pub fill_paint: Option<usize>,
    pub stroke_paint: Option<usize>,
    pub clip: Option<ClipId>,
    pub image: Option<usize>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub view_x: f64,
    pub view_y: f64,
    pub view_width: f64,
    pub view_height: f64,
    pub aspect: vexel_svg_values::Aspect,
}
#[derive(Debug)]
pub struct Image {
    pub pixels: ImagePixels,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub draw_x: f64,
    pub draw_y: f64,
    pub draw_width: f64,
    pub draw_height: f64,
}
#[derive(Debug)]
pub struct ClipGeometry {
    pub edges: Vec<Edge>,
    pub shapes: Vec<ClipChild>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipChild {
    pub edges: TableRange,
    pub evenodd: bool,
    pub filled: bool,
}
#[derive(Debug)]
pub struct Clip {
    pub geometry: ClipGeometry,
    pub parent: Option<ClipId>,
    pub owner_node: Option<TransformId>,
    pub owner_before_suffix: bool,
    pub object_bounding_box: bool,
    pub bounds: [f64; 4],
    pub user_inverse: Affine,
}
#[derive(Debug)]
pub struct ClipWorkCounts {
    pub chain_elements: usize,
    pub child_shapes: usize,
    pub edges: usize,
}
#[derive(Debug)]
pub struct Pattern {
    pub tile: vexel_raster::Tile,
    pub definition: ElementId,
    pub tile_width: f64,
    pub tile_height: f64,
    pub content_transform: Affine,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mask {
    pub content: SceneId,
    pub luminance: bool,
    pub region_object_bounding_box: bool,
    pub custom_region: bool,
    pub region: [f64; 4],
    pub region_transform: Affine,
    pub source_offset: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    GaussianBlur,
    Offset,
    Flood,
    Blend,
    Composite,
    Merge,
    ColorMatrix,
    DropShadow,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterInput {
    Previous,
    Graphic,
    Alpha,
    Result(usize),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterPrimitive {
    pub kind: FilterKind,
    pub input: FilterInput,
    pub input2: FilterInput,
    pub merge_inputs: [FilterInput; 8],
    pub merge_count: u8,
    pub mode: u8,
    pub rgba: [u8; 4],
    pub values: [f64; 20],
    pub source_offset: usize,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Filter {
    pub primitives: TableRange,
    pub linear_rgb: bool,
    pub filter_units_object_bounding_box: bool,
    pub primitive_units_object_bounding_box: bool,
    pub custom_region: bool,
    pub region: [f64; 4],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpacityGroup {
    pub shapes: TableRange,
    pub parent: Option<GroupId>,
    pub opacity: f64,
    pub filter: Option<FilterId>,
    pub mask: Option<MaskId>,
    pub blend_mode: u8,
    pub isolation: bool,
}

impl ErrorKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalid-argument",
            Self::Xml(_) => "xml",
            Self::Unsupported => "unsupported",
            Self::Number => "number",
            Self::Path => "path",
            Self::Viewport => "viewport",
            Self::OutOfMemory => "oom",
            Self::Canceled => "canceled",
            Self::Limit(_) => "limit",
            Self::UnsupportedFeTurbulence => "unsupported-feTurbulence",
            Self::UnsupportedFeDisplacementMap => "unsupported-feDisplacementMap",
            Self::UnsupportedFeConvolveMatrix => "unsupported-feConvolveMatrix",
            Self::UnsupportedFeImage => "unsupported-feImage",
            Self::UnsupportedFeLighting => "unsupported-feLighting",
            Self::UnsupportedFeMorphology => "unsupported-feMorphology",
        }
    }
}
// Shape-record bytes alone stay within 512 MiB at the cap (see shapes.md for the full bound).
const _: () = assert!(MAX_SHAPES * std::mem::size_of::<Shape>() <= 512 * 1024 * 1024);
impl Limits {
    pub(crate) fn effective(self) -> Self {
        let d = Self::default();
        Self {
            source_bytes: self.source_bytes.min(d.source_bytes),
            reserved_bytes: self.reserved_bytes,
            scenes: self.scenes.min(d.scenes),
            shapes: self.shapes.min(d.shapes),
            edges: self.edges.min(d.edges),
            commands: self.commands.min(d.commands),
            numbers: self.numbers.min(d.numbers),
            transforms: self.transforms.min(d.transforms),
            clips: self.clips.min(d.clips),
            patterns: self.patterns.min(d.patterns),
            masks: self.masks.min(d.masks),
            filters: self.filters.min(d.filters),
            stops: self.stops.min(d.stops),
            groups: self.groups.min(d.groups),
        }
    }
}
