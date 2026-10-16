mod support;
use support::*;
use vexel_geometry::*;
use vexel_scene::{ErrorKind, LimitKind};
#[test]
fn original_css_three_stylesheet_limits_and_zero_opacity() {
    for style in [
        format!("<style>{}</style>", "x".repeat(1024 * 1024 + 1)),
        format!("<style>{}</style>", "rect{fill:red;}".repeat(4097)),
        format!("<style>rect{{{}}}</style>", "fill:red;".repeat(16385)),
    ] {
        let e = decode(&format!("<svg width=\"8\" height=\"8\">{style}</svg>")).unwrap_err();
        assert!(matches!(e.kind, ErrorKind::Limit(_)));
        assert_eq!(e.offset, 0);
    }
    let g=decode("<svg width=\"8\" height=\"8\"><style>rect { opacity: 0; }</style><rect width=\"4\" height=\"4\"/></svg>").unwrap();
    assert_eq!(g.root().groups[0].opacity, 0.0);
}
#[test]
fn original_css_rule_match_work_limit() {
    let rules = (0..2048)
        .map(|i| format!(".c{i} {{ fill: red; }}"))
        .collect::<String>();
    let source = format!(
        "<svg width=\"8\" height=\"8\"><style>{rules}</style>{}</svg>",
        "<rect width=\"1\" height=\"1\"/>".repeat(2050)
    );
    let e = decode(&source).unwrap_err();
    assert_eq!(e.kind, ErrorKind::Limit(LimitKind::PaintVisits));
    assert_eq!(e.offset, 0);
}
#[test]
fn original_composition_singular_and_64_affine_samples() {
    let left = Affine {
        a: 1.,
        b: 0.,
        c: 0.,
        d: 1.,
        e: 10.,
        f: 20.,
    };
    let right = Affine {
        a: 2.,
        b: 0.,
        c: 0.,
        d: 3.,
        e: 0.,
        f: 0.,
    };
    let m = affine_compose(left, right).unwrap();
    assert_eq!(
        affine_map_point(m, Point { x: 1., y: 2. }).unwrap(),
        Point { x: 12., y: 26. }
    );
    assert_eq!(
        affine_map_bounds(m, 0., 0., 2., 1.).unwrap(),
        [10., 20., 4., 3.]
    );
    assert_eq!(
        affine_map_bounds(
            Affine {
                a: 0.,
                b: 1.,
                c: -1.,
                d: 0.,
                e: 0.,
                f: 0.
            },
            0.,
            0.,
            2.,
            1.
        )
        .unwrap(),
        [-1., 0., 1., 2.]
    );
    let p = affine_map_point(
        Affine {
            a: 0.,
            b: 0.,
            c: 0.,
            d: 0.,
            e: 1.,
            f: 1.,
        },
        Point { x: 3., y: 4. },
    )
    .unwrap();
    assert_eq!(p, Point { x: 1., y: 1. });
    assert!(
        affine_compose(
            Affine {
                a: f64::NAN,
                ..left
            },
            left
        )
        .is_none()
    );
    for i in 1..=64 {
        let k = i as f64;
        let l = Affine {
            a: 1. + k / 100.,
            b: 0.25,
            c: -0.125,
            d: 0.75 + k / 200.,
            e: k,
            f: -k / 2.,
        };
        let r = Affine {
            a: 0.5 + k / 300.,
            b: -0.2,
            c: 0.4,
            d: 1. + k / 400.,
            e: -k / 3.,
            f: k / 4.,
        };
        let p = Point {
            x: k / 5.,
            y: -k / 7.,
        };
        let inner = affine_map_point(r, p).unwrap();
        let expected = Point {
            x: l.a * inner.x + l.c * inner.y + l.e,
            y: l.b * inner.x + l.d * inner.y + l.f,
        };
        let actual = affine_map_point(affine_compose(l, r).unwrap(), p).unwrap();
        assert!((actual.x - expected.x).abs() <= 1e-10);
        assert!((actual.y - expected.y).abs() <= 1e-10);
    }
}
#[test]
fn original_viewbox_meet_geometry_and_hidden_nested_priority() {
    let g=decode("<svg width=\"16\" height=\"16\" viewBox=\"0 0 32 16\"><rect x=\"0\" y=\"0\" width=\"32\" height=\"16\"/></svg>").unwrap();
    assert_eq!(g.root().viewport.view_width, 32.0);
    assert_eq!(g.root().viewport.view_height, 16.0);
    let g=decode("<svg width=\"8\" height=\"8\"><g display=\"none\" transform=\"broken\"><svg width=\"bad\"><path d=\"broken\"/></svg></g><rect width=\"1\" height=\"1\"/></svg>").unwrap();
    assert_eq!(g.root().shapes.len(), 1);
}
