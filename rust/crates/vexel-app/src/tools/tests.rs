//! AC 1, 2 and 5: drawing gestures and the pen tool.

use super::tests_support::*;
use super::{CanvasMap, Key, MAX_PEN_VERTICES, Outcome, PointerInput, ScaleMap, ToolController};
use crate::model::Tool;
use vexel_v1_document::{V1Document, V1DocumentError, validate_path_data};

fn controller(tool: Tool) -> ToolController {
    let mut c = ToolController::new();
    c.set_tool(tool);
    c
}

#[test]
fn rectangle_ellipse_and_line_drags_add_one_exact_element_through_a_scaled_map() {
    let cases = [
        (
            Tool::Rectangle,
            "<rect x=\"1e1\" y=\"2e1\" width=\"4e1\" height=\"2e1\" fill=\"#4f8cff\" id=\"rect-1\"/>",
        ),
        (
            Tool::Ellipse,
            "<ellipse cx=\"3e1\" cy=\"3e1\" rx=\"2e1\" ry=\"1e1\" fill=\"#4f8cff\" id=\"ellipse-1\"/>",
        ),
        (
            Tool::Line,
            "<line x1=\"1e1\" y1=\"2e1\" x2=\"5e1\" y2=\"4e1\" stroke=\"#1f2937\" stroke-width=\"2e0\" id=\"line-1\"/>",
        ),
    ];
    for (tool, body) in cases {
        let mut doc = blank();
        let mut c = controller(tool);
        let got = one_undoable_entry(&mut doc, |d| {
            assert_eq!(
                drag(&mut c, d, (10.0, 20.0), (50.0, 40.0)),
                Ok(Outcome::Committed)
            );
        });
        assert_eq!(got, wrap(body), "{tool:?}");
        assert!(c.selection().is_none());
    }
}

#[test]
fn reverse_drags_and_repeated_shapes_get_unique_ids() {
    let mut doc = blank();
    let mut c = controller(Tool::Rectangle);
    drag(&mut c, &mut doc, (50.0, 40.0), (10.0, 20.0)).unwrap();
    drag(&mut c, &mut doc, (60.0, 40.0), (90.0, 80.0)).unwrap();
    let text = src(&doc);
    assert!(text.contains(
        "<rect x=\"1e1\" y=\"2e1\" width=\"4e1\" height=\"2e1\" fill=\"#4f8cff\" id=\"rect-1\"/>"
    ));
    assert!(text.contains("id=\"rect-2\""));
    assert_eq!(doc.history_count(), 2);
}

#[test]
fn ids_skip_ids_already_present_in_the_document() {
    let mut doc = with(
        "<rect id=\"rect-1\" x=\"0\" y=\"0\" width=\"5\" height=\"5\"/><rect id=\"rect-2\" x=\"0\" y=\"0\" width=\"5\" height=\"5\"/>",
    );
    let mut c = controller(Tool::Rectangle);
    drag(&mut c, &mut doc, (10.0, 20.0), (50.0, 40.0)).unwrap();
    assert!(src(&doc).contains("id=\"rect-3\""));
}

#[test]
fn clicks_cancels_escape_and_off_canvas_input_add_nothing() {
    for tool in [
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Line,
        Tool::Pen,
        Tool::Select,
    ] {
        let mut doc = blank();
        let before = doc.source().unwrap();
        let mut c = controller(tool);
        let m = map();
        // Click without drag.
        down(&mut c, &mut doc, &m, (20.0, 20.0)).unwrap();
        up(&mut c, &mut doc, &m, (20.0, 20.0)).unwrap();
        // Sub-threshold wobble.
        down(&mut c, &mut doc, &m, (20.0, 20.0)).unwrap();
        mv(&mut c, &mut doc, &m, (20.4, 20.4)).unwrap();
        up(&mut c, &mut doc, &m, (20.4, 20.4)).unwrap();
        // Cancel mid-drag, then a stray Up.
        down(&mut c, &mut doc, &m, (20.0, 20.0)).unwrap();
        mv(&mut c, &mut doc, &m, (80.0, 60.0)).unwrap();
        c.pointer(PointerInput::Cancel, &mut doc, &m).unwrap();
        up(&mut c, &mut doc, &m, (80.0, 60.0)).unwrap();
        // Escape mid-drag, then a stray Up.
        down(&mut c, &mut doc, &m, (20.0, 20.0)).unwrap();
        mv(&mut c, &mut doc, &m, (80.0, 60.0)).unwrap();
        c.key(Key::Escape, &mut doc).unwrap();
        up(&mut c, &mut doc, &m, (80.0, 60.0)).unwrap();
        // Presses outside the artboard never start a gesture.
        assert_eq!(
            c.pointer(PointerInput::Down { x: 0.0, y: 0.0 }, &mut doc, &m),
            Ok(Outcome::Ignored)
        );
        up(&mut c, &mut doc, &m, (80.0, 60.0)).unwrap();
        c.key(Key::Enter, &mut doc).unwrap();
        assert_untouched(&doc, &before, 0);
    }
}

