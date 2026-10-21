//! In the live preview, `tools/list` and the `tools/call` gate must agree: a
//! listed tool is never refused as unknown, and an unlisted tool always is.
use vexel_json::JsonDocument;
use vexel_protocol::{ProtocolBackend, ProtocolSession, ToolResponse, handle, tool_schemas};

struct Backend;
impl ProtocolBackend for Backend {
    fn call_tool(
        &mut self,
        _request: &JsonDocument<'_>,
        _arguments: usize,
        _name: &str,
    ) -> Option<ToolResponse> {
        Some(ToolResponse {
            structured_json: b"{}".to_vec(),
            content_json: Vec::new(),
            is_error: false,
        })
    }
}

fn call(session: &mut ProtocolSession, request: &str) -> String {
    String::from_utf8(handle(session, request.as_bytes(), Some(&mut Backend)).unwrap()).unwrap()
}

fn names(doc: &JsonDocument<'_>, list: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut item = doc.token(list).unwrap().first_child;
    while let Some(at) = item {
        let name = doc.object_get(at, b"name").unwrap();
        out.push(String::from_utf8(doc.decoded_string(name).unwrap()).unwrap());
        item = doc.token(at).unwrap().next_sibling;
    }
    out
}

#[test]
fn tools_schema_matches_dispatch() {
    let mut session = ProtocolSession::live_preview();
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#;
    call(&mut session, init);
    call(
        &mut session,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    );

    let listing = call(
        &mut session,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    );
    let doc = JsonDocument::parse(listing.as_bytes()).unwrap();
    let result = doc.object_get(doc.root(), b"result").unwrap();
    let advertised = names(&doc, doc.object_get(result, b"tools").unwrap());
    assert_eq!(advertised.len(), 53);

    let full = tool_schemas().unwrap();
    let full_doc = JsonDocument::parse(&full).unwrap();
    let declared = names(&full_doc, full_doc.root());

    for name in &declared {
        let reply = call(
            &mut session,
            &format!(
                r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"{name}","arguments":{{}}}}}}"#
            ),
        );
        let refused_unknown = reply.contains("\"code\":\"unknown_tool\"");
        assert_eq!(
            refused_unknown,
            !advertised.contains(name),
            "{name}: listed={} but gate refused_unknown={refused_unknown}",
            advertised.contains(name)
        );
    }
}
