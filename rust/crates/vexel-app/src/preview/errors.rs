//! Whitelisted tool error text. Anything not listed here collapses to the
//! generic message so internal detail and caller text never leak.

use vexel_protocol::ToolResponse;

pub(super) fn tool_error(message: &str) -> ToolResponse {
    let text = match message {
        "preview window is closed" => message,
        "nothing to undo" => message,
        "nothing to redo" => message,
        "preview scene limit reached" => message,
        "preview source limit reached" => message,
        "unsupported preview shape" => message,
        "unknown preview document" => message,
        "invalid preview fill color" => message,
        "unsupported preview operation" => message,
        "path not granted" => message,
        "path not found" => message,
        "unsupported file type" => message,
        "invalid project file" => message,
        "unknown element" => message,
        "unsupported attribute" => message,
        "document has no path" => message,
        "export_svg too large" => message,
        "file operation failed" => message,
        "project history unavailable" => message,
        "preview allocation failed" => message,
        "export could not render the animation" => message,
        m if m.starts_with("element_add ") => m,
        m if DOC_TOOL_PREFIXES.iter().any(|p| m.starts_with(p)) => m,
        m if m.starts_with("native_timeline_") => m,
        m if m.starts_with("native timeline ") => m,
        m if m.starts_with("audio") => m,
        m if m.starts_with("batch op ") => m,
        m if m.starts_with("export ") => m,
        m if m.starts_with("render_png ") => m,
        _ => "invalid preview tool arguments",
    };
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            c if (c as u32) < 0x20 || matches!(c, '\u{2028}' | '\u{2029}') => {
                escaped.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => escaped.push(c),
        }
    }
    let mut s = b"{\"error\":\"".to_vec();
    s.extend_from_slice(escaped.as_bytes());
    s.extend_from_slice(b"\"}");
    ToolResponse {
        structured_json: s,
        content_json: format!("[{{\"type\":\"text\",\"text\":\"{escaped}\"}}]").into_bytes(),
        is_error: true,
    }
}

/// Error messages of the document-tool families start with the tool name.
const DOC_TOOL_PREFIXES: &[&str] = &[
    "element_group",
    "element_add_fragment",
    "use_",
    "element_ungroup",
    "element_duplicate",
    "element_reorder",
    "element_reparent",
    "element_get",
    "element_query",
    "tree_get",
    "element_set_",
    "gradient_",
    "clip_",
    "rig_",
    "mouth_set",
    "lip_sync",
    "eyes_set",
    "blink",
    "text_",
    "attribute_",
];
