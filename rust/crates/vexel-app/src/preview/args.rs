//! Bounded, typed argument parsing for the file, attribute, delete, line and
//! path tools. Every value is checked here; the session applies only
//! operations that already passed these bounds.

use super::{
    AttributeChange, JsonDocument, JsonType, Operation, PreviewArgumentError, Shape, number_field,
    parse_color, string_field, validate_shape,
};

pub(super) fn rejected_element_add_attribute(template: &str, key: &str) -> String {
    let allowed = vexel_protocol::element_add_keys(template)
        .map(|keys| {
            keys.iter()
                .map(|key| std::str::from_utf8(key).expect("static attribute keys are UTF-8"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    format!("element_add {template} does not accept attribute \"{key}\"; allowed: {allowed}")
}

pub(super) fn rejected_element_add_parent(parent: Option<&str>) -> String {
    match parent {
        Some(parent) => format!(
            "element_add parentId \"{parent}\" is invalid; use \"root\" or an existing group id"
        ),
        None => "element_add parentId is required; use \"root\" for the top level or an existing group id".to_owned(),
    }
}

const BAD_ARGS: &str = "invalid preview tool arguments";
/// Longest accepted `path` argument in bytes (matches the access policy cap).
const PATH_LIMIT: usize = 1023;
/// Longest accepted SVG path data in bytes.
pub(crate) const PATH_DATA_LIMIT: usize = 4096;
const ID_LIMIT: usize = 128;
const COORDINATE_LIMIT: f64 = 100_000.0;
const MAX_STROKE_WIDTH: f64 = 256.0;

/// A decoded string no longer than `max` bytes; the raw token is bounded first
/// so an oversize value is never fully decoded.
fn bounded_string(doc: &JsonDocument<'_>, obj: usize, key: &[u8], max: usize) -> Option<String> {
    let token = doc.object_get(obj, key)?;
    if doc.raw_token(token)?.len() > max * 6 + 2 {
        return None;
    }
    let text = String::from_utf8(doc.decoded_string(token)?).ok()?;
    (text.len() <= max).then_some(text)
}

fn path_argument(doc: &JsonDocument<'_>, args: usize) -> Result<String, &'static str> {
    bounded_string(doc, args, b"path", PATH_LIMIT)
        .filter(|p| !p.is_empty() && !p.contains('\0'))
        .ok_or(BAD_ARGS)
}

fn element_id(doc: &JsonDocument<'_>, args: usize) -> Result<String, &'static str> {
    bounded_string(doc, args, b"elementId", ID_LIMIT)
        .filter(|id| !id.is_empty())
        .ok_or(BAD_ARGS)
}

pub(super) fn open(doc: &JsonDocument<'_>, args: usize) -> Result<Operation, &'static str> {
    Ok(Operation::Open {
        path: path_argument(doc, args)?,
    })
}

pub(super) fn save_as(doc: &JsonDocument<'_>, args: usize) -> Result<Operation, &'static str> {
    Ok(Operation::SaveAs {
        path: path_argument(doc, args)?,
    })
}

pub(super) fn delete(doc: &JsonDocument<'_>, args: usize) -> Result<Operation, &'static str> {
    Ok(Operation::Delete {
        element_id: element_id(doc, args)?,
    })
}

pub(super) fn attribute_set(
    doc: &JsonDocument<'_>,
    args: usize,
) -> Result<Operation, &'static str> {
    let element_id = element_id(doc, args)?;
    let name = bounded_string(doc, args, b"name", 64).ok_or(BAD_ARGS)?;
    let value = bounded_string(doc, args, b"value", 64).ok_or(BAD_ARGS)?;
    let change = match name.as_str() {
        "fill" => {
            parse_color(&value)?;
            AttributeChange::Fill(value)
        }
        "stroke" => {
            parse_color(&value)?;
            AttributeChange::Stroke(value)
        }
        "stroke-width" => AttributeChange::StrokeWidth(width(&value)?),
        "transform" => {
            let (dx, dy) = translate(&value).ok_or("unsupported attribute")?;
            AttributeChange::Translate(dx, dy)
        }
        _ => return Err("unsupported attribute"),
    };
    Ok(Operation::AttributeSet { element_id, change })
}

pub(super) fn line(doc: &JsonDocument<'_>, attrs: usize) -> Result<Operation, &'static str> {
    let num = |key: &[u8]| number_field(doc, attrs, key);
    let stroke = string_field(doc, attrs, b"stroke").ok_or(BAD_ARGS)?;
    parse_color(&stroke)?;
    Ok(Operation::AddLine {
        x1: num(b"x1")?,
        y1: num(b"y1")?,
        x2: num(b"x2")?,
        y2: num(b"y2")?,
        stroke,
        width: optional_width(doc, attrs)?,
    })
}

pub(super) fn path(doc: &JsonDocument<'_>, attrs: usize) -> Result<Operation, &'static str> {
    let d = bounded_string(doc, attrs, b"d", PATH_DATA_LIMIT)
        .filter(|d| !d.is_empty())
        .ok_or(BAD_ARGS)?;
    vexel_v1_document::validate_path_data(&d).map_err(|_| BAD_ARGS)?;
    Ok(Operation::AddPath {
        d,
        fill: optional_paint(doc, attrs, b"fill")?,
        stroke: optional_paint(doc, attrs, b"stroke")?,
        width: optional_width(doc, attrs)?,
    })
}

/// `#rrggbb` or `none`; absent means `none`.
fn optional_paint(
    doc: &JsonDocument<'_>,
    attrs: usize,
    key: &[u8],
) -> Result<String, &'static str> {
    if doc.object_get(attrs, key).is_none() {
        return Ok("none".to_owned());
    }
    let value = string_field(doc, attrs, key).ok_or(BAD_ARGS)?;
    if value != "none" {
        parse_color(&value)?;
    }
    Ok(value)
}

fn optional_width(doc: &JsonDocument<'_>, attrs: usize) -> Result<f64, &'static str> {
    if doc.object_get(attrs, b"stroke-width").is_none() {
        return Ok(1.0);
    }
    width(&string_field(doc, attrs, b"stroke-width").ok_or(BAD_ARGS)?)
}

fn number(text: &str) -> Option<f64> {
    (!text.is_empty()
        && text.len() <= 32
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || b"+-.eE".contains(&b)))
    .then(|| text.parse::<f64>().ok())
    .flatten()
    .filter(|n| n.is_finite())
}

fn width(text: &str) -> Result<f64, &'static str> {
    number(text)
        .filter(|w| (0.0..=MAX_STROKE_WIDTH).contains(w))
        .ok_or(BAD_ARGS)
}

/// Exactly `translate(dx dy)` or `translate(dx,dy)` with finite values of at
/// most 100000 in magnitude.
fn translate(value: &str) -> Option<(f64, f64)> {
    let inner = value.strip_prefix("translate(")?.strip_suffix(')')?;
    let (a, b) = if inner.contains(',') {
        let (a, b) = inner.split_once(',')?;
        if b.contains(',') {
            return None;
        }
        (a.trim(), b.trim())
    } else {
        let mut parts = inner.split_ascii_whitespace();
        let pair = (parts.next()?, parts.next()?);
        parts.next().is_none().then_some(pair)?
    };
    let (dx, dy) = (number(a)?, number(b)?);
    (dx.abs() <= COORDINATE_LIMIT && dy.abs() <= COORDINATE_LIMIT).then_some((dx, dy))
}

/// `element_add` for rect, ellipse, line and path templates; every attribute
/// is checked against the template's allowed keys and value bounds.
pub(super) fn element_add(
    doc: &JsonDocument<'_>,
    args: usize,
) -> Result<Operation, PreviewArgumentError> {
    let parent = string_field(doc, args, b"parentId");
    if parent.as_deref().is_some_and(|parent| parent != "root") {
        return Err(PreviewArgumentError::Actionable(
            rejected_element_add_parent(parent.as_deref()),
        ));
    }
    let template = string_field(doc, args, b"template").ok_or(PreviewArgumentError::Generic(
        "invalid preview tool arguments",
    ))?;
    let attrs = doc
        .object_get(args, b"attributes")
        .ok_or(PreviewArgumentError::Generic(
            "invalid preview tool arguments",
        ))?;
    if doc.token(attrs).is_none_or(|t| t.kind != JsonType::Object) {
        return Err(PreviewArgumentError::Generic(
            "invalid preview tool arguments",
        ));
    }
    let allowed = vexel_protocol::element_add_keys(&template)
        .ok_or(PreviewArgumentError::Generic("unsupported preview shape"))?;
    let mut key = doc.token(attrs).and_then(|t| t.first_child);
    while let Some(key_token) = key {
        let name = doc
            .decoded_string(key_token)
            .ok_or(PreviewArgumentError::Generic(
                "invalid preview tool arguments",
            ))?;
        if !allowed
            .iter()
            .any(|candidate| *candidate == name.as_slice())
        {
            let rejected =
                String::from_utf8(name).expect("decoded JSON attribute key is valid UTF-8");
            return Err(PreviewArgumentError::Actionable(
                rejected_element_add_attribute(&template, &rejected),
            ));
        }
        let value = doc.token(key_token).and_then(|t| t.next_sibling).ok_or(
            PreviewArgumentError::Generic("invalid preview tool arguments"),
        )?;
        if doc.token(value).is_none_or(|t| t.kind != JsonType::String) {
            return Err(PreviewArgumentError::Generic(
                "invalid preview tool arguments",
            ));
        }
        key = doc.token(value).and_then(|t| t.next_sibling);
    }
    match template.as_str() {
        "line" => return Ok(line(doc, attrs)?),
        "path" => return Ok(path(doc, attrs)?),
        _ => {}
    }
    let num = |k: &[u8]| number_field(doc, attrs, k);
    let fill = parse_color(
        string_field(doc, attrs, b"fill")
            .ok_or(PreviewArgumentError::Generic(
                "invalid preview tool arguments",
            ))?
            .as_str(),
    )?;
    let shape = match template.as_str() {
        "rect" => Shape::Rect {
            x: num(b"x")?,
            y: num(b"y")?,
            width: num(b"width")?,
            height: num(b"height")?,
            rgb: fill,
        },
        "ellipse" => Shape::Ellipse {
            cx: num(b"cx")?,
            cy: num(b"cy")?,
            rx: num(b"rx")?,
            ry: num(b"ry")?,
            rgb: fill,
        },
        _ => return Err(PreviewArgumentError::Generic("unsupported preview shape")),
    };
    validate_shape(shape).map_err(PreviewArgumentError::Generic)?;
    Ok(Operation::Add(shape))
}
