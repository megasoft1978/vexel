//! Argument parsing helpers for the paint/transform/text tool family.

use super::PreviewArgumentError;
use vexel_json::{JsonDocument, JsonType};
use vexel_v1_document::GradientStop;

fn bad(tool: &str, key: &str, form: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!(
        "{tool}: argument {key} is missing or invalid; use {form}"
    ))
}

pub(super) fn string(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &str,
    max: usize,
    form: &str,
) -> Result<String, PreviewArgumentError> {
    let token = doc
        .object_get(args, key.as_bytes())
        .ok_or_else(|| bad(tool, key, form))?;
    let value = doc
        .decoded_string(token)
        .filter(|v| !v.is_empty() && v.len() <= max)
        .and_then(|v| String::from_utf8(v).ok())
        .filter(|_| {
            doc.raw_token(token)
                .is_some_and(|raw| raw.len() <= max.saturating_mul(6).saturating_add(2))
        });
    value.ok_or_else(|| bad(tool, key, form))
}

pub(super) fn opt_string(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &str,
    max: usize,
    form: &str,
) -> Result<Option<String>, PreviewArgumentError> {
    if doc.object_get(args, key.as_bytes()).is_none() {
        return Ok(None);
    }
    string(doc, args, tool, key, max, form).map(Some)
}

pub(super) fn id(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &str,
) -> Result<String, PreviewArgumentError> {
    string(
        doc,
        args,
        tool,
        key,
        256,
        "an existing id of 1-256 UTF-8 bytes, e.g. `vexel-object-1`",
    )
}

const NUMBER_FORM: &str = "a finite JSON number, e.g. 12.5";

pub(super) fn number(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &str,
) -> Result<f64, PreviewArgumentError> {
    doc.object_get(args, key.as_bytes())
        .filter(|t| doc.token(*t).is_some_and(|v| v.kind == JsonType::Number))
        .and_then(|t| doc.raw_token(t))
        .filter(|raw| raw.len() <= 40)
        .and_then(|raw| std::str::from_utf8(raw).ok())
        .and_then(|text| text.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .ok_or_else(|| bad(tool, key, NUMBER_FORM))
}

pub(super) fn opt_number(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &str,
) -> Result<Option<f64>, PreviewArgumentError> {
    if doc.object_get(args, key.as_bytes()).is_none() {
        return Ok(None);
    }
    number(doc, args, tool, key).map(Some)
}

pub(super) fn index(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &str,
) -> Result<usize, PreviewArgumentError> {
    doc.object_get(args, key.as_bytes())
        .filter(|t| doc.token(*t).is_some_and(|v| v.kind == JsonType::Number))
        .and_then(|t| doc.uint(t))
        .and_then(|v| usize::try_from(v).ok())
        .filter(|v| *v < 16)
        .ok_or_else(|| bad(tool, key, "an integer from 0 through 15"))
}

/// Parses `offset:#rrggbb[:opacity],...` into 2..=16 stops.
pub(super) fn stops(tool: &str, text: &str) -> Result<Vec<GradientStop>, PreviewArgumentError> {
    let form = "`offset:#rrggbb:opacity,...` with 2-16 stops, offsets 0-1 ascending, e.g. `0:#ff0000:1,1:#0000ff:1`";
    let mut out = Vec::new();
    for part in text.split(',') {
        let mut fields = part.split(':');
        let (Some(offset), Some(color)) = (fields.next(), fields.next()) else {
            return Err(bad(tool, "stops", form));
        };
        let opacity = fields.next().unwrap_or("1");
        let parse = |s: &str| s.trim().parse::<f64>().ok().filter(|v| v.is_finite());
        let (Some(offset), Some(opacity), None) = (parse(offset), parse(opacity), fields.next())
        else {
            return Err(bad(tool, "stops", form));
        };
        out.push(GradientStop {
            offset,
            color: color.trim().to_owned(),
            opacity,
        });
        if out.len() > 16 {
            return Err(bad(tool, "stops", form));
        }
    }
    if out.len() < 2 {
        return Err(bad(tool, "stops", form));
    }
    Ok(out)
}

pub(super) fn choice<'a>(
    tool: &str,
    key: &str,
    value: &str,
    options: &[&'a str],
) -> Result<&'a str, PreviewArgumentError> {
    options
        .iter()
        .copied()
        .find(|o| *o == value)
        .ok_or_else(|| {
            let list = options
                .iter()
                .map(|o| format!("`{o}`"))
                .collect::<Vec<_>>()
                .join(", ");
            bad(tool, key, &format!("one of {list}"))
        })
}
