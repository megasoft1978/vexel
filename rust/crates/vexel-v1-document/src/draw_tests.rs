use super::*;

const HEAD: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"100\" viewBox=\"0 0 200 100\"";
const PRESERVE: &[u8] = include_bytes!("../../../fixtures/v1/draw/preserve.svg");

fn blank() -> V1Document {
    V1Document::new(200, 100).unwrap()
}
fn text(doc: &V1Document) -> String {
    String::from_utf8(doc.source().unwrap()).unwrap()
}
fn wrap(body: &str) -> String {
    format!("{HEAD}>{body}</svg>")
}
fn with(body: &str) -> V1Document {
    V1Document::from_source(wrap(body).as_bytes()).unwrap()
}

/// Asserts `op` fails with `expected` and leaves bytes, history and dirty state alone.
fn assert_atomic(
    doc: &mut V1Document,
    expected: V1DocumentError,
    op: impl FnOnce(&mut V1Document) -> Result<(), V1DocumentError>,
) {
    let before = doc.source().unwrap();
    let (count, cursor, dirty) = (doc.history_count(), doc.history_cursor(), doc.is_dirty());
    assert_eq!(op(doc), Err(expected));
    assert_eq!(doc.source().unwrap(), before);
    assert_eq!(
        (doc.history_count(), doc.history_cursor(), doc.is_dirty()),
        (count, cursor, dirty)
    );
}

/// One edit, one history entry; undo/redo restore exact bytes.
fn assert_one_undoable_entry(
    doc: &mut V1Document,
    op: impl FnOnce(&mut V1Document) -> Result<(), V1DocumentError>,
) -> String {
    let before = doc.source().unwrap();
    let count = doc.history_count();
    op(doc).unwrap();
    assert_eq!(doc.history_count(), count + 1);
    assert!(doc.is_dirty());
    let after = doc.source().unwrap();
    assert_ne!(after, before);
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), after);
    String::from_utf8(after).unwrap()
}

#[test]
fn add_line_writes_exact_source_in_one_undoable_entry() {
    let mut doc = blank();
    let got = assert_one_undoable_entry(&mut doc, |d| {
        d.add_line(10.0, 20.0, 40.0, 60.0, "#a1B2c3", 2.0)
    });
    assert_eq!(
        got,
        wrap(
            "<line x1=\"1e1\" y1=\"2e1\" x2=\"4e1\" y2=\"6e1\" stroke=\"#a1B2c3\" stroke-width=\"2e0\" id=\"vexel-object-1\"/>"
        )
    );
    let mut doc = blank();
    doc.add_line_with_id(40.0, 60.0, 10.0, 20.0, "#000000", 0.0, "line-1")
        .unwrap();
    assert!(text(&doc).contains("x1=\"4e1\" y1=\"6e1\" x2=\"1e1\" y2=\"2e1\" stroke=\"#000000\" stroke-width=\"0e0\" id=\"line-1\""));
}

#[test]
fn add_line_rejects_bad_input_atomically() {
    let mut doc = blank();
    for (x1, y1, x2, y2, c, w, e) in [
        (
            f64::NAN,
            0.0,
            1.0,
            1.0,
            "#ffffff",
            1.0,
            V1DocumentError::InvalidGeometry,
        ),
        (
            0.0,
            0.0,
            201.0,
            1.0,
            "#ffffff",
            1.0,
            V1DocumentError::InvalidGeometry,
        ),
        (
            5.0,
            5.0,
            5.0,
            5.0,
            "#ffffff",
            1.0,
            V1DocumentError::InvalidGeometry,
        ),
        (
            0.0,
            0.0,
            1.0,
            1.0,
            "red",
            1.0,
            V1DocumentError::InvalidColor,
        ),
        (
            0.0,
            0.0,
            1.0,
            1.0,
            "none",
            1.0,
            V1DocumentError::InvalidColor,
        ),
        (
            0.0,
            0.0,
            1.0,
            1.0,
            "#ffffff",
            -1.0,
            V1DocumentError::InvalidStrokeWidth,
        ),
        (
            0.0,
            0.0,
            1.0,
            1.0,
            "#ffffff",
            f64::INFINITY,
            V1DocumentError::InvalidStrokeWidth,
        ),
        (
            0.0,
            0.0,
            1.0,
            1.0,
            "#ffffff",
            256.5,
            V1DocumentError::InvalidStrokeWidth,
        ),
    ] {
        assert_atomic(&mut doc, e, |d| d.add_line(x1, y1, x2, y2, c, w));
    }
    assert_atomic(&mut doc, V1DocumentError::InvalidStableId, |d| {
        d.add_line_with_id(0.0, 0.0, 1.0, 1.0, "#ffffff", 1.0, "bad id")
    });
    doc.add_line_with_id(0.0, 0.0, 1.0, 1.0, "#ffffff", 1.0, "dup")
        .unwrap();
    assert_atomic(
        &mut doc,
        V1DocumentError::Edit(EditError::DuplicateId),
        |d| d.add_line_with_id(0.0, 0.0, 2.0, 2.0, "#ffffff", 1.0, "dup"),
    );
}

