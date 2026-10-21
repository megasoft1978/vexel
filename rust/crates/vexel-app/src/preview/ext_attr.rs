//! attribute_remove and text_set routed onto the live document.

use super::{DOCUMENT_ID, PreviewArgumentError};
use crate::project_session::SessionEditError;
use crate::session::{AppDocument, McpOutcome};
use vexel_json::{JsonDocument, JsonType};
use vexel_v1_document::{ElementKind, V1DocumentError};

const ID_LIMIT: usize = 128;
const NAME_LIMIT: usize = 64;
const TEXT_LIMIT: usize = 256 * 4;

const REMOVE_NAME: &str = "attribute_remove: name must be one of opacity, fill-opacity, stroke-opacity, stroke-linecap, stroke-linejoin, stroke-dasharray, stroke-dashoffset, fill-rule, visibility, display, fill, stroke, stroke-width";

/// One validated request of this tool family.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    RemoveAttribute { element_id: String, name: String },
    SetText { element_id: String, text: String },
}

fn bad(message: &'static str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(message.to_owned())
}

/// A string argument of at most `max` bytes (raw token bounded first).
fn string_arg(doc: &JsonDocument<'_>, args: usize, key: &[u8], max: usize) -> Option<String> {
    let token = doc.object_get(args, key)?;
    if doc.token(token)?.kind != JsonType::String || doc.raw_token(token)?.len() > max * 6 + 2 {
        return None;
    }
    let text = String::from_utf8(doc.decoded_string(token)?).ok()?;
    (text.len() <= max).then_some(text)
}

fn element_id(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &'static str,
) -> Result<String, PreviewArgumentError> {
    let message = match tool {
        "text_set" => "text_set: elementId must be a non-empty string of at most 128 bytes",
        _ => "attribute_remove: elementId must be a non-empty string of at most 128 bytes",
    };
    string_arg(doc, args, b"elementId", ID_LIMIT)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| bad(message))
}

/// `None` when `tool` is not in this family.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Op, PreviewArgumentError>> {
    let tool: &'static str = match tool {
        "attribute_remove" => "attribute_remove",
        "text_set" => "text_set",
        _ => return None,
    };
    Some(parse_family(doc, args, tool))
}

fn parse_family(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &'static str,
) -> Result<Op, PreviewArgumentError> {
    if string_arg(doc, args, b"documentId", 64).as_deref() != Some(DOCUMENT_ID) {
        return Err(PreviewArgumentError::Generic("unknown preview document"));
    }
    let element_id = element_id(doc, args, tool)?;
    if tool == "text_set" {
        let text = string_arg(doc, args, b"text", TEXT_LIMIT).ok_or_else(|| {
            bad("text_set: text must be a string of 1 to 256 characters without control characters")
        })?;
        let count = text.chars().count();
        if !(1..=256).contains(&count) || text.chars().any(char::is_control) {
            return Err(bad(
                "text_set: text must be a string of 1 to 256 characters without control characters",
            ));
        }
        return Ok(Op::SetText { element_id, text });
    }
    let name = string_arg(doc, args, b"name", NAME_LIMIT).ok_or_else(|| bad(REMOVE_NAME))?;
    if !REMOVABLE.contains(&name.as_str()) {
        return Err(bad(REMOVE_NAME));
    }
    Ok(Op::RemoveAttribute { element_id, name })
}

const REMOVABLE: &[&str] = &[
    "opacity",
    "fill-opacity",
    "stroke-opacity",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-dasharray",
    "stroke-dashoffset",
    "fill-rule",
    "visibility",
    "display",
    "fill",
    "stroke",
    "stroke-width",
];

fn edit_error(tool: &str, error: SessionEditError<V1DocumentError>) -> &'static str {
    match (tool, error) {
        (_, SessionEditError::History(_)) => "project history unavailable",
        ("text_set", SessionEditError::Operation(V1DocumentError::MissingStableId))
        | ("text_set", SessionEditError::Operation(V1DocumentError::InvalidStableId)) => {
            "text_set: element not found"
        }
        ("text_set", SessionEditError::Operation(_)) => {
            "text_set: text could not be set; use 1 to 256 characters on a text element"
        }
        (_, SessionEditError::Operation(V1DocumentError::MissingStableId))
        | (_, SessionEditError::Operation(V1DocumentError::InvalidStableId)) => {
            "attribute_remove: element not found"
        }
        (_, SessionEditError::Operation(V1DocumentError::Unsupported)) => {
            "attribute_remove: attribute is not removable; use a name from the documented list"
        }
        (_, SessionEditError::Operation(_)) => {
            "attribute_remove: attribute is not present on the element"
        }
    }
}

fn reply(operation: &str, id: &str, app: &AppDocument) -> McpOutcome {
    McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"{operation}\",\"elementId\":\"{id}\",\"dirty\":{}}}",
        app.is_dirty()
    ))
}

pub(crate) fn apply(app: &mut AppDocument, op: Op) -> Result<McpOutcome, &'static str> {
    match op {
        Op::RemoveAttribute { element_id, name } => {
            app.document()
                .element_kind(&element_id)
                .map_err(|_| "attribute_remove: element not found")?;
            app.edit(|doc| doc.remove_attribute(&element_id, &name))
                .map_err(|e| edit_error("attribute_remove", e))?;
            Ok(reply("attribute_remove", &element_id, app))
        }
        Op::SetText { element_id, text } => {
            match app.document().element_kind(&element_id) {
                Ok(ElementKind::Text) => {}
                Ok(_) => return Err("text_set: element is not a text element"),
                Err(_) => return Err("text_set: element not found"),
            }
            app.edit(|doc| doc.set_text(&element_id, &text))
                .map_err(|e| edit_error("text_set", e))?;
            Ok(reply("text_set", &element_id, app))
        }
    }
}

#[cfg(test)]
#[path = "ext_attr_tests.rs"]
mod tests;
