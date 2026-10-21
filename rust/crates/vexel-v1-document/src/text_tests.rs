use crate::{TextAnchor, TextSpec, V1Document, V1DocumentError};

fn spec(content: &str) -> TextSpec {
    TextSpec {
        x: 10.0,
        y: 40.0,
        content: content.to_owned(),
        font_size: 24.0,
        anchor: TextAnchor::Middle,
        fill: "#112233".into(),
    }
}

#[test]
fn add_text_escapes_content_and_round_trips_history() {
    let mut doc = V1Document::new(64, 64).unwrap();
    let before = doc.source().unwrap();
    let id = doc.add_text(&spec("<&>\"'")).unwrap();
    assert_eq!(id, "vexel-object-1");
    let after = doc.source().unwrap();
    let source = String::from_utf8(after.clone()).unwrap();
    assert!(source.contains("&lt;&amp;&gt;\"'"), "{source}");
    assert_eq!(doc.history_count(), 1);
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), after);
}

#[test]
fn set_text_edits_only_text_and_round_trips_history() {
    let mut doc = V1Document::new(64, 64).unwrap();
    let id = doc.add_text(&spec("old")).unwrap();
    let before = doc.source().unwrap();
    doc.set_text(&id, "new<&>").unwrap();
    let after = doc.source().unwrap();
    let source = String::from_utf8(after.clone()).unwrap();
    assert!(source.contains(">new&lt;&amp;&gt;</text>"));
    assert!(source.contains("x=\"1e1\" y=\"4e1\""));
    assert_eq!(doc.history_count(), 2);
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), after);
}

#[test]
fn text_validation_and_wrong_kind_errors_are_atomic() {
    let mut doc = V1Document::new(64, 64).unwrap();
    let id = doc.add_text(&spec("valid")).unwrap();
    let before = doc.source().unwrap();
    let history = doc.history_count();
    assert_eq!(doc.set_text(&id, ""), Err(V1DocumentError::InvalidGeometry));
    assert_eq!(
        doc.set_text(&id, &"x".repeat(257)),
        Err(V1DocumentError::InvalidGeometry)
    );
    assert_eq!(
        doc.set_text(&id, "bad\ntext"),
        Err(V1DocumentError::InvalidGeometry)
    );
    assert_eq!(
        doc.set_text("no-such-id", "ok"),
        Err(V1DocumentError::MissingStableId)
    );
    assert_eq!(doc.source().unwrap(), before);
    assert_eq!(doc.history_count(), history);

    doc.add_rectangle(1.0, 1.0, 5.0, 5.0, "#ffffff").unwrap();
    let count = doc.history_count();
    assert_eq!(
        doc.set_text("vexel-object-2", "no"),
        Err(V1DocumentError::Unsupported)
    );
    assert_eq!(doc.history_count(), count);
}

#[test]
fn add_text_rejects_invalid_specs_without_editing() {
    for (mut invalid, expected) in [
        (
            {
                let mut s = spec("x");
                s.x = f64::NAN;
                s
            },
            V1DocumentError::InvalidGeometry,
        ),
        (
            {
                let mut s = spec("x");
                s.font_size = 0.0;
                s
            },
            V1DocumentError::InvalidGeometry,
        ),
        (
            {
                let mut s = spec("x");
                s.fill = "red".into();
                s
            },
            V1DocumentError::InvalidColor,
        ),
        (
            {
                let mut s = spec("x");
                s.content = "\u{7}".into();
                s
            },
            V1DocumentError::InvalidGeometry,
        ),
    ] {
        let mut doc = V1Document::new(64, 64).unwrap();
        let before = doc.source().unwrap();
        assert_eq!(doc.add_text(&invalid), Err(expected));
        assert_eq!(doc.source().unwrap(), before);
        assert_eq!(doc.history_count(), 0);
        invalid.content.clear();
    }
}

#[test]
fn text_fill_is_one_undoable_edit() {
    let mut doc = V1Document::new(64, 64).unwrap();
    let id = doc.add_text(&spec("x")).unwrap();
    let before = doc.source().unwrap();
    doc.set_text_fill(&id, "#abcdef").unwrap();
    let after = doc.source().unwrap();
    assert!(
        String::from_utf8(after.clone())
            .unwrap()
            .contains("fill=\"#abcdef\"")
    );
    doc.undo().unwrap();
    assert_eq!(doc.source().unwrap(), before);
    doc.redo().unwrap();
    assert_eq!(doc.source().unwrap(), after);
}