#[test]
fn add_path_writes_exact_source_and_validates_the_grammar() {
    let mut doc = blank();
    let got = assert_one_undoable_entry(&mut doc, |d| {
        d.add_path_with_id(
            "M 10 20 L 30 40 C 1 2 3 4 5 6 Z",
            "none",
            "#102030",
            2.5,
            "path-1",
        )
    });
    assert_eq!(
        got,
        wrap(
            "<path d=\"M 10 20 L 30 40 C 1 2 3 4 5 6 Z\" fill=\"none\" stroke=\"#102030\" stroke-width=\"2.5e0\" id=\"path-1\"/>"
        )
    );
    for ok in [
        "M0 0L10 10",
        "m 1,2 l3 4 5 6 h1 v2 z",
        "M0 0C1 2 3 4 5 6 7 8 9 10 11 12Z",
        "M-1.5e1-2.5.5.5 H 3 V4",
        "M0,0 10,10 20,0",
        "M0 330 Q90 250 180 325 Q260 235 355 315 L512 512 L0 512Z",
        "M0 0 q1 2 3 4 t5 6 s1 2 3 4 T1 1 Q1 2 3 4 5 6 7 8",
    ] {
        assert_eq!(validate_path_data(ok), Ok(()), "{ok}");
    }
    for bad in [
        "",
        "L 1 2",
        "M 1",
        "M 1 2 3",
        "M 1 2 A 1 1 0 0 0 3 4",
        "M 1 2 Q 1 2 3",
        "M 1 2 S 1 2 3",
        "M 1 2 T 1",
        "M 1 2 Z 3 4",
        "M 1 2 L nan 4",
        "M 1 2 L 1e999 4",
        "M 1 2 L 2e6 4",
        "M 1 2 L 1,,2",
        "M 1 2 L , 1 2",
        "M 1 2 L 1 2 x",
        "M 1 2 L 1e 2",
        "M 1 2 L - 2",
    ] {
        assert_eq!(
            validate_path_data(bad),
            Err(V1DocumentError::InvalidPathData),
            "{bad:?}"
        );
    }
    let mut doc = blank();
    assert_atomic(&mut doc, V1DocumentError::InvalidPathData, |d| {
        d.add_path("M 1 2 A 1 1 0 0 0 3 4", "none", "#000000", 1.0)
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidColor, |d| {
        d.add_path("M 1 2 L 3 4", "red", "#000000", 1.0)
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidColor, |d| {
        d.add_path("M 1 2 L 3 4", "none", "red", 1.0)
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidStrokeWidth, |d| {
        d.add_path("M 1 2 L 3 4", "none", "#000000", -0.5)
    });
}

#[test]
fn path_data_size_and_segment_limits_are_enforced() {
    let at_limit_segments = format!("M 0 0{}", " L 1 1".repeat(MAX_PATH_SEGMENTS - 1));
    assert_eq!(validate_path_data(&at_limit_segments), Ok(()));
    let over_segments = format!("M 0 0{}", " L 1 1".repeat(MAX_PATH_SEGMENTS));
    assert_eq!(
        validate_path_data(&over_segments),
        Err(V1DocumentError::InvalidPathData)
    );
    let huge = format!("M 0 0 {}", "L 1 1 ".repeat(MAX_PATH_DATA_BYTES / 6 + 1));
    assert!(huge.len() > MAX_PATH_DATA_BYTES);
    assert_eq!(
        validate_path_data(&huge),
        Err(V1DocumentError::InvalidPathData)
    );
    let mut doc = blank();
    assert_atomic(&mut doc, V1DocumentError::InvalidPathData, |d| {
        d.add_path(&huge, "none", "#000000", 1.0)
    });
    let long_id = "a".repeat(129);
    assert_atomic(&mut doc, V1DocumentError::InvalidStableId, |d| {
        d.add_path_with_id("M 0 0 L 1 1", "none", "#000000", 1.0, &long_id)
    });
    let max_id = "a".repeat(128);
    doc.add_path_with_id("M 0 0 L 1 1", "none", "#000000", 1.0, &max_id)
        .unwrap();
    assert!(doc.element_kind(&max_id).is_ok());
}

#[test]
fn set_fill_changes_only_the_fill_attribute() {
    let mut doc = with(
        "<rect id=\"r\" x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#000000\" data-k='v'/>",
    );
    let got = assert_one_undoable_entry(&mut doc, |d| d.set_fill("r", "#ABCDEF"));
    assert_eq!(
        got,
        wrap(
            "<rect id=\"r\" x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#ABCDEF\" data-k='v'/>"
        )
    );
    let mut doc = with("<ellipse id=\"e\" cx=\"1\" cy=\"2\" rx=\"3\" ry=\"4\"/>");
    let got = assert_one_undoable_entry(&mut doc, |d| d.set_fill("e", "#010203"));
    assert_eq!(
        got,
        wrap("<ellipse id=\"e\" cx=\"1\" cy=\"2\" rx=\"3\" ry=\"4\" fill=\"#010203\"/>")
    );
    let mut doc = with("<g id=\"g\"/><rect id=\"r\" x=\"1\" y=\"2\" width=\"3\" height=\"4\"/>");
    assert_atomic(&mut doc, V1DocumentError::InvalidColor, |d| {
        d.set_fill("r", "blue")
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidColor, |d| {
        d.set_fill("r", "#12345")
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidColor, |d| {
        d.set_fill("r", "#12345g")
    });
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.set_fill("nope", "#000000")
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidStableId, |d| {
        d.set_fill("no pe", "#000000")
    });
    assert_atomic(&mut doc, V1DocumentError::Unsupported, |d| {
        d.set_fill("g", "#000000")
    });
}

#[test]
fn set_stroke_changes_stroke_and_width_as_one_edit() {
    let mut doc = with(
        "<line id=\"l\" x1=\"0\" y1=\"0\" x2=\"5\" y2=\"5\" stroke=\"#000000\" stroke-width=\"1\"/>",
    );
    let got = assert_one_undoable_entry(&mut doc, |d| d.set_stroke("l", "#ff0000", 4.5));
    assert_eq!(
        got,
        wrap(
            "<line id=\"l\" x1=\"0\" y1=\"0\" x2=\"5\" y2=\"5\" stroke=\"#ff0000\" stroke-width=\"4.5e0\"/>"
        )
    );
    let mut doc = with("<path id=\"p\" d=\"M 0 0 L 4 4\" fill=\"none\" stroke=\"#000000\"/>");
    let got = assert_one_undoable_entry(&mut doc, |d| d.set_stroke("p", "#00ff00", 0.0));
    assert_eq!(
        got,
        wrap(
            "<path id=\"p\" d=\"M 0 0 L 4 4\" fill=\"none\" stroke=\"#00ff00\" stroke-width=\"0e0\"/>"
        )
    );
    let mut doc = with("<circle id=\"c\" cx=\"5\" cy=\"5\" r=\"2\"/>");
    for width in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 256.01] {
        assert_atomic(&mut doc, V1DocumentError::InvalidStrokeWidth, |d| {
            d.set_stroke("c", "#000000", width)
        });
    }
    assert_atomic(&mut doc, V1DocumentError::InvalidColor, |d| {
        d.set_stroke("c", "#00000", 1.0)
    });
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.set_stroke("zz", "#000000", 1.0)
    });
    doc.set_stroke("c", "#000000", 256.0).unwrap();
}

#[test]
fn delete_removes_one_element_and_undo_restores_exact_bytes() {
    let mut doc = with(
        "<rect id=\"a\" x=\"1\" y=\"2\" width=\"3\" height=\"4\"/><!-- keep --><ellipse id=\"b\" cx=\"1\" cy=\"2\" rx=\"3\" ry=\"4\"/>",
    );
    let got = assert_one_undoable_entry(&mut doc, |d| d.delete("a"));
    assert_eq!(
        got,
        wrap("<!-- keep --><ellipse id=\"b\" cx=\"1\" cy=\"2\" rx=\"3\" ry=\"4\"/>")
    );
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.delete("a")
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidStableId, |d| d.delete(""));
}

