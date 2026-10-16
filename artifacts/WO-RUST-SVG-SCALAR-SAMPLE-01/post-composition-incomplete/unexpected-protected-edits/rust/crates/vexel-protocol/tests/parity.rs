use vexel_json::{JsonDocument, JsonType, MAX_BYTES};
use vexel_protocol::{
    AsyncBackend, AsyncDisposition, AsyncPayloadKind, ProtocolBackend, ProtocolSession,
    ToolResponse, handle, handle_async, legacy_progress, request_expects_response, tool_schemas,
    validate_async_payload, validate_tool_arguments,
};

fn json(bytes: &[u8]) -> JsonDocument<'_> {
    JsonDocument::parse(bytes).expect("valid JSON")
}
fn assert_response(request: &str, response: &[u8]) {
    let req = json(request.as_bytes());
    let res = json(response);
    let root = res.root();
    assert_eq!(
        res.string_equals(res.object_get(root, b"jsonrpc").unwrap(), b"2.0"),
        true
    );
    let req_id = req.object_get(req.root(), b"id").unwrap();
    let res_id = res.object_get(root, b"id").unwrap();
    assert_eq!(
        req.raw_token(req_id),
        res.raw_token(res_id),
        "original JSON-RPC id bytes are preserved"
    );
    let result = res.object_get(root, b"result");
    let error = res.object_get(root, b"error");
    assert_ne!(
        result.is_some(),
        error.is_some(),
        "response has exactly one of result/error"
    );
    assert_eq!(
        res.token(result.or(error).unwrap()).unwrap().kind,
        JsonType::Object
    );
}

#[derive(Default)]
struct SyncFake {
    calls: usize,
    resource_calls: usize,
    empty_content: bool,
    is_error: bool,
}
impl ProtocolBackend for SyncFake {
    fn call_tool(
        &mut self,
        _request: &JsonDocument<'_>,
        _args: usize,
        name: &str,
    ) -> Option<ToolResponse> {
        self.calls += 1;
        (name == "document_list").then(|| ToolResponse {
            structured_json: b"{\"documents\":[]}".to_vec(),
            content_json: if self.empty_content {
                Vec::new()
            } else {
                b"[{\"type\":\"text\",\"text\":\"sync\"}]".to_vec()
            },
            is_error: self.is_error,
        })
    }
    fn read_resource(&mut self, _request: &JsonDocument<'_>, _uri: &str) -> Option<Vec<u8>> {
        self.resource_calls += 1;
        Some(b"[{\"uri\":\"vexel://documents/current/source\",\"text\":\"<svg/>\",\"mimeType\":\"image/svg+xml\"}]".to_vec())
    }
}

#[derive(Default)]
struct AsyncFake {
    available: bool,
    disposition: AsyncDisposition,
    payload: Vec<u8>,
    calls: usize,
    gets: usize,
    cancels: usize,
    legacy_cancels: usize,
    saw_tasks: bool,
    get_failed: bool,
    get_rpc_error: bool,
}
impl AsyncBackend for AsyncFake {
    fn tasks_available(&self) -> bool {
        self.available
    }
    fn call_tool(
        &mut self,
        _request: &JsonDocument<'_>,
        _args: usize,
        _name: &str,
        modern: bool,
        supports_tasks: bool,
    ) -> (AsyncDisposition, Vec<u8>) {
        self.calls += 1;
        self.saw_tasks = modern && supports_tasks;
        (self.disposition, self.payload.clone())
    }
    fn get_task(
        &mut self,
        _request: &JsonDocument<'_>,
        task_id: &str,
    ) -> (AsyncDisposition, Vec<u8>) {
        self.gets += 1;
        if self.get_rpc_error {
            return (
                AsyncDisposition::Error,
                br#"{"code":-32023,"message":"task unavailable"}"#.to_vec(),
            );
        }
        if task_id != "job-1" {
            return (
                AsyncDisposition::Error,
                br#"{"code":-32023,"message":"task unavailable"}"#.to_vec(),
            );
        }
        if self.get_failed {
            return (AsyncDisposition::Result, br#"{"resultType":"complete","taskId":"job-1","status":"failed","createdAt":"2026-10-14T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:02Z","ttlMs":900000,"error":{"code":-32024,"message":"task failed"}}"#.to_vec());
        }
        (AsyncDisposition::Result, br#"{"resultType":"complete","taskId":"job-1","status":"completed","createdAt":"2026-10-14T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:01Z","ttlMs":900000,"result":{"content":[],"isError":true}}"#.to_vec())
    }
    fn cancel_task(
        &mut self,
        _request: &JsonDocument<'_>,
        _task_id: &str,
    ) -> (AsyncDisposition, Vec<u8>) {
        self.cancels += 1;
        (
            AsyncDisposition::Result,
            br#"{"resultType":"complete"}"#.to_vec(),
        )
    }
    fn cancel_request(&mut self, _request: &JsonDocument<'_>, _request_id: usize) {
        self.legacy_cancels += 1;
    }
}

const DISCOVER: &str = r#"{"jsonrpc":"2.0","id":"d-1","method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}"#;
const TASKS_META: &str = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{"extensions":{"io.modelcontextprotocol/tasks":{}}}}"#;
fn task_tool(id: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"document_list","arguments":{{}},{TASKS_META}}}}}"#
    )
}
fn task_method(method: &str, id: &str, task_id: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{{"taskId":"{task_id}",{TASKS_META}}}}}"#
    )
}

