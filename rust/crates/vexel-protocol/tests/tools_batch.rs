//! The `batch` tool is declared but not advertised or routed yet. It stays out
//! of the full schema and the live preview list, and each batch call is charged
//! its weight against the per-minute request budget.
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

fn initialized(session: &mut ProtocolSession) {
    call(
        session,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#,
    );
    call(
        session,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    );
}

fn batch_request(id: usize, count: usize) -> String {
    let op = r#"{"tool":"tree_get","arguments":{"documentId":"doc-1"}}"#;
    let ops = vec![op; count].join(",");
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"batch","arguments":{{"documentId":"doc-1","ops":[{ops}]}}}}}}"#
    )
}

#[test]
fn batch_is_not_in_the_full_or_live_preview_listing() {
    let full = tool_schemas().unwrap();
    assert!(!String::from_utf8_lossy(&full).contains("\"name\":\"batch\""));

    let mut session = ProtocolSession::live_preview();
    initialized(&mut session);
    let listing = call(
        &mut session,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    );
    assert!(!listing.contains("\"name\":\"batch\""));
    let refused = call(&mut session, &batch_request(3, 1));
    assert!(refused.contains("\"code\":\"unknown_tool\""), "{refused}");
}

#[test]
fn batch_is_refused_outside_live_preview() {
    let mut session = ProtocolSession::default();
    initialized(&mut session);
    let reply = call(&mut session, &batch_request(2, 1));
    assert!(reply.contains("\"code\":\"unknown_tool\""), "{reply}");
}

#[test]
fn batch_weight_counts_against_the_request_budget() {
    // initialize and its notification cost 2. A 256-op batch costs 1 + 256/8 = 33,
    // so 18 batches reach 596 (within 600) and the next one is refused.
    let mut session = ProtocolSession::default();
    initialized(&mut session);
    for id in 2..=19 {
        let reply = call(&mut session, &batch_request(id, 256));
        assert!(!reply.contains("rate limit exceeded"), "call {id}: {reply}");
    }
    let limited = call(&mut session, &batch_request(20, 256));
    assert!(limited.contains("rate limit exceeded"), "{limited}");
}