#[test]
fn flat_drags_do_not_create_degenerate_boxes() {
    for tool in [Tool::Rectangle, Tool::Ellipse] {
        let mut doc = blank();
        let before = doc.source().unwrap();
        let mut c = controller(tool);
        assert_eq!(
            drag(&mut c, &mut doc, (10.0, 20.0), (80.0, 20.0)),
            Ok(Outcome::Cancelled)
        );
        assert_eq!(
            drag(&mut c, &mut doc, (10.0, 20.0), (10.0, 90.0)),
            Ok(Outcome::Cancelled)
        );
        assert_untouched(&doc, &before, 0);
    }
}

#[test]
fn a_drag_released_off_canvas_uses_the_last_valid_point() {
    let mut doc = blank();
    let mut c = controller(Tool::Rectangle);
    let m = map();
    down(&mut c, &mut doc, &m, (10.0, 20.0)).unwrap();
    mv(&mut c, &mut doc, &m, (50.0, 40.0)).unwrap();
    assert_eq!(
        mv_raw(&mut c, &mut doc, &m, 5000.0, 5000.0),
        Ok(Outcome::Ignored)
    );
    assert_eq!(
        mv_raw(&mut c, &mut doc, &m, 5000.0, 5000.0),
        Ok(Outcome::Ignored)
    );
    assert_eq!(
        c.pointer(
            PointerInput::Up {
                x: 5000.0,
                y: 5000.0
            },
            &mut doc,
            &m
        ),
        Ok(Outcome::Committed)
    );
    assert!(src(&doc).contains("<rect x=\"1e1\" y=\"2e1\" width=\"4e1\" height=\"2e1\""));
}

fn mv_raw(
    c: &mut ToolController,
    d: &mut V1Document,
    m: &dyn CanvasMap,
    x: f64,
    y: f64,
) -> super::tests_support::Res {
    c.pointer(PointerInput::Move { x, y }, d, m)
}

#[test]
fn switching_tools_abandons_the_gesture() {
    let mut doc = blank();
    let before = doc.source().unwrap();
    let mut c = controller(Tool::Rectangle);
    let m = map();
    down(&mut c, &mut doc, &m, (10.0, 20.0)).unwrap();
    c.set_tool(Tool::Ellipse);
    up(&mut c, &mut doc, &m, (50.0, 40.0)).unwrap();
    assert_untouched(&doc, &before, 0);
}

#[test]
fn pen_three_clicks_and_enter_make_an_m_l_l_path() {
    let mut doc = blank();
    let mut c = controller(Tool::Pen);
    let got = one_undoable_entry(&mut doc, |d| {
        for p in [(10.0, 20.0), (30.0, 40.0), (50.0, 20.0)] {
            assert_eq!(click(&mut c, d, p), Ok(Outcome::Updated));
        }
        assert_eq!(c.pen_vertices().len(), 3);
        assert_eq!(enter(&mut c, d), Ok(Outcome::Committed));
    });
    let d = "M 10 20 L 30 40 L 50 20";
    assert_eq!(
        got,
        wrap(&format!(
            "<path d=\"{d}\" fill=\"none\" stroke=\"#1f2937\" stroke-width=\"2e0\" id=\"path-1\"/>"
        ))
    );
    assert_eq!(validate_path_data(d), Ok(()));
    assert!(c.pen_vertices().is_empty());
}

