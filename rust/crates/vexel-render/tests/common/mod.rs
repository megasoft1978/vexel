//! Shared hand-built shape helpers for the vexel-render tests.
#![allow(dead_code)]

pub mod parity_data;

use std::ops::Range;
use vexel_geometry::affine_identity;
use vexel_raster::Tile;
use vexel_render::{
    FlatEdge, Linecap, Linejoin, RenderError, RenderShape, StrokeStyle, ViewBox, ViewMap,
    render_shapes,
};

pub const NEVER: &dyn Fn() -> bool = &|| false;

pub fn edge(x0: f64, y0: f64, x1: f64, y1: f64) -> FlatEdge {
    FlatEdge { x0, y0, x1, y1 }
}

/// Closed polygon through `points`, one edge per consecutive pair.
pub fn polygon(points: &[(f64, f64)]) -> Vec<FlatEdge> {
    (0..points.len())
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            edge(a.0, a.1, b.0, b.1)
        })
        .collect()
}

/// Open polyline: one edge per consecutive pair, no closing edge.
pub fn polyline(points: &[(f64, f64)]) -> Vec<FlatEdge> {
    points.windows(2).map(|w| edge(w[0].0, w[0].1, w[1].0, w[1].1)).collect()
}

pub fn empty_tile(width: u32, height: u32) -> Tile {
    Tile::from_owned_rgba(width, height, vec![0; width as usize * height as usize * 4])
        .expect("tile")
}

pub fn pixel(tile: &Tile, x: u32, y: u32) -> [u8; 4] {
    let o = (y as usize * tile.width() as usize + x as usize) * 4;
    tile.rgba()[o..o + 4].try_into().unwrap()
}

/// Solid filled shape with no stroke.
pub fn fill_shape<'a>(edges: &'a [FlatEdge], rgba: [u8; 4], evenodd: bool) -> RenderShape<'a> {
    RenderShape {
        edges,
        subpaths: &[],
        subpath_closed: &[],
        stroke_closed: true,
        evenodd,
        filled: true,
        fill_rgba: rgba,
        stroke: None,
        transform: affine_identity(),
    }
}

pub fn style(rgba: [u8; 4], width: f64, cap: Linecap, join: Linejoin, miter_limit: f64) -> StrokeStyle {
    StrokeStyle { rgba, width, miter_limit, dash_offset: 0.0, cap, join, crisp_edges: false }
}

/// Unfilled stroked path.
pub fn stroke_shape<'a>(
    edges: &'a [FlatEdge],
    closed: bool,
    stroke: StrokeStyle,
) -> RenderShape<'a> {
    RenderShape {
        edges,
        subpaths: &[],
        subpath_closed: &[],
        stroke_closed: closed,
        evenodd: false,
        filled: false,
        fill_rgba: [0; 4],
        stroke: Some(stroke),
        transform: affine_identity(),
    }
}

pub fn with_subpaths<'a>(
    mut shape: RenderShape<'a>,
    ranges: &'a [Range<usize>],
    closed: &'a [bool],
) -> RenderShape<'a> {
    shape.subpaths = ranges;
    shape.subpath_closed = closed;
    shape
}

/// Square `size` x `size` tile whose view box is `0 0 size size` (scale 1).
pub fn unit_view(size: u32) -> ViewMap {
    ViewMap::from_viewbox(
        ViewBox { x: 0.0, y: 0.0, width: f64::from(size), height: f64::from(size) },
        size,
        size,
        None,
    )
    .unwrap()
}

pub fn render(shapes: &[RenderShape<'_>], size: u32) -> Tile {
    try_render(shapes, size).expect("render")
}

pub fn try_render(shapes: &[RenderShape<'_>], size: u32) -> Result<Tile, RenderError> {
    let mut tile = empty_tile(size, size);
    render_shapes(shapes, unit_view(size), &mut tile, NEVER)?;
    Ok(tile)
}

/// Samples of a pixel satisfying `inside`, using the 4x4 grid locations.
pub fn count_samples(x: u32, y: u32, inside: impl Fn(f64, f64) -> bool) -> u32 {
    let offsets = [0.125, 0.375, 0.625, 0.875];
    let mut n = 0;
    for oy in offsets {
        for ox in offsets {
            if inside(f64::from(x) + ox, f64::from(y) + oy) {
                n += 1;
            }
        }
    }
    n
}

/// Alpha of an opaque colour covering `n` of 16 samples: round(255 n / 16).
pub fn alpha_for(n: u32) -> u8 {
    ((255 * n + 8) / 16) as u8
}
