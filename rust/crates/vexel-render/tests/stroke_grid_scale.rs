//! S5: a single stroke of about 200k segments (the shape of the corpus file
//! generated-giant_path_t2: a zigzag over a 1600x1000 box) must render within a
//! generous bound. The stroke grid must accept such a path; when it refuses it,
//! every sample scans all primitives and the render takes minutes.
mod common;
use common::*;
use std::time::{Duration, Instant};
use vexel_render::{Linecap, Linejoin, ViewBox, ViewMap, render_shapes};

const SEGMENTS: usize = 200_000;

#[test]
fn stroke_with_200k_segments_renders_within_bound() {
    let points: Vec<(f64, f64)> =
        (0..=SEGMENTS).map(|i| ((i % 1600) as f64, ((i * 37) % 1000) as f64)).collect();
    let edges = polyline(&points);
    let stroke = style([23, 77, 134, 255], 0.35, Linecap::Butt, Linejoin::Miter, 4.0);
    let shape = stroke_shape(&edges, false, stroke);
    let vb = ViewBox { x: 0.0, y: 0.0, width: 1600.0, height: 1000.0 };
    let view = ViewMap::from_viewbox(vb, 128, 128, None).unwrap();
    let mut tile = empty_tile(128, 128);
    let start = Instant::now();
    render_shapes(&[shape], view, &mut tile, NEVER).expect("render");
    let elapsed = start.elapsed();
    eprintln!("MEASURE stroke_200k_segments_ms={}", elapsed.as_millis());
    // Generous: the grid render takes well under a second in release; the
    // debug profile builds and tests the same primitives several times slower.
    let bound =
        if cfg!(debug_assertions) { Duration::from_secs(60) } else { Duration::from_secs(10) };
    assert!(elapsed <= bound, "200k-segment stroke took {elapsed:?}, bound {bound:?}");
}
