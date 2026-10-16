use vexel_geometry::{Edge, Point, affine_identity};
use vexel_scene::{Paint, PaintKind, PaintStop, Spread, TableRange};
use vexel_svg_paint::{edge_span, sample};

fn linear(range: TableRange) -> Paint {
    Paint {
        kind: PaintKind::Linear,
        spread: Spread::Pad,
        object_bounding_box: false,
        coordinates: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        bounds: [0.0; 4],
        user_inverse: affine_identity(),
        gradient_inverse: affine_identity(),
        stops: range,
        pattern: None,
        pattern_inverse: affine_identity(),
        pattern_opacity: 1.0,
    }
}

#[test]
fn edge_span_preserves_empty_and_invalid_range_contract() {
    let edges = [Edge {
        a: Point { x: 0.0, y: 0.0 },
        b: Point { x: 1.0, y: 0.0 },
    }];
    let mut formed = true;
    assert!(edge_span(&edges, 0, 0, &mut formed).is_none());
    assert!(!formed);
    assert_eq!(edge_span(&edges, 0, 1, &mut formed).unwrap().len(), 1);
    assert!(formed);
    assert!(edge_span(&edges, usize::MAX, 2, &mut formed).is_none());
    assert!(!formed);
}

#[test]
fn linear_sampling_matches_lround_interpolation_at_midpoint() {
    let stops = [
        PaintStop {
            offset: 0.0,
            rgba: [255, 0, 0, 255],
        },
        PaintStop {
            offset: 1.0,
            rgba: [0, 0, 255, 255],
        },
    ];
    let mut rgba = [9; 4];
    sample(
        &linear(TableRange { first: 0, count: 2 }),
        &stops,
        0.5,
        0.0,
        &mut rgba,
    );
    assert_eq!(rgba, [128, 0, 128, 255]);
}

#[test]
fn exact_gradient_sample_matches_frozen_pixel_channel_oracle() {
    let stops = [
        PaintStop {
            offset: 0.0,
            rgba: [255, 0, 0, 255],
        },
        PaintStop {
            offset: 1.0,
            rgba: [0, 0, 255, 255],
        },
    ];
    let mut paint = linear(TableRange { first: 0, count: 2 });
    paint.object_bounding_box = true;
    paint.bounds = [0.0, 0.0, 16.0, 16.0];
    for (x, expected) in [
        (0.5, [247, 0, 8, 255]),
        (8.5, [120, 0, 135, 255]),
        (15.5, [8, 0, 247, 255]),
    ] {
        let mut rgba = [0; 4];
        sample(&paint, &stops, x, 8.5, &mut rgba);
        assert_eq!(rgba, expected);
    }
}

#[test]
fn invalid_stop_ranges_and_order_leave_destination_untouched() {
    let valid = [
        PaintStop {
            offset: 0.0,
            rgba: [0, 0, 0, 255],
        },
        PaintStop {
            offset: 1.0,
            rgba: [255, 255, 255, 255],
        },
    ];
    let mut rgba = [7; 4];
    sample(
        &linear(TableRange {
            first: usize::MAX,
            count: 2,
        }),
        &valid,
        0.5,
        0.0,
        &mut rgba,
    );
    assert_eq!(rgba, [7; 4]);
    let descending = [valid[1], valid[0]];
    sample(
        &linear(TableRange { first: 0, count: 2 }),
        &descending,
        0.5,
        0.0,
        &mut rgba,
    );
    assert_eq!(rgba, [7; 4]);
}

#[test]
fn repeat_reflect_degenerate_linear_and_invalid_geometry_match_sampler_rules() {
    let stops = [
        PaintStop {
            offset: 0.0,
            rgba: [255, 0, 0, 255],
        },
        PaintStop {
            offset: 1.0,
            rgba: [0, 0, 255, 255],
        },
    ];
    let mut paint = linear(TableRange { first: 0, count: 2 });
    paint.spread = Spread::Repeat;
    let mut rgba = [0; 4];
    sample(&paint, &stops, 1.25, 0.0, &mut rgba);
    assert_eq!(rgba, [191, 0, 64, 255]);
    sample(&paint, &stops, -0.25, 0.0, &mut rgba);
    assert_eq!(rgba, [64, 0, 191, 255]);
    paint.spread = Spread::Reflect;
    sample(&paint, &stops, 1.25, 0.0, &mut rgba);
    assert_eq!(rgba, [64, 0, 191, 255]);
    sample(&paint, &stops, -0.25, 0.0, &mut rgba);
    assert_eq!(rgba, [191, 0, 64, 255]);
    paint.coordinates[2] = 0.0;
    paint.coordinates[3] = 0.0;
    sample(&paint, &stops, 0.0, 0.0, &mut rgba);
    assert_eq!(rgba, [0, 0, 255, 255]);
    paint.coordinates[2] = f64::NAN;
    rgba = [17; 4];
    sample(&paint, &stops, 0.5, 0.0, &mut rgba);
    assert_eq!(rgba, [17; 4]);
}

#[test]
fn radial_focal_degenerate_fallback_and_negative_discriminant_no_write() {
    let stops = [
        PaintStop {
            offset: 0.0,
            rgba: [255, 0, 0, 255],
        },
        PaintStop {
            offset: 1.0,
            rgba: [0, 0, 255, 255],
        },
    ];
    let mut paint = linear(TableRange { first: 0, count: 2 });
    paint.kind = PaintKind::Radial;
    paint.coordinates = [1.0, 0.5, 0.5, 0.5, 0.5, 0.0];
    let mut rgba = [0; 4];
    sample(&paint, &stops, 1.0, 0.5, &mut rgba);
    assert_eq!(rgba, [128, 0, 128, 255]);
    paint.coordinates = [0.5, 0.5, 0.25, 1.0, 0.5, 0.0];
    rgba = [23; 4];
    sample(&paint, &stops, 0.9, 1.0, &mut rgba);
    assert_eq!(rgba, [23; 4]);
}
