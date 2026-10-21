//! Lane-owned: element_group/ungroup/duplicate/reorder routed onto the live document (coordinator stub).

use super::{DOCUMENT_ID, JsonType, PreviewArgumentError};
use crate::session::{AppDocument, McpOutcome};
use vexel_json::JsonDocument;

/// One validated request of this tool family.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Group(String),
    Ungroup(String),
    Duplicate(String),
    Reorder {
        element_id: String,
        target_id: String,
        before: bool,
    },
}

fn invalid(tool: &str, argument: &str, form: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!(
        "{tool} argument {argument} is missing or invalid; use {form}"
    ))
}

fn string(
    doc: &JsonDocument<'_>,
    args: usize,
    key: &'static str,
    tool: &str,
) -> Result<String, PreviewArgumentError> {
    let token = doc.object_get(args, key.as_bytes());
    let value = token.and_then(|token| doc.decoded_string(token));
    value
        .and_then(|bytes| {
            (bytes.len() <= 256
                && !bytes.is_empty()
                && token.is_some_and(|t| doc.raw_token(t).is_some_and(|r| r.len() <= 1538)))
            .then(|| String::from_utf8(bytes).ok())
            .flatten()
        })
        .ok_or_else(|| invalid(tool, key, "a non-empty UTF-8 string of at most 256 bytes"))
}

/// `None` when `tool` is not in this family.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Op, PreviewArgumentError>> {
    let operation = match tool {
        "element_group" => (|| {
            document(doc, args, tool)?;
            Ok(Op::Group(string(doc, args, "elementId", tool)?))
        })(),
        "element_ungroup" => (|| {
            document(doc, args, tool)?;
            Ok(Op::Ungroup(string(doc, args, "elementId", tool)?))
        })(),
        "element_duplicate" => (|| {
            document(doc, args, tool)?;
            Ok(Op::Duplicate(string(doc, args, "elementId", tool)?))
        })(),
        "element_reorder" => (|| {
            document(doc, args, tool)?;
            let element_id = string(doc, args, "elementId", tool)?;
            let target_id = string(doc, args, "targetId", tool)?;
            let before = doc
                .object_get(args, b"before")
                .filter(|t| {
                    doc.token(*t)
                        .is_some_and(|v| matches!(v.kind, JsonType::True | JsonType::False))
                })
                .and_then(|t| doc.bool(t))
                .ok_or_else(|| invalid(tool, "before", "a JSON boolean"))?;
            Ok(Op::Reorder {
                element_id,
                target_id,
                before,
            })
        })(),
        _ => return None,
    };
    Some(operation)
}

fn document(doc: &JsonDocument<'_>, args: usize, tool: &str) -> Result<(), PreviewArgumentError> {
    let value = doc
        .object_get(args, b"documentId")
        .and_then(|t| doc.decoded_string(t));
    if value.as_deref() == Some(DOCUMENT_ID.as_bytes()) {
        Ok(())
    } else {
        Err(invalid(tool, "documentId", "\"preview-document\""))
    }
}

pub(crate) fn apply(app: &mut AppDocument, op: Op) -> Result<McpOutcome, &'static str> {
    let result = app.edit(|doc| match op {
        Op::Group(id) => doc
            .group(&[id])
            .map(|new_id| format!("{{\"ok\":true,\"elementId\":\"{new_id}\"}}")),
        Op::Ungroup(id) => doc.ungroup(&id).map(|ids| {
            format!(
                "{{\"ok\":true,\"elementIds\":[{}]}}",
                ids.iter()
                    .map(|id| format!("\"{id}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }),
        Op::Duplicate(id) => doc
            .duplicate(&id)
            .map(|new_id| format!("{{\"ok\":true,\"elementId\":\"{new_id}\"}}")),
        Op::Reorder {
            element_id,
            target_id,
            before,
        } => doc
            .reorder_relative(&element_id, &target_id, before)
            .map(|()| "{\"ok\":true}".to_owned()),
    });
    match result {
        Ok(reply) => Ok(McpOutcome::Reply(reply)),
        Err(crate::project_session::SessionEditError::Operation(error)) => Err(match error {
            vexel_v1_document::V1DocumentError::MissingStableId => {
                "element_group: element not found"
            }
            vexel_v1_document::V1DocumentError::UnsupportedTransform => {
                "element_ungroup: transformed groups are unsupported"
            }
            _ => "element_group: operation is unsupported for this element",
        }),
        Err(crate::project_session::SessionEditError::History(_)) => {
            Err("element_group: project history unavailable")
        }
    }
}

#[cfg(test)]
#[path = "ext_structure_tests.rs"]
mod tests;
