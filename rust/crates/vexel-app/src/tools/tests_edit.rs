//! AC 3, 4, 5, 6, 7, 8 and routing: select/move/delete, recolour, preservation, reopen, bounds.

use super::tests_support::*;
use super::{stage_local, Key, Outcome, PointerInput, ScaleMap, ToolController, ToolError};
use crate::model::Tool;
use vexel_ui::Rect;
use vexel_v1_document::{ElementKind, V1Document, V1DocumentError};

const OVERLAP: &str = "<rect id=\"a\" x=\"10\" y=\"10\" width=\"50\" height=\"50\"/>\
    <rect id=\"b\" x=\"30\" y=\"30\" width=\"50\" height=\"50\"/>\
    <path id=\"p\" d=\"M 100 20 L 140 20 L 120 60 Z\" fill=\"#778899\"/>\
    <rect id=\"rot\" x=\"150\" y=\"10\" width=\"10\" height=\"10\" transform=\"rotate(45)\"/>\
    <polygon id=\"poly\" points=\"0,0 1,1 2,0\"/>";

fn selector() -> ToolController {
    ToolController::new()
}

#[test]
fn down_selects_the_topmost_shape_and_drag_moves_it_by_the_mapped_delta() {
    let mut doc = with(OVERLAP);
    let mut c = selector();
    assert_eq!(click(&mut c, &mut doc, (40.0, 40.0)), Ok(Outcome::Updated));
    assert_eq!(
        c.selection(),
        Some("b"),
        "overlap goes to the later (topmost) shape"
    );
    assert_eq!(doc.history_count(), 0, "selecting is not an edit");
    assert_eq!(click(&mut c, &mut doc, (15.0, 15.0)), Ok(Outcome::Updated));
    assert_eq!(c.selection(), Some("b"));

    let got = one_undoable_entry(&mut doc, |d| {
        assert_eq!(
            drag(&mut c, d, (40.0, 40.0), (50.0, 43.5)),
            Ok(Outcome::Committed)
        );
    });
    assert!(
        got.contains("<rect id=\"b\" x=\"4e1\" y=\"3.35e1\" width=\"50\" height=\"50\"/>"),
        "{got}"
    );
    assert!(got.contains("<rect id=\"a\" x=\"10\" y=\"10\""));
    assert_eq!(c.selection(), Some("b"));
}

#[test]
fn dragging_a_path_adds_a_translate_and_keeps_the_d_bytes() {
    let mut doc = with(OVERLAP);
    let mut c = selector();
    let got = one_undoable_entry(&mut doc, |d| {
        drag(&mut c, d, (120.0, 30.0), (125.0, 40.0)).unwrap();
    });
    assert!(got.contains("<path id=\"p\" d=\"M 100 20 L 140 20 L 120 60 Z\" fill=\"#778899\" transform=\"translate(5 10)\"/>"), "{got}");
    // A second move updates the same transform instead of stacking a new one.
    drag(&mut c, &mut doc, (125.0, 40.0), (127.0, 41.0)).unwrap();
    assert!(src(&doc).contains("transform=\"translate(7 11)\"/>"));
    assert_eq!(
        src(&doc).matches("transform=").count(),
        2,
        "one for the rotated rect, one for the path"
    );
}

#[test]
fn delete_removes_the_selection_and_empty_clicks_clear_it() {
    let mut doc = with(OVERLAP);
    let mut c = selector();
    click(&mut c, &mut doc, (40.0, 40.0)).unwrap();
    let got = one_undoable_entry(&mut doc, |d| {
        assert_eq!(c.key(Key::Delete, d), Ok(Outcome::Committed));
    });
    assert!(!got.contains("id=\"b\""));
    assert!(c.selection().is_none());
    assert_eq!(c.key(Key::Delete, &mut doc), Ok(Outcome::Ignored));

    click(&mut c, &mut doc, (15.0, 15.0)).unwrap();
    assert_eq!(c.selection(), Some("a"));
    let count = doc.history_count();
    assert_eq!(
        down(&mut c, &mut doc, &map(), (190.0, 90.0)),
        Ok(Outcome::Updated)
    );
    assert!(c.selection().is_none());
    up(&mut c, &mut doc, &map(), (190.0, 90.0)).unwrap();
    assert_eq!(doc.history_count(), count);
    assert_eq!(
        down(&mut c, &mut doc, &map(), (190.0, 90.0)),
        Ok(Outcome::Ignored)
    );
}

