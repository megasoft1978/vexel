//! Read-only query tools over the live source-owned SVG document.

use super::{JsonDocument, JsonType, PreviewArgumentError};
use crate::session::{AppDocument, McpOutcome};
use vexel_json::JsonBuffer;
use vexel_v1_document::Query;

const JSON_LIMIT: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Get {
        id: String,
    },
    Query {
        id: Option<String>,
        tag: Option<String>,
        attribute: Option<String>,
        value: Option<String>,
        cursor: usize,
        limit: usize,
    },
    Tree {
        cursor: usize,
        limit: usize,
    },
}

fn invalid(tool: &str, arg: &str, form: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!(
        "{tool}: argument {arg} is missing or invalid; use {form}"
    ))
}

fn string(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
    form: &str,
) -> Result<String, PreviewArgumentError> {
    let token = doc.object_get(args, key.as_bytes());
    let bytes = token.and_then(|token| doc.decoded_string(token));
    bytes
        .filter(|bytes| {
            !bytes.is_empty() && bytes.len() <= 1024 && std::str::from_utf8(bytes).is_ok()
        })
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(|| invalid(tool, key, form))
}

fn optional_string(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
) -> Result<Option<String>, PreviewArgumentError> {
    if doc.object_get(args, key.as_bytes()).is_none() {
        return Ok(None);
    }
    let token = doc.object_get(args, key.as_bytes()).unwrap();
    let bytes = doc.decoded_string(token);
    bytes
        .filter(|bytes| bytes.len() <= 1024 && std::str::from_utf8(bytes).is_ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(Some)
        .ok_or_else(|| invalid(tool, key, "a UTF-8 string of at most 1024 bytes"))
}

fn number(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
    default: usize,
    max: usize,
) -> Result<usize, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, key.as_bytes()) else {
        return Ok(default);
    };
    doc.token(token)
        .filter(|token| token.kind == JsonType::Number)
        .and_then(|_| doc.uint(token))
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value <= max)
        .ok_or_else(|| {
            invalid(
                tool,
                key,
                if key == "limit" {
                    "an integer from 1 through 512"
                } else {
                    "a non-negative integer"
                },
            )
        })
}

fn page_limit(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<usize, PreviewArgumentError> {
    let limit = number(doc, args, tool, "limit", 100, 512)?;
    if limit == 0 {
        return Err(invalid(tool, "limit", "an integer from 1 through 512"));
    }
    Ok(limit)
}

fn parse_one(doc: &JsonDocument<'_>, args: usize, tool: &str) -> Result<Op, PreviewArgumentError> {
    Ok(match tool {
        "element_get" => Op::Get {
            id: string(
                doc,
                args,
                tool,
                "elementId",
                "a non-empty UTF-8 element id of at most 1024 bytes",
            )?,
        },
        "element_query" => {
            let id = optional_string(doc, args, tool, "id")?;
            let tag = optional_string(doc, args, tool, "tag")?;
            let attribute = optional_string(doc, args, tool, "attribute")?;
            let value = optional_string(doc, args, tool, "value")?;
            if value.is_some() && attribute.is_none() {
                return Err(invalid(
                    tool,
                    "attribute",
                    "an attribute name when value is supplied",
                ));
            }
            Op::Query {
                id,
                tag,
                attribute,
                value,
                cursor: number(doc, args, tool, "cursor", 0, usize::MAX)?,
                limit: page_limit(doc, args, tool)?,
            }
        }
        "tree_get" => Op::Tree {
            cursor: number(doc, args, tool, "cursor", 0, usize::MAX)?,
            limit: page_limit(doc, args, tool)?,
        },
        _ => {
            return Err(invalid(
                tool,
                "tool",
                "element_get, element_query or tree_get",
            ));
        }
    })
}

pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Op, PreviewArgumentError>> {
    matches!(tool, "element_get" | "element_query" | "tree_get").then(|| parse_one(doc, args, tool))
}

fn qbuf() -> Result<JsonBuffer, &'static str> {
    JsonBuffer::new(1024, JSON_LIMIT).map_err(|_| "preview allocation failed")
}
fn quoted(out: &mut JsonBuffer, text: &str) -> Result<(), &'static str> {
    out.quoted(text.as_bytes())
        .map_err(|_| "preview allocation failed")
}
fn optional(out: &mut JsonBuffer, value: Option<&str>) -> Result<(), &'static str> {
    match value {
        Some(value) => quoted(out, value),
        None => out.literal("null").map_err(|_| "preview allocation failed"),
    }
}

