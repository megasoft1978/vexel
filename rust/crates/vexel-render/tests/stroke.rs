//! AC-2: stroke width, caps, joins, miter limit, open/closed, degenerate input.
//! Expected values are derived from the geometric definition by counting the
//! 4x4 sample grid, independent of the renderer's own predicates.
mod common;
use common::*;
use std::ops::Range;
use vexel_render::{Linecap, Linejoin};

const RED: [u8; 4] = [255, 0, 0, 255];

fn covered(tile: &vexel_raster::Tile, x: u32, y: u32, n: u32) {
    let expected = if n == 0 { [0; 4] } else { [255, 0, 0, alpha_for(n)] };
    assert_eq!(pixel(tile, x, y), expected, "pixel ({x},{y}) with {n}/16 samples");
}

fn line_tile(cap: Linecap, width: f64) -> vexel_raster::Tile {
    let edges = polyline(&[(4.0, 8.0), (12.0, 8.0)]);
    render(&[stroke_shape(&edges, false, style(RED, width, cap, Linejoin::Miter, 4.0))], 16)
}

#[test]
fn width_covers_exact_band() {
    let tile = line_tile(Linecap::Butt, 4.0);
    for x in 4..12 {
        for y in 6..10 {
            assert_eq!(pixel(&tile, x, y), RED, "({x},{y})");
        }
        assert_eq!(pixel(&tile, x, 5), [0; 4]);
        assert_eq!(pixel(&tile, x, 10), [0; 4]);
    }
    assert_eq!(pixel(&tile, 3, 8), [0; 4], "butt cap does not extend");
    assert_eq!(pixel(&tile, 12, 8), [0; 4]);
}

#[test]
fn odd_width_gives_half_coverage_rows() {
    let tile = line_tile(Linecap::Butt, 3.0);
    // Band y 6.5..9.5: rows 6 and 9 are half covered (8/16 -> 128).
    assert_eq!(pixel(&tile, 6, 6), [255, 0, 0, 128]);
    assert_eq!(pixel(&tile, 6, 9), [255, 0, 0, 128]);
    assert_eq!(pixel(&tile, 6, 7), RED);
    assert_eq!(pixel(&tile, 6, 5), [0; 4]);
}

#[test]
fn square_cap_extends_by_half_width() {
    let tile = line_tile(Linecap::Square, 4.0);
    assert_eq!(pixel(&tile, 2, 8), RED);
    assert_eq!(pixel(&tile, 13, 8), RED);
    assert_eq!(pixel(&tile, 1, 8), [0; 4]);
    assert_eq!(pixel(&tile, 14, 8), [0; 4]);
    assert_eq!(pixel(&tile, 2, 6), RED, "square corner is full");
}

#[test]
fn round_cap_is_a_half_disc() {
    let tile = line_tile(Linecap::Round, 4.0);
    let disc = |cx: f64| move |x: f64, y: f64| (x - cx).hypot(y - 8.0) <= 2.0 && ((cx == 4.0 && x < 4.0) || (cx == 12.0 && x > 12.0));
    for (px, py) in [(2, 6), (2, 7), (3, 7), (2, 9), (3, 6)] {
        covered(&tile, px, py, count_samples(px, py, disc(4.0)));
    }
    for (px, py) in [(12, 6), (13, 7), (13, 9)] {
        covered(&tile, px, py, count_samples(px, py, disc(12.0)));
    }
    assert_eq!(pixel(&tile, 3, 8), RED, "inside the disc");
    assert_ne!(pixel(&tile, 2, 6)[3], 0);
    assert_ne!(pixel(&tile, 2, 6)[3], 255, "corner pixel is partial");
    assert_eq!(pixel(&tile, 1, 8), [0; 4]);
}