#[test]
fn escape_without_a_gesture_deselects_and_stale_selection_is_dropped_after_undo() {
    let mut doc = blank();
    let mut c = ToolController::new();
    c.set_tool(Tool::Rectangle);
    drag(&mut c, &mut doc, (10.0, 20.0), (50.0, 40.0)).unwrap();
    c.set_tool(Tool::Select);
    click(&mut c, &mut doc, (30.0, 30.0)).unwrap();
    assert_eq!(c.selection(), Some("rect-1"));
    assert_eq!(c.key(Key::Escape, &mut doc), Ok(Outcome::Updated));
    assert!(c.selection().is_none());
    click(&mut c, &mut doc, (30.0, 30.0)).unwrap();
    doc.undo().unwrap();
    assert_eq!(
        c.key(Key::Delete, &mut doc),
        Ok(Outcome::Ignored),
        "the shape no longer exists"
    );
    assert!(c.selection().is_none());
}

#[test]
fn unsupported_targets_and_transforms_return_typed_errors_without_changes() {
    let mut doc = with(OVERLAP);
    let mut c = selector();
    let before = doc.source().unwrap();

    // Non-translate transform: selectable by geometry, move refused.
    click(&mut c, &mut doc, (155.0, 15.0)).unwrap();
    assert_eq!(c.selection(), Some("rot"));
    assert_eq!(
        drag(&mut c, &mut doc, (155.0, 15.0), (165.0, 25.0)),
        Err(ToolError::Document(V1DocumentError::UnsupportedTransform))
    );
    assert_untouched(&doc, &before, 0);

    // An element kind the tools cannot move.
    c.select_id(&doc, "poly").unwrap();
    assert_eq!(
        c.move_selection(&mut doc, 5.0, 5.0),
        Err(ToolError::Document(V1DocumentError::Unsupported))
    );
    assert_eq!(
        c.set_fill(&mut doc, "#000000"),
        Err(ToolError::Document(V1DocumentError::Unsupported))
    );
    assert_eq!(
        c.fill(),
        "#4f8cff",
        "a failed recolour leaves the default fill alone"
    );
    assert_untouched(&doc, &before, 0);

    assert_eq!(
        c.select_id(&doc, "nope"),
        Err(ToolError::Document(V1DocumentError::MissingStableId))
    );
    assert_eq!(c.selection(), Some("poly"));
    // Unsupported-but-selected elements can still be deleted.
    assert_eq!(c.key(Key::Delete, &mut doc), Ok(Outcome::Committed));
    assert!(!src(&doc).contains("polygon"));
}

#[test]
fn set_fill_and_set_stroke_recolour_only_the_selection_and_update_defaults() {
    let mut doc = with(
        "<rect id=\"r\" x=\"10\" y=\"10\" width=\"50\" height=\"50\" fill=\"#000000\" opacity=\"0.5\"/>",
    );
    let mut c = selector();
    click(&mut c, &mut doc, (30.0, 30.0)).unwrap();
    let got = one_undoable_entry(&mut doc, |d| c.set_fill(d, "#ABCDEF").unwrap());
    assert!(got.contains("fill=\"#ABCDEF\" opacity=\"0.5\"/>"), "{got}");
    let got = one_undoable_entry(&mut doc, |d| c.set_stroke(d, "#102030", 3.0).unwrap());
    assert!(
        got.contains("opacity=\"0.5\" stroke=\"#102030\" stroke-width=\"3e0\"/>"),
        "{got}"
    );
    assert_eq!((c.fill(), c.stroke()), ("#ABCDEF", ("#102030", 3.0)));
}

