//! Dashes on a closed subpath carry their own caps: a dash that starts at the
//! closing point while the previous dash ended before it keeps its square-cap
//! corners (the shape bounds must include them). Expected coverage is
//! computed analytically from the cap rectangle.

mod common;

use common::*;
use vexel_render::{
    DashedShape, FlatEdge, Linecap, Linejoin, ViewBox, ViewMap, render_dashed_shapes,
};

const A: (f64, f64) = (74.5, 16.4);
const B: (f64, f64) = (83.9, 24.7);
const C: (f64, f64) = (85.2, 31.8);
const WIDTH: f64 = 5.33;
/// Device pixels per user unit.
const SCALE: f64 = 6.4;

fn render(cap: Linecap, closed: bool) -> vexel_raster::Tile {
    let mut pts = vec![A, B, C];
    if closed {
        pts.push(A);
    }
    let edges = polyline(&pts);
    let shape = stroke_shape(
        &edges,
        closed,
        style([0, 0, 0, 255], WIDTH, cap, Linejoin::Bevel, 4.0),
    );
    let mut tile = empty_tile(640, 640);
    let view = ViewMap::from_viewbox(
        ViewBox { x: 0.0, y: 0.0, width: 100.0, height: 100.0 },
        640,
        640,
        None,
    )
    .unwrap();
    let item = DashedShape { shape, dashes: &[3.0, 2.0] };
    render_dashed_shapes(&[item], view, &mut tile, &|| false).unwrap();
    tile
}

/// Sample points inside the start-cap rectangle of the first dash, near each
/// of its two outer corners (0.97 of the way from the start point).
fn corner_samples() -> [(f64, f64); 2] {
    let len = (B.0 - A.0).hypot(B.1 - A.1);
    let (dx, dy) = ((B.0 - A.0) / len, (B.1 - A.1) / len);
    let (nx, ny) = (-dy, dx);
    let r = WIDTH / 2.0;
    [1.0, -1.0].map(|s| {
        let corner = (A.0 - dx * r + nx * r * s, A.1 - dy * r + ny * r * s);
        (
            (A.0 + 0.97 * (corner.0 - A.0)) * SCALE,
            (A.1 + 0.97 * (corner.1 - A.1)) * SCALE,
        )
    })
}

#[test]
fn closed_dashed_square_cap_paints_start_corners() {
    let tile = render(Linecap::Square, true);
    for (x, y) in corner_samples() {
        let a = pixel(&tile, x.floor() as u32, y.floor() as u32)[3];
        assert!(a >= 128, "corner ({x:.2},{y:.2}) alpha {a}");
    }
}

#[test]
fn butt_caps_leave_the_corners_empty() {
    let butt = render(Linecap::Butt, true);
    for (x, y) in corner_samples() {
        assert_eq!(pixel(&butt, x.floor() as u32, y.floor() as u32)[3], 0);
    }
}
