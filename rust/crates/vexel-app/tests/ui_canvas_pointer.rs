//! WO-RUST-UI-CANVAS-01 acceptance 2 (hit-testing through the view) and
//! 7 (zoom performance and the recompose counter).
#[path = "ui_canvas_support.rs"]
mod support;

use std::time::{Duration, Instant};
use support::{COMPOSE_V1, Scene, composed, render};
use vexel_app::canvas::{View, ViewMap};
use vexel_app::compose::DocumentView;
use vexel_app::tools::CanvasMap;
use vexel_json::JsonDocument;
use vexel_ui::Rect;

const STAGE: Rect = Rect {
    x: 60,
    y: 48,
    width: 420,
    height: 300,
};

/// What the shell loop does each iteration: ask for the compose size and show
/// when it is not the current one, then wait for the compose to land.
fn settle(view: &mut DocumentView, backing: u32) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let size = view.compose_target(STAGE, backing);
        if !view.is_current(1, size) {
            view.show(COMPOSE_V1, 1, size);
        }
        while view.composed().is_none() {
            assert!(Instant::now() < deadline, "compose timed out");
            view.poll();
            std::thread::sleep(Duration::from_millis(2));
        }
        if view.is_current(1, size) && view.compose_target(STAGE, backing) == size {
            return;
        }
    }
}

fn window(map: &ViewMap, doc: (f64, f64)) -> (u32, u32) {
    let p = map.doc_to_view(doc.0, doc.1);
    (
        (p.0.round() as u32) + STAGE.x,
        (p.1.round() as u32) + STAGE.y,
    )
}

#[test]
fn identity_view_map_equals_the_compose_point_map() {
    let doc = composed(COMPOSE_V1, (320, 240));
    let map = ViewMap::new(View::IDENTITY, (320.0, 240.0), (320, 240), doc.map).unwrap();
    for y in (0..240).step_by(7) {
        for x in (0..320).step_by(5) {
            let (x, y) = (f64::from(x) + 0.25, f64::from(y) + 0.75);
            assert_eq!(map.to_doc(x, y), doc.map.canvas_to_doc(x, y), "{x},{y}");
        }
    }
    assert_eq!(map.to_doc(-0.5, 3.0), None);
    assert_eq!(map.to_doc(320.0, 3.0), None);
}

#[test]
fn a_click_at_200_percent_with_pan_selects_what_the_identity_view_selects() {
    let mut view = DocumentView::new();
    view.set_name("compose-v1.svg");
    settle(&mut view, 1);
    // Document (viewBox) points and the scene index of the shape there.
    let cases: [((f64, f64), Option<usize>); 6] = [
        ((14.0, 11.0), Some(0)),
        ((40.0, 12.0), Some(1)),
        ((64.0, 12.0), Some(2)),
        ((18.0, 51.0), Some(6)),
        ((70.0, 40.0), Some(5)),
        ((45.0, 55.0), None),
    ];
    let pick = |view: &mut DocumentView, point: (f64, f64)| {
        let at = window(&view.view_map().unwrap(), point);
        assert!(
            view.pointer_with(STAGE, at, true),
            "{point:?} on the artboard"
        );
        view.selected_shape().map(|s| s.scene_index)
    };
    let fit: Vec<_> = cases.iter().map(|(p, _)| pick(&mut view, *p)).collect();
    assert_eq!(fit, cases.iter().map(|(_, i)| *i).collect::<Vec<_>>());
    let natural = view.artboard().unwrap();
    assert_eq!(natural, (160.0, 120.0));

    view.canvas.fit = false;
    view.canvas.view = View {
        zoom: 2.0,
        pan: (50.0, 50.0),
    };
    let zoomed: Vec<_> = cases.iter().map(|(p, _)| pick(&mut view, *p)).collect();
    assert_eq!(zoomed, fit, "same elements at 200% with a 50 px pan");
    // The mapped screen points really moved: the view is not the fit view.
    let a = window(&view.view_map().unwrap(), (14.0, 11.0));
    assert_eq!(a, (STAGE.x + 50 + 56, STAGE.y + 50 + 44));
    // Outside the artboard nothing is hit.
    assert!(!view.pointer_with(STAGE, (STAGE.x + 20, STAGE.y + 20), true));
}