#[test]
fn pen_click_drag_sets_a_cubic_handle_with_a_mirrored_incoming_control() {
    let mut doc = blank();
    let mut c = controller(Tool::Pen);
    let m = map();
    click(&mut c, &mut doc, (10.0, 20.0)).unwrap();
    // Press at (50,20) and drag to (60,30): out handle (60,30), in handle (40,10).
    down(&mut c, &mut doc, &m, (50.0, 20.0)).unwrap();
    mv(&mut c, &mut doc, &m, (60.0, 30.0)).unwrap();
    up(&mut c, &mut doc, &m, (60.0, 30.0)).unwrap();
    click(&mut c, &mut doc, (90.0, 20.0)).unwrap();
    assert_eq!(enter(&mut c, &mut doc), Ok(Outcome::Committed));
    let d = "M 10 20 C 10 20 40 10 50 20 C 60 30 90 20 90 20";
    assert!(
        src(&doc).contains(&format!("<path d=\"{d}\" fill=\"none\"")),
        "{}",
        src(&doc)
    );
    assert_eq!(validate_path_data(d), Ok(()));
    assert_eq!(doc.history_count(), 1);
}

#[test]
fn pen_close_path_adds_z_and_the_current_fill() {
    let mut doc = blank();
    let mut c = controller(Tool::Pen);
    assert_eq!(c.close_path(&mut doc), Ok(Outcome::Ignored));
    for p in [(10.0, 20.0), (30.0, 40.0)] {
        click(&mut c, &mut doc, p).unwrap();
    }
    assert_eq!(
        c.close_path(&mut doc),
        Ok(Outcome::Ignored),
        "two vertices cannot close"
    );
    assert_eq!(c.pen_vertices().len(), 2);
    click(&mut c, &mut doc, (50.0, 20.0)).unwrap();
    c.set_fill(&mut doc, "#ff8800").unwrap();
    assert_eq!(c.close_path(&mut doc), Ok(Outcome::Committed));
    assert!(src(&doc).contains(
        "<path d=\"M 10 20 L 30 40 L 50 20 L 10 20 Z\" fill=\"#ff8800\" stroke=\"#1f2937\""
    ));
    assert_eq!(doc.history_count(), 1);
}

#[test]
fn pen_double_click_finishes_without_a_duplicate_vertex() {
    let mut doc = blank();
    let mut c = controller(Tool::Pen);
    click(&mut c, &mut doc, (10.0, 20.0)).unwrap();
    click(&mut c, &mut doc, (30.0, 40.0)).unwrap();
    // A native double-click delivers a second Down/Up pair at the same point first.
    assert_eq!(click(&mut c, &mut doc, (30.0, 40.0)), Ok(Outcome::Ignored));
    let (x, y) = canvas((30.0, 40.0));
    assert_eq!(
        c.pointer(PointerInput::DoubleClick { x, y }, &mut doc, &map()),
        Ok(Outcome::Committed)
    );
    assert!(src(&doc).contains("d=\"M 10 20 L 30 40\""));
    assert_eq!(doc.history_count(), 1);
}

#[test]
fn pen_with_fewer_than_two_vertices_or_escape_commits_nothing() {
    let mut doc = blank();
    let before = doc.source().unwrap();
    let mut c = controller(Tool::Pen);
    click(&mut c, &mut doc, (10.0, 20.0)).unwrap();
    assert_eq!(enter(&mut c, &mut doc), Ok(Outcome::Cancelled));
    assert!(c.pen_vertices().is_empty());
    click(&mut c, &mut doc, (10.0, 20.0)).unwrap();
    click(&mut c, &mut doc, (30.0, 40.0)).unwrap();
    click(&mut c, &mut doc, (50.0, 20.0)).unwrap();
    assert_eq!(c.key(Key::Escape, &mut doc), Ok(Outcome::Cancelled));
    assert!(c.pen_vertices().is_empty());
    assert_eq!(enter(&mut c, &mut doc), Ok(Outcome::Ignored));
    assert_untouched(&doc, &before, 0);
}

