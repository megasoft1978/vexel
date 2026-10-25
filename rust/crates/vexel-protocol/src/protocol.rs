use crate::{
    AsyncBackend, AsyncDisposition, ProtocolBackend, ProtocolError, append_raw, append_string,
};

mod tool_methods;
use std::time::Instant;
use tool_methods::{append_resource_read, append_task_method, append_tool_call};
use vexel_json::{JsonBuffer, JsonDocument, JsonType, MAX_BYTES};

const LEGACY: &[u8] = b"2025-11-25";
const SHIPPING: &[u8] = b"2025-06-18";
const CURRENT: &[u8] = b"2026-07-28";
const LIMIT: usize = MAX_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProtocolVersion {
    Shipping,
    Legacy,
    Current,
}

#[derive(Debug, Default)]
pub struct ProtocolSession {
    pub initialized: bool,
    pub modern: bool,
    live_preview: bool,
    protocol_version: Option<ProtocolVersion>,
    window_started: Option<Instant>,
    requests_in_window: u32,
}
impl ProtocolSession {
    pub fn live_preview() -> Self {
        Self {
            live_preview: true,
            ..Self::default()
        }
    }
    pub fn reset(&mut self) {
        let live_preview = self.live_preview;
        *self = Self {
            live_preview,
            ..Self::default()
        };
    }
    pub fn protocol_version(&self) -> Option<&'static str> {
        match self.protocol_version {
            Some(ProtocolVersion::Shipping) => Some("2025-06-18"),
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
    let params = doc.object_get(root, b"params");
    let weight = crate::tools_batch::request_weight(&doc, method, params);
    rate_check(
        session,
        has_id,
        weight,
        id.and_then(|i| doc.raw_token(i)),
        &mut out,
    )?;
    if !out.is_empty() {
        return bytes(out);
    }
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
            out.append(&crate::tools::tool_schemas_for_preview(
                session.live_preview,
            )?)
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
            if session.live_preview {
                append_error(
                    &mut out,
                    id.and_then(|i| doc.raw_token(i)),
                    -32601,
                    "method not found",
                    None,
                )?;
            } else {
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
        }
    } else if method == "resources/list" && !session.live_preview {
        if has_id {
            response_prefix(&mut out, &doc, id.unwrap())?;
            lit(
                &mut out,
                r#""resources":[{"uri":"vexel://documents/current/source","name":"Current SVG source","description":"Source-preserving SVG text for the current document.","mimeType":"image/svg+xml"},{"uri":"vexel://documents/current/tree","name":"Current SVG tree","description":"Bounded element-tree summary for the current document.","mimeType":"application/json"},{"uri":"vexel://documents/current/render","name":"Last document render","description":"Last bounded PNG render.","mimeType":"image/png"}]"#,
            )?;
            response_suffix(&mut out, session.modern)?;
        }
    } else if method == "resources/read" && !session.live_preview {
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
    } else if method == "prompts/list" && !session.live_preview {
        if has_id {
            response_prefix(&mut out, &doc, id.unwrap())?;
            lit(
                &mut out,
                r#""prompts":[{"name":"inspect-document","description":"Inspect the open SVG structure and identify unsupported content.","arguments":[]}]"#,
            )?;
            response_suffix(&mut out, session.modern)?;
        }
    } else if method == "prompts/get" && !session.live_preview {
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
/// Per-session request budget per 60 s window (owner raised from 120 for scripted scene building).
/// Requests are weighted: a batch costs 1 + ops/8, every other request 1.
const MAX_REQUESTS_PER_MINUTE: u32 = 600;
fn rate_check(
    session: &mut ProtocolSession,
    has_id: bool,
    weight: u32,
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
    session.requests_in_window = session.requests_in_window.saturating_add(weight);
    if session.requests_in_window > MAX_REQUESTS_PER_MINUTE && has_id {
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
fn append_capabilities(
    out: &mut JsonBuffer,
    tasks: bool,
    preview: bool,
) -> Result<(), ProtocolError> {
    lit(out, "\"capabilities\":{")?;
    if tasks && !preview {
        lit(out, r#""io.modelcontextprotocol/tasks":{},"#)?;
    }
    lit(out, r#""tools":{"listChanged":false}"#)?;
    if !preview {
        lit(
            out,
            r#","resources":{"subscribe":false,"listChanged":false},"prompts":{"listChanged":false}"#,
        )?;
    }
    lit(out, "},")
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
    let shipping_preview =
        session.live_preview && version.is_some_and(|v| doc.string_equals(v, SHIPPING));
    if !shipping_preview && !version.is_some_and(|v| doc.string_equals(v, LEGACY)) {
        return append_error(
            out,
            doc.raw_token(id),
            -32004,
            "unsupported protocol version",
            Some(r#"{"supported":["2025-11-25"]}"#),
        );
    }
    session.protocol_version = Some(if shipping_preview {
        ProtocolVersion::Shipping
    } else {
        ProtocolVersion::Legacy
    });
    response_prefix(out, doc, id)?;
    lit(out, r#""protocolVersion":""#)?;
    lit(
        out,
        if shipping_preview {
            "2025-06-18"
        } else {
            "2025-11-25"
        },
    )?;
    lit(out, r#"","#)?;
    append_capabilities(out, false, session.live_preview)?;
    let instructions = if session.live_preview {
        "Vexel live preview supports inspecting the current document, adding basic rectangles and ellipses, and undo/redo."
    } else {
        "Vexel MCP controls documents open in the Vexel app. File opens and Save As are restricted to user-granted directories."
    };
    lit(
        out,
        r#""serverInfo":{"name":"Vexel","version":"0.1.0"},"instructions":"#,
    )?;
    append_string(out, instructions.as_bytes())?;
    lit(out, "}}")
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
    append_capabilities(out, tasks, session.live_preview)?;
    let instructions = if session.live_preview {
        "Vexel live preview supports inspecting the current document, adding basic rectangles and ellipses, and undo/redo."
    } else {
        "Vexel MCP controls documents open in the Vexel app. File opens and Save As are restricted to user-granted directories."
    };
    lit(out, r#""instructions":"#)?;
    append_string(out, instructions.as_bytes())?;
    lit(out, r#","ttlMs":10000,"cacheScope":"connection""#)?;
    response_suffix(out, true)
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