/// Open L: down the left side then right along the top. Vertex at (3,5).
fn corner_tile(join: Linejoin, miter_limit: f64) -> vexel_raster::Tile {
    let edges = polyline(&[(3.0, 13.0), (3.0, 5.0), (11.0, 5.0)]);
    render(&[stroke_shape(&edges, false, style(RED, 4.0, Linecap::Butt, join, miter_limit))], 16)
}

#[test]
fn miter_join_fills_the_outer_corner() {
    let tile = corner_tile(Linejoin::Miter, 4.0);
    for (x, y) in [(1, 3), (2, 3), (1, 4), (2, 4)] {
        assert_eq!(pixel(&tile, x, y), RED, "({x},{y})");
    }
    assert_eq!(pixel(&tile, 0, 3), [0; 4]);
}

#[test]
fn bevel_join_cuts_the_outer_corner() {
    let tile = corner_tile(Linejoin::Bevel, 4.0);
    // Bevel triangle: vertex (3,5), (1,5), (3,3); outer edge is x + y = 6.
    let bevel = |x: f64, y: f64| x + y >= 6.0 && x <= 3.0 && y <= 5.0;
    for (px, py) in [(1, 3), (2, 3), (1, 4), (2, 4)] {
        covered(&tile, px, py, count_samples(px, py, bevel));
    }
    assert_eq!(pixel(&tile, 1, 3), [0; 4]);
    assert_eq!(pixel(&tile, 2, 4), RED);
}

#[test]
fn round_join_is_a_disc_at_the_vertex() {
    let tile = corner_tile(Linejoin::Round, 4.0);
    let disc = |x: f64, y: f64| (x - 3.0).hypot(y - 5.0) <= 2.0 && x <= 3.0 && y <= 5.0;
    for (px, py) in [(1, 3), (2, 3), (1, 4), (2, 4)] {
        covered(&tile, px, py, count_samples(px, py, disc));
    }
    assert_ne!(pixel(&tile, 1, 3)[3], 0);
}

/// Acute chevron with apex (8,4); miter ratio = 1 / sin(36.87 deg) = 1.667.
fn chevron(miter_limit: f64) -> vexel_raster::Tile {
    let edges = polyline(&[(2.0, 12.0), (8.0, 4.0), (14.0, 12.0)]);
    render(&[stroke_shape(&edges, false, style(RED, 2.0, Linecap::Butt, Linejoin::Miter, miter_limit))], 16)
}

#[test]
fn miter_limit_crossing_falls_back_to_bevel() {
    let limit_high = chevron(4.0);
    let limit_low = chevron(1.5);
    // Apex of the miter reaches y = 4 - 1.667 = 2.33, so row 2 is touched only by the miter.
    assert_ne!(pixel(&limit_high, 8, 2)[3], 0, "miter above limit ratio draws the spike");
    assert_eq!(pixel(&limit_low, 8, 2), [0; 4], "ratio 1.667 > 1.5 -> bevel, no spike");
    // Just below/above the ratio: 1.667 <= 1.7 keeps the miter, > 1.6 does not.
    assert_ne!(chevron(1.7).rgba(), chevron(1.6).rgba());
    assert_eq!(chevron(1.7).rgba(), limit_high.rgba(), "any limit >= ratio gives the same miter");
    assert_eq!(chevron(1.6).rgba(), limit_low.rgba(), "any limit < ratio gives the same bevel");
}

fn square_loop() -> Vec<vexel_render::FlatEdge> {
    polygon(&[(4.0, 4.0), (12.0, 4.0), (12.0, 12.0), (4.0, 12.0)])
}

#[test]
fn closed_subpath_joins_the_seam_open_does_not() {
    let edges = square_loop();
    let st = style(RED, 2.0, Linecap::Butt, Linejoin::Miter, 4.0);
    let closed = render(&[stroke_shape(&edges, true, st)], 16);
    let open = render(&[stroke_shape(&edges, false, st)], 16);
    // Outer corner square [3,4]x[3,4] at the start/end vertex (4,4).
    assert_eq!(pixel(&closed, 3, 3), RED, "closed: miter join across the seam");
    assert_eq!(pixel(&open, 3, 3), [0; 4], "open: butt ends leave the corner empty");
    // The other three corners are joined in both.
    for tile in [&closed, &open] {
        assert_eq!(pixel(tile, 12, 3), RED);
        assert_eq!(pixel(tile, 12, 12), RED);
        assert_eq!(pixel(tile, 3, 12), RED);
    }
}

