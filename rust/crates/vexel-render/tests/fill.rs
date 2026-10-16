//! AC-1: fill rules and exact coverage values.
mod common;
use common::*;
use std::ops::Range;

const RED: [u8; 4] = [255, 0, 0, 255];

#[test]
fn half_pixel_rectangle_covers_8_of_16_samples() {
    // Rows 1 covers y 1.0..1.5 of the rectangle: samples at 0.125 and 0.375 are
    // inside, 0.625 and 0.875 outside -> 8/16 -> alpha 128 of 255.
    let rect = polygon(&[(1.0, 1.0), (3.0, 1.0), (3.0, 2.5), (1.0, 2.5)]);
    let tile = render(&[fill_shape(&rect, RED, false)], 4);
    assert_eq!(pixel(&tile, 1, 1), RED, "fully covered keeps exact colour");
    assert_eq!(pixel(&tile, 2, 2), [255, 0, 0, 128]);
    assert_eq!(pixel(&tile, 1, 2), [255, 0, 0, 128]);
    assert_eq!(pixel(&tile, 0, 1), [0; 4], "empty stays transparent black");
    assert_eq!(pixel(&tile, 1, 3), [0; 4]);
    assert_eq!(pixel(&tile, 3, 1), [0; 4]);
}

#[test]
fn half_pixel_in_x_and_quarter_coverage() {
    let half = polygon(&[(1.0, 1.0), (2.5, 1.0), (2.5, 3.0), (1.0, 3.0)]);
    let tile = render(&[fill_shape(&half, RED, false)], 4);
    assert_eq!(pixel(&tile, 2, 1), [255, 0, 0, 128]);
    // Only the 0.125 column is inside: 4/16 -> (255*4 + 8) / 16 = 64.
    let quarter = polygon(&[(1.0, 1.0), (1.25, 1.0), (1.25, 3.0), (1.0, 3.0)]);
    let tile = render(&[fill_shape(&quarter, RED, false)], 4);
    assert_eq!(pixel(&tile, 1, 1), [255, 0, 0, 64]);
}

#[test]
fn rgb_is_averaged_over_nontransparent_samples_only() {
    // Half coverage of a half-transparent colour: alpha 128 * 8 samples.
    let rect = polygon(&[(0.0, 0.0), (1.0, 0.0), (1.0, 0.5), (0.0, 0.5)]);
    let tile = render(&[fill_shape(&rect, [10, 20, 30, 128], false)], 2);
    // alpha = (128*8 + 8) / 16 = 64; rgb unchanged because empty samples carry no weight.
    assert_eq!(pixel(&tile, 0, 0), [10, 20, 30, 64]);
}

fn star_points() -> Vec<(f64, f64)> {
    // Pentagram {5/2}: vertices visited 0,2,4,1,3 around a circle.
    let vertex = |k: usize| {
        let angle = -std::f64::consts::FRAC_PI_2 + k as f64 * 2.0 * std::f64::consts::PI / 5.0;
        (50.0 + 40.0 * angle.cos(), 50.0 + 40.0 * angle.sin())
    };
    [0, 2, 4, 1, 3].iter().map(|k| vertex(*k)).collect()
}

#[test]
fn self_intersecting_star_nonzero_vs_evenodd() {
    let star = polygon(&star_points());
    let nonzero = render(&[fill_shape(&star, RED, false)], 100);
    let evenodd = render(&[fill_shape(&star, RED, true)], 100);
    // The pentagon at the centre is wound twice: filled by nonzero, a hole for evenodd.
    assert_eq!(pixel(&nonzero, 50, 50), RED);
    assert_eq!(pixel(&evenodd, 50, 50), [0; 4]);
    // A point inside one arm (near the top tip) is wound once: filled by both.
    assert_eq!(pixel(&nonzero, 50, 16), RED);
    assert_eq!(pixel(&evenodd, 50, 16), RED);
    // Outside the star stays empty for both.
    assert_eq!(pixel(&nonzero, 2, 2), [0; 4]);
    assert_eq!(pixel(&evenodd, 97, 97), [0; 4]);
}

fn ring(inner_reversed: bool) -> Vec<vexel_render::FlatEdge> {
    let outer = [(2.0, 2.0), (14.0, 2.0), (14.0, 14.0), (2.0, 14.0)];
    let mut inner = vec![(6.0, 6.0), (10.0, 6.0), (10.0, 10.0), (6.0, 10.0)];
    if inner_reversed {
        inner.reverse();
    }
    let mut edges = polygon(&outer);
    edges.extend(polygon(&inner));
    edges
}

#[test]
fn nested_ring_same_direction_nonzero_fills_evenodd_holes() {
    let edges = ring(false);
    let subpaths: [Range<usize>; 2] = [0..4, 4..8];
    let closed = [true, true];
    let shape = |evenodd| with_subpaths(fill_shape(&edges, RED, evenodd), &subpaths, &closed);
    assert_eq!(pixel(&render(&[shape(false)], 16), 8, 8), RED);
    assert_eq!(pixel(&render(&[shape(true)], 16), 8, 8), [0; 4]);
    assert_eq!(pixel(&render(&[shape(true)], 16), 3, 3), RED, "ring body is filled");
}

#[test]
fn nested_ring_opposite_direction_is_a_hole_for_both_rules() {
    let edges = ring(true);
    let subpaths: [Range<usize>; 2] = [0..4, 4..8];
    let closed = [true, true];
    for evenodd in [false, true] {
        let tile = render(&[with_subpaths(fill_shape(&edges, RED, evenodd), &subpaths, &closed)], 16);
        assert_eq!(pixel(&tile, 8, 8), [0; 4], "evenodd={evenodd}");
        assert_eq!(pixel(&tile, 3, 3), RED);
    }
}

#[test]
fn shapes_composite_source_over_in_document_order() {
    let a = polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    let b = polygon(&[(2.0, 0.0), (4.0, 0.0), (4.0, 4.0), (2.0, 4.0)]);
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    let tile = render(&[fill_shape(&a, red, false), fill_shape(&b, blue, false)], 4);
    assert_eq!(pixel(&tile, 1, 1), red);
    assert_eq!(pixel(&tile, 3, 1), blue);
    let tile = render(&[fill_shape(&b, blue, false), fill_shape(&a, red, false)], 4);
    assert_eq!(pixel(&tile, 3, 1), red);
    // Half-transparent blue over opaque red: C integer arithmetic, alpha stays 255.
    let half = [0, 0, 255, 128];
    let tile = render(&[fill_shape(&a, red, false), fill_shape(&b, half, false)], 4);
    assert_eq!(pixel(&tile, 3, 1), [127, 0, 128, 255]);
}

#[test]
fn unfilled_shape_paints_no_fill_even_with_a_fill_colour() {
    // C excludes unfilled (open line) shapes from fill coverage.
    let edges = polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    let mut shape = fill_shape(&edges, RED, false);
    shape.filled = false;
    let tile = render(&[shape], 4);
    assert!(tile.rgba().iter().all(|b| *b == 0));
}