#[test]
fn select_resize_resizes_rect_ellipse_line_and_path_with_one_undo_step() {
    for (body, id, center, handle, end, expected) in [
        (
            "<rect id=\"r\" x=\"10\" y=\"10\" width=\"50\" height=\"40\" fill=\"#000000\"/>",
            "r",
            (20.0, 20.0),
            (60.0, 50.0),
            (80.0, 70.0),
            "width=\"7e1\"",
        ),
        (
            "<ellipse id=\"e\" cx=\"35\" cy=\"30\" rx=\"25\" ry=\"20\" fill=\"#000000\"/>",
            "e",
            (35.0, 30.0),
            (60.0, 50.0),
            (80.0, 70.0),
            "rx=\"3.5e1\"",
        ),
        (
            "<line id=\"l\" x1=\"10\" y1=\"20\" x2=\"60\" y2=\"20\" stroke=\"#000000\"/>",
            "l",
            (35.0, 20.0),
            (60.0, 20.0),
            (80.0, 20.0),
            "x2=\"8e1\"",
        ),
        (
            "<path id=\"p\" d=\"M 10 10 L 60 10 L 60 50 Z\" fill=\"#000000\"/>",
            "p",
            (35.0, 20.0),
            (60.0, 50.0),
            (80.0, 70.0),
            "M 1e1 1e1 L 8e1 1e1 L 8e1 7e1 Z",
        ),
    ] {
        let mut doc = with(body);
        let mut c = ToolController::new();
        click(&mut c, &mut doc, center).unwrap();
        let map = map();
        down(&mut c, &mut doc, &map, handle).unwrap();
        mv(&mut c, &mut doc, &map, end).unwrap();
        let count = doc.history_count();
        assert_eq!(
            up(&mut c, &mut doc, &map, end).unwrap(),
            Outcome::Committed,
            "{id}"
        );
        assert_eq!(doc.history_count(), count + 1, "{id}");
        let after = src(&doc);
        assert!(after.contains(expected), "{id}: {after}");
        doc.undo().unwrap();
        assert!(src(&doc).contains(body), "undo {id}");
    }
}

#[test]
fn inspector_recolor_dispatch_uses_controller_edits_and_is_undoable() {
    let mut doc =
        with("<rect id=\"r\" x=\"10\" y=\"10\" width=\"50\" height=\"40\" fill=\"#000000\"/>");
    let mut c = ToolController::new();
    c.select_id(&doc, "r").unwrap();
    let filled = one_undoable_entry(&mut doc, |d| c.set_fill(d, "#e53935").unwrap());
    assert!(filled.contains("fill=\"#e53935\""));
    let stroked = one_undoable_entry(&mut doc, |d| c.set_stroke(d, "#1e88e5", 3.0).unwrap());
    assert!(stroked.contains("stroke=\"#1e88e5\""));
    assert!(stroked.contains("stroke-width=\"3e0\""));
}

#[test]
fn invalid_colours_widths_and_ids_return_typed_errors_and_keep_source_identical() {
    let mut doc = with("<rect id=\"r\" x=\"10\" y=\"10\" width=\"50\" height=\"50\"/>");
    let mut c = selector();
    click(&mut c, &mut doc, (30.0, 30.0)).unwrap();
    let before = doc.source().unwrap();
    for colour in ["red", "#12345", "#12345g", "", "#1234567"] {
        assert_eq!(
            c.set_fill(&mut doc, colour),
            Err(ToolError::Document(V1DocumentError::InvalidColor))
        );
        assert_eq!(
            c.set_stroke(&mut doc, colour, 1.0),
            Err(ToolError::Document(V1DocumentError::InvalidColor))
        );
    }
    for width in [-0.1, f64::NAN, f64::INFINITY, 257.0] {
        assert_eq!(
            c.set_stroke(&mut doc, "#000000", width),
            Err(ToolError::Document(V1DocumentError::InvalidStrokeWidth))
        );
    }
    assert_eq!(
        c.select_id(&doc, "ghost"),
        Err(ToolError::Document(V1DocumentError::MissingStableId))
    );
    assert_untouched(&doc, &before, 0);
    assert_eq!((c.fill(), c.stroke()), ("#4f8cff", ("#1f2937", 2.0)));
}

