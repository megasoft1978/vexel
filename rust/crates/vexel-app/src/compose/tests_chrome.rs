//! Document-driven chrome and inspector, driven headlessly through the real
//! open path (session + compose view), without a window.
use super::tests::fixture;
use super::tests_open::{TempDir, wait_for};
use super::*;
use crate::chrome::{AppState, Inspector, NO_DOCUMENT, NO_SELECTION, ShapeProps, chrome_text};
use crate::session::AppDocument;
use vexel_ui::Rect;

const STAGE: Rect = Rect {
    x: 60,
    y: 48,
    width: 320,
    height: 240,
};

fn opened(dir: &TempDir) -> (DocumentView, AppDocument) {
    let path = dir.file("compose-v1.svg", &fixture("compose-v1.svg"));
    let mut view = DocumentView::new();
    let mut app = AppDocument::new(10, 10).unwrap();
    view.open(&mut app, &path, (STAGE.width, STAGE.height));
    wait_for(&mut view, |v| v.composed().is_some());
    (view, app)
}

/// Window position of a document point (the fixture is 80x60 units drawn at
/// 4 canvas px per unit in the 320x240 stage).
fn at(x: f64, y: f64) -> (u32, u32) {
    (STAGE.x + (x * 4.0) as u32, STAGE.y + (y * 4.0) as u32)
}

#[test]
fn chrome_after_open_names_file_size_and_shapes_and_survives_a_failed_open() {
    let dir = TempDir::new();
    let (mut view, mut app) = opened(&dir);
    let text = chrome_text(&AppState::from_session(&app, &view));
    for shown in [&text.title, &text.top, &text.status] {
        assert!(shown.contains("compose-v1.svg"), "{shown}");
        assert!(!shown.contains(NO_DOCUMENT), "{shown}");
    }
    for shown in [&text.top, &text.status] {
        assert!(
            shown.contains("160x120"),
            "document size, not stage: {shown}"
        );
        assert!(shown.contains("7 shapes"), "{shown}");
    }
    assert!(text.status.contains(&dir.0.display().to_string()), "path");
    assert_eq!(text.inspector, Inspector::NoSelection);
    view.open(&mut app, &dir.0.join("missing.svg"), (320, 240));
    let after = chrome_text(&AppState::from_session(&app, &view));
    assert!(
        after.status.contains("Cannot open missing.svg"),
        "{}",
        after.status
    );
    assert!(after.status.contains("compose-v1.svg") && after.status.contains("7 shapes"));
    assert!(after.top.contains("compose-v1.svg") && after.title.contains("compose-v1.svg"));
}

#[test]
fn chrome_shows_the_dirty_marker_after_an_edit() {
    let dir = TempDir::new();
    let (view, mut app) = opened(&dir);
    app.edit(|d| d.add_rectangle_with_id(1.0, 1.0, 5.0, 5.0, "#112233", "r-1"))
        .unwrap();
    assert!(app.is_dirty());
    let text = chrome_text(&AppState::from_session(&app, &view));
    assert!(text.title.contains('\u{2022}') && text.top.contains('\u{2022}'));
    assert!(text.status.contains("modified"));
}

type Case = ((f64, f64), usize, Option<[u8; 4]>, Option<[u8; 4]>, f64);

#[test]
fn select_pick_feeds_the_inspector_from_the_document_shape_and_deselect_clears_it() {
    let dir = TempDir::new();
    let (mut view, app) = opened(&dir);
    // (document point, scene index, fill, stroke, stroke width) per compose-v1.svg.
    let cases: [Case; 3] = [
        (
            (14.0, 11.0),
            0,
            Some([0xe5, 0x39, 0x35, 255]),
            Some([0x1a, 0x23, 0x7e, 255]),
            2.0,
        ),
        ((40.0, 12.0), 1, Some([0x43, 0xa0, 0x47, 255]), None, 0.0),
        (
            (64.0, 12.0),
            2,
            Some([0xfd, 0xd8, 0x35, 255]),
            Some([0, 0, 0, 255]),
            1.0,
        ),
    ];
    for (point, index, fill, stroke, width) in cases {
        assert!(view.pointer_with(STAGE, at(point.0, point.1), true));
        let state = AppState::from_session(&app, &view);
        let props = state.selection.expect("a pick selects the shape");
        let doc_shape = view
            .composed()
            .unwrap()
            .shapes
            .iter()
            .find(|s| s.scene_index == index)
            .unwrap();
        assert_eq!(props, ShapeProps::from_shape(doc_shape));
        assert_eq!(
            (props.index, props.fill, props.stroke),
            (index, fill, stroke)
        );
        assert_eq!(props.stroke_width, width);
        assert_eq!(props.transform, doc_shape.transform);
        let chrome = chrome_text(&AppState {
            open: true,
            ..state
        });
        assert_eq!(chrome.inspector, Inspector::Shape(props));
    }
    // The nested rotated group carries its composed transform.
    assert!(view.pointer_with(STAGE, at(18.0, 51.0), true));
    let props = ShapeProps::from_shape(view.selected_shape().unwrap());
    let (sin, cos) = 20f64.to_radians().sin_cos();
    let t = props.transform;
    for (got, want) in [
        (t.a, 0.5 * cos),
        (t.b, 0.5 * sin),
        (t.c, -0.5 * sin),
        (t.d, 0.5 * cos),
        (t.e, 12.0),
        (t.f, 46.0),
    ] {
        assert!((got - want).abs() < 1e-9, "{t:?}");
    }
    assert_eq!(props.fill, Some([0x39, 0x49, 0xab, 255]));
    // Other tools never change the selection; empty canvas clears it.
    assert!(view.pointer_with(STAGE, at(40.0, 12.0), false));
    assert_eq!(view.selected_shape().map(|s| s.scene_index), Some(6));
    assert!(view.pointer_with(STAGE, at(45.0, 55.0), true));
    assert!(view.selected_shape().is_none());
    let state = AppState {
        open: true,
        ..AppState::from_session(&app, &view)
    };
    assert_eq!(chrome_text(&state).inspector, Inspector::NoSelection);
    assert_eq!(NO_SELECTION, "No selection");
}