#[test]
fn translate_updates_coordinates_or_a_translate_transform() {
    let mut doc = with("<rect id=\"r\" x=\"1e1\" y=\"20\" width=\"3\" height=\"4\"/>");
    let got = assert_one_undoable_entry(&mut doc, |d| d.translate("r", 5.0, -2.5));
    assert_eq!(
        got,
        wrap("<rect id=\"r\" x=\"1.5e1\" y=\"1.75e1\" width=\"3\" height=\"4\"/>")
    );
    let mut doc = with("<rect id=\"r\" width=\"3\" height=\"4\"/>");
    let got = assert_one_undoable_entry(&mut doc, |d| d.translate("r", 1.0, 2.0));
    assert_eq!(
        got,
        wrap("<rect id=\"r\" width=\"3\" height=\"4\" x=\"1e0\" y=\"2e0\"/>")
    );
    let mut doc = with(
        "<ellipse id=\"e\" cx=\"10\" cy=\"20\" rx=\"3\" ry=\"4\"/><circle id=\"c\" cx=\"1\" cy=\"2\" r=\"3\"/>",
    );
    let got = assert_one_undoable_entry(&mut doc, |d| d.translate("e", 1.0, 1.0));
    assert!(got.contains("cx=\"1.1e1\" cy=\"2.1e1\""), "{got}");
    let got = assert_one_undoable_entry(&mut doc, |d| d.translate("c", -1.0, 0.0));
    assert!(got.contains("cx=\"0e0\" cy=\"2e0\""), "{got}");
    let mut doc = with("<line id=\"l\" x1=\"0\" y1=\"1\" x2=\"2\" y2=\"3\"/>");
    let got = assert_one_undoable_entry(&mut doc, |d| d.translate("l", 10.0, 10.0));
    assert_eq!(
        got,
        wrap("<line id=\"l\" x1=\"1e1\" y1=\"1.1e1\" x2=\"1.2e1\" y2=\"1.3e1\"/>")
    );
    let mut doc = with(
        "<path id=\"p\" d=\"M 1 2 L 3 4\"/><path id=\"q\" d=\"M 1 2 L 3 4\" transform=\"translate(1 2)\"/>",
    );
    let got = assert_one_undoable_entry(&mut doc, |d| d.translate("p", 5.0, 6.0));
    assert!(
        got.contains("<path id=\"p\" d=\"M 1 2 L 3 4\" transform=\"translate(5 6)\"/>"),
        "{got}"
    );
    let got = assert_one_undoable_entry(&mut doc, |d| d.translate("q", 5.0, 6.0));
    assert!(
        got.contains("d=\"M 1 2 L 3 4\" transform=\"translate(6 8)\"/>"),
        "{got}"
    );
}

