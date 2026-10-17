//! WO-RUST-UI-CANVAS-01 acceptance 1 (view math) and 6 (zoom controls).
#[path = "ui_canvas_support.rs"]
mod support;

use support::Lcg;
use vexel_app::canvas::{
    ControlKind, FIT_MARGIN, FIT_MAX_ZOOM, MAX_ZOOM, MIN_ZOOM, View, controls,
};

const STAGES: [(f64, f64); 8] = [
    (260.0, 140.0),
    (386.0, 164.0),
    (640.0, 480.0),
    (1280.0, 800.0),
    (3840.0, 2160.0),
    (1000.0, 200.0),
    (300.0, 2000.0),
    (2000.0, 300.0),
];
const DOCS: [(f64, f64); 9] = [
    (160.0, 120.0),
    (1.0, 1.0),
    (20.0, 20.0),
    (100.0, 500.0),
    (500.0, 100.0),
    (400.0, 300.0),
    (800.0, 600.0),
    (4000.0, 3000.0),
    (30.0, 10.0),
];

#[test]
fn fit_centres_with_margin_and_never_exceeds_400_percent() {
    let mut checked = 0;
    for stage in STAGES {
        for doc in DOCS {
            let limit =
                ((stage.0 - 2.0 * FIT_MARGIN) / doc.0).min((stage.1 - 2.0 * FIT_MARGIN) / doc.1);
            let view = View::fit(stage, doc);
            assert!(
                view.zoom <= FIT_MAX_ZOOM + 1e-12,
                "{stage:?} {doc:?}: {}",
                view.zoom
            );
            if limit < MIN_ZOOM {
                // Below the zoom floor the margin cannot be honoured.
                assert_eq!(view.zoom, MIN_ZOOM);
                continue;
            }
            checked += 1;
            let (x0, y0, x1, y1) = view.artboard(doc);
            for margin in [x0, y0, stage.0 - x1, stage.1 - y1] {
                assert!(
                    margin >= FIT_MARGIN - 1e-6,
                    "{stage:?} {doc:?}: margin {margin}"
                );
            }
            assert!((x0 - (stage.0 - x1)).abs() < 1e-6, "centred x");
            assert!((y0 - (stage.1 - y1)).abs() < 1e-6, "centred y");
            assert!(view.zoom <= limit + 1e-12);
            if limit >= FIT_MAX_ZOOM {
                assert_eq!(view.zoom, FIT_MAX_ZOOM, "Fit magnifies to 400%, no more");
            } else {
                assert!(
                    (view.zoom - limit).abs() < 1e-12,
                    "fills the available space"
                );
            }
        }
    }
    assert!(checked > 40, "{checked}");
}

#[test]
fn zoom_at_keeps_the_point_under_the_pointer_and_clamps() {
    let mut rng = Lcg(7);
    for _ in 0..1000 {
        let view = View {
            zoom: View::clamp_zoom(rng.next() * 40.0),
            pan: (rng.next() * 2000.0 - 1000.0, rng.next() * 2000.0 - 1000.0),
        };
        let point = (rng.next() * 3000.0, rng.next() * 2000.0);
        let target = rng.next() * 60.0;
        let before = view.view_to_doc(point);
        let after = view.zoom_at(point, target);
        let back = after.doc_to_view(before);
        assert!((back.0 - point.0).abs() < 0.5 && (back.1 - point.1).abs() < 0.5);
        assert!((MIN_ZOOM..=MAX_ZOOM).contains(&after.zoom));
    }
    let v = View::IDENTITY;
    assert_eq!(v.zoom_at((0.0, 0.0), 1e-9).zoom, MIN_ZOOM);
    assert_eq!(v.zoom_at((0.0, 0.0), 1e9).zoom, MAX_ZOOM);
    assert_eq!(v.zoom_at((0.0, 0.0), f64::NAN).zoom, 1.0);
    assert_eq!(View::actual((300.0, 200.0), (100.0, 50.0)).zoom, 1.0);
}

