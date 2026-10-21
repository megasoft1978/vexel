use super::*;
use crate::preview::ext_structure;
use crate::session::AppDocument;
use vexel_json::JsonDocument;

fn parsed(tool: &str, args: &str) -> Result<ext_structure::Op, PreviewArgumentError> {
    let json = if args.is_empty() {
        "{\"documentId\":\"preview-document\"}".to_owned()
    } else {
        format!("{{\"documentId\":\"preview-document\",{args}}}")
    };
    let doc = JsonDocument::parse(json.as_bytes()).unwrap();
    ext_structure::parse(&doc, doc.root(), tool).unwrap()
}

fn run(app: &mut AppDocument, tool: &str, args: &str) -> String {
    let op = parsed(tool, args).unwrap();
    match ext_structure::apply(app, op).unwrap() {
        McpOutcome::Reply(json) => json,
        other => panic!("{other:?}"),
    }
}

fn add_rect(app: &mut AppDocument, id: &str) {
    app.edit(|doc| doc.add_rectangle_with_id(1.0, 2.0, 4.0, 6.0, "#123456", id))
        .unwrap();
}

#[test]
fn parse_validates_ids_document_and_boolean() {
    assert!(parsed("element_group", "\"elementId\":\"a\"").is_ok());
    assert!(
        parsed(
            "element_reorder",
            "\"elementId\":\"a\",\"targetId\":\"b\",\"before\":true"
        )
        .is_ok()
    );
    assert!(
        parsed(
            "element_reorder",
            "\"elementId\":\"a\",\"targetId\":\"b\",\"before\":\"yes\""
        )
        .is_err()
    );
    let json = b"{\"documentId\":\"other\",\"elementId\":\"a\"}";
    let doc = JsonDocument::parse(json).unwrap();
    assert!(
        ext_structure::parse(&doc, doc.root(), "element_duplicate")
            .unwrap()
            .is_err()
    );
    assert!(parsed("element_ungroup", "").is_err());
}

#[test]
fn group_ungroup_duplicate_and_reorder_apply_and_share_history() {
    let mut app = AppDocument::new(64, 64).unwrap();
    add_rect(&mut app, "vexel-object-10");
    add_rect(&mut app, "vexel-object-11");
    let before = app.source_bytes().unwrap();
    let grouped = run(
        &mut app,
        "element_group",
        "\"elementId\":\"vexel-object-10\"",
    );
    assert_eq!(grouped, r#"{"ok":true,"elementId":"vexel-object-1"}"#);
    let grouped_source = String::from_utf8(app.source_bytes().unwrap()).unwrap();
    assert!(
        grouped_source.contains("<g id=\"vexel-object-1\">"),
        "{grouped_source}"
    );
    app.undo().unwrap();
    assert_eq!(app.source_bytes().unwrap(), before);
    app.redo().unwrap();
    let duplicate = run(
        &mut app,
        "element_duplicate",
        "\"elementId\":\"vexel-object-1\"",
    );
    assert!(duplicate.contains("\"elementId\":\"vexel-object-2\""));
    let reorder = run(
        &mut app,
        "element_reorder",
        "\"elementId\":\"vexel-object-11\",\"targetId\":\"vexel-object-1\",\"before\":true",
    );
    assert_eq!(reorder, r#"{"ok":true}"#);
    run(
        &mut app,
        "element_ungroup",
        "\"elementId\":\"vexel-object-1\"",
    );
    let after = app.source_bytes().unwrap();
    app.undo().unwrap();
    app.redo().unwrap();
    assert_eq!(app.source_bytes().unwrap(), after);
}

#[test]
fn apply_errors_do_not_change_source_or_history() {
    let mut app = AppDocument::new(64, 64).unwrap();
    let before = app.source_bytes().unwrap();
    assert_eq!(
        ext_structure::apply(&mut app, ext_structure::Op::Duplicate("missing".into())),
        Err("element_group: element not found")
    );
    assert_eq!(app.source_bytes().unwrap(), before);
    assert!(!app.can_undo());
}
