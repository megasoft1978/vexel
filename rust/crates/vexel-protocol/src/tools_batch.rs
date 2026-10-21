//! Schema, argument validation and rate-limit weight of the preview-only
//! `batch` tool. Dispatch lives in `vexel-app` (`preview/batch.rs`).
use super::ToolSchema;
use crate::{ToolValidationError, validate_tool_arguments};
use vexel_json::{JsonDocument, JsonType};

pub(crate) const BATCH_MAX_OPS: usize = 256;

/// Tools a batch may not contain: batch itself, file lifecycle, export, render,
/// save, history and playback.
const BATCH_FORBIDDEN: &[&str] = &[
    "batch",
    "document_open",
    "document_close",
    "save",
    "save_as",
    "export_svg",
    "export_animation",
    "render_png",
    "undo",
    "redo",
    "native_timeline_undo",
    "native_timeline_redo",
    "animation_control",
];

pub(crate) const BATCH_TOOLS: &[ToolSchema] = &[ToolSchema {
    name: "batch",
    description: r#"Run 1 to 256 preview operations on one document, in order. ops is [{"tool":"<name>","arguments":{...}}]; each entry is validated exactly as if it were called alone. atomic (default true) applies all operations or none. Nested batch and file, export, render, open, close, save, undo, redo and playback tools are not allowed. A batch costs 1 + ops/8 of the per-minute request budget. Example: {"documentId":"doc-1","ops":[{"tool":"element_add","arguments":{"parentId":"root","template":"rect"}}]}."#,
    properties: r#"{"documentId":{"type":"string"},"atomic":{"type":"boolean","default":true},"ops":{"type":"array","minItems":1,"maxItems":256,"items":{"type":"object","properties":{"tool":{"type":"string","minLength":1,"maxLength":63},"arguments":{"type":"object"}},"required":["tool","arguments"],"additionalProperties":false}}}"#,
    required: "documentId,ops",
    fields: "documentId:string;atomic:boolean;ops:array",
}];

/// Validates the `ops` array of a batch: 1 to 256 entries, each an object with
/// exactly `tool` and `arguments`, a permitted tool name, and arguments that
/// pass that tool's own validation.
pub(crate) fn validate_ops(doc: &JsonDocument<'_>, ops: usize) -> Result<(), ToolValidationError> {
    let count = doc.token(ops).map_or(0, |t| t.child_count);
    if !(1..=BATCH_MAX_OPS).contains(&count) {
        return Err(ToolValidationError::InvalidBatchLength);
    }
    let mut item = doc.token(ops).and_then(|t| t.first_child);
    while let Some(op) = item {
        validate_op(doc, op)?;
        item = doc.token(op).and_then(|t| t.next_sibling);
    }
    Ok(())
}

fn validate_op(doc: &JsonDocument<'_>, op: usize) -> Result<(), ToolValidationError> {
    if doc.token(op).is_none_or(|t| t.kind != JsonType::Object) {
        return Err(ToolValidationError::InvalidArguments);
    }
    let mut tool = None;
    let mut arguments = None;
    let mut key = doc.token(op).and_then(|t| t.first_child);
    while let Some(key_index) = key {
        let Some(value) = doc.token(key_index).and_then(|t| t.next_sibling) else {
            return Err(ToolValidationError::InvalidArguments);
        };
        let slot = if doc.string_equals(key_index, b"tool") {
            &mut tool
        } else if doc.string_equals(key_index, b"arguments") {
            &mut arguments
        } else {
            return Err(ToolValidationError::UnexpectedArgument);
        };
        if slot.replace(value).is_some() {
            return Err(ToolValidationError::UnexpectedArgument);
        }
        key = doc.token(value).and_then(|t| t.next_sibling);
    }
    let (Some(tool), Some(arguments)) = (tool, arguments) else {
        return Err(ToolValidationError::MissingRequired);
    };
    let name = doc
        .decoded_string(tool)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or(ToolValidationError::InvalidArguments)?;
    if BATCH_FORBIDDEN.contains(&name.as_str()) {
        return Err(ToolValidationError::BatchToolNotAllowed);
    }
    validate_tool_arguments(doc, arguments, &name)
}

