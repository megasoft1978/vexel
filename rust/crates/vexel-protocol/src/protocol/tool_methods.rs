use super::{
    LIMIT, ProtocolError, ProtocolSession, append_error, lit, response_prefix, response_suffix,
};
use crate::{
    AsyncBackend, AsyncDisposition, AsyncPayloadKind, ProtocolBackend, append_raw, append_string,
    validate_async_payload, validate_tool_arguments,
};
use vexel_json::{JsonBuffer, JsonDocument, JsonType};

pub(super) fn append_tool_call<B: ProtocolBackend + ?Sized, A: AsyncBackend + ?Sized>(
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
    if session.live_preview
        && ![
            "document_list",
            "document_info",
            "element_add",
            "undo",
            "redo",
            "animation_list",
            "animation_add",
            "animation_control",
            "export_animation",
        ]
        .contains(&name.as_str())
    {
        return append_tool_error(out, doc, id, session.modern, "unknown tool", "unknown_tool");
    }
    if !session.live_preview && name == "animation_control" {
        return append_tool_error(out, doc, id, session.modern, "unknown tool", "unknown_tool");
    }
    if session.live_preview && name == "element_add" {
        let template = doc.object_get(args, b"template");
        if !template
            .is_some_and(|v| doc.string_equals(v, b"rect") || doc.string_equals(v, b"ellipse"))
        {
            return append_tool_error(
                out,
                doc,
                id,
                session.modern,
                "invalid tool arguments",
                "invalid_arguments",
            );
        }
        if doc.object_get(args, b"attributes").is_none() {
            return append_tool_error(
                out,
                doc,
                id,
                session.modern,
                "required argument missing",
                "invalid_arguments",
            );
        }
    }
    if session.live_preview && name == "animation_add" {
        let fragment = doc
            .object_get(args, b"xml")
            .and_then(|v| doc.decoded_string(v));
        let target_len = doc
            .object_get(args, b"targetId")
            .and_then(|v| doc.decoded_string(v))
            .map_or(0, |v| v.len());
        if !(1..=128).contains(&target_len)
            || !fragment.is_some_and(|xml| {
                xml.len() <= 4096
                    && xml.starts_with(b"<animate")
                    && xml.ends_with(b"/>")
                    && !xml.contains(&b'&')
                    && xml.iter().filter(|byte| **byte == b'<').count() == 1
            })
        {
            return append_tool_error(
                out,
                doc,
                id,
                session.modern,
                "animation must be one bounded animate element",
                "invalid_arguments",
            );
        }
    }
    if session.live_preview && name == "animation_control" {
        let action = doc.object_get(args, b"action");
        let valid_action = action.is_some_and(|value| {
            [b"play".as_slice(), b"pause", b"seek"]
                .iter()
                .any(|candidate| doc.string_equals(value, candidate))
        });
        let has_time = doc.object_get(args, b"timeMs").is_some();
        let time_valid = doc
            .object_get(args, b"timeMs")
            .and_then(|v| doc.raw_token(v))
            .and_then(|bytes| std::str::from_utf8(&bytes).ok())
            .and_then(|value| value.parse::<f64>().ok())
            .is_some_and(|value| value.is_finite() && (0.0..=10_000.0).contains(&value));
        if !valid_action
            || (action.is_some_and(|v| doc.string_equals(v, b"seek")) && !has_time)
            || (action.is_some_and(|v| !doc.string_equals(v, b"seek")) && has_time)
            || (has_time && !time_valid)
        {
            return append_tool_error(
                out,
                doc,
                id,
                session.modern,
                "animation control requires play, pause, or seek with a seek time",
                "invalid_arguments",
            );
        }
    }
    if session.live_preview && name == "export_animation" {
        let number = |key: &[u8]| {
            doc.object_get(args, key)
                .and_then(|v| doc.raw_token(v))
                .and_then(|bytes| std::str::from_utf8(&bytes).ok())
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|value| value.is_finite())
        };
        let valid_path = doc
            .object_get(args, b"path")
            .and_then(|v| doc.decoded_string(v))
            .is_some_and(|path| !path.is_empty() && path.len() < 1024 && !path.contains(&0));
        let valid_format = doc
            .object_get(args, b"format")
            .is_some_and(|v| doc.string_equals(v, b"gif"));
        let valid_fps = number(b"fps").is_some_and(|v| v.fract() == 0.0 && (1.0..=30.0).contains(&v));
        let valid_duration = number(b"duration").is_some_and(|v| (0.01..=10.0).contains(&v));
        if !(valid_path && valid_format && valid_fps && valid_duration) {
            return append_tool_error(
                out,
                doc,
                id,
                session.modern,
                "export requires a path, gif format, fps 1-30 and duration 0.01-10 seconds",
                "invalid_arguments",
            );
        }
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
pub(super) fn append_task_method<A: AsyncBackend + ?Sized>(
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
pub(super) fn append_resource_read<B: ProtocolBackend + ?Sized>(
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
