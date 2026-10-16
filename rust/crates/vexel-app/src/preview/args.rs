//! Bounded, typed argument parsing for the file, attribute, delete, line and
//! path tools. Every value is checked here; the session applies only
//! operations that already passed these bounds.

use super::{AttributeChange, JsonDocument, Operation, number_field, parse_color, string_field};

pub(super) const LINE_KEYS: &[&[u8]] = &[b"x1", b"y1", b"x2", b"y2", b"stroke", b"stroke-width"];
pub(super) const PATH_KEYS: &[&[u8]] = &[b"d", b"fill", b"stroke", b"stroke-width"];

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