#[test]
fn view_changes_recompose_only_beyond_twice_the_cached_resolution() {
    let mut view = DocumentView::new();
    view.set_name("compose-v1.svg");
    settle(&mut view, 1);
    let base = view.compose_requests();
    assert!(base >= 1);
    // Pin updated (POLISH-01): Fit now magnifies up to 400%, so the zoom
    // factors below are relative to the fitted zoom instead of absolute.
    let z0 = view.canvas.view.zoom;
    let size = view.compose_target(STAGE, 1);

    // Zoom within the cached band, and pan: no compose at all.
    for zoom in [1.5, 1.9, 0.6, 1.0].map(|z| z * z0) {
        view.canvas
            .zoom_by((200.0, 150.0), zoom / view.canvas.view.zoom);
        let target = view.compose_target(STAGE, 1);
        assert_eq!(target, size, "zoom {zoom}");
        assert!(view.is_current(1, target));
    }
    view.canvas.pan_by((300.0, -80.0));
    assert_eq!(view.compose_target(STAGE, 1), size);
    assert_eq!(view.compose_requests(), base);

    // Beyond 2x the cache is replaced once, then 2.5x is served from the new one.
    view.canvas
        .zoom_by((200.0, 150.0), 2.5 * z0 / view.canvas.view.zoom);
    let bigger = view.compose_target(STAGE, 1);
    assert!(bigger.0 > size.0 * 2 - 3, "{bigger:?} vs {size:?}");
    settle(&mut view, 1);
    assert_eq!(view.compose_requests(), base + 1);
    view.canvas
        .zoom_by((200.0, 150.0), 4.0 * z0 / view.canvas.view.zoom);
    assert_eq!(
        view.compose_target(STAGE, 1),
        bigger,
        "4x is within 2x of the 2.5x cache"
    );
    assert_eq!(view.compose_requests(), base + 1);
    // A Retina backing doubles the wanted resolution: one recompose.
    settle(&mut view, 2);
    assert_eq!(view.compose_requests(), base + 2);
}

#[test]
fn a_thousand_pan_events_present_within_the_frame_budget() {
    let mut view = DocumentView::new();
    view.set_name("compose-v1.svg");
    settle(&mut view, 1);
    let base = view.compose_requests();
    let natural = view.artboard().unwrap();
    let shown = view.composed().unwrap();
    let frame = shown.frame.clone();
    let mut scene = Scene {
        window: (640, 440),
        stage: Rect {
            x: 60,
            y: 48,
            width: 520,
            height: 360,
        },
        scale: 1,
        dark: true,
        view: view.canvas.view,
        artboard: natural,
        frame: Some(&frame),
        selection: None,
        hover: None,
    };
    let started = Instant::now();
    for i in 0..1000u32 {
        view.canvas
            .pan_by((((i % 7) as f64) - 3.0, ((i % 5) as f64) - 2.0));
        view.compose_target(STAGE, 1);
        scene.view = view.canvas.view;
        let shot = render(&scene);
        assert_eq!(shot.px.len(), 640 * 440 * 4);
    }
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(view.compose_requests(), base, "panning never composes");
    // Frame budget for 1000 pan presents in a debug build. This is the test's own
    // allowance (15,657 ms, the value it originally borrowed from the compose perf-gate
    // case before WO-RUST-PIPELINE-PERF-01 tightened that baseline to a decode-speed
    // figure that says nothing about presenting frames).
    let max: u64 = 15_657;
    if std::env::var("VEXEL_PERF_PRINT").is_ok_and(|v| v == "1") {
        println!("pan 1000 presents: {elapsed:.1} ms (budget {max} ms)");
    }
    assert!(
        elapsed <= max as f64,
        "1000 pan presents took {elapsed:.0} ms, budget {max} ms"
    );
}
