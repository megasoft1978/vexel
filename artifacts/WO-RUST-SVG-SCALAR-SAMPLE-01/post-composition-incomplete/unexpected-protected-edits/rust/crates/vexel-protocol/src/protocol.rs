use crate::{
    AsyncBackend, AsyncDisposition, AsyncPayloadKind, ProtocolBackend, ProtocolError, append_raw,
    append_string, validate_async_payload, validate_tool_arguments,
};
use std::time::Instant;
use vexel_json::{JsonBuffer, JsonDocument, JsonType, MAX_BYTES};

const LEGACY: &[u8] = b"2025-11-25";
const CURRENT: &[u8] = b"2026-07-28";
const LIMIT: usize = MAX_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProtocolVersion {
    Legacy,
    Current,
}

#[derive(Debug, Default)]
pub struct ProtocolSession {
    pub initialized: bool,
    pub modern: bool,
    protocol_version: Option<ProtocolVersion>,
    window_started: Option<Instant>,
    requests_in_window: u32,
}
impl ProtocolSession {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn protocol_version(&self) -> Option<&'static str> {
        match self.protocol_version {
            Some(ProtocolVersion::Legacy) => Some("2025-11-25"),
            Some(ProtocolVersion::Current) => Some("2026-07-28"),
            None => None,
        }
    }
}

struct NoAsync;
impl AsyncBackend for NoAsync {
    fn tasks_available(&self) -> bool {
        false
    }
    fn call_tool(
        &mut self,
        _: &JsonDocument<'_>,
        _: usize,
        _: &str,
        _: bool,
        _: bool,
    ) -> (AsyncDisposition, Vec<u8>) {
        (AsyncDisposition::Unhandled, Vec::new())
    }
    fn get_task(&mut self, _: &JsonDocument<'_>, _: &str) -> (AsyncDisposition, Vec<u8>) {
        (AsyncDisposition::Failed, Vec::new())
    }
    fn cancel_task(&mut self, _: &JsonDocument<'_>, _: &str) -> (AsyncDisposition, Vec<u8>) {
        (AsyncDisposition::Failed, Vec::new())
    }
}
pub fn handle<B: ProtocolBackend + ?Sized>(
    session: &mut ProtocolSession,
    line: &[u8],
    backend: Option<&mut B>,
) -> Result<Vec<u8>, ProtocolError> {
    handle_async::<B, NoAsync>(session, line, backend, None)
}