pub(crate) fn apply(app: &mut AppDocument, op: Op) -> Result<McpOutcome, &'static str> {
    let doc = app.document();
    let mut out = qbuf()?;
    match op {
        Op::Get { id } => {
            let info = doc
                .element_get(&id)
                .map_err(|_| "element_get: element not found")?;
            out.literal("{\"id\":")
                .map_err(|_| "preview allocation failed")?;
            quoted(&mut out, &info.id)?;
            out.literal(",\"tag\":")
                .map_err(|_| "preview allocation failed")?;
            quoted(&mut out, &info.tag)?;
            out.literal(",\"kind\":")
                .map_err(|_| "preview allocation failed")?;
            quoted(&mut out, &format!("{:?}", info.kind).to_lowercase())?;
            out.literal(",\"parent\":")
                .map_err(|_| "preview allocation failed")?;
            optional(&mut out, info.parent.as_deref())?;
            out.literal(",\"children\":[")
                .map_err(|_| "preview allocation failed")?;
            for (i, child) in info.children.iter().enumerate() {
                if i > 0 {
                    out.literal(",").map_err(|_| "preview allocation failed")?;
                }
                quoted(&mut out, child)?;
            }
            out.literal("],\"attributes\":{")
                .map_err(|_| "preview allocation failed")?;
            for (i, (key, value)) in info.attributes.iter().enumerate() {
                if i > 0 {
                    out.literal(",").map_err(|_| "preview allocation failed")?;
                }
                quoted(&mut out, key)?;
                out.literal(":").map_err(|_| "preview allocation failed")?;
                quoted(&mut out, value)?;
            }
            out.literal("},\"bounds\":")
                .map_err(|_| "preview allocation failed")?;
            if let Some((x, y, w, h)) = info.bounds {
                out.literal(&format!("[{}, {}, {}, {}]", x, y, w, h))
                    .map_err(|_| "preview allocation failed")?;
            } else {
                out.literal("null")
                    .map_err(|_| "preview allocation failed")?;
            }
            out.literal("}").map_err(|_| "preview allocation failed")?;
        }
        Op::Query {
            id,
            tag,
            attribute,
            value,
            cursor,
            limit,
        } => {
            let q = Query {
                kind: None,
                id_prefix: None,
                id,
                tag,
                attribute: attribute.map(|name| (name, value)),
                cursor,
                limit,
            };
            let ids = doc
                .element_query(&q)
                .map_err(|_| "element_query: query failed")?;
            out.literal("{\"ids\":[")
                .map_err(|_| "preview allocation failed")?;
            for (i, id) in ids.iter().enumerate() {
                if i > 0 {
                    out.literal(",").map_err(|_| "preview allocation failed")?;
                }
                quoted(&mut out, id)?;
            }
            out.literal("],\"nextCursor\":")
                .map_err(|_| "preview allocation failed")?;
            if ids.len() == limit {
                out.literal(&cursor.saturating_add(ids.len()).to_string())
                    .map_err(|_| "preview allocation failed")?;
            } else {
                out.literal("null")
                    .map_err(|_| "preview allocation failed")?;
            }
            out.literal("}").map_err(|_| "preview allocation failed")?;
        }
        Op::Tree { cursor, limit } => {
            let entries = doc
                .tree_page(cursor, limit)
                .map_err(|_| "tree_get: query failed")?;
            out.literal("{\"entries\":[")
                .map_err(|_| "preview allocation failed")?;
            for (i, entry) in entries.iter().enumerate() {
                if i > 0 {
                    out.literal(",").map_err(|_| "preview allocation failed")?;
                }
                out.literal("{\"id\":")
                    .map_err(|_| "preview allocation failed")?;
                optional(&mut out, entry.id.as_deref())?;
                out.literal(",\"tag\":")
                    .map_err(|_| "preview allocation failed")?;
                quoted(&mut out, &entry.tag)?;
                out.literal(&format!(",\"depth\":{},\"parentId\":", entry.depth))
                    .map_err(|_| "preview allocation failed")?;
                optional(&mut out, entry.parent_id.as_deref())?;
                out.literal("}").map_err(|_| "preview allocation failed")?;
            }
            out.literal("],\"nextCursor\":")
                .map_err(|_| "preview allocation failed")?;
            if entries.len() == limit {
                out.literal(&cursor.saturating_add(entries.len()).to_string())
                    .map_err(|_| "preview allocation failed")?;
            } else {
                out.literal("null")
                    .map_err(|_| "preview allocation failed")?;
            }
            out.literal("}").map_err(|_| "preview allocation failed")?;
        }
    }
    String::from_utf8(out.as_bytes().to_vec())
        .map(McpOutcome::Reply)
        .map_err(|_| "preview allocation failed")
}

#[cfg(test)]
#[path = "ext_query_tests.rs"]
mod tests;