/// Rate-limit weight of one request. A `batch` call costs 1 + ops/8; every other
/// request costs 1. `params` is the request's `params` token.
pub(crate) fn request_weight(doc: &JsonDocument<'_>, method: &str, params: Option<usize>) -> u32 {
    let Some(params) = params.filter(|_| method == "tools/call") else {
        return 1;
    };
    if !doc
        .object_get(params, b"name")
        .is_some_and(|name| doc.string_equals(name, b"batch"))
    {
        return 1;
    }
    let ops = doc
        .object_get(params, b"arguments")
        .and_then(|args| doc.object_get(args, b"ops"))
        .and_then(|ops| doc.token(ops))
        .filter(|t| t.kind == JsonType::Array)
        .map_or(0, |t| t.child_count);
    u32::try_from(1 + ops / 8).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ops_json(count: usize, tool: &str) -> String {
        let op = format!(r#"{{"tool":"{tool}","arguments":{{"documentId":"doc-1"}}}}"#);
        vec![op; count].join(",")
    }

    fn args(ops: &str) -> String {
        format!(r#"{{"documentId":"doc-1","ops":[{ops}]}}"#)
    }

    fn validate(source: &str) -> Result<(), ToolValidationError> {
        let doc = JsonDocument::parse(source.as_bytes()).unwrap();
        validate_tool_arguments(&doc, doc.root(), "batch")
    }

    #[test]
    fn schema_declares_batch_with_bounded_ops() {
        let tool = &BATCH_TOOLS[0];
        assert_eq!(tool.name, "batch");
        assert_eq!(tool.required, "documentId,ops");
        assert!(tool.properties.contains(r#""minItems":1,"maxItems":256"#));
        assert!(tool.properties.contains(r#""atomic":{"type":"boolean","default":true}"#));
        assert!(tool.properties.contains(r#""additionalProperties":false"#));
    }

    #[test]
    fn accepts_valid_batches() {
        assert_eq!(validate(&args(&ops_json(1, "tree_get"))), Ok(()));
        assert_eq!(validate(&args(&ops_json(256, "tree_get"))), Ok(()));
        let atomic = r#"{"documentId":"doc-1","atomic":false,"ops":[{"tool":"tree_get","arguments":{"documentId":"doc-1"}}]}"#;
        assert_eq!(validate(atomic), Ok(()));
    }

    #[test]
    fn rejects_ops_outside_one_to_256() {
        assert_eq!(
            validate(&args("")),
            Err(ToolValidationError::InvalidBatchLength)
        );
        assert_eq!(
            validate(&args(&ops_json(257, "tree_get"))),
            Err(ToolValidationError::InvalidBatchLength)
        );
    }

    #[test]
    fn rejects_nested_and_non_batchable_tools() {
        for tool in [
            "batch",
            "document_open",
            "document_close",
            "save",
            "save_as",
            "export_svg",
            "export_animation",
            "render_png",
            "undo",
            "redo",
            "native_timeline_undo",
            "native_timeline_redo",
            "animation_control",
        ] {
            assert_eq!(
                validate(&args(&ops_json(1, tool))),
                Err(ToolValidationError::BatchToolNotAllowed),
                "{tool}"
            );
        }
    }

    #[test]
    fn unknown_op_tool_is_unknown() {
        assert_eq!(
            validate(&args(&ops_json(1, "no_such_tool"))),
            Err(ToolValidationError::UnknownTool)
        );
    }

    #[test]
    fn op_arguments_follow_the_called_tool_rules() {
        let missing = r#"{"documentId":"doc-1","ops":[{"tool":"element_add","arguments":{"parentId":"root"}}]}"#;
        assert_eq!(validate(missing), Err(ToolValidationError::MissingRequired));
        let extra = r#"{"documentId":"doc-1","ops":[{"tool":"tree_get","arguments":{"documentId":"doc-1","bogus":1}}]}"#;
        assert_eq!(validate(extra), Err(ToolValidationError::UnexpectedArgument));
    }

    #[test]
    fn op_shape_is_exact() {
        let no_arguments = r#"{"documentId":"doc-1","ops":[{"tool":"tree_get"}]}"#;
        assert_eq!(validate(no_arguments), Err(ToolValidationError::MissingRequired));
        let extra_key = r#"{"documentId":"doc-1","ops":[{"tool":"tree_get","arguments":{"documentId":"doc-1"},"id":1}]}"#;
        assert_eq!(validate(extra_key), Err(ToolValidationError::UnexpectedArgument));
        let not_object = r#"{"documentId":"doc-1","ops":["tree_get"]}"#;
        assert_eq!(validate(not_object), Err(ToolValidationError::InvalidArguments));
        let tool_not_string = r#"{"documentId":"doc-1","ops":[{"tool":3,"arguments":{}}]}"#;
        assert_eq!(validate(tool_not_string), Err(ToolValidationError::InvalidArguments));
    }

    #[test]
    fn top_level_arguments_are_checked() {
        assert_eq!(
            validate(r#"{"ops":[{"tool":"tree_get","arguments":{}}]}"#),
            Err(ToolValidationError::MissingRequired)
        );
        assert_eq!(
            validate(r#"{"documentId":"doc-1"}"#),
            Err(ToolValidationError::MissingRequired)
        );
        assert_eq!(
            validate(r#"{"documentId":"doc-1","atomic":"yes","ops":[{"tool":"tree_get","arguments":{}}]}"#),
            Err(ToolValidationError::WrongType("atomic"))
        );
    }

    #[test]
    fn weight_is_one_plus_ops_over_eight() {
        let weight = |source: &str| {
            let doc = JsonDocument::parse(source.as_bytes()).unwrap();
            let params = doc.object_get(doc.root(), b"params");
            let method = "tools/call";
            request_weight(&doc, method, params)
        };
        let call = |ops: &str| {
            format!(
                r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"batch","arguments":{{"documentId":"doc-1","ops":[{ops}]}}}}}}"#
            )
        };
        assert_eq!(weight(&call(&ops_json(1, "tree_get"))), 1);
        assert_eq!(weight(&call(&ops_json(7, "tree_get"))), 1);
        assert_eq!(weight(&call(&ops_json(8, "tree_get"))), 2);
        assert_eq!(weight(&call(&ops_json(256, "tree_get"))), 33);
    }

    #[test]
    fn weight_is_one_for_other_requests() {
        let other = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"tree_get","arguments":{"documentId":"doc-1"}}}"#;
        let doc = JsonDocument::parse(other.as_bytes()).unwrap();
        let params = doc.object_get(doc.root(), b"params");
        assert_eq!(request_weight(&doc, "tools/call", params), 1);
        assert_eq!(request_weight(&doc, "tools/list", params), 1);
        assert_eq!(request_weight(&doc, "tools/call", None), 1);
    }
}
