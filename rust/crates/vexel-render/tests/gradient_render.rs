//! Pixel checks of gradient paint through `render_shapes`: spread methods,
//! bounding-box versus user-space units, radial focal point, gradientTransform
//! and gradient strokes. Expected values are worked out from the geometry; a
//! pixel is the mean of its 16 samples, so a linear ramp reads at the pixel
//! centre within a small tolerance.
mod common;
use common::*;
use vexel_geometry::{Affine, affine_identity};
use vexel_render::{FlatEdge, GradientRef, Linecap, Linejoin};
use vexel_scene::{Paint, PaintKind, PaintStop, Spread, TableRange};

const STOPS: [PaintStop; 2] = [
    PaintStop {
        offset: 0.0,
        rgba: [0, 0, 0, 255],
    },
    PaintStop {
        offset: 1.0,
        rgba: [255, 255, 255, 255],
    },
];

fn scene_paint(kind: PaintKind, coordinates: [f64; 6], spread: Spread) -> Paint {
    Paint {
        kind,
        spread,
        object_bounding_box: false,
        coordinates,
        bounds: [0.0; 4],
        user_inverse: affine_identity(),
        gradient_inverse: affine_identity(),
        stops: TableRange { first: 0, count: 2 },
        pattern: None,
        pattern_inverse: affine_identity(),
        pattern_opacity: 1.0,
    }
}

fn square(size: f64) -> Vec<FlatEdge> {
    polygon(&[(0.0, 0.0), (size, 0.0), (size, size), (0.0, size)])
}

/// Asserts an opaque grey pixel whose red channel is within `tol` of `expected`.
fn grey_near(actual: [u8; 4], expected: i32, tol: i32) {
    assert_eq!(actual[3], 255, "{actual:?} is not opaque");
    assert!(
        actual[0] == actual[1] && actual[1] == actual[2],
        "{actual:?} is not grey"
    );
    assert!(
        (i32::from(actual[0]) - expected).abs() <= tol,
        "{actual:?} expected about {expected}"
    );
}

#[test]
fn pad_holds_the_end_colours_outside_the_ramp() {
    let edges = square(8.0);
    let gradient = scene_paint(
        PaintKind::Linear,
        [2.0, 0.0, 6.0, 0.0, 0.0, 0.0],
        Spread::Pad,
    );
    let mut shape = fill_shape(&edges, [0; 4], false);
    shape.fill_gradient = Some(GradientRef {
        paint: &gradient,
        stops: &STOPS,
    });
    let tile = render(&[shape], 8);
    for y in 0..8 {
        assert_eq!(pixel(&tile, 0, y), [0, 0, 0, 255]);
        assert_eq!(pixel(&tile, 7, y), [255, 255, 255, 255]);
    }
    grey_near(pixel(&tile, 4, 4), 159, 2);
    grey_near(pixel(&tile, 2, 4), 32, 2);
}

#[test]
fn reflect_mirrors_the_ramp_past_its_end() {
    let edges = square(8.0);
    let gradient = scene_paint(
        PaintKind::Linear,
        [0.0, 0.0, 4.0, 0.0, 0.0, 0.0],
        Spread::Reflect,
    );
    let mut shape = fill_shape(&edges, [0; 4], false);
    shape.fill_gradient = Some(GradientRef {
        paint: &gradient,
        stops: &STOPS,
    });
    let tile = render(&[shape], 8);
    grey_near(pixel(&tile, 4, 4), 223, 2);
    grey_near(pixel(&tile, 5, 4), 159, 2);
    grey_near(pixel(&tile, 6, 4), 96, 2);
    grey_near(pixel(&tile, 7, 4), 32, 2);
}

#[test]
fn repeat_restarts_the_ramp_past_its_end() {
    let edges = square(8.0);
    let gradient = scene_paint(
        PaintKind::Linear,
        [0.0, 0.0, 4.0, 0.0, 0.0, 0.0],
        Spread::Repeat,
    );
    let mut shape = fill_shape(&edges, [0; 4], false);
    shape.fill_gradient = Some(GradientRef {
        paint: &gradient,
        stops: &STOPS,
    });
    let tile = render(&[shape], 8);
    grey_near(pixel(&tile, 4, 4), 32, 2);
    grey_near(pixel(&tile, 5, 4), 96, 2);
    grey_near(pixel(&tile, 6, 4), 159, 2);
    grey_near(pixel(&tile, 7, 4), 223, 2);
}