#[test]
fn tool_edits_preserve_untouched_bytes_of_a_foreign_fixture_through_undo_redo() {
    let original = PRESERVE.to_vec();
    let mut doc = V1Document::from_source(&original).unwrap();
    let mut c = selector();
    drag(&mut c, &mut doc, (30.0, 30.0), (35.0, 33.0)).unwrap();
    assert!(
        contains(&doc.source().unwrap(), b"x=\"1.5e1\"  y=\"1.3e1\""),
        "{}",
        src(&doc)
    );
    c.set_fill(&mut doc, "#ffffff").unwrap();
    c.set_tool(Tool::Rectangle);
    drag(&mut c, &mut doc, (120.0, 70.0), (160.0, 90.0)).unwrap();
    c.set_tool(Tool::Select);
    c.select_id(&doc, "w1").unwrap();
    assert_eq!(c.key(Key::Delete, &mut doc), Ok(Outcome::Committed));
    click(&mut c, &mut doc, (120.0, 40.0)).unwrap();
    assert_eq!(c.selection(), Some("p1"));
    drag(&mut c, &mut doc, (120.0, 40.0), (130.0, 45.0)).unwrap();
    let last = doc.source().unwrap();

    for keep in [
        &b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- header comment: keep me -->"[..],
        b"xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" xmlns:x=\"urn:example:x\"",
        b"<!-- before rect -->",
        b"<x:note x:keep=\"yes\">unknown element</x:note>",
        b"<linearGradient id=\"g1\"><stop offset=\"0\" stop-color=\"#000000\"/></linearGradient>",
        b"inkscape:label=\"Base\"   x:custom=\"a b\"/>",
        b"<!-- between shapes -->",
        b"<polygon id=\"poly\" points=\"0,0 10,0 5,8\" fill=\"#445566\"/>",
        b"d=\"M 100 20 L 140 20 L 120 60 Z\" fill=\"#778899\"  x:flag=\"1\" transform=\"translate(10 5)\"/>",
        b"<!-- tail comment -->",
    ] {
        assert!(contains(&last, keep), "{}", String::from_utf8_lossy(keep));
    }
    assert!(!contains(&last, b"x:widget"));

    while doc.history_cursor() > 0 {
        doc.undo().unwrap();
    }
    assert_eq!(doc.source().unwrap(), original);
    assert!(!doc.is_dirty());
    while doc.history_cursor() < doc.history_count() {
        doc.redo().unwrap();
    }
    assert_eq!(doc.source().unwrap(), last);
}

#[test]
fn a_document_built_only_through_tools_reopens_with_equal_bytes_and_ids() {
    let mut doc = blank();
    let mut c = ToolController::new();
    c.set_tool(Tool::Rectangle);
    drag(&mut c, &mut doc, (10.0, 20.0), (50.0, 40.0)).unwrap();
    c.set_tool(Tool::Ellipse);
    drag(&mut c, &mut doc, (60.0, 20.0), (90.0, 40.0)).unwrap();
    c.set_tool(Tool::Line);
    drag(&mut c, &mut doc, (5.0, 60.0), (95.0, 90.0)).unwrap();
    c.set_tool(Tool::Pen);
    for p in [(100.0, 20.0), (150.0, 60.0), (120.0, 90.0)] {
        click(&mut c, &mut doc, p).unwrap();
    }
    c.close_path(&mut doc).unwrap();
    c.set_tool(Tool::Select);
    click(&mut c, &mut doc, (30.0, 30.0)).unwrap();
    c.set_fill(&mut doc, "#00ff00").unwrap();
    drag(&mut c, &mut doc, (30.0, 30.0), (35.0, 35.0)).unwrap();
    click(&mut c, &mut doc, (150.0, 60.0)).unwrap();
    c.set_stroke(&mut doc, "#ff0000", 4.0).unwrap();
    click(&mut c, &mut doc, (70.0, 30.0)).unwrap();
    assert_eq!(c.key(Key::Delete, &mut doc), Ok(Outcome::Committed));

    let bytes = doc.source().unwrap();
    let reopened = V1Document::from_source(&bytes).unwrap();
    assert_eq!(reopened.source().unwrap(), bytes);
    assert!(!reopened.is_dirty());
    assert_eq!(reopened.history_count(), 0);
    for (id, kind) in [
        ("rect-1", ElementKind::Rect),
        ("line-1", ElementKind::Line),
        ("path-1", ElementKind::Path),
    ] {
        assert_eq!(reopened.element_kind(id), Ok(kind), "{id}");
    }
    assert_eq!(
        reopened.element_kind("ellipse-1"),
        Err(V1DocumentError::MissingStableId)
    );
}