#[test]
fn translate_refuses_unsupported_targets_transforms_and_values() {
    let mut doc = with(
        "<polygon id=\"poly\" points=\"0,0 1,1 2,0\"/><rect id=\"rot\" x=\"1\" y=\"1\" width=\"2\" height=\"2\" transform=\"rotate(10)\"/>\
         <path id=\"mix\" d=\"M 0 0 L 1 1\" transform=\"translate(1 2) scale(2)\"/><rect id=\"u\" x=\"1px\" y=\"1\" width=\"2\" height=\"2\"/>",
    );
    assert_atomic(&mut doc, V1DocumentError::Unsupported, |d| {
        d.translate("poly", 1.0, 1.0)
    });
    assert_atomic(&mut doc, V1DocumentError::UnsupportedTransform, |d| {
        d.translate("rot", 1.0, 1.0)
    });
    assert_atomic(&mut doc, V1DocumentError::UnsupportedTransform, |d| {
        d.translate("mix", 1.0, 1.0)
    });
    assert_atomic(&mut doc, V1DocumentError::InvalidGeometry, |d| {
        d.translate("u", 1.0, 1.0)
    });
    assert_atomic(&mut doc, V1DocumentError::MissingStableId, |d| {
        d.translate("none", 1.0, 1.0)
    });
    for (dx, dy) in [(f64::NAN, 0.0), (0.0, f64::INFINITY), (2.0e6, 0.0)] {
        assert_atomic(&mut doc, V1DocumentError::InvalidGeometry, |d| {
            d.translate("rot", dx, dy)
        });
    }
}

