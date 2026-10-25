//! Optional members of `element_add` applied in the same edit as the shape:
//! explicit `id`, `opacity`, `transform` and a gradient paint reference.

use super::{JsonDocument, JsonType, PreviewArgumentError};
use vexel_v1_document::TransformSpec;

/// Longest explicit element id, in bytes.
const ID_LIMIT: usize = 128;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AddExtras {
    pub id: Option<String>,
    pub opacity: Option<f64>,
    pub transform: Option<TransformSpec>,
    /// Gradient id and whether it paints the stroke instead of the fill.
    pub gradient: Option<(String, bool)>,
}

fn invalid(key: &str, form: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!("element_add argument {key} is invalid; use {form}"))
}

fn text(
    doc: &JsonDocument<'_>,
    args: usize,
    key: &str,
    form: &str,
) -> Result<Option<String>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, key.as_bytes()) else {
        return Ok(None);
    };
    doc.raw_token(token)
        .filter(|raw| raw.len() <= ID_LIMIT * 6 + 2)
        .and_then(|_| doc.decoded_string(token))
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|value| !value.is_empty() && value.len() <= ID_LIMIT)
        .map(Some)
        .ok_or_else(|| invalid(key, form))
}

pub(super) fn number(doc: &JsonDocument<'_>, token: usize) -> Option<f64> {
    doc.token(token)
        .filter(|t| t.kind == JsonType::Number)
        .and_then(|_| doc.raw_token(token))
        .filter(|raw| raw.len() <= 32)
        .and_then(|raw| std::str::from_utf8(raw).ok()?.parse::<f64>().ok())
        .filter(|n| n.is_finite())
}

fn explicit_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    (bytes[0].is_ascii_alphabetic() || bytes[0] == b'_')
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(b))
}

pub(super) fn transform(
    doc: &JsonDocument<'_>,
    token: usize,
) -> Result<TransformSpec, PreviewArgumentError> {
    let form = "an object with number members translateX, translateY, rotate, pivotX, pivotY, scaleX, scaleY, skewX";
    let object = doc
        .token(token)
        .filter(|t| t.kind == JsonType::Object)
        .ok_or_else(|| invalid("transform", form))?;
    let mut spec = TransformSpec::default();
    let mut key = object.first_child;
    while let Some(at) = key {
        let value = doc.token(at).and_then(|t| t.next_sibling);
        let name = doc.decoded_string(at).unwrap_or_default();
        let n = value
            .and_then(|v| number(doc, v))
            .ok_or_else(|| invalid("transform", form))?;
        match name.as_slice() {
            b"translateX" => spec.translate.0 = n,
            b"translateY" => spec.translate.1 = n,
            b"rotate" => spec.rotate_deg = n,
            b"pivotX" => spec.pivot.0 = n,
            b"pivotY" => spec.pivot.1 = n,
            b"scaleX" => spec.scale.0 = n,
            b"scaleY" => spec.scale.1 = n,
            b"skewX" => spec.skew_x_deg = n,
            _ => return Err(invalid("transform", form)),
        }
        key = value
            .and_then(|v| doc.token(v))
            .and_then(|t| t.next_sibling);
    }
    Ok(spec)
}

/// `None` when the call carries none of the optional members.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
) -> Result<Option<AddExtras>, PreviewArgumentError> {
    let id_form = "1-128 characters from A-Z a-z 0-9 _ - . starting with a letter or _";
    let id = text(doc, args, "id", id_form)?;
    if id.as_deref().is_some_and(|id| !explicit_id(id)) {
        return Err(invalid("id", id_form));
    }
    let opacity = match doc.object_get(args, b"opacity") {
        None => None,
        Some(token) => Some(
            number(doc, token)
                .filter(|n| (0.0..=1.0).contains(n))
                .ok_or_else(|| invalid("opacity", "a number from 0 through 1"))?,
        ),
    };
    let transform = doc
        .object_get(args, b"transform")
        .map(|token| transform(doc, token))
        .transpose()?;
    if transform.is_some_and(|spec| spec.to_matrix().iter().any(|v| !v.is_finite())) {
        return Err(invalid("transform", "values that give a finite matrix"));
    }
    let gradient = text(doc, args, "gradientId", "a gradient id from gradient_add")?;
    let stroke = match text(doc, args, "paint", "`fill` or `stroke`")?.as_deref() {
        None | Some("fill") => false,
        Some("stroke") => true,
        Some(_) => return Err(invalid("paint", "`fill` or `stroke`")),
    };
    if gradient.is_none() && doc.object_get(args, b"paint").is_some() {
        return Err(invalid("paint", "gradientId together with paint"));
    }
    let extras = AddExtras {
        id,
        opacity,
        transform,
        gradient: gradient.map(|id| (id, stroke)),
    };
    Ok((extras != AddExtras::default()).then_some(extras))
}
