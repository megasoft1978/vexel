//! Parsing of the preview `batch` tool: every entry is parsed with the same
//! `parse_operation` a single call uses, before any of them is applied.

use super::{JsonDocument, JsonType, Operation, PreviewArgumentError, parse_operation_in};

/// One parsed batch entry; `tool` is kept for the per-operation result.
#[derive(Clone, Debug, PartialEq)]
pub struct BatchOp {
    pub tool: String,
    pub operation: Operation,
}

/// A validated batch: operations in order, all-or-nothing when `atomic`.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    pub atomic: bool,
    pub ops: Vec<BatchOp>,
}

/// Operations that touch files, history, playback or exports never run in a
/// batch (the protocol layer rejects them first; this keeps the app safe if
/// it is reached another way).
fn allowed(operation: &Operation) -> bool {
    !matches!(
        operation,
        Operation::Batch(_)
            | Operation::Open { .. }
            | Operation::Save
            | Operation::SaveAs { .. }
            | Operation::Undo
            | Operation::Redo
            | Operation::Control(_)
            | Operation::Export { .. }
            | Operation::ExportSvg
            | Operation::ExportVideo(_)
            | Operation::ExportFrames(_)
            | Operation::RenderPng { .. }
    )
}

pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
) -> Result<Operation, PreviewArgumentError> {
    let bad = || PreviewArgumentError::Generic("invalid preview tool arguments");
    let atomic = match doc.object_get(args, b"atomic") {
        None => true,
        Some(token) => match doc.raw_token(token) {
            Some(b"true") => true,
            Some(b"false") => false,
            _ => return Err(bad()),
        },
    };
    let ops = doc
        .object_get(args, b"ops")
        .filter(|t| doc.token(*t).is_some_and(|t| t.kind == JsonType::Array))
        .ok_or_else(bad)?;
    let mut parsed = Vec::new();
    let mut item = doc.token(ops).and_then(|t| t.first_child);
    while let Some(op) = item {
        let index = parsed.len();
        let tool = doc
            .object_get(op, b"tool")
            .and_then(|t| doc.decoded_string(t))
            .and_then(|b| String::from_utf8(b).ok())
            .ok_or_else(bad)?;
        let arguments = doc.object_get(op, b"arguments").ok_or_else(bad)?;
        let fail = |message: &str| {
            PreviewArgumentError::Actionable(format!("batch op {index} ({tool}): {message}"))
        };
        let operation =
            parse_operation_in(doc, arguments, &tool, true).map_err(|error| match error {
                PreviewArgumentError::Generic(m) => fail(m),
                PreviewArgumentError::Actionable(m) => fail(&m),
            })?;
        if !allowed(&operation) {
            return Err(fail("tool is not allowed in a batch"));
        }
        parsed.push(BatchOp { tool, operation });
        item = doc.token(op).and_then(|t| t.next_sibling);
    }
    Ok(Operation::Batch(Batch {
        atomic,
        ops: parsed,
    }))
}
