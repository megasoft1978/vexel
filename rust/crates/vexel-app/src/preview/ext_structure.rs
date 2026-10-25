//! element_group/ungroup/duplicate/reorder/reparent routed onto the live document.

use super::{DOCUMENT_ID, JsonType, PreviewArgumentError};
use crate::session::{AppDocument, McpOutcome};
use vexel_json::JsonDocument;

/// One validated request of this tool family.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// 1..=1024 sibling ids wrapped in one group.
    Group(Vec<String>),
    Ungroup(String),
    Duplicate(String),
    Reorder {
        element_id: String,
        target_id: String,
        before: bool,
    },
    Reparent {
        element_id: String,
        parent_id: String,
        index: Option<usize>,
    },
}

/// Most ids one `element_group` call may wrap (the document layer's cap).
const MAX_GROUP_IDS: usize = vexel_v1_document::MAX_GROUP_IDS;

fn invalid(tool: &str, argument: &str, form: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!(
        "{tool} argument {argument} is missing or invalid; use {form}"
    ))
}

pub(super) fn string(
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
            Ok(Op::Group(group_ids(doc, args, tool)?))
        })(),
        "element_reparent" => (|| {
            document(doc, args, tool)?;
            let index = match doc.object_get(args, b"index") {
                None => None,
                Some(token) => Some(
                    doc.uint(token)
                        .filter(|_| doc.token(token).is_some_and(|t| t.kind == JsonType::Number))
                        .and_then(|n| usize::try_from(n).ok())
                        .filter(|n| *n <= 100_000)
                        .ok_or_else(|| {
                            invalid(tool, "index", "an integer from 0 through 100000")
                        })?,
                ),
            };
            Ok(Op::Reparent {
                element_id: string(doc, args, "elementId", tool)?,
                parent_id: string(doc, args, "parentId", tool)?,
                index,
            })
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

/// `elementId` (one id) or `elementIds` (1..=1024 ids), not both.
fn group_ids(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<Vec<String>, PreviewArgumentError> {
    let Some(list) = doc.object_get(args, b"elementIds") else {
        return Ok(vec![string(doc, args, "elementId", tool)?]);
    };
    let form = "elementId (one id) or elementIds (1-1024 non-empty id strings), not both";
    let token = doc.token(list).filter(|t| t.kind == JsonType::Array);
    if doc.object_get(args, b"elementId").is_some()
        || token.is_none_or(|t| !(1..=MAX_GROUP_IDS).contains(&t.child_count))
    {
        return Err(invalid(tool, "elementIds", form));
    }
    let mut ids = Vec::new();
    ids.try_reserve(MAX_GROUP_IDS)
        .map_err(|_| PreviewArgumentError::Generic("preview allocation failed"))?;
    let mut item = token.and_then(|t| t.first_child);
    while let Some(at) = item {
        let id = doc
            .raw_token(at)
            .filter(|raw| raw.len() <= 1538)
            .and_then(|_| doc.decoded_string(at))
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .filter(|id| !id.is_empty() && id.len() <= 256)
            .ok_or_else(|| invalid(tool, "elementIds", form))?;
        ids.push(id);
        item = doc.token(at).and_then(|t| t.next_sibling);
    }
    Ok(ids)
}

pub(super) fn document(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<(), PreviewArgumentError> {
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
    if let Op::Reparent {
        element_id,
        parent_id,
        index,
    } = op
    {
        return reparent(app, &element_id, &parent_id, index);
    }
    let result = app.edit(|doc| match op {
        Op::Reparent { .. } => Err(vexel_v1_document::V1DocumentError::Unsupported),
        Op::Group(ids) => doc
            .group(&ids)
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

fn reparent(
    app: &mut AppDocument,
    element_id: &str,
    parent_id: &str,
    index: Option<usize>,
) -> Result<McpOutcome, &'static str> {
    app.edit(|doc| doc.reparent(element_id, parent_id, index))
        .map_err(|error| match error {
            crate::project_session::SessionEditError::Operation(
                vexel_v1_document::V1DocumentError::MissingStableId,
            ) => "element_reparent: elementId or parentId is not an element id in the document",
            crate::project_session::SessionEditError::Operation(_) => {
                "element_reparent: parentId must be \"root\" or a group that is neither the element nor inside it"
            }
            crate::project_session::SessionEditError::History(_) => {
                "element_reparent: project history unavailable"
            }
        })?;
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"elementId\":\"{element_id}\",\"parentId\":\"{parent_id}\"}}"
    )))
}

#[cfg(test)]
#[path = "ext_structure_tests.rs"]
mod tests;