#[test]
fn element_kind_classifies_by_tag() {
    let doc = with(
        "<rect id=\"r\"/><ellipse id=\"e\"/><circle id=\"c\"/><line id=\"l\"/><path id=\"p\"/><g id=\"g\"/><text id=\"t\"/><defs id=\"d\"/>",
    );
    for (id, kind) in [
        ("r", ElementKind::Rect),
        ("e", ElementKind::Ellipse),
        ("c", ElementKind::Circle),
        ("l", ElementKind::Line),
        ("p", ElementKind::Path),
        ("g", ElementKind::Group),
        ("t", ElementKind::Text),
        ("d", ElementKind::Other),
    ] {
        assert_eq!(doc.element_kind(id), Ok(kind));
    }
    assert_eq!(
        doc.element_kind("zz"),
        Err(V1DocumentError::MissingStableId)
    );
    assert_eq!(
        doc.element_kind("a b"),
        Err(V1DocumentError::InvalidStableId)
    );
}

#[test]
fn hit_test_returns_topmost_shape_with_documented_tolerance() {
    let doc = with(
        "<rect id=\"back\" x=\"10\" y=\"10\" width=\"50\" height=\"50\"/>\
         <rect id=\"front\" x=\"30\" y=\"30\" width=\"50\" height=\"50\"/>\
         <ellipse id=\"oval\" cx=\"150\" cy=\"50\" rx=\"20\" ry=\"10\"/>\
         <circle id=\"dot\" cx=\"100\" cy=\"20\" r=\"5\"/>\
         <line id=\"ln\" x1=\"90\" y1=\"90\" x2=\"190\" y2=\"90\"/>\
         <path id=\"pa\" d=\"M 100 40 L 120 40 L 110 70 Z\"/>\
         <rect x=\"0\" y=\"0\" width=\"200\" height=\"5\"/>",
    );
    assert_eq!(doc.hit_test(40.0, 40.0).as_deref(), Some("front"));
    assert_eq!(doc.hit_test(15.0, 15.0).as_deref(), Some("back"));
    assert_eq!(doc.hit_test(70.0, 70.0).as_deref(), Some("front"));
    assert_eq!(doc.hit_test(150.0, 50.0).as_deref(), Some("oval"));
    assert_eq!(
        doc.hit_test(168.0, 58.0),
        None,
        "ellipse corner is outside the curve"
    );
    assert_eq!(doc.hit_test(100.0, 20.0).as_deref(), Some("dot"));
    assert_eq!(doc.hit_test(100.0, 14.0), None);
    assert_eq!(doc.hit_test(140.0, 90.0).as_deref(), Some("ln"));
    assert_eq!(
        doc.hit_test(140.0, 90.0 + HIT_TOLERANCE).as_deref(),
        Some("ln")
    );
    assert_eq!(doc.hit_test(140.0, 90.0 + HIT_TOLERANCE + 0.5), None);
    assert_eq!(
        doc.hit_test(110.0, 60.0).as_deref(),
        Some("pa"),
        "path uses its bounding box"
    );
    assert_eq!(doc.hit_test(122.0, 40.0).as_deref(), Some("pa"));
    assert_eq!(doc.hit_test(123.0, 40.0), None);
    assert_eq!(
        doc.hit_test(100.0, 2.0),
        None,
        "elements without ids are never reported"
    );
    assert_eq!(doc.hit_test(f64::NAN, 1.0), None);
    assert_eq!(doc.hit_test(-5.0, 40.0), None);
}

#[test]
fn hit_test_follows_translate_only_and_ignores_other_transforms() {
    let doc = with(
        "<rect id=\"moved\" x=\"0\" y=\"0\" width=\"10\" height=\"10\" transform=\"translate(100 50)\"/>\
         <path id=\"pm\" d=\"M 0 0 L 10 10\" transform=\"translate(20 0)\"/>\
         <rect id=\"rot\" x=\"150\" y=\"10\" width=\"10\" height=\"10\" transform=\"rotate(45)\"/>",
    );
    assert_eq!(doc.hit_test(105.0, 55.0).as_deref(), Some("moved"));
    assert_eq!(doc.hit_test(5.0, 5.0), None);
    assert_eq!(doc.hit_test(25.0, 5.0).as_deref(), Some("pm"));
    assert_eq!(doc.hit_test(155.0, 15.0).as_deref(), Some("rot"));
}

