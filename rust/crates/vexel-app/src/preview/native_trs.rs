//! TRS value given as a JSON object, converted to the timeline's canonical
//! `{"tx":..,"rotation_deg":..}` text. Every accepted spelling is listed in
//! `FORM`, which the tool descriptions and docs/mcp.md repeat.

use super::native_args::invalid;
use super::{JsonDocument, JsonType, PreviewArgumentError};

pub(super) const FORM: &str = "an array [tx,ty,rotationDeg,sx,sy,pivotX,pivotY(,skewXDeg)] or an object with optional translate ([x,y] or {x,y}) or tx/ty, rotate (degrees; also rotation, rotationDeg), scale (number, [x,y] or {x,y}) or sx/sy, pivot ([x,y] or {x,y}) or pivotX/pivotY, skewX (degrees); omitted parts are identity (translate 0, rotate 0, scale 1, pivot 0)";

// tx, ty, rotation_deg, sx, sy, pivot_x, pivot_y, skew_x_deg
const NAMES: [&str; 8] = [
    "tx",
    "ty",
    "rotation_deg",
    "sx",
    "sy",
    "pivot_x",
    "pivot_y",
    "skew_x_deg",
];

fn number(doc: &JsonDocument<'_>, token: usize) -> Option<f64> {
    if doc.token(token)?.kind != JsonType::Number {
        return None;
    }
    let text = std::str::from_utf8(doc.raw_token(token)?).ok()?;
    text.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && v.abs() <= 1e9)
}

/// Reads `[x,y]` or `{x,y}` into two numbers.
fn pair(doc: &JsonDocument<'_>, token: usize) -> Option<[f64; 2]> {
    let item = doc.token(token)?;
    match item.kind {
        // An array has two items; an object has two members (key and value tokens).
        JsonType::Array if item.child_count == 2 => Some([
            number(doc, doc.array_get(token, 0)?)?,
            number(doc, doc.array_get(token, 1)?)?,
        ]),
        JsonType::Object if item.child_count == 4 => Some([
            number(doc, doc.object_get(token, b"x")?)?,
            number(doc, doc.object_get(token, b"y")?)?,
        ]),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum Shape {
    One,
    Pair,
    Scale,
}

pub(super) fn from_object(
    doc: &JsonDocument<'_>,
    object: usize,
    tool: &str,
) -> Result<Vec<u8>, PreviewArgumentError> {
    let mut slots = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0];
    let bad = |key: &str, what: &str| invalid(tool, &format!("value.{key}"), what, None);
    let mut key = doc.token(object).and_then(|t| t.first_child);
    while let Some(key_token) = key {
        let value = doc
            .token(key_token)
            .and_then(|t| t.next_sibling)
            .ok_or_else(|| invalid(tool, "value", FORM, None))?;
        key = doc.token(value).and_then(|t| t.next_sibling);
        let name = doc
            .decoded_string(key_token)
            .and_then(|k| String::from_utf8(k).ok())
            .ok_or_else(|| invalid(tool, "value", FORM, None))?;
        let shown: String = name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .take(32)
            .collect();
        let (first, shape) = match name.as_str() {
            "tx" | "translateX" | "translate_x" => (0, Shape::One),
            "ty" | "translateY" | "translate_y" => (1, Shape::One),
            "translate" => (0, Shape::Pair),
            "rotate" | "rotation" | "rotationDeg" | "rotation_deg" | "rotateDeg" => (2, Shape::One),
            "sx" | "scaleX" | "scale_x" => (3, Shape::One),
            "sy" | "scaleY" | "scale_y" => (4, Shape::One),
            "scale" => (3, Shape::Scale),
            "pivotX" | "pivot_x" => (5, Shape::One),
            "pivotY" | "pivot_y" => (6, Shape::One),
            "pivot" => (5, Shape::Pair),
            "skewX" | "skewXDeg" | "skew_x_deg" | "skew" => (7, Shape::One),
            _ => return Err(bad(&shown, &format!("only the TRS fields: {FORM}"))),
        };
        match shape {
            Shape::One => {
                slots[first] = number(doc, value).ok_or_else(|| bad(&shown, "a finite number"))?;
            }
            Shape::Pair => {
                let [a, b] = pair(doc, value)
                    .ok_or_else(|| bad(&shown, "[x,y] or {x,y} of finite numbers"))?;
                slots[first] = a;
                slots[first + 1] = b;
            }
            Shape::Scale => {
                let [a, b] = match number(doc, value) {
                    Some(n) => [n, n],
                    None => pair(doc, value)
                        .ok_or_else(|| bad(&shown, "a number, [x,y] or {x,y} of finite numbers"))?,
                };
                slots[3] = a;
                slots[4] = b;
            }
        }
    }
    let mut out = String::from("{");
    for (i, (name, slot)) in NAMES.iter().zip(slots).enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("\"{name}\":{slot}"));
    }
    out.push('}');
    Ok(out.into_bytes())
}