#[test]
fn pen_vertices_are_capped_and_the_longest_path_is_accepted_by_the_document() {
    let mut doc = V1Document::new(4096, 4096).unwrap();
    let mut c = controller(Tool::Pen);
    let m = ScaleMap::identity(4096.0, 4096.0).unwrap();
    let capacity = c.pen_capacity();
    for i in 0..(MAX_PEN_VERTICES + 50) {
        let x = 4095.999 - (i % 2) as f64 * 4095.0 - i as f64 * 0.001;
        let handle = (0.001 + i as f64 * 0.0001, 4095.999);
        c.pointer(
            PointerInput::Down {
                x,
                y: 3.001 + i as f64,
            },
            &mut doc,
            &m,
        )
        .unwrap();
        c.pointer(
            PointerInput::Move {
                x: handle.0,
                y: handle.1,
            },
            &mut doc,
            &m,
        )
        .unwrap();
        c.pointer(
            PointerInput::Up {
                x: handle.0,
                y: handle.1,
            },
            &mut doc,
            &m,
        )
        .unwrap();
        assert!(c.pen_vertices().len() <= MAX_PEN_VERTICES);
    }
    assert_eq!(c.pen_vertices().len(), MAX_PEN_VERTICES);
    assert_eq!(c.pen_capacity(), capacity);
    assert_eq!(c.close_path(&mut doc), Ok(Outcome::Committed));
    let source = src(&doc);
    let d_start = source.find("d=\"").unwrap() + 3;
    let d = &source[d_start..d_start + source[d_start..].find('"').unwrap()];
    assert!(
        d.len() <= vexel_v1_document::MAX_PATH_DATA_BYTES,
        "{}",
        d.len()
    );
    assert_eq!(validate_path_data(d), Ok(()));
}

#[test]
fn undo_and_redo_restore_exact_bytes_for_every_drawing_action() {
    let mut doc = blank();
    let mut c = controller(Tool::Rectangle);
    one_undoable_entry(&mut doc, |d| {
        drag(&mut c, d, (10.0, 20.0), (50.0, 40.0)).unwrap();
    });
    c.set_tool(Tool::Ellipse);
    one_undoable_entry(&mut doc, |d| {
        drag(&mut c, d, (60.0, 20.0), (90.0, 40.0)).unwrap();
    });
    c.set_tool(Tool::Line);
    one_undoable_entry(&mut doc, |d| {
        drag(&mut c, d, (5.0, 60.0), (95.0, 90.0)).unwrap();
    });
    c.set_tool(Tool::Pen);
    one_undoable_entry(&mut doc, |d| {
        for p in [(100.0, 20.0), (150.0, 60.0), (120.0, 90.0)] {
            click(&mut c, d, p).unwrap();
        }
        enter(&mut c, d).unwrap();
    });
    assert_eq!(doc.history_count(), 4);
}

#[test]
fn document_errors_surface_as_typed_tool_errors_and_leave_no_gesture() {
    // A map that does not clamp can hand the document out-of-bounds geometry.
    struct Wide;
    impl CanvasMap for Wide {
        fn to_doc(&self, x: f64, y: f64) -> Option<(f64, f64)> {
            Some((x, y))
        }
    }
    let mut doc = blank();
    let before = doc.source().unwrap();
    let mut c = controller(Tool::Rectangle);
    c.pointer(PointerInput::Down { x: 10.0, y: 10.0 }, &mut doc, &Wide)
        .unwrap();
    let result = c.pointer(PointerInput::Up { x: 500.0, y: 50.0 }, &mut doc, &Wide);
    assert_eq!(
        result,
        Err(super::ToolError::Document(V1DocumentError::InvalidGeometry))
    );
    assert_untouched(&doc, &before, 0);
    assert_eq!(
        c.pointer(PointerInput::Up { x: 1.0, y: 1.0 }, &mut doc, &Wide),
        Ok(Outcome::Ignored)
    );
}