#[test]
fn bounding_box_units_map_through_the_shape_box() {
    // Filled box [2,6] x [2,6]; pixel (3,3) lies inside it.
    let edges = polygon(&[(2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0)]);
    let mut boxed = scene_paint(
        PaintKind::Linear,
        [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        Spread::Pad,
    );
    boxed.object_bounding_box = true;
    boxed.bounds = [2.0, 2.0, 6.0, 6.0];
    let mut shape = fill_shape(&edges, [0; 4], false);
    shape.fill_gradient = Some(GradientRef {
        paint: &boxed,
        stops: &STOPS,
    });
    // (x - 2) / 4 over the pixel: mean 0.375.
    grey_near(pixel(&render(&[shape], 8), 3, 3), 96, 2);

    let user = scene_paint(
        PaintKind::Linear,
        [0.0, 0.0, 8.0, 0.0, 0.0, 0.0],
        Spread::Pad,
    );
    let mut shape = fill_shape(&edges, [0; 4], false);
    shape.fill_gradient = Some(GradientRef {
        paint: &user,
        stops: &STOPS,
    });
    // x / 8 over the pixel: mean 0.4375.
    grey_near(pixel(&render(&[shape], 8), 3, 3), 112, 2);
}

#[test]
fn radial_focal_point_moves_the_ramp() {
    // Circle centre (4,4), radius 4. Pixel (5,4) is about t = 0.595 with the
    // focal point at (2,4) and t = 0.395 with the focal point at the centre.
    let edges = square(8.0);
    let focal = scene_paint(
        PaintKind::Radial,
        [4.0, 4.0, 4.0, 2.0, 4.0, 0.0],
        Spread::Pad,
    );
    let mut shape = fill_shape(&edges, [0; 4], false);
    shape.fill_gradient = Some(GradientRef {
        paint: &focal,
        stops: &STOPS,
    });
    let tile = render(&[shape], 8);
    grey_near(pixel(&tile, 5, 4), 152, 4);
    // Outside the circle every sample is past the end, so the corner is white.
    assert_eq!(pixel(&tile, 0, 0), [255, 255, 255, 255]);

    let centred = scene_paint(
        PaintKind::Radial,
        [4.0, 4.0, 4.0, 4.0, 4.0, 0.0],
        Spread::Pad,
    );
    let mut shape = fill_shape(&edges, [0; 4], false);
    shape.fill_gradient = Some(GradientRef {
        paint: &centred,
        stops: &STOPS,
    });
    grey_near(pixel(&render(&[shape], 8), 5, 4), 101, 4);
}

#[test]
fn gradient_transform_scales_the_ramp() {
    // gradientTransform scale(8) is applied as its inverse, scale(1/8), to the
    // device point; the ramp then runs over x in [0, 8].
    let edges = polygon(&[(2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0)]);
    let mut gradient = scene_paint(
        PaintKind::Linear,
        [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        Spread::Pad,
    );
    gradient.gradient_inverse = Affine {
        a: 0.125,
        b: 0.0,
        c: 0.0,
        d: 0.125,
        e: 0.0,
        f: 0.0,
    };
    let mut shape = fill_shape(&edges, [0; 4], false);
    shape.fill_gradient = Some(GradientRef {
        paint: &gradient,
        stops: &STOPS,
    });
    grey_near(pixel(&render(&[shape], 8), 3, 3), 112, 2);
}

#[test]
fn gradient_stroke_ignores_the_solid_alpha() {
    // Stroke (0,4)-(8,4), width 2, solid colour fully transparent: only the
    // gradient colours it. Pixel (4,3) covers the stroke's upper half.
    let edges = polyline(&[(0.0, 4.0), (8.0, 4.0)]);
    let gradient = scene_paint(
        PaintKind::Linear,
        [0.0, 0.0, 8.0, 0.0, 0.0, 0.0],
        Spread::Pad,
    );
    let st = style([0, 0, 0, 0], 2.0, Linecap::Butt, Linejoin::Miter, 4.0);
    let mut shape = stroke_shape(&edges, false, st);
    shape.stroke_gradient = Some(GradientRef {
        paint: &gradient,
        stops: &STOPS,
    });
    // Mean of x / 8 over [4, 5]: 0.5625.
    grey_near(pixel(&render(&[shape], 8), 4, 3), 143, 2);
}
