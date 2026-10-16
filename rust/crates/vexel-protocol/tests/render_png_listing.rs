//! `render_png` in the protocol: listed (with its schema) only in the live
//! preview profile, absent from the standard listing, argument errors typed.

use vexel_json::JsonDocument;
use vexel_protocol::{ProtocolBackend, ProtocolSession, ToolResponse, handle, tool_schemas};

#[derive(Default)]
struct Backend {
    calls: Vec<String>,
}

impl ProtocolBackend for Backend {
    fn call_tool(&mut self, _: &JsonDocument<'_>, _: usize, name: &str) -> Option<ToolResponse> {
        self.calls.push(name.to_owned());
        Some(ToolResponse {
            structured_json: b"{}".to_vec(),
            content_json: b"[]".to_vec(),
            is_error: false,
        })
    }
}

fn call(session: &mut ProtocolSession, backend: &mut Backend, line: &str) -> String {
    let out = handle(session, line.as_bytes(), Some(backend)).unwrap();
    String::from_utf8(out).unwrap()
}

fn live() -> (ProtocolSession, Backend) {
    let mut session = ProtocolSession::live_preview();
    let mut backend = Backend::default();
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#;
    call(&mut session, &mut backend, init);
    call(
        &mut session,
        &mut backend,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    );
    (session, backend)
}

fn tool(session: &mut ProtocolSession, backend: &mut Backend, arguments: &str) -> String {
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"render_png","arguments":{arguments}}}}}"#
    );
    call(session, backend, &line)
}

#[test]
fn preview_listing_has_render_png_with_its_schema() {
    let (mut session, mut backend) = live();
    let listing = call(
        &mut session,
        &mut backend,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    );
    assert!(listing.contains(r#""name":"render_png""#));
    for needle in [
        r#""width":{"type":"integer","minimum":1,"maximum":4096}"#,
        r#""height":{"type":"integer","minimum":1,"maximum":4096}"#,
        r#""path":{"type":"string","minLength":1,"maxLength":1023}"#,
        r#""required":["documentId","path"]"#,
    ] {
        assert!(listing.contains(needle), "{needle}");
    }
}

#[test]
fn standard_listing_keeps_the_original_render_png_schema() {
    let full = String::from_utf8(tool_schemas().unwrap()).unwrap();
    assert!(full.contains("Render a bounded image of an open SVG region."));
    assert!(full.contains(r#""region":{"type":"object"}"#));
    assert!(!full.contains("PNG file inside a granted directory"));
}

#[test]
fn bad_arguments_are_invalid_arguments_and_valid_calls_reach_the_backend() {
    let (mut session, mut backend) = live();
    for arguments in [
        r#"{"documentId":"d"}"#,
        r#"{"path":"/tmp/x.png"}"#,
        r#"{"documentId":"d","path":7}"#,
        r#"{"documentId":"d","path":"/tmp/x.png","width":"9"}"#,
        r#"{"documentId":"d","path":"/tmp/x.png","extra":1}"#,
    ] {
        let out = tool(&mut session, &mut backend, arguments);
        assert!(out.contains("invalid_arguments"), "{arguments}: {out}");
    }
    assert!(backend.calls.is_empty());
    let out = tool(
        &mut session,
        &mut backend,
        r#"{"documentId":"d","path":"/tmp/x.png","width":10,"height":10}"#,
    );
    assert!(out.contains(r#""isError":false"#), "{out}");
    assert_eq!(backend.calls, ["render_png"]);
}