#[test]
fn edits_leave_untouched_source_bytes_identical_including_undo_redo() {
    let original = PRESERVE.to_vec();
    let tail = find(&original, b"</svg>");

    // Insertion: only the bytes at the closing tag position change.
    let mut doc = V1Document::from_source(&original).unwrap();
    doc.add_line_with_id(1.0, 1.0, 9.0, 9.0, "#000000", 1.0, "line-1")
        .unwrap();
    let inserted = doc.source().unwrap();
    assert_eq!(&inserted[..tail], &original[..tail]);
    assert_eq!(
        &inserted[inserted.len() - (original.len() - tail)..],
        &original[tail..]
    );
    assert!(contains(&inserted, b"id=\"line-1\""));

    // An attribute edit rewrites only the targeted value.
    let mut doc = V1Document::from_source(&original).unwrap();
    doc.set_fill("base", "#ffffff").unwrap();
    let filled = doc.source().unwrap();
    let value_start = find(&original, b"fill=\"#112233\"") + 6;
    assert_eq!(&filled[..value_start], &original[..value_start]);
    assert_eq!(&filled[value_start..value_start + 7], b"#ffffff");
    assert_eq!(&filled[value_start + 7..], &original[value_start + 7..]);

    // Appending attributes touches only the end of one start tag.
    let mut doc = V1Document::from_source(&original).unwrap();
    doc.set_stroke("p1", "#123456", 2.0).unwrap();
    let stroked = doc.source().unwrap();
    let poly_end = find(&original, b"x:flag=\"1\"") + 10;
    assert_eq!(&stroked[..poly_end], &original[..poly_end]);
    assert_eq!(
        &stroked[stroked.len() - (original.len() - poly_end)..],
        &original[poly_end..]
    );
    assert!(contains(
        &stroked,
        b"x:flag=\"1\" stroke=\"#123456\" stroke-width=\"2e0\"/>"
    ));

    // A mixed sequence preserves every unrelated construct through undo and redo.
    let mut doc = V1Document::from_source(&original).unwrap();
    doc.add_line_with_id(1.0, 1.0, 9.0, 9.0, "#000000", 1.0, "line-1")
        .unwrap();
    doc.translate("p1", 3.0, 4.0).unwrap();
    doc.translate("base", 1.0, 1.0).unwrap();
    doc.set_stroke("p1", "#123456", 2.0).unwrap();
    doc.delete("base").unwrap();
    let last = doc.source().unwrap();
    for keep in [
        &b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- header comment: keep me -->"[..],
        b"<!-- before rect -->",
        b"<x:note x:keep=\"yes\">unknown element</x:note>",
        b"<linearGradient id=\"g1\"><stop offset=\"0\" stop-color=\"#000000\"/></linearGradient>",
        b"<!-- between shapes -->",
        b"<polygon id=\"poly\" points=\"0,0 10,0 5,8\" fill=\"#445566\"/>",
        b"<x:widget id=\"w1\" size=\"3\"/>",
        b"<!-- tail comment -->",
    ] {
        assert!(contains(&last, keep), "{}", String::from_utf8_lossy(keep));
    }
    assert!(contains(&last, b"fill=\"#778899\"  x:flag=\"1\""));
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
fn tool_built_documents_reopen_with_equal_bytes_and_ids() {
    let mut doc = blank();
    doc.add_rectangle_with_id(10.0, 10.0, 50.0, 40.0, "#112233", "rect-1")
        .unwrap();
    doc.add_ellipse_with_id(60.0, 10.0, 90.0, 40.0, "#223344", "ellipse-1")
        .unwrap();
    doc.add_line_with_id(1.0, 1.0, 99.0, 50.0, "#334455", 3.0, "line-1")
        .unwrap();
    doc.add_path_with_id(
        "M 1 2 C 3 4 5 6 7 8 L 9 9",
        "none",
        "#445566",
        1.5,
        "path-1",
    )
    .unwrap();
    doc.translate("rect-1", 4.0, 4.0).unwrap();
    doc.translate("path-1", 2.0, 3.0).unwrap();
    doc.set_fill("ellipse-1", "#ff00ff").unwrap();
    doc.set_stroke("rect-1", "#00ff00", 2.0).unwrap();
    doc.delete("line-1").unwrap();
    let bytes = doc.source().unwrap();
    let reopened = V1Document::from_source(&bytes).unwrap();
    assert_eq!(reopened.source().unwrap(), bytes);
    assert!(!reopened.is_dirty());
    for (id, kind) in [
        ("rect-1", ElementKind::Rect),
        ("ellipse-1", ElementKind::Ellipse),
        ("path-1", ElementKind::Path),
    ] {
        assert_eq!(reopened.element_kind(id), Ok(kind));
    }
    assert_eq!(
        reopened.element_kind("line-1"),
        Err(V1DocumentError::MissingStableId)
    );
}

fn find(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap()
}
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}