#[test]
fn ten_thousand_synthetic_events_keep_preview_state_bounded_and_commit_nothing() {
    let mut doc = with(OVERLAP);
    let before = doc.source().unwrap();
    let m = ScaleMap::identity(200.0, 100.0).unwrap();
    let mut c = ToolController::new();
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % 2001) as f64 / 10.0
    };
    let mut events = 0usize;
    let mut feed = |c: &mut ToolController, doc: &mut V1Document, event: PointerInput| {
        events += 1;
        let _ = c.pointer(event, doc, &m);
        assert!(c.pen_vertices().len() <= super::MAX_PEN_VERTICES);
    };
    let capacity = c.pen_capacity();
    // Pen: every press/release adds a vertex until the cap, then is ignored.
    c.set_tool(Tool::Pen);
    for i in 0..2500 {
        let (x, y) = (next(), next());
        feed(
            &mut c,
            &mut doc,
            [
                PointerInput::Down { x, y },
                PointerInput::Move { x: y, y: x },
                PointerInput::Move { x, y },
                PointerInput::Up { x, y },
            ][i % 4],
        );
    }
    assert_eq!(c.pen_vertices().len(), super::MAX_PEN_VERTICES);
    assert_eq!(c.pen_capacity(), capacity);
    c.key(Key::Escape, &mut doc).unwrap();
    // Drag tools: long move storms that always end in Cancel.
    for tool in [Tool::Rectangle, Tool::Line, Tool::Select] {
        c.set_tool(tool);
        for i in 0..2500 {
            let (x, y) = (next(), next());
            feed(
                &mut c,
                &mut doc,
                [
                    PointerInput::Down { x, y },
                    PointerInput::Move { x: y, y: x },
                    PointerInput::Move { x, y },
                    PointerInput::Cancel,
                ][i % 4],
            );
        }
    }
    assert_eq!(events, 10_000);
    assert!(c.pen_vertices().is_empty());
    assert_eq!(c.pen_capacity(), capacity);
    assert_untouched(&doc, &before, 0);
}

#[test]
fn live_pointer_routing_is_limited_to_the_stage_rectangle() {
    let stage = Rect {
        x: 54,
        y: 48,
        width: 700,
        height: 500,
    };
    assert_eq!(stage_local(stage, (54, 48)), Some((0.0, 0.0)));
    assert_eq!(stage_local(stage, (100, 60)), Some((46.0, 12.0)));
    assert_eq!(stage_local(stage, (753, 547)), Some((699.0, 499.0)));
    for outside in [
        (53, 100),
        (100, 47),
        (754, 100),
        (100, 548),
        (-1, -1),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
    ] {
        assert_eq!(stage_local(stage, outside), None, "{outside:?}");
    }
    // Rail and top-bar points fall outside the stage of a real layout.
    let layout = crate::model::Layout::new(1280, 820, crate::model::Mode::Draw).unwrap();
    assert_eq!(
        stage_local(
            layout.stage,
            (layout.tools[0].x as i32 + 2, layout.tools[0].y as i32 + 2)
        ),
        None
    );
    assert_eq!(
        stage_local(
            layout.stage,
            (layout.draw_mode.x as i32 + 2, layout.draw_mode.y as i32 + 2)
        ),
        None
    );
    assert!(stage_local(
        layout.stage,
        (layout.stage.x as i32 + 5, layout.stage.y as i32 + 5)
    )
    .is_some());
}

