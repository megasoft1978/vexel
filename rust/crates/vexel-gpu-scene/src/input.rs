//! Plain-data scene the app lowers its render tree into (ADR-0035). It mirrors
//! what `compose::render_scene` hands the CPU rasterizer (`RenderNode`,
//! `RenderShape`, `RenderClip`, `RenderGroup`) after all coordinate mapping:
//! every point is already in device pixels (top-left origin, +y down), so this
//! crate needs no `ViewMap`.
//!
//! # Adapter contract (written by the app lane)
//!
//! * `RenderShape.edges` * `ViewMap` -> [`GpuSegment`] in device space. The
//!   edge list must be closed per subpath: the CPU rasterizer closes implicitly
//!   for the fill test, here every ring needs its closing segment explicitly.
//! * `evenodd` -> [`GpuShape::evenodd`]. `filled == false` -> `fill: None`.
//! * `fill_rgba` -> [`GpuPaint::Solid`] (straight alpha). `fill_gradient` ->
//!   [`GpuPaint::Linear`] / [`GpuPaint::Radial`] with `inverse` = the single
//!   affine device -> gradient space obtained by composing device -> user
//!   (`ViewMap`), `Paint::user_inverse`, the object-bounding-box
//!   normalisation and `Paint::gradient_inverse`, then the paint-specific
//!   normalisation below. Stops are copied as they are in `PaintStop` order.
//!   - Linear: gradient space x is `t` itself, i.e. the matrix maps the
//!     gradient vector `(x1,y1)->(x2,y2)` to `(0,0)->(1,0)`.
//!   - Radial: gradient space is the unit circle (centre `(0,0)`, radius 1)
//!     rotated so the focal point lies at `(focal_ratio, 0)`.
//! * Matrix order is `[a, b, c, d, e, f]` with `x' = a*x + c*y + e` and
//!   `y' = b*x + d*y + f`, as `vexel_geometry::Affine`.
//! * A group with opacity 1 and no mask should be flattened by the adapter.
//! * Anything not representable here (filters, masks, patterns, images, text,
//!   animation overlays) must make the adapter skip the GPU route.
//! * `content_rect` is the `ViewMap` content rectangle in device pixels
//!   (`[x0, y0, x1, y1]`); meet letterboxing makes it smaller than the canvas.

/// One flattened line segment in device pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSegment {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

/// Gradient spread (`SPREAD_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuSpread {
    Pad,
    Reflect,
    Repeat,
}

/// One gradient stop, straight-alpha RGBA8.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuColorStop {
    pub offset: f64,
    pub rgba: [u8; 4],
}

/// A linear or radial gradient, see the module docs for `inverse`.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuGradient {
    pub inverse: [f64; 6],
    pub spread: GpuSpread,
    pub stops: Vec<GpuColorStop>,
    /// Radial only: signed focal distance in unit-circle space.
    pub focal_ratio: f64,
}

/// Paint of one fill or stroke.
#[derive(Debug, Clone, PartialEq)]
pub enum GpuPaint {
    /// Straight-alpha RGBA8.
    Solid([u8; 4]),
    Linear(GpuGradient),
    Radial(GpuGradient),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuJoin {
    Miter,
    Round,
    Bevel,
}

/// Stroke primitive: the centre line is the owning shape's `edges`; the
/// stroke is evaluated in the space `transform` maps to device space, as the
/// CPU does. Not rendered yet (`Unsupported(Strokes)`); a later lane expands
/// it to filled polygons.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuStroke {
    pub paint: GpuPaint,
    pub width: f64,
    pub miter_limit: f64,
    pub cap: GpuCap,
    pub join: GpuJoin,
    pub dashes: Vec<f64>,
    pub dash_offset: f64,
    /// Local space -> device space, `[a, b, c, d, e, f]`.
    pub transform: [f64; 6],
}

/// One clip path (union of its filled children under one rule) in device
/// space. A chain of them is an intersection. Not rendered yet.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuClipPath {
    pub edges: Vec<GpuSegment>,
    pub evenodd: bool,
}

/// A leaf shape.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuShape {
    pub edges: Vec<GpuSegment>,
    pub evenodd: bool,
    pub fill: Option<GpuPaint>,
    pub stroke: Option<GpuStroke>,
    /// Clip chain, outermost first (empty when unclipped).
    pub clips: Vec<GpuClipPath>,
}

/// An opacity group. Not rendered yet (`Unsupported(Groups)`).
#[derive(Debug, Clone, PartialEq)]
pub struct GpuGroup {
    pub opacity: f64,
    pub clips: Vec<GpuClipPath>,
    pub children: Vec<GpuNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpuNode {
    Shape(GpuShape),
    Group(GpuGroup),
}

/// The whole frame; nodes are in paint order (later paints over earlier).
#[derive(Debug, Clone, PartialEq)]
pub struct GpuScene {
    pub width: u32,
    pub height: u32,
    /// Straight-alpha RGBA8 the frame starts from (transparent for the CPU).
    pub background: [u8; 4],
    pub content_rect: [f64; 4],
    pub nodes: Vec<GpuNode>,
}

impl GpuScene {
    /// Empty scene whose content rectangle is the whole canvas.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            background: [0; 4],
            content_rect: [0.0, 0.0, f64::from(width), f64::from(height)],
            nodes: Vec::new(),
        }
    }

    /// Appends a filled shape.
    pub fn push_fill(&mut self, edges: Vec<GpuSegment>, evenodd: bool, paint: GpuPaint) {
        self.nodes.push(GpuNode::Shape(GpuShape {
            edges,
            evenodd,
            fill: Some(paint),
            stroke: None,
            clips: Vec::new(),
        }));
    }
}
