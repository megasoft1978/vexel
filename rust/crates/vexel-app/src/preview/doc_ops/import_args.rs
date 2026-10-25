//! Arguments of `document_import`: bounded, with messages that name the
//! offending member.

use super::super::add_extras::number;
use super::super::ext_structure::{document, string};
use super::super::{JsonType, PreviewArgumentError};
use crate::session::mcp_import::ImportOp as Op;
use vexel_json::JsonDocument;

const TOOL: &str = "document_import";
/// Longest path accepted (matches the tool schema).
const PATH_LIMIT: usize = 1023;

fn invalid(argument: &str, form: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!(
        "{TOOL} argument {argument} is missing or invalid; use {form}"
    ))
}

fn flag(
    doc: &JsonDocument<'_>,
    args: usize,
    key: &'static str,
    default: bool,
) -> Result<bool, PreviewArgumentError> {
    match doc.object_get(args, key.as_bytes()) {
        None => Ok(default),
        Some(token) => match doc.token(token).map(|t| t.kind) {
            Some(JsonType::True) => Ok(true),
            Some(JsonType::False) => Ok(false),
            _ => Err(invalid(key, "true or false")),
        },
    }
}

/// `None` when `tool` is not `document_import`.
pub(in crate::preview) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Op, PreviewArgumentError>> {
    (tool == TOOL).then(|| {
        document(doc, args, tool)?;
        let path = doc
            .object_get(args, b"path")
            .filter(|t| {
                doc.raw_token(*t)
                    .is_some_and(|raw| raw.len() <= PATH_LIMIT * 6 + 2)
            })
            .and_then(|t| doc.decoded_string(t))
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .filter(|p| !p.is_empty() && p.len() <= PATH_LIMIT && !p.contains('\0'))
            .ok_or_else(|| invalid("path", "a .vexel file path of at most 1023 bytes"))?;
        let parent = match doc.object_get(args, b"parentId") {
            None => "root".to_owned(),
            Some(_) => string(doc, args, "parentId", tool)?,
        };
        let prefix = match doc.object_get(args, b"idPrefix") {
            None => None,
            Some(_) => Some(string(doc, args, "idPrefix", tool)?)
                .filter(|p| vexel_v1_document::valid_prefix(p))
                .map(Some)
                .ok_or_else(|| {
                    invalid(
                        "idPrefix",
                        "1-64 characters from A-Z a-z 0-9 _ - . starting with a letter or _",
                    )
                })?,
        };
        let form = "five finite numbers [tx, ty, rotationDegrees, sx, sy]";
        let transform = match doc.object_get(args, b"transform") {
            None => None,
            Some(token) => {
                let items = doc
                    .token(token)
                    .filter(|t| t.kind == JsonType::Array && t.child_count == 5)
                    .ok_or_else(|| invalid("transform", form))?;
                let mut out = [0.0; 5];
                let mut item = items.first_child;
                for slot in &mut out {
                    let at = item.ok_or_else(|| invalid("transform", form))?;
                    *slot = number(doc, at)
                        .filter(|n| n.abs() <= 1e9)
                        .ok_or_else(|| invalid("transform", form))?;
                    item = doc.token(at).and_then(|t| t.next_sibling);
                }
                Some(out)
            }
        };
        Ok(Op {
            path,
            parent,
            prefix,
            transform,
            include_timeline: flag(doc, args, "includeTimeline", true)?,
            include_rig: flag(doc, args, "includeRig", false)?,
        })
    })
}