#[test]
fn scale_map_rejects_invalid_parameters_and_out_of_artboard_points() {
    use super::CanvasMap;
    assert!(ScaleMap::new((0.0, 0.0), 0.0, (10.0, 10.0)).is_none());
    assert!(ScaleMap::new((f64::NAN, 0.0), 1.0, (10.0, 10.0)).is_none());
    assert!(ScaleMap::new((0.0, 0.0), 1.0, (0.0, 10.0)).is_none());
    let m = map();
    assert_eq!(m.to_doc(10.0, 20.0), Some((0.0, 0.0)));
    assert_eq!(m.to_doc(410.0, 220.0), Some((200.0, 100.0)));
    assert_eq!(m.to_doc(9.0, 20.0), None);
    assert_eq!(m.to_doc(411.0, 20.0), None);
    assert_eq!(m.to_doc(f64::NAN, 20.0), None);
}

#[test]
fn edit_shortcuts_group_duplicate_reorder_and_undo() {
    use super::EditKey;
    let mut doc = with("<rect id=\"a\" x=\"10\" y=\"10\" width=\"20\" height=\"20\"/><rect id=\"b\" x=\"50\" y=\"10\" width=\"20\" height=\"20\"/>");
    let mut c = ToolController::new();
    c.set_tool(Tool::Select);
    c.select_id(&doc, "a").unwrap();
    let before = doc.source().unwrap();
    assert_eq!(
        c.edit_key(EditKey::Duplicate, &mut doc),
        Ok(Outcome::Committed)
    );
    assert_eq!(
        doc.history_count(),
        2,
        "duplicate and offset are two entries"
    );
    let duplicate = c.selection().unwrap().to_owned();
    assert_eq!(
        doc.geometry_bounds(&duplicate).unwrap(),
        (22.0, 22.0, 42.0, 42.0)
    );
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
    c.select_id(&doc, "a").unwrap();
    assert_eq!(c.edit_key(EditKey::Group, &mut doc), Ok(Outcome::Ignored));
    assert_eq!(
        c.edit_key(EditKey::Forward, &mut doc),
        Ok(Outcome::Committed)
    );
}

#[test]
fn group_selects_as_unit_then_second_click_selects_child() {
    let mut doc =
        with("<g id=\"g\"><rect id=\"child\" x=\"10\" y=\"10\" width=\"20\" height=\"20\"/></g>");
    let mut c = ToolController::new();
    c.set_tool(Tool::Select);
    assert_eq!(click(&mut c, &mut doc, (15.0, 15.0)), Ok(Outcome::Updated));
    assert_eq!(c.selection(), Some("g"));
    assert_eq!(click(&mut c, &mut doc, (15.0, 15.0)), Ok(Outcome::Updated));
    assert_eq!(c.selection(), Some("child"));
    assert_eq!(c.key(Key::Escape, &mut doc), Ok(Outcome::Updated));
    assert_eq!(c.selection(), None);
}

#[test]
fn rotation_affordance_previews_and_commits_one_transform_edit() {
    let mut doc = with("<rect id=\"r\" x=\"20\" y=\"20\" width=\"20\" height=\"20\"/>");
    let mut c = ToolController::new();
    c.set_tool(Tool::Select);
    c.select_id(&doc, "r").unwrap();
    let m = map();
    let start = (20.0 - 6.0, 20.0 - 6.0);
    down(&mut c, &mut doc, &m, start).unwrap();
    mv(&mut c, &mut doc, &m, (34.0, 14.0)).unwrap();
    assert!(c.rotate_preview().is_some());
    let count = doc.history_count();
    assert_eq!(
        up(&mut c, &mut doc, &m, (34.0, 14.0)),
        Ok(Outcome::Committed)
    );
    assert_eq!(doc.history_count(), count + 1);
    let after = doc.source().unwrap();
    doc.undo().unwrap();
    assert!(!String::from_utf8(doc.source().unwrap())
        .unwrap()
        .contains("transform="));
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), after);
}
