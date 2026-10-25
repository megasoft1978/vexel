//! Every tool in the live-preview `tools/list` must reach the preview dispatch
//! (`parse_operation`) rather than fail as an unknown or unrouted tool. Each
//! call's arguments are built from its listed schema, so protocol validation
//! passes and the call reaches the dispatcher. No grants are given, so no
//! call can write to disk.
mod common;

use common::{Rig, no_grants};
use vexel_app::preview::DOCUMENT_ID;
use vexel_json::{JsonDocument, JsonType};

fn text(doc: &JsonDocument<'_>, token: usize) -> String {
    String::from_utf8(doc.decoded_string(token).expect("string token")).unwrap()
}

/// A minimal JSON value that satisfies one listed property schema.
fn sample(doc: &JsonDocument<'_>, schema: usize) -> String {
    if let Some(choices) = doc.object_get(schema, b"enum") {
        let first = doc.token(choices).unwrap().first_child.expect("enum value");
        return format!("\"{}\"", text(doc, first));
    }
    let ty = doc.object_get(schema, b"type").expect("typed property");
    let ty = match doc.token(ty).unwrap().kind {
        JsonType::Array => doc.token(ty).unwrap().first_child.expect("type union"),
        _ => ty,
    };
    match text(doc, ty).as_str() {
        "string" => "\"x\"".to_owned(),
        "integer" | "number" => "1".to_owned(),
        "boolean" => "true".to_owned(),
        "object" => "{}".to_owned(),
        "array" => "[]".to_owned(),
        other => panic!("unhandled schema type {other}"),
    }
}

/// `arguments` JSON holding each required field of one listed tool.
fn arguments(doc: &JsonDocument<'_>, tool: usize) -> String {
    let schema = doc.object_get(tool, b"inputSchema").expect("inputSchema");
    let properties = doc.object_get(schema, b"properties").expect("properties");
    let required = doc.object_get(schema, b"required").expect("required");
    let mut fields = Vec::new();
    let mut item = doc.token(required).unwrap().first_child;
    while let Some(at) = item {
        let field = text(doc, at);
        let value = if field == "documentId" {
            format!("\"{DOCUMENT_ID}\"")
        } else {
            let property = doc
                .object_get(properties, field.as_bytes())
                .expect("property");
            sample(doc, property)
        };
        fields.push(format!("\"{field}\":{value}"));
        item = doc.token(at).unwrap().next_sibling;
    }
    format!("{{{}}}", fields.join(","))
}

#[test]
fn tools_schema_matches_dispatch() {
    let mut rig = Rig::new(no_grants());
    let listing = rig.line(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#);
    let doc = JsonDocument::parse(listing.as_bytes()).unwrap();
    let result = doc.object_get(doc.root(), b"result").unwrap();
    let tools = doc.object_get(result, b"tools").unwrap();

    let mut checked = 0;
    let mut item = doc.token(tools).unwrap().first_child;
    while let Some(tool) = item {
        let name = text(&doc, doc.object_get(tool, b"name").unwrap());
        let args = arguments(&doc, tool);
        let request = format!(
            r#"{{"jsonrpc":"2.0","id":{},"method":"tools/call","params":{{"name":"{name}","arguments":{args}}}}}"#,
            100 + checked
        );
        let out = rig.line(&request);
        assert!(out.contains("\"result\""), "{name} got no result: {out}");
        assert!(
            !out.contains("unknown tool") && !out.contains("unsupported preview operation"),
            "{name} is advertised but not dispatchable: {out}"
        );
        checked += 1;
        item = doc.token(tool).unwrap().next_sibling;
    }
    assert_eq!(checked, 68, "preview tools/list changed size");
}
