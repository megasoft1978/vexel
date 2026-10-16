#![forbid(unsafe_code)]
//! Scalar solid-colour shape rasterizer for the ADR-0029 V1 slice.
//!
//! A direct port of the C reference (`src/raster/shape_render*.c`): a 4x4
//! sample grid per pixel, nonzero/evenodd fill, stroke with caps, joins and
//! miter limit, and per-shape source-over compositing in document order.
//! Dashes are available through [`render_dashed_shapes`]; [`render_shapes`] and
//! [`from_scene_shape`] stay solid-only. Non-solid paint, clips, masks, filters, opacity groups, text,
//! animation, SMIL and crisp edges are deferred and reported as
//! [`RenderError::Unsupported`]. No performance claim is made.
//!
//! Cancellation: when the `control` callback returns true, rendering stops at
//! the next pixel row with [`RenderError::Cancelled`]; the tile contents are
//! then unspecified and the caller should discard the tile.

mod adapter;
mod bounds;
mod composite;
mod dash;
mod fill;
mod presentation;
mod render;
mod stroke;
mod validate;
mod view;

pub use adapter::{SceneFeatures, from_scene_shape, from_scene_shape_with_dashes};
pub use dash::MAX_DASH_VALUES;
pub use presentation::{OFFSCREEN_BACKING_SCALE, offscreen_tile_size, render_offscreen};
pub use render::{DashedShape, render_dashed_shapes, render_shapes};
pub use view::{PreserveAspect, ViewBox, ViewMap};

use std::ops::Range;
pub use vexel_geometry::Affine;
pub use vexel_scene::{Linecap, Linejoin};

/// Samples per pixel axis of the anti-aliasing grid.
pub const SAMPLES_PER_AXIS: usize = 4;
/// Largest accepted edge count across all shapes (C `SVG_SHAPE_MAX_EDGES`).
pub const MAX_EDGES: usize = 4 * 1024 * 1024;
/// Largest accepted shape count (C bounds shapes by `SVG_SHAPE_MAX_EDGES`).
pub const MAX_SHAPES: usize = MAX_EDGES;
/// Largest accepted subpath count (a subpath owns at least one edge slot).
pub const MAX_SUBPATHS: usize = MAX_EDGES;
/// Stroke primitive build steps (C `RASTER_DASH_CACHE_MAX_STEPS`).
pub const MAX_STROKE_STEPS: usize = 1_000_000;
/// Stroke primitive byte budget (C `RASTER_DASH_CACHE_BYTE_LIMIT`).
pub const MAX_STROKE_BYTES: usize = 16 * 1024 * 1024;
/// Largest accepted absolute coordinate, as in `vexel_geometry::affine_map_point`.
pub const MAX_COORDINATE: f64 = 1.0e6;

/// One flattened line segment in document (post-transform) space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatEdge {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl From<vexel_geometry::Edge> for FlatEdge {
    fn from(edge: vexel_geometry::Edge) -> Self {
        Self { x0: edge.a.x, y0: edge.a.y, x1: edge.b.x, y1: edge.b.y }
    }
}

/// Solid stroke parameters; mirrors `vexel_scene::Stroke` without
/// `dash_values`, `paint_server` and `paint_order`.
///
/// `dash_offset` is validated as finite (as the C renderer does) and is only
/// used by [`render_dashed_shapes`]. `crisp_edges: true` makes
/// [`render_shapes`] return `Unsupported(CrispEdges)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeStyle {
    pub rgba: [u8; 4],
    pub width: f64,
    pub miter_limit: f64,
    pub dash_offset: f64,
    pub cap: Linecap,
    pub join: Linejoin,
    pub crisp_edges: bool,
}

/// One shape ready to rasterize.
///
/// `edges` are the shape's own edges in document space (after the element
/// transform), exactly the C `doc->edges` slice of one shape. `subpaths` are
/// edge index ranges into `edges` and `subpath_closed` (same length) says
/// whether `closepath` explicitly closed each one; the C reference consumes
/// that flag. When `subpaths` is empty, `stroke_closed` applies to the whole
/// edge list (C `shape_stroke_closed`). `transform` maps local space to
/// document space; strokes are evaluated in local space through its inverse.
/// `filled == false` (open line geometry) paints no fill whatever `fill_rgba`
/// holds, as in C; such a shape only contributes its stroke.
#[derive(Debug, Clone, Copy)]
pub struct RenderShape<'a> {
    pub edges: &'a [FlatEdge],
    pub subpaths: &'a [Range<usize>],
    pub subpath_closed: &'a [bool],
    pub stroke_closed: bool,
    pub evenodd: bool,
    pub filled: bool,
    pub fill_rgba: [u8; 4],
    pub stroke: Option<StrokeStyle>,
    pub transform: Affine,
}

/// A deferred feature that must never be silently skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedKind {
    Dash,
    GradientPaint,
    PatternPaint,
    ImagePaint,
    Clip,
    Mask,
    Filter,
    OpacityGroup,
    Text,
    AnimationOverlay,
    Smil,
    CrispEdges,
    PaintOrder,
}

/// Rendering failure. No variant panics; all are returned to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    Unsupported(UnsupportedKind),
    /// Non-finite, overflowing, out-of-range or inconsistent input.
    InvalidGeometry,
    /// The output tile was not entirely transparent black on entry.
    TileNotEmpty,
    /// An edge, shape, subpath, tile-area or stroke-work bound was exceeded,
    /// or scratch memory could not be reserved.
    LimitExceeded,
    /// The `control` callback requested cancellation.
    Cancelled,
}