#[test]
fn explicit_subpath_closed_flags_are_honoured() {
    let edges = square_loop();
    let ranges: [Range<usize>; 1] = [0..4];
    let st = style(RED, 2.0, Linecap::Butt, Linejoin::Miter, 4.0);
    let closed = [true];
    let tile = render(&[with_subpaths(stroke_shape(&edges, false, st), &ranges, &closed)], 16);
    assert_eq!(pixel(&tile, 3, 3), RED);
    assert_eq!(pixel(&tile, 3, 8), RED, "closed: left side present");
    // An unclosed subpath always owns an implicit fill-closing last edge
    // (C: `edge_count` includes it), which the stroke drops.
    let open = [false];
    let tile = render(&[with_subpaths(stroke_shape(&edges, false, st), &ranges, &open)], 16);
    assert_eq!(pixel(&tile, 3, 8), [0; 4], "open: implicit closing edge (left side) dropped");
    assert_eq!(pixel(&tile, 4, 8), [0; 4]);
    assert_eq!(pixel(&tile, 8, 4), RED);
    assert_eq!(pixel(&tile, 8, 12), RED);
}

#[test]
fn zero_width_or_transparent_stroke_draws_nothing() {
    let edges = polyline(&[(2.0, 8.0), (14.0, 8.0)]);
    for st in [
        style(RED, 0.0, Linecap::Round, Linejoin::Round, 4.0),
        style([255, 0, 0, 0], 4.0, Linecap::Round, Linejoin::Round, 4.0),
    ] {
        let tile = render(&[stroke_shape(&edges, false, st)], 16);
        assert!(tile.rgba().iter().all(|b| *b == 0));
    }
}

#[test]
fn zero_length_segment_follows_the_cap() {
    let edges = vec![edge(8.0, 8.0, 8.0, 8.0)];
    let draw = |cap| render(&[stroke_shape(&edges, false, style(RED, 4.0, cap, Linejoin::Miter, 4.0))], 16);
    let butt = draw(Linecap::Butt);
    assert!(butt.rgba().iter().all(|b| *b == 0), "butt: nothing");
    let square = draw(Linecap::Square);
    assert_eq!(pixel(&square, 6, 6), RED);
    assert_eq!(pixel(&square, 9, 9), RED);
    assert_eq!(pixel(&square, 5, 8), [0; 4]);
    let round = draw(Linecap::Round);
    for (px, py) in [(7, 7), (6, 6), (6, 7), (9, 9)] {
        covered(&round, px, py, count_samples(px, py, |x, y| (x - 8.0).hypot(y - 8.0) <= 2.0));
    }
    assert_eq!(pixel(&round, 7, 7), RED);
    assert_ne!(pixel(&round, 6, 6)[3], 255);
}

#[test]
fn stroke_is_painted_over_fill() {
    let edges = polygon(&[(4.0, 4.0), (12.0, 4.0), (12.0, 12.0), (4.0, 12.0)]);
    let mut shape = fill_shape(&edges, [0, 0, 255, 255], false);
    shape.stroke = Some(style(RED, 2.0, Linecap::Butt, Linejoin::Miter, 4.0));
    let tile = render(&[shape], 16);
    assert_eq!(pixel(&tile, 8, 8), [0, 0, 255, 255], "interior is fill");
    assert_eq!(pixel(&tile, 8, 4), RED, "centre line is stroke over fill");
    assert_eq!(pixel(&tile, 8, 3), RED, "outer half of the stroke");
}