#[test]
fn legacy_lifecycle_sync_discovery_tools_resources_prompts_and_reset() {
    let mut session = ProtocolSession::default();
    let mut backend = SyncFake::default();
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#;
    let response = handle(&mut session, init.as_bytes(), Some(&mut backend)).unwrap();
    assert_response(init, &response);
    assert!(String::from_utf8_lossy(&response).contains("2025-11-25"));
    let initialized = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    assert!(
        handle::<SyncFake>(&mut session, initialized.as_bytes(), None)
            .unwrap()
            .is_empty()
    );
    assert!(session.initialized);
    let repeated = handle(&mut session, init.as_bytes(), Some(&mut backend)).unwrap();
    assert_response(init, &repeated);
    assert!(String::from_utf8_lossy(&repeated).contains("initialize is not valid"));
    for (method, needle) in [
        ("tools/list", "document_open"),
        ("resources/list", "vexel://documents/current/source"),
        ("prompts/list", "inspect-document"),
    ] {
        let request = format!(r#"{{"jsonrpc":"2.0","id":17,"method":"{method}","params":{{}}}}"#);
        let output = handle(&mut session, request.as_bytes(), Some(&mut backend)).unwrap();
        assert_response(&request, &output);
        assert!(String::from_utf8_lossy(&output).contains(needle));
    }
    let prompt =
        r#"{"jsonrpc":"2.0","id":18,"method":"prompts/get","params":{"name":"inspect-document"}}"#;
    let output = handle(&mut session, prompt.as_bytes(), Some(&mut backend)).unwrap();
    assert_response(prompt, &output);
    assert!(String::from_utf8_lossy(&output).contains("unsupported features"));
    let read = r#"{"jsonrpc":"2.0","id":19,"method":"resources/read","params":{"uri":"vexel://documents/current/source"}}"#;
    let output = handle(&mut session, read.as_bytes(), Some(&mut backend)).unwrap();
    assert_response(read, &output);
    assert!(String::from_utf8_lossy(&output).contains("<svg/>"));
    assert_eq!(backend.resource_calls, 1);
    session.reset();
    assert!(!session.initialized && !session.modern);
    let rejected = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
    let output = handle(&mut session, rejected.as_bytes(), Some(&mut backend)).unwrap();
    assert_response(rejected, &output);
    assert!(String::from_utf8_lossy(&output).contains("-32002"));
}

#[test]
fn current_discovery_metadata_merging_original_ids_and_task_capability_reset() {
    let mut session = ProtocolSession::default();
    let mut sync = SyncFake::default();
    let mut async_backend = AsyncFake {
        available: true,
        ..Default::default()
    };
    let output = handle_async(
        &mut session,
        DISCOVER.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(DISCOVER, &output);
    assert!(session.modern);
    assert!(String::from_utf8_lossy(&output).contains("io.modelcontextprotocol/tasks"));
    let call = task_tool("1.00");
    async_backend.disposition = AsyncDisposition::Result;
    async_backend.payload=br#"{"resultType":"complete","content":[],"isError":false,"_meta":{"trace":"keep","io.modelcontextprotocol/serverInfo":{"name":"spoof","version":"x"}}}"#.to_vec();
    let output = handle_async(
        &mut session,
        call.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(&call, &output);
    let value = String::from_utf8_lossy(&output);
    assert!(value.contains("1.00"));
    assert!(value.contains("\"trace\":\"keep\""));
    assert!(value.contains("\"name\":\"Vexel\""));
    assert!(async_backend.saw_tasks);
    // A reset forgets negotiation and a subsequent discovery without readiness omits Tasks.
    session.reset();
    async_backend.available = false;
    let output = handle_async(
        &mut session,
        DISCOVER.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(DISCOVER, &output);
    assert!(!String::from_utf8_lossy(&output).contains("io.modelcontextprotocol/tasks"));
    session.reset();
    let no_experimental = r#"{"jsonrpc":"2.0","id":9,"method":"experimental/tasks/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}"#;
    let output = handle_async(
        &mut session,
        no_experimental.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(no_experimental, &output);
    assert!(String::from_utf8_lossy(&output).contains("-32601"));
}

#[test]
fn flat_task_create_detailed_terminal_empty_cancel_and_rpc_error_are_distinct() {
    let mut session = ProtocolSession::default();
    let mut sync = SyncFake::default();
    let mut backend=AsyncFake{available:true,disposition:AsyncDisposition::Result,payload:br#"{"resultType":"task","taskId":"job-1","status":"working","createdAt":"2026-10-14T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:00Z","ttlMs":900000,"pollIntervalMs":1000,"_meta":{"seed":"yes"}}"#.to_vec(),..Default::default()};
    let _ = handle_async(
        &mut session,
        DISCOVER.as_bytes(),
        Some(&mut sync),
        Some(&mut backend),
    )
    .unwrap();
    let request = task_tool("22");
    let output = handle_async(
        &mut session,
        request.as_bytes(),
        Some(&mut sync),
        Some(&mut backend),
    )
    .unwrap();
    assert_response(&request, &output);
    let parsed = json(&output);
    let result = parsed.object_get(parsed.root(), b"result").unwrap();
    assert!(parsed.object_get(result, b"taskId").is_some());
    assert!(parsed.object_get(result, b"result").is_none());
    assert_eq!(
        parsed.raw_token(parsed.object_get(parsed.root(), b"id").unwrap()),
        Some(&b"22"[..])
    );
    assert!(!String::from_utf8_lossy(&output).contains("notifications/progress"));
    let get = task_method("tasks/get", "23", "job-1");
    let output = handle_async(
        &mut session,
        get.as_bytes(),
        Some(&mut sync),
        Some(&mut backend),
    )
    .unwrap();
    assert_response(&get, &output);
    assert!(String::from_utf8_lossy(&output).contains("\"status\":\"completed\""));
    assert!(String::from_utf8_lossy(&output).contains("\"isError\":true"));
    assert_eq!(backend.gets, 1);
    backend.get_failed = true;
    let failed_get = task_method("tasks/get", "26", "job-1");
    let output = handle_async(
        &mut session,
        failed_get.as_bytes(),
        Some(&mut sync),
        Some(&mut backend),
    )
    .unwrap();
    assert_response(&failed_get, &output);
    let failed = json(&output);
    let failed_result = failed.object_get(failed.root(), b"result").unwrap();
    assert!(failed.object_get(failed.root(), b"error").is_none());
    assert!(failed.string_equals(
        failed.object_get(failed_result, b"status").unwrap(),
        b"failed"
    ));
    let task_error = failed.object_get(failed_result, b"error").unwrap();
    assert_eq!(
        failed.raw_token(failed.object_get(task_error, b"code").unwrap()),
        Some(&b"-32024"[..])
    );
    backend.get_failed = false;
    backend.get_rpc_error = true;
    let rpc_get = task_method("tasks/get", "27", "job-1");
    let output = handle_async(
        &mut session,
        rpc_get.as_bytes(),
        Some(&mut sync),
        Some(&mut backend),
    )
    .unwrap();
    assert_response(&rpc_get, &output);
    let rpc = json(&output);
    assert!(rpc.object_get(rpc.root(), b"result").is_none());
    let rpc_error = rpc.object_get(rpc.root(), b"error").unwrap();
    assert_eq!(
        rpc.raw_token(rpc.object_get(rpc_error, b"code").unwrap()),
        Some(&b"-32023"[..])
    );
    backend.get_rpc_error = false;
    let cancel = task_method("tasks/cancel", "24", "job-1");
    let output = handle_async(
        &mut session,
        cancel.as_bytes(),
        Some(&mut sync),
        Some(&mut backend),
    )
    .unwrap();
    assert_response(&cancel, &output);
    assert!(String::from_utf8_lossy(&output).contains("\"resultType\":\"complete\""));
    assert!(!String::from_utf8_lossy(&output).contains("taskId"));
    assert_eq!(backend.cancels, 1);
    let bad = task_method("tasks/get", "25", "foreign");
    let output = handle_async(
        &mut session,
        bad.as_bytes(),
        Some(&mut sync),
        Some(&mut backend),
    )
    .unwrap();
    assert_response(&bad, &output);
    assert!(String::from_utf8_lossy(&output).contains("task unavailable"));
    assert!(String::from_utf8_lossy(&output).contains("\"error\":{\"code\":-32023"));
}

#[test]
fn sync_fallback_malformed_backends_metadata_and_legacy_deferred_cancel() {
    let mut session = ProtocolSession::default();
    let mut sync = SyncFake::default();
    let mut async_backend = AsyncFake {
        available: true,
        disposition: AsyncDisposition::Unhandled,
        ..Default::default()
    };
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#;
    let _ = handle_async(
        &mut session,
        init.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    let n = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let _ = handle_async(
        &mut session,
        n.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    let call = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"document_list","arguments":{}}}"#;
    let out = handle_async(
        &mut session,
        call.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(call, &out);
    assert!(String::from_utf8_lossy(&out).contains("structuredContent"));
    assert_eq!(sync.calls, 1);
    sync.empty_content = true;
    for (is_error, expected) in [
        (false, b"Tool completed.".as_slice()),
        (true, b"Tool request failed".as_slice()),
    ] {
        sync.is_error = is_error;
        let out = handle_async(
            &mut session,
            call.as_bytes(),
            Some(&mut sync),
            Some(&mut async_backend),
        )
        .unwrap();
        assert_response(call, &out);
        let envelope = json(&out);
        let result = envelope.object_get(envelope.root(), b"result").unwrap();
        let content = envelope.object_get(result, b"content").unwrap();
        assert_eq!(envelope.token(content).unwrap().kind, JsonType::Array);
        assert_eq!(envelope.token(content).unwrap().child_count, 1);
        let block = envelope.token(content).unwrap().first_child.unwrap();
        assert!(envelope.string_equals(envelope.object_get(block, b"type").unwrap(), b"text"));
        assert_eq!(
            envelope
                .decoded_string(envelope.object_get(block, b"text").unwrap())
                .unwrap(),
            expected
        );
    }
    sync.empty_content = false;
    sync.is_error = false;
    async_backend.disposition = AsyncDisposition::Deferred;
    let out = handle_async(
        &mut session,
        call.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert!(out.is_empty());
    let cancel =
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"legacy-1"}}"#;
    assert!(
        handle_async(
            &mut session,
            cancel.as_bytes(),
            Some(&mut sync),
            Some(&mut async_backend)
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(async_backend.legacy_cancels, 1);
    let mut modern = ProtocolSession::default();
    let out = handle_async(
        &mut modern,
        DISCOVER.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(DISCOVER, &out);
    let missing = r#"{"jsonrpc":"2.0","id":30,"method":"ping","params":{}}"#;
    let out = handle_async(
        &mut modern,
        missing.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(missing, &out);
    assert!(String::from_utf8_lossy(&out).contains("-32022"));
    async_backend.disposition = AsyncDisposition::Result;
    async_backend.payload = b"{bad".to_vec();
    let call = task_tool("31");
    let out = handle_async(
        &mut modern,
        call.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(&call, &out);
    assert!(String::from_utf8_lossy(&out).contains("invalid JSON"));
    async_backend.payload=br#"{"resultType":"task","taskId":"job-1","status":"working","createdAt":"2026-10-14T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:00Z","ttlMs":900000}"#.to_vec();
    let without_caps = r#"{"jsonrpc":"2.0","id":32,"method":"tools/call","params":{"name":"document_list","arguments":{},"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}"#;
    let out = handle_async(
        &mut modern,
        without_caps.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(without_caps, &out);
    assert!(String::from_utf8_lossy(&out).contains("-32021"));
    assert!(String::from_utf8_lossy(&out).contains("requiredCapabilities"));
    async_backend.disposition = AsyncDisposition::Deferred;
    let out = handle_async(
        &mut modern,
        without_caps.as_bytes(),
        Some(&mut sync),
        Some(&mut async_backend),
    )
    .unwrap();
    assert_response(without_caps, &out);
    assert!(String::from_utf8_lossy(&out).contains("deferred response is unavailable"));
}

#[test]
fn async_payload_limits_content_shapes_timestamps_ids_and_cancel_ack() {
    assert!(validate_async_payload(&vec![b' '; MAX_BYTES + 1], AsyncPayloadKind::TaskGet).is_err());
    let many_blocks = format!(
        "{{\"content\":[{}]}}",
        std::iter::repeat_n(r#"{"type":"text","text":""}"#, 14_000)
            .collect::<Vec<_>>()
            .join(",")
    );
    assert!(validate_async_payload(many_blocks.as_bytes(), AsyncPayloadKind::ToolResult).is_err());
    let task=br#"{"resultType":"task","taskId":"t","status":"cancelled","createdAt":"2024-02-29T23:59:60.123+14:00","lastUpdatedAt":"2024-03-01T00:00:00Z","ttlMs":900000}"#;
    assert_eq!(
        validate_async_payload(task, AsyncPayloadKind::ToolResult),
        Ok(true)
    );
    let nul=br#"{"resultType":"task","taskId":"\u0000","status":"working","createdAt":"2026-10-14T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:00Z","ttlMs":1}"#;
    assert!(validate_async_payload(nul, AsyncPayloadKind::ToolResult).is_err());
    let long_id = "x".repeat(1025);
    let too_long = format!(
        r#"{{"resultType":"task","taskId":"{long_id}","status":"working","createdAt":"2026-10-14T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:00Z","ttlMs":1}}"#
    );
    assert!(validate_async_payload(too_long.as_bytes(), AsyncPayloadKind::ToolResult).is_err());
    let bad_time=br#"{"resultType":"task","taskId":"t","status":"working","createdAt":"2026-02-29T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:00Z","ttlMs":1}"#;
    assert!(validate_async_payload(bad_time, AsyncPayloadKind::ToolResult).is_err());
    let bad_poll=br#"{"resultType":"task","taskId":"t","status":"working","createdAt":"2026-10-14T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:00Z","ttlMs":1,"pollIntervalMs":0}"#;
    assert!(validate_async_payload(bad_poll, AsyncPayloadKind::ToolResult).is_err());
    let malformed = [
        br#"{"content":[null]}"#.as_slice(),
        br#"{"content":[{}]}"#,
        br#"{"content":[{"type":"text"}]}"#,
        br#"{"content":[{"type":"video"}]}"#,
        br#"{"content":[{"type":"image","data":"x"}]}"#,
        br#"{"content":[{"type":"resource","resource":{"text":"x"}}]}"#,
        br#"{"content":[{"type":"resource_link","uri":"vexel://x"}]}"#,
    ];
    for bad in malformed {
        assert!(
            validate_async_payload(bad, AsyncPayloadKind::ToolResult).is_err(),
            "{}",
            String::from_utf8_lossy(bad)
        );
    }
    let valid=br#"{"content":[{"type":"text","text":"ok"},{"type":"image","data":"YQ==","mimeType":"image/png"},{"type":"audio","data":"YQ==","mimeType":"audio/wav"},{"type":"resource","resource":{"uri":"vexel://x","text":"x"}},{"type":"resource_link","uri":"vexel://x","name":"x"}]}"#;
    assert_eq!(
        validate_async_payload(valid, AsyncPayloadKind::ToolResult),
        Ok(false)
    );
    let completed=br#"{"resultType":"complete","taskId":"t","status":"failed","createdAt":"2026-10-14T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:00Z","ttlMs":1,"error":{"code":-32024,"message":"failed"}}"#;
    assert_eq!(
        validate_async_payload(completed, AsyncPayloadKind::TaskGet),
        Ok(false)
    );
    let missing_error=br#"{"resultType":"complete","taskId":"t","status":"failed","createdAt":"2026-10-14T00:00:00Z","lastUpdatedAt":"2026-10-14T00:00:00Z","ttlMs":1}"#;
    assert!(validate_async_payload(missing_error, AsyncPayloadKind::TaskGet).is_err());
    assert_eq!(
        validate_async_payload(
            br#"{"resultType":"complete"}"#,
            AsyncPayloadKind::TaskCancel
        ),
        Ok(false)
    );
    assert!(
        validate_async_payload(
            br#"{"resultType":"complete","taskId":"surplus"}"#,
            AsyncPayloadKind::TaskCancel
        )
        .is_err()
    );
}

#[test]
fn legacy_progress_is_monotone_bounded_and_atomic() {
    let mut out = vexel_json::JsonBuffer::new(16, 2048).unwrap();
    let mut prior = -1;
    legacy_progress(br#""p""#, 250, &mut prior, "quarter \" done", &mut out).unwrap();
    let doc = json(out.as_bytes());
    assert_eq!(
        doc.object_get(doc.root(), b"method")
            .map(|v| doc.decoded_string(v).unwrap()),
        Some(b"notifications/progress".to_vec())
    );
    assert_eq!(prior, 250);
    for (token, p, message) in [
        (b"1.5".as_slice(), 500, "decimal"),
        (b"[]", 500, "array"),
        (b"\"p\"", 250, "same"),
        (b"true", 500, "bool"),
    ] {
        let before = out.as_bytes().to_vec();
        assert!(legacy_progress(token, p, &mut prior, message, &mut out).is_err());
        assert_eq!(out.as_bytes(), before);
        assert_eq!(prior, 250);
    }
    let long = "x".repeat(128);
    assert!(legacy_progress(b"17", 300, &mut prior, &long, &mut out).is_err());
    assert_eq!(prior, 250);
    assert!(legacy_progress(&vec![b' '; 1025], 300, &mut prior, "large", &mut out).is_err());
    assert!(legacy_progress(b"17", 1001, &mut prior, "range", &mut out).is_err());
    legacy_progress(b"17", 260, &mut prior, "integer token", &mut out).unwrap();
    assert_eq!(prior, 260);
}

#[test]
fn every_tool_schema_and_validation_registry_is_available() {
    let bytes = tool_schemas().unwrap();
    let schemas = json(&bytes);
    assert_eq!(schemas.token(schemas.root()).unwrap().kind, JsonType::Array);
    assert_eq!(schemas.token(schemas.root()).unwrap().child_count, 42);
    let doc = json(br#"{"documentId":"doc","mode":"ping_pong"}"#);
    validate_tool_arguments(&doc, doc.root(), "native_timeline_set_loop").unwrap();
    let bad = json(br#"{"documentId":"doc","mode":"bounce"}"#);
    assert!(validate_tool_arguments(&bad, bad.root(), "native_timeline_set_loop").is_err());
    let args = json(
        br#"{"documentId":"doc","trackId":"ink","tick":12,"assetName":"cel0","label":"pose"}"#,
    );
    validate_tool_arguments(&args, args.root(), "native_timeline_set_cel").unwrap();
    let unknown = json(br#"{}"#);
    assert!(validate_tool_arguments(&unknown, unknown.root(), "no_tool").is_err());
}

#[test]
fn notifications_do_not_emit_and_unknown_requests_do() {
    assert!(!request_expects_response(
        br#"{"jsonrpc":"2.0","method":"notifications/progress"}"#
    ));
    assert!(!request_expects_response(
        br#"{"jsonrpc":"2.0","method":"notifications/cancelled"}"#
    ));
    assert!(request_expects_response(
        br#"{"jsonrpc":"2.0","method":"made/up"}"#
    ));
    let mut session = ProtocolSession::default();
    let request = r#"{"jsonrpc":"2.0","method":"notifications/progress"}"#;
    let out = handle::<SyncFake>(&mut session, request.as_bytes(), None).unwrap();
    assert!(out.is_empty());
}

#[test]
fn request_rate_limit_is_per_session_and_has_an_rpc_error_envelope() {
    let mut session = ProtocolSession::default();
    for id in 0..120 {
        let request = format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"ping","params":{{}}}}"#);
        let output = handle::<SyncFake>(&mut session, request.as_bytes(), None).unwrap();
        assert_response(&request, &output);
    }
    let request = r#"{"jsonrpc":"2.0","id":120,"method":"ping","params":{}}"#;
    let output = handle::<SyncFake>(&mut session, request.as_bytes(), None).unwrap();
    assert_response(request, &output);
    assert!(String::from_utf8_lossy(&output).contains("rate limit exceeded"));
    session.reset();
    let output = handle::<SyncFake>(&mut session, request.as_bytes(), None).unwrap();
    assert_response(request, &output);
    assert!(String::from_utf8_lossy(&output).contains("\"result\":{}"));
}