pub fn handle_async<B: ProtocolBackend + ?Sized, A: AsyncBackend + ?Sized>(
    session: &mut ProtocolSession,
    line: &[u8],
    mut backend: Option<&mut B>,
    mut async_backend: Option<&mut A>,
) -> Result<Vec<u8>, ProtocolError> {
    if line.is_empty() || line.len() > LIMIT {
        return Err(ProtocolError::InvalidInput);
    }
    let mut out = buffer()?;
    let doc = match JsonDocument::parse(line) {
        Ok(v) => v,
        Err(_) => {
            append_error(&mut out, None, -32700, "parse error", None)?;
            return bytes(out);
        }
    };
    let root = doc.root();
    if doc.token(root).is_none_or(|t| t.kind != JsonType::Object) {
        append_error(&mut out, None, -32600, "invalid request", None)?;
        return bytes(out);
    }
    let id = doc.object_get(root, b"id");
    let method_token = doc.object_get(root, b"method");
    let valid_id = id.is_none_or(|v| {
        doc.token(v)
            .is_some_and(|t| matches!(t.kind, JsonType::String | JsonType::Number))
    });
    let Some(method_token) =
        method_token.filter(|v| doc.token(*v).is_some_and(|t| t.kind == JsonType::String))
    else {
        if let Some(id) = id {
            if valid_id {
                append_error(
                    &mut out,
                    Some(doc.raw_token(id).unwrap_or(b"null")),
                    -32600,
                    "invalid request",
                    None,
                )?;
            }
        }
        return bytes(out);
    };
    if !doc.string_equals(
        doc.object_get(root, b"jsonrpc").unwrap_or(usize::MAX),
        b"2.0",
    ) || !valid_id
    {
        if let Some(id) = id.filter(|_| valid_id) {
            append_error(
                &mut out,
                Some(doc.raw_token(id).unwrap_or(b"null")),
                -32600,
                "invalid request",
                None,
            )?;
        }
        return bytes(out);
    }
    let method_bytes = doc
        .decoded_string(method_token)
        .ok_or(ProtocolError::InvalidInput)?;
    let method = std::str::from_utf8(&method_bytes).unwrap_or("");
    let has_id = id.is_some();
    if !has_id && !method.starts_with("notifications/") {
        return Ok(Vec::new());
    }
    rate_check(session, has_id, id.and_then(|i| doc.raw_token(i)), &mut out)?;
    if !out.is_empty() {
        return bytes(out);
    }
    let params = doc.object_get(root, b"params");
    let modern_probe = method == "server/discover";
    let modern_request = request_is_modern(&doc, params);
    if modern_probe {
        if has_id {
            if valid_modern_envelope(&doc, params) {
                append_discover(
                    session,
                    &doc,
                    id.unwrap(),
                    async_backend
                        .as_deref()
                        .is_some_and(|b| b.tasks_available()),
                    &mut out,
                )?;
            } else {
                append_error(
                    &mut out,
                    id.and_then(|i| doc.raw_token(i)),
                    -32022,
                    "server/discover requires current protocol version metadata",
                    Some(r#"{"supported":["2026-07-28"]}"#),
                )?;
            }
        }
        return bytes(out);
    }
    if session.modern || modern_request {
        if !valid_modern_envelope(&doc, params) {
            if has_id {
                append_error(
                    &mut out,
                    id.and_then(|i| doc.raw_token(i)),
                    -32022,
                    "unsupported or missing protocol version metadata",
                    Some(r#"{"supported":["2026-07-28"]}"#),
                )?;
            }
            return bytes(out);
        }
        session.modern = true;
        session.protocol_version = Some(ProtocolVersion::Current);
    } else if method != "initialize"
        && method != "notifications/initialized"
        && !session.initialized
        && method != "ping"
    {
        if has_id {
            append_error(
                &mut out,
                id.and_then(|i| doc.raw_token(i)),
                -32002,
                "initialize must complete before requests",
                None,
            )?;
        }
        return bytes(out);
    }
    if method == "initialize" {
        if has_id {
            append_initialize(session, &doc, params, id.unwrap(), &mut out)?;
        }
    } else if method == "notifications/initialized" {
        session.initialized = true;
    } else if method == "notifications/cancelled" {
        if !has_id && session.initialized && !session.modern {
            if let (Some(a), Some(p)) = (async_backend.as_deref_mut(), params) {
                if doc.token(p).is_some_and(|t| t.kind == JsonType::Object) {
                    if let Some(req) = doc.object_get(p, b"requestId").filter(|v| {
                        doc.token(*v)
                            .is_some_and(|t| matches!(t.kind, JsonType::String | JsonType::Number))
                    }) {
                        a.cancel_request(&doc, req);
                    }
                }
            }
        }
    } else if method == "notifications/progress" {
    } else if method == "ping" {
        if has_id {
            response_prefix(&mut out, &doc, id.unwrap())?;
            response_suffix(&mut out, session.modern)?;
        }
    } else if method == "tools/list" {
        if has_id {
            response_prefix(&mut out, &doc, id.unwrap())?;
            lit(&mut out, "\"tools\":")?;
            out.append(&crate::tool_schemas()?)
                .map_err(ProtocolError::Buffer)?;
            response_suffix(&mut out, session.modern)?;
        }
    } else if method == "tools/call" {
        if has_id {
            append_tool_call(
                session,
                &doc,
                params,
                id.unwrap(),
                backend.as_deref_mut(),
                async_backend.as_deref_mut(),
                &mut out,
            )?;
        }
    } else if method == "tasks/get" || method == "tasks/cancel" {
        if has_id {
            append_task_method(
                session.modern,
                &doc,
                params,
                id.unwrap(),
                method,
                async_backend.as_deref_mut(),
                &mut out,
            )?;
        }
    } else if method == "resources/list" {
        if has_id {
            response_prefix(&mut out, &doc, id.unwrap())?;
            lit(
                &mut out,
                r#""resources":[{"uri":"vexel://documents/current/source","name":"Current SVG source","description":"Source-preserving SVG text for the current document.","mimeType":"image/svg+xml"},{"uri":"vexel://documents/current/tree","name":"Current SVG tree","description":"Bounded element-tree summary for the current document.","mimeType":"application/json"},{"uri":"vexel://documents/current/render","name":"Last document render","description":"Last bounded PNG render.","mimeType":"image/png"}]"#,
            )?;
            response_suffix(&mut out, session.modern)?;
        }
    } else if method == "resources/read" {
        if has_id {
            append_resource_read(
                session,
                &doc,
                params,
                id.unwrap(),
                backend.as_deref_mut(),
                &mut out,
            )?;
        }
    } else if method == "prompts/list" {
        if has_id {
            response_prefix(&mut out, &doc, id.unwrap())?;
            lit(
                &mut out,
                r#""prompts":[{"name":"inspect-document","description":"Inspect the open SVG structure and identify unsupported content.","arguments":[]}]"#,
            )?;
            response_suffix(&mut out, session.modern)?;
        }
    } else if method == "prompts/get" {
        if has_id {
            response_prefix(&mut out, &doc, id.unwrap())?;
            if params
                .and_then(|p| doc.object_get(p, b"name"))
                .is_some_and(|v| doc.string_equals(v, b"inspect-document"))
            {
                lit(
                    &mut out,
                    r#""description":"Inspect the open SVG document.","messages":[{"role":"user","content":{"type":"text","text":"Use document_list and document_info, then inspect a bounded tree page. Report unsupported features before suggesting edits."}}]"#,
                )?;
            } else {
                lit(&mut out, "\"messages\":[]")?;
            }
            response_suffix(&mut out, session.modern)?;
        }
    } else if has_id {
        append_error(
            &mut out,
            id.and_then(|i| doc.raw_token(i)),
            -32601,
            "method not found",
            None,
        )?;
    }
    bytes(out)
}

fn buffer() -> Result<JsonBuffer, ProtocolError> {
    JsonBuffer::new(256, LIMIT).map_err(ProtocolError::Buffer)
}
fn bytes(b: JsonBuffer) -> Result<Vec<u8>, ProtocolError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(b.len())
        .map_err(|_| ProtocolError::Allocation)?;
    result.extend_from_slice(b.as_bytes());
    Ok(result)
}
fn lit(out: &mut JsonBuffer, s: &str) -> Result<(), ProtocolError> {
    out.literal(s).map_err(ProtocolError::Buffer)
}
fn rate_check(
    session: &mut ProtocolSession,
    has_id: bool,
    id: Option<&[u8]>,
    out: &mut JsonBuffer,
) -> Result<(), ProtocolError> {
    let now = Instant::now();
    if session
        .window_started
        .is_none_or(|start| now.duration_since(start).as_millis() >= 60_000)
    {
        session.window_started = Some(now);
        session.requests_in_window = 0;
    }
    session.requests_in_window = session.requests_in_window.saturating_add(1);
    if session.requests_in_window > 120 && has_id {
        append_error(out, id, -32000, "rate limit exceeded", None)?;
    }
    Ok(())
}
fn append_error(
    out: &mut JsonBuffer,
    id: Option<&[u8]>,
    code: i32,
    message: &str,
    data: Option<&str>,
) -> Result<(), ProtocolError> {
    lit(out, "{\"jsonrpc\":\"2.0\",\"id\":")?;
    out.append(id.unwrap_or(b"null"))
        .map_err(ProtocolError::Buffer)?;
    lit(out, ",\"error\":{\"code\":")?;
    append_i32(out, code)?;
    lit(out, ",\"message\":")?;
    append_string(out, message.as_bytes())?;
    if let Some(data) = data {
        lit(out, ",\"data\":")?;
        lit(out, data)?;
    }
    lit(out, "}}")
}
fn append_i32(out: &mut JsonBuffer, value: i32) -> Result<(), ProtocolError> {
    let mut digits = [0u8; 10];
    let mut n = value.unsigned_abs();
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if value < 0 {
        out.append(b"-").map_err(ProtocolError::Buffer)?;
    }
    out.append(&digits[at..]).map_err(ProtocolError::Buffer)
}
fn response_prefix(
    out: &mut JsonBuffer,
    doc: &JsonDocument<'_>,
    id: usize,
) -> Result<(), ProtocolError> {
    lit(out, "{\"jsonrpc\":\"2.0\",\"id\":")?;
    append_raw(out, doc, id)?;
    lit(out, ",\"result\":{")
}
fn response_suffix(out: &mut JsonBuffer, modern: bool) -> Result<(), ProtocolError> {
    if modern {
        let comma = !(out.as_bytes().last() == Some(&b'{'));
        if comma {
            lit(out, ",")?;
        }
        lit(
            out,
            r#""_meta":{"io.modelcontextprotocol/serverInfo":{"name":"Vexel","version":"0.1.0"}}"#,
        )?;
    }
    lit(out, "}}")
}
fn request_is_modern(doc: &JsonDocument<'_>, params: Option<usize>) -> bool {
    params
        .and_then(|p| doc.object_get(p, b"_meta"))
        .and_then(|m| doc.object_get(m, b"io.modelcontextprotocol/protocolVersion"))
        .is_some_and(|v| doc.string_equals(v, CURRENT))
}
fn valid_modern_envelope(doc: &JsonDocument<'_>, params: Option<usize>) -> bool {
    request_is_modern(doc, params)
}
fn append_capabilities(out: &mut JsonBuffer, tasks: bool) -> Result<(), ProtocolError> {
    lit(out, "\"capabilities\":{")?;
    if tasks {
        lit(out, r#""io.modelcontextprotocol/tasks":{},"#)?;
    }
    lit(
        out,
        r#""tools":{"listChanged":false},"resources":{"subscribe":false,"listChanged":false},"prompts":{"listChanged":false}},"#,
    )
}
fn append_initialize(
    session: &mut ProtocolSession,
    doc: &JsonDocument<'_>,
    params: Option<usize>,
    id: usize,
    out: &mut JsonBuffer,
) -> Result<(), ProtocolError> {
    if session.initialized || session.modern {
        return append_error(
            out,
            doc.raw_token(id),
            -32600,
            "initialize is not valid in this protocol session",
            None,
        );
    }
    let version = params.and_then(|p| doc.object_get(p, b"protocolVersion"));
    if !version.is_some_and(|v| doc.string_equals(v, LEGACY)) {
        return append_error(
            out,
            doc.raw_token(id),
            -32004,
            "unsupported protocol version",
            Some(r#"{"supported":["2025-11-25"]}"#),
        );
    }
    session.protocol_version = Some(ProtocolVersion::Legacy);
    response_prefix(out, doc, id)?;
    lit(out, r#""protocolVersion":"2025-11-25","#)?;
    append_capabilities(out, false)?;
    lit(
        out,
        r#""serverInfo":{"name":"Vexel","version":"0.1.0"},"instructions":"Vexel MCP controls documents open in the Vexel app. File opens and Save As are restricted to user-granted directories."}}"#,
    )
}
fn append_discover(
    session: &mut ProtocolSession,
    doc: &JsonDocument<'_>,
    id: usize,
    tasks: bool,
    out: &mut JsonBuffer,
) -> Result<(), ProtocolError> {
    session.modern = true;
    session.protocol_version = Some(ProtocolVersion::Current);
    response_prefix(out, doc, id)?;
    lit(out, r#""supportedVersions":["2026-07-28"],"#)?;
    append_capabilities(out, tasks)?;
    lit(
        out,
        r#""instructions":"Vexel MCP controls documents open in the Vexel app. File opens and Save As are restricted to user-granted directories.","ttlMs":10000,"cacheScope":"connection""#,
    )?;
    response_suffix(out, true)
}

fn append_tool_call<B: ProtocolBackend + ?Sized, A: AsyncBackend + ?Sized>(
    session: &mut ProtocolSession,
    doc: &JsonDocument<'_>,
    params: Option<usize>,
    id: usize,
    mut backend: Option<&mut B>,
    mut async_backend: Option<&mut A>,
    out: &mut JsonBuffer,
) -> Result<(), ProtocolError> {
    let name_token = params.and_then(|p| doc.object_get(p, b"name"));
    let args = params.and_then(|p| doc.object_get(p, b"arguments"));
    let name = name_token
        .and_then(|v| doc.decoded_string(v))
        .and_then(|v| String::from_utf8(v).ok());
    let (Some(name), Some(args)) = (name, args) else {
        return append_error(
            out,
            doc.raw_token(id),
            -32602,
            "tools/call requires a tool name and arguments",
            None,
        );
    };
    if name.is_empty() || name.len() > 63 || name.contains('\0') {
        return append_error(
            out,
            doc.raw_token(id),
            -32602,
            "tools/call requires a tool name and arguments",
            None,
        );
    }
    if let Err(error) = validate_tool_arguments(doc, args, &name) {
        let (msg, code) = match error {
            crate::ToolValidationError::UnknownTool => ("unknown tool", "unknown_tool"),
            crate::ToolValidationError::UnexpectedArgument => {
                ("unexpected argument", "invalid_arguments")
            }
            crate::ToolValidationError::WrongType => {
                ("argument has the wrong type", "invalid_arguments")
            }
            crate::ToolValidationError::MissingRequired => {
                ("required argument missing", "invalid_arguments")
            }
            crate::ToolValidationError::InvalidLoopMode => (
                "mode is not a supported native loop value",
                "invalid_arguments",
            ),
            crate::ToolValidationError::InvalidTemplateAttributes => (
                "template attributes must map strings to strings",
                "invalid_arguments",
            ),
            crate::ToolValidationError::InvalidTransactionLength => (
                "transaction must contain between 1 and 64 edits",
                "invalid_arguments",
            ),
            crate::ToolValidationError::InvalidArguments => {
                ("invalid tool arguments", "invalid_arguments")
            }
        };
        return append_tool_error(out, doc, id, session.modern, msg, code);
    }
    if let Some(a) = async_backend.as_deref_mut() {
        let ready = a.tasks_available();
        let supports = session.modern && request_supports_tasks(doc, params) && ready;
        let (disposition, payload) = a.call_tool(doc, args, &name, session.modern, supports);
        if disposition != AsyncDisposition::Unhandled {
            return async_callback_response(
                doc,
                id,
                session.modern,
                supports,
                ready,
                AsyncPayloadKind::ToolResult,
                disposition,
                &payload,
                out,
            );
        }
    }
    let Some(b) = backend.as_deref_mut() else {
        return append_tool_error(
            out,
            doc,
            id,
            session.modern,
            "Vexel broker is unavailable",
            "broker_unavailable",
        );
    };
    let Some(result) = b.call_tool(doc, args, &name) else {
        return append_tool_error(
            out,
            doc,
            id,
            session.modern,
            "The tool could not complete its request",
            "tool_failed",
        );
    };
    if result.structured_json.is_empty() {
        return append_tool_error(
            out,
            doc,
            id,
            session.modern,
            "The tool could not complete its request",
            "tool_failed",
        );
    }
    response_prefix(out, doc, id)?;
    lit(out, "\"content\":")?;
    if result.content_json.is_empty() {
        lit(out, "[{\"type\":\"text\",\"text\":")?;
        append_string(
            out,
            if result.is_error {
                b"Tool request failed"
            } else {
                b"Tool completed."
            },
        )?;
        lit(out, "}]")?;
    } else {
        out.append(&result.content_json)
            .map_err(ProtocolError::Buffer)?;
    }
    lit(out, ",\"structuredContent\":")?;
    out.append(&result.structured_json)
        .map_err(ProtocolError::Buffer)?;
    lit(
        out,
        if result.is_error {
            ",\"isError\":true"
        } else {
            ",\"isError\":false"
        },
    )?;
    response_suffix(out, session.modern)
}
fn append_tool_error(
    out: &mut JsonBuffer,
    doc: &JsonDocument<'_>,
    id: usize,
    modern: bool,
    message: &str,
    code: &str,
) -> Result<(), ProtocolError> {
    response_prefix(out, doc, id)?;
    lit(out, "\"content\":[{\"type\":\"text\",\"text\":")?;
    append_string(out, message.as_bytes())?;
    lit(out, "}],\"structuredContent\":{\"error\":{\"code\":")?;
    append_string(out, code.as_bytes())?;
    lit(out, ",\"message\":")?;
    append_string(out, message.as_bytes())?;
    lit(out, "}},\"isError\":true")?;
    response_suffix(out, modern)
}
fn request_supports_tasks(doc: &JsonDocument<'_>, params: Option<usize>) -> bool {
    params
        .and_then(|p| doc.object_get(p, b"_meta"))
        .and_then(|m| doc.object_get(m, b"io.modelcontextprotocol/clientCapabilities"))
        .and_then(|c| doc.object_get(c, b"extensions"))
        .and_then(|e| doc.object_get(e, b"io.modelcontextprotocol/tasks"))
        .is_some_and(|t| doc.token(t).is_some_and(|v| v.kind == JsonType::Object))
}

fn async_callback_response(
    doc: &JsonDocument<'_>,
    id: usize,
    modern: bool,
    supports: bool,
    ready: bool,
    kind: AsyncPayloadKind,
    disposition: AsyncDisposition,
    payload: &[u8],
    out: &mut JsonBuffer,
) -> Result<(), ProtocolError> {
    if disposition == AsyncDisposition::Deferred {
        if modern {
            return append_error(
                out,
                doc.raw_token(id),
                -32603,
                "deferred response is unavailable for the current protocol",
                None,
            );
        }
        return Ok(());
    }
    if disposition == AsyncDisposition::Failed {
        return append_error(
            out,
            doc.raw_token(id),
            -32603,
            "asynchronous protocol operation failed",
            None,
        );
    }
    if !matches!(
        disposition,
        AsyncDisposition::Result | AsyncDisposition::Error
    ) {
        return append_error(
            out,
            doc.raw_token(id),
            -32603,
            "invalid asynchronous protocol disposition",
            None,
        );
    }
    let is_error = disposition == AsyncDisposition::Error;
    let validation_kind = if is_error {
        AsyncPayloadKind::Error
    } else {
        kind
    };
    let is_task = match validate_async_payload(payload, validation_kind) {
        Ok(v) => v,
        Err(()) => {
            return append_error(
                out,
                doc.raw_token(id),
                -32603,
                "asynchronous backend returned invalid JSON",
                None,
            );
        }
    };
    let overhead = 512usize
        .checked_add(doc.raw_token(id).map_or(0, |v| v.len()))
        .ok_or(ProtocolError::InvalidInput)?;
    if overhead >= LIMIT || payload.len() > LIMIT - overhead {
        return append_error(
            out,
            doc.raw_token(id),
            -32603,
            "asynchronous result exceeds the response limit",
            None,
        );
    }
    if is_task && (!supports || !ready) {
        return append_error(
            out,
            doc.raw_token(id),
            -32021,
            "task support was not negotiated for this request",
            Some(r#"{"requiredCapabilities":{"io.modelcontextprotocol/tasks":{}}}"#),
        );
    }
    append_async_payload(out, doc, id, payload, modern, is_error)
}
fn append_async_payload(
    out: &mut JsonBuffer,
    request: &JsonDocument<'_>,
    id: usize,
    payload: &[u8],
    modern: bool,
    is_error: bool,
) -> Result<(), ProtocolError> {
    lit(out, "{\"jsonrpc\":\"2.0\",\"id\":")?;
    append_raw(out, request, id)?;
    if is_error {
        lit(out, ",\"error\":")?;
        out.append(payload).map_err(ProtocolError::Buffer)?;
        return lit(out, "}");
    }
    lit(out, ",\"result\":")?;
    if !modern {
        out.append(payload).map_err(ProtocolError::Buffer)?;
        return lit(out, "}");
    }
    let result = JsonDocument::parse(payload).map_err(|_| ProtocolError::InvalidInput)?;
    let root = result.root();
    lit(out, "{")?;
    let mut first = true;
    let mut key = result.token(root).and_then(|t| t.first_child);
    while let Some(k) = key {
        let value = result
            .token(k)
            .and_then(|t| t.next_sibling)
            .ok_or(ProtocolError::InvalidInput)?;
        let next = result.token(value).and_then(|t| t.next_sibling);
        if !result.string_equals(k, b"_meta") {
            if !first {
                lit(out, ",")?;
            }
            append_raw(out, &result, k)?;
            lit(out, ":")?;
            append_raw(out, &result, value)?;
            first = false;
        }
        key = next;
    }
    if !first {
        lit(out, ",")?;
    }
    lit(out, "\"_meta\":{")?;
    let meta = result.object_get(root, b"_meta");
    let mut meta_first = true;
    if let Some(meta) = meta {
        let mut key = result.token(meta).and_then(|t| t.first_child);
        while let Some(k) = key {
            let value = result
                .token(k)
                .and_then(|t| t.next_sibling)
                .ok_or(ProtocolError::InvalidInput)?;
            let next = result.token(value).and_then(|t| t.next_sibling);
            if !result.string_equals(k, b"io.modelcontextprotocol/serverInfo") {
                if !meta_first {
                    lit(out, ",")?;
                }
                append_raw(out, &result, k)?;
                lit(out, ":")?;
                append_raw(out, &result, value)?;
                meta_first = false;
            }
            key = next;
        }
    }
    if !meta_first {
        lit(out, ",")?;
    }
    lit(
        out,
        r#""io.modelcontextprotocol/serverInfo":{"name":"Vexel","version":"0.1.0"}}}"#,
    )?;
    lit(out, "}")
}
fn append_task_method<A: AsyncBackend + ?Sized>(
    modern: bool,
    doc: &JsonDocument<'_>,
    params: Option<usize>,
    id: usize,
    method: &str,
    async_backend: Option<&mut A>,
    out: &mut JsonBuffer,
) -> Result<(), ProtocolError> {
    let Some(a) = async_backend else {
        return append_error(out, doc.raw_token(id), -32601, "method not found", None);
    };
    if !modern || !a.tasks_available() {
        return append_error(out, doc.raw_token(id), -32601, "method not found", None);
    }
    if !request_supports_tasks(doc, params) {
        return append_error(
            out,
            doc.raw_token(id),
            -32021,
            "task support was not negotiated for this request",
            Some(r#"{"requiredCapabilities":{"io.modelcontextprotocol/tasks":{}}}"#),
        );
    }
    let task_id = params
        .and_then(|p| doc.object_get(p, b"taskId"))
        .and_then(|v| doc.decoded_string(v));
    let Some(task_id) = task_id.filter(|v| !v.is_empty() && v.len() <= 1024 && !v.contains(&0))
    else {
        return append_error(
            out,
            doc.raw_token(id),
            -32602,
            "task method requires a valid taskId",
            None,
        );
    };
    let task_id = String::from_utf8(task_id).map_err(|_| ProtocolError::InvalidInput)?;
    let (disposition, payload) = if method == "tasks/get" {
        a.get_task(doc, &task_id)
    } else {
        a.cancel_task(doc, &task_id)
    };
    async_callback_response(
        doc,
        id,
        true,
        true,
        true,
        if method == "tasks/get" {
            AsyncPayloadKind::TaskGet
        } else {
            AsyncPayloadKind::TaskCancel
        },
        disposition,
        &payload,
        out,
    )
}
fn append_resource_read<B: ProtocolBackend + ?Sized>(
    session: &mut ProtocolSession,
    doc: &JsonDocument<'_>,
    params: Option<usize>,
    id: usize,
    backend: Option<&mut B>,
    out: &mut JsonBuffer,
) -> Result<(), ProtocolError> {
    let uri = params
        .and_then(|p| doc.object_get(p, b"uri"))
        .and_then(|t| doc.decoded_string(t));
    let Some(uri) = uri.filter(|v| v.len() < 256 && !v.contains(&0)) else {
        return append_error(
            out,
            doc.raw_token(id),
            -32602,
            "resources/read requires a URI",
            None,
        );
    };
    let uri = String::from_utf8(uri).map_err(|_| ProtocolError::InvalidInput)?;
    let contents = backend.and_then(|b| b.read_resource(doc, &uri));
    let Some(contents) = contents else {
        return append_error(out, doc.raw_token(id), -32002, "resource not found", None);
    };
    response_prefix(out, doc, id)?;
    lit(out, "\"contents\":")?;
    out.append(&contents).map_err(ProtocolError::Buffer)?;
    response_suffix(out, session.modern)
}

pub fn request_expects_response(line: &[u8]) -> bool {
    let Ok(doc) = JsonDocument::parse(line) else {
        return true;
    };
    let root = doc.root();
    if doc.token(root).is_none_or(|t| t.kind != JsonType::Object) {
        return true;
    }
    let method = doc.object_get(root, b"method");
    let known = method.is_some_and(|m| {
        [
            b"notifications/initialized".as_slice(),
            b"notifications/cancelled",
            b"notifications/progress",
        ]
        .iter()
        .any(|v| doc.string_equals(m, v))
    });
    if known {
        false
    } else {
        doc.object_get(root, b"id").is_some() || true
    }
}
