use crate::preview::DOCUMENT_ID;
use crate::preview::PreviewArgumentError;
use crate::preview::ext_query::{Op, apply, parse};
use crate::session::{AppDocument, McpOutcome};
use vexel_json::JsonDocument;
use vexel_v1_document::V1Document;

fn parsed(tool: &str, args: &str) -> Result<Op, PreviewArgumentError> {
    let json = format!("{{\"documentId\":\"{DOCUMENT_ID}\",{args}}}");
    let doc = JsonDocument::parse(json.as_bytes()).unwrap();
    parse(&doc, doc.root(), tool).unwrap()
}

fn reply(result: Result<McpOutcome, &'static str>) -> String {
    let McpOutcome::Reply(text) = result.unwrap() else {
        panic!("expected reply")
    };
    text
}

#[test]
fn query_and_tree_pages_are_document_ordered_and_read_only() {
    let source = br#"<svg width="20" height="20"><g id="g"><g><rect id="r" title="a&amp;b"/></g><circle/></g><rect id="r2"/></svg>"#;
    let mut app = AppDocument::from_document(V1Document::from_source(source).unwrap()).unwrap();
    let bytes = app.source_bytes().unwrap();
    let revision = app.state_revision();
    assert_eq!(
        reply(apply(
            &mut app,
            parsed("element_query", "\"tag\":\"rect\",\"limit\":1").unwrap()
        )),
        r#"{"ids":["r"],"nextCursor":1}"#
    );
    assert_eq!(
        reply(apply(
            &mut app,
            parsed("element_query", "\"tag\":\"rect\",\"cursor\":1").unwrap()
        )),
        r#"{"ids":["r2"],"nextCursor":null}"#
    );
    let tree = reply(apply(&mut app, parsed("tree_get", "\"limit\":3").unwrap()));
    assert!(tree.contains(r#"{"id":"g","tag":"g","depth":1,"parentId":null}"#));
    assert!(tree.ends_with(r#""nextCursor":3}"#));
    assert_eq!(app.source_bytes().unwrap(), bytes);
    assert_eq!(app.state_revision(), revision);
    assert!(!app.can_undo());
}

#[test]
fn get_escapes_attributes_and_query_matches_exact_id() {
    let source = br#"<svg width="20" height="20"><rect id="r" title="a&amp;b" x="1" y="2" width="3" height="4"/></svg>"#;
    let mut app = AppDocument::from_document(V1Document::from_source(source).unwrap()).unwrap();
    let got = reply(apply(
        &mut app,
        parsed("element_get", "\"elementId\":\"r\"").unwrap(),
    ));
    assert!(got.contains(r#""title":"a&amp;b""#));
    assert!(got.contains(r#""bounds":[1, 2, 4, 6]"#));
    assert_eq!(
        reply(apply(
            &mut app,
            parsed("element_query", "\"id\":\"missing\"").unwrap()
        )),
        r#"{"ids":[],"nextCursor":null}"#
    );
}

#[test]
fn parse_rejects_bad_page_and_value_without_attribute() {
    assert!(parsed("tree_get", "\"limit\":0").is_err());
    assert!(parsed("tree_get", "\"limit\":513").is_err());
    assert!(parsed("element_query", "\"value\":\"x\"").is_err());
}
