use vexel_geometry::{affine_identity, Edge, Point};
use vexel_scene::{Paint, PaintKind, PaintStop, Spread, TableRange};
use vexel_svg_paint::{edge_span, sample, GradientCache};

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
    let edges = [Edge { a: Point { x: 0.0, y: 0.0 }, b: Point { x: 1.0, y: 0.0 } }];
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
        PaintStop { offset: 0.0, rgba: [255, 0, 0, 255] },
        PaintStop { offset: 1.0, rgba: [0, 0, 255, 255] },
    ];
    let mut rgba = [9; 4];
    sample(&linear(TableRange { first: 0, count: 2 }), &stops, 0.5, 0.0, &mut rgba);
    assert_eq!(rgba, [128, 0, 128, 255]);
}

#[test]
fn invalid_stop_ranges_and_order_leave_destination_untouched() {
    let valid = [
        PaintStop { offset: 0.0, rgba: [0, 0, 0, 255] },
        PaintStop { offset: 1.0, rgba: [255, 255, 255, 255] },
    ];
    let mut rgba = [7; 4];
    sample(&linear(TableRange { first: usize::MAX, count: 2 }), &valid, 0.5, 0.0, &mut rgba);
    assert_eq!(rgba, [7; 4]);
    let descending = [valid[1], valid[0]];
    sample(&linear(TableRange { first: 0, count: 2 }), &descending, 0.5, 0.0, &mut rgba);
    assert_eq!(rgba, [7; 4]);
}

#[test]
fn gradient_cache_separates_viewport_context_and_owns_stops() {
    let mut cache = GradientCache::default();
    let stops = vec![
        PaintStop { offset: 0.0, rgba: [255, 0, 0, 255] },
        PaintStop { offset: 1.0, rgba: [0, 0, 255, 255] },
    ];
    cache.insert_resolved(7, [16.0, 16.0], linear(TableRange { first: 0, count: 2 }), stops).unwrap();
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get(7, [16.0, 16.0]).unwrap().1.len(), 2);
    assert!(cache.get(7, [32.0, 16.0]).is_none());
    assert!(cache.get(8, [16.0, 16.0]).is_none());
    assert!(cache.allocated_bytes().unwrap() > 0);
}