#[test]
fn view_to_doc_inverts_doc_to_view_on_seeded_points() {
    let mut rng = Lcg(99);
    for _ in 0..1000 {
        let view = View {
            zoom: View::clamp_zoom(rng.next() * 40.0),
            pan: (rng.next() * 4000.0 - 2000.0, rng.next() * 4000.0 - 2000.0),
        };
        let p = (rng.next() * 5000.0 - 1000.0, rng.next() * 5000.0 - 1000.0);
        let q = view.view_to_doc(view.doc_to_view(p));
        assert!(
            (q.0 - p.0).abs() < 1e-6 && (q.1 - p.1).abs() < 1e-6,
            "{p:?} {q:?}"
        );
    }
}

#[test]
fn zoom_label_is_a_whole_percent() {
    for (zoom, label) in [
        (1.0, "100%"),
        (0.1, "10%"),
        (32.0, "3200%"),
        (0.126, "13%"),
        (1.25, "125%"),
    ] {
        let view = View {
            zoom,
            pan: (0.0, 0.0),
        };
        let cluster = controls(400, 300, view.percent()).unwrap();
        assert_eq!(cluster.label, label);
    }
}

#[test]
fn zoom_cluster_has_28px_targets_stays_in_the_stage_and_clear_of_the_artboard_corner() {
    let doc_sizes = [
        (160.0, 120.0),
        (100.0, 500.0),
        (1000.0, 100.0),
        (120.0, 100.0),
        (1.0, 1.0),
    ];
    for width in [260.0, 261.0, 300.0, 386.0, 640.0, 1280.0, 3840.0] {
        // Stage heights of 180 and up (the chrome order owns the real minimum).
        for height in [180.0, 200.0, 480.0, 2160.0] {
            let cluster = controls(width as u32, height as u32, 100).expect("fits");
            let container = cluster.container;
            assert!(container.x + container.width <= width as u32, "{width}");
            assert!(container.y + container.height <= height as u32, "{height}");
            let kinds: Vec<ControlKind> = cluster.items.iter().map(|c| c.kind).collect();
            assert_eq!(
                kinds,
                [
                    ControlKind::ZoomOut,
                    ControlKind::Label,
                    ControlKind::ZoomIn,
                    ControlKind::Fit
                ]
            );
            for c in &cluster.items {
                assert!(c.rect.width >= 28 && c.rect.height >= 28, "{c:?}");
                assert!(!c.tooltip.is_empty());
                let inside = c.rect.x >= container.x
                    && c.rect.x + c.rect.width <= container.x + container.width
                    && c.rect.y >= container.y
                    && c.rect.y + c.rect.height <= container.y + container.height;
                assert!(inside, "{c:?} in {container:?}");
            }
            for doc in doc_sizes {
                let view = View::fit((width, height), doc);
                let (x0, y0, x1, y1) = view.artboard(doc);
                // The artboard's top-left 100 px square, limited to the artboard.
                let corner = (x0, y0, (x0 + 100.0).min(x1), (y0 + 100.0).min(y1));
                let c = (
                    f64::from(container.x),
                    f64::from(container.y),
                    f64::from(container.x + container.width),
                    f64::from(container.y + container.height),
                );
                let overlap = corner.0 < c.2 && c.0 < corner.2 && corner.1 < c.3 && c.1 < corner.3;
                assert!(
                    !overlap,
                    "{width}x{height} doc {doc:?}: {corner:?} vs {c:?}"
                );
            }
        }
    }
    assert!(
        controls(100, 400, 100).is_none(),
        "too narrow to hold the cluster"
    );
}

#[test]
fn zoom_cluster_hit_testing_maps_points_to_controls() {
    let cluster = controls(600, 400, 100).unwrap();
    for c in &cluster.items {
        let (cx, cy) = (
            f64::from(c.rect.x + c.rect.width / 2),
            f64::from(c.rect.y + c.rect.height / 2),
        );
        assert_eq!(cluster.hit(cx, cy), Some(c.kind));
        assert!(cluster.contains(cx, cy));
    }
    assert_eq!(cluster.hit(300.0, 100.0), None);
    assert!(!cluster.contains(300.0, 100.0));
}
