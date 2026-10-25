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

use std::sync::Arc;

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

/// Join geometry the CPU precomputed (`vexel-render` `export::JoinKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuJoinKind {
    /// Degenerate or collinear: never hits.
    Never,
    /// Disc of the stroke radius around the vertex.
    Round,
    /// Triangle `(vertex, p1, p2)` (bevel, or miter beyond the limit).
    Triangle,
    /// Triangles `(vertex, p1, miter)` and `(vertex, miter, p2)`.
    Miter,
}

/// One stroke segment in stroke-local space (dashes already applied).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuStrokeSegment {
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub start_cap: GpuCap,
    pub end_cap: GpuCap,
}

/// One join in stroke-local space with its exact precomputed hit geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuStrokeJoin {
    pub vertex: [f64; 2],
    pub kind: GpuJoinKind,
    /// Triangle corner `p1` (ignored for `Never` / `Round`).
    pub p1: [f64; 2],
    /// Miter apex (only `Miter`).
    pub miter: [f64; 2],
    pub p2: [f64; 2],
    /// Exact hit box `[x0, y0, x1, y1]`; otherwise `|p - vertex| <= join_reach`
    /// per axis.
    pub tight: Option<[f64; 4]>,
}

/// A stroke exactly as the CPU evaluates it: per sample, a hit against
/// segments and joins in stroke-local space (`vexel-render` `export.rs`
/// documents the semantics; the app lowers `export_stroke` output). The
/// sample is mapped to local space with the inverse of `transform`.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuStroke {
    pub paint: GpuPaint,
    /// Half the stroke width.
    pub radius: f64,
    /// `radius * miter_limit` for miter joins, else `radius`.
    pub join_reach: f64,
    /// Local space -> device space, `[a, b, c, d, e, f]` (shape transform
    /// composed with the view map).
    pub transform: [f64; 6],
    pub segments: Vec<GpuStrokeSegment>,
    pub joins: Vec<GpuStrokeJoin>,
}

/// One filled child of a clip path with its own fill rule (device space,
/// closed rings, like [`GpuShape::edges`]).
#[derive(Debug, Clone, PartialEq)]
pub struct GpuClipChild {
    pub edges: Vec<GpuSegment>,
    pub evenodd: bool,
}

/// One clip path in device space: the UNION of its filled children, each
/// under its own rule (a clip with no children contains nothing). A chain of
/// clip paths (`RenderClip` parent chain, group clips, shape clip) is an
/// INTERSECTION. Clips are shared by `Arc`: the encoder stores the edges of
/// one `Arc` once however many shapes use it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GpuClipPath {
    pub children: Vec<GpuClipChild>,
}

impl GpuClipPath {
    /// A clip of one child.
    pub fn single(edges: Vec<GpuSegment>, evenodd: bool) -> Self {
        Self {
            children: vec![GpuClipChild { edges, evenodd }],
        }
    }
}

/// A leaf shape.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuShape {
    pub edges: Vec<GpuSegment>,
    pub evenodd: bool,
    pub fill: Option<GpuPaint>,
    pub stroke: Option<GpuStroke>,
    /// Clip chain, outermost first (empty when unclipped).
    pub clips: Vec<Arc<GpuClipPath>>,
}

/// An opacity group. Its clips apply to every shape below it, exactly like
/// the CPU (`group.rs`: the clip joins each child's chain; the group layer
/// itself is not clipped), whether or not the group is layered.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuGroup {
    pub opacity: f64,
    pub clips: Vec<Arc<GpuClipPath>>,
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
