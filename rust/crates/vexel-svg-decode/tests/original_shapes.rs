#[path = "support/original_shape_inputs.rs"]
mod inputs;
mod support;
use inputs::*;
use support::*;
use vexel_scene::{ErrorKind, LimitKind};

#[test]
fn original_smooth_equivalences_use_real_lexical_animation_and_scene() {
    let pairs = [
        (
            "M1 1 C2 1 3 2 4 2 S6 3 7 4",
            "M1 1 C2 1 3 2 4 2 C5 2 6 3 7 4",
        ),
        (
            "M1 1 c1 0 2 1 3 1 s2 1 3 2 2 1 3 2",
            "M1 1 c1 0 2 1 3 1 c1 0 2 1 3 2 c1 1 2 1 3 2",
        ),
        ("M1 1 Q2 3 4 1 T7 1", "M1 1 Q2 3 4 1 Q6 -1 7 1"),
        ("M1 1 q1 2 3 0 t3 0 3 0", "M1 1 q1 2 3 0 q2 -2 3 0 q1 2 3 0"),
        (
            "M1 1 C2 2 3 3 4 4 L7 7 S8 8 9 9",
            "M1 1 C2 2 3 3 4 4 L7 7 C7 7 8 8 9 9",
        ),
        ("M1 1 Q2 3 4 1 L7 1 T9 1", "M1 1 Q2 3 4 1 L7 1 Q7 1 9 1"),
        (
            "M1 1 C2 2 3 3 4 4 Z S6 6 7 7",
            "M1 1 C2 2 3 3 4 4 Z C1 1 6 6 7 7",
        ),
    ];
    for (a, b) in pairs {
        let a = decode(&format!(
            "<svg width=\"16\" height=\"16\"><path d=\"{a}\"/></svg>"
        ))
        .unwrap_or_else(|e| panic!("left path {a}: {e:?}"));
        let b = decode(&format!(
            "<svg width=\"16\" height=\"16\"><path d=\"{b}\"/></svg>"
        ))
        .unwrap_or_else(|e| panic!("right path {b}: {e:?}"));
        assert_eq!(a.root().edges.len(), b.root().edges.len());
        for (x, y) in a.root().edges.iter().zip(b.root().edges) {
            for (u, v) in [
                (x.a.x, y.a.x),
                (x.a.y, y.a.y),
                (x.b.x, y.b.x),
                (x.b.y, y.b.y),
            ] {
                assert!((u - v).abs() < 1e-12)
            }
        }
    }
}
#[test]
fn original_shape_main_pure_errors_and_offsets() {
    let e = decode(BAD).unwrap_err();
    assert_eq!(e.kind, ErrorKind::Path);
    assert!(e.offset > 0);
    for (s, needle, group, kind) in [
        (MALFORMED_SMOOTH, "S1", "S1 2 3".len(), ErrorKind::Path),
        (MALFORMED_QUAD_SMOOTH, "T1", 2, ErrorKind::Path),
        (SMOOTH_OVERFLOW, "s2", 1, ErrorKind::Number),
        (QUAD_OVERFLOW, "t2", 1, ErrorKind::Number),
    ] {
        let e = decode(s).unwrap_err();
        assert_eq!((e.kind, e.offset), (kind, offset(s, needle) + group));
    }
    for (s, kind) in [
        (LEADING_TRANSFORM_COMMA, ErrorKind::Unsupported),
        (REPEATED_TRANSFORM_COMMA, ErrorKind::Unsupported),
        (ADJACENT_TRANSFORM_NUMBERS, ErrorKind::Unsupported),
        (ADJACENT_POINT_NUMBERS, ErrorKind::Path),
        (EXTREME_ELLIPSE, ErrorKind::Number),
    ] {
        let e = decode(s).unwrap_err();
        assert_eq!(e.kind, kind);
        assert!(e.offset > 0);
    }
}
#[test]
fn original_primitive_geometry_shape_flags_rounding_and_zero_records() {
    let defs = decode(DEFINITIONS).unwrap();
    assert_eq!(defs.root().shapes.len(), 1);
    assert_eq!(defs.root().edges.len(), 4);
    let transformed = decode(TRANSFORMED).unwrap();
    let v = transformed.root();
    assert_eq!(v.shapes.len(), 6);
    assert!(!v.shapes[5].filled);
    assert!(v.edges.iter().any(|e| (e.a.x - 4.0).abs() < 1e-12
        && (e.a.y - 5.0).abs() < 1e-12
        && (e.b.x - 6.0).abs() < 1e-12));
    let line = decode(DIAGONAL_LINE).unwrap();
    assert_eq!(line.root().edges.len(), 1);
    assert!(!line.root().shapes[0].filled);
    let lengths = decode(LENGTHS).unwrap();
    let v = lengths.root();
    assert!((v.edges[0].a.x - 16.0).abs() < 1e-12);
    assert!((v.edges[0].a.y - 96.0 / 2.54).abs() < 1e-12);
    assert!((v.edges[0].b.x - 48.0).abs() < 1e-12);
    assert!((v.edges[1].b.y - (96.0 / 2.54 + 96.0 / 72.0)).abs() < 1e-12);
    assert!((v.edges[v.shapes[1].edges.first].a.x - 4.0).abs() < 1e-12);
    let zero = decode(ZERO_SHAPE).unwrap();
    assert_eq!(zero.root().edges.len(), 0);
    assert_eq!(zero.root().shapes.len(), 2);
    assert!(zero.root().path_numbers.is_empty());
    let blank = decode(BLANK).unwrap();
    assert!(blank.root().shapes.is_empty());
    let overshoot = decode(OVERSHOOT).unwrap();
    assert!(
        overshoot
            .root()
            .edges
            .iter()
            .any(|e| e.a.x > 1.0 || e.b.x > 1.0)
    );
}
#[test]
fn original_eccentric_ellipse_chord_error_bound() {
    let graph = decode(ECCENTRIC_ELLIPSE).unwrap();
    let edges = graph.root().edges;
    assert!(edges.len() <= 4096);
    let mut max_error = 0.0f64;
    for (i, e) in edges.iter().enumerate() {
        for sample in 1..8 {
            let angle = 6.283185307179586 * (i as f64 + sample as f64 / 8.0) / edges.len() as f64;
            let x = 100000.0 + 100000.0 * angle.cos();
            let y = 100000.0 + 1000.0 * angle.sin();
            let dx = e.b.x - e.a.x;
            let dy = e.b.y - e.a.y;
            let t = (((x - e.a.x) * dx + (y - e.a.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            max_error = max_error.max((x - (e.a.x + t * dx)).hypot(y - (e.a.y + t * dy)));
        }
    }
    assert!(max_error <= 0.250001);
}
#[test]
fn original_many_shapes_limit() {
    let mut source = String::from(PATH_START);
    for _ in 0..70000 {
        source.push_str(PATH_MIDDLE);
    }
    source.push_str(PATH_END);
    assert!(matches!(
        decode(&source).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Shapes)
    ));
}
