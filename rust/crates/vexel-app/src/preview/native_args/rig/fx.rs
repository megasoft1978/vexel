//! Arguments of `effect_apply`: a preset name, optional target, idPrefix and
//! remove flag, plus the bounded parameters that preset accepts.

use super::{JsonDocument, JsonType, PreviewArgumentError, document, invalid, members, number};
use crate::preview::native_timeline::mcp_face::FaceOp;
use crate::preview::native_timeline::mcp_fx_spec::{FxSpec, PARAMS, Preset};

fn text(doc: &JsonDocument<'_>, at: usize, max: usize) -> Option<String> {
    if doc.token(at)?.kind != JsonType::String || doc.raw_token(at)?.len() > max * 6 + 2 {
        return None;
    }
    String::from_utf8(doc.decoded_string(at)?)
        .ok()
        .filter(|s| !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control))
}

pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<FaceOp, PreviewArgumentError> {
    document(doc, args, tool)?;
    let all = members(doc, args, 32).ok_or_else(|| invalid(tool, "arguments", "an object"))?;
    let preset = all
        .iter()
        .find(|(k, _)| k == "preset")
        .and_then(|(_, at)| text(doc, *at, 32))
        .and_then(|name| Preset::parse(&name))
        .ok_or_else(|| {
            invalid(
                tool,
                "preset",
                "speed_lines, glow, chromatic_split, film_grain, halftone, vhs_scanlines, motion_blur_dir, smear, vignette or flash",
            )
        })?;
    let mut spec = FxSpec {
        preset,
        target: None,
        prefix: None,
        remove: false,
        nums: Vec::new(),
        mode: None,
        color: None,
        blend: None,
    };
    for (name, at) in &all {
        match name.as_str() {
            "documentId" | "preset" => {}
            "target" => {
                spec.target = Some(
                    text(doc, *at, 128).ok_or_else(|| invalid(tool, "target", "an element id"))?,
                )
            }
            "idPrefix" => {
                spec.prefix = Some(text(doc, *at, 48).ok_or_else(|| {
                    invalid(tool, "idPrefix", "1-48 characters of A-Z a-z 0-9 _ -")
                })?)
            }
            "remove" => {
                spec.remove = match doc.raw_token(*at) {
                    Some(b"true") => true,
                    Some(b"false") => false,
                    _ => return Err(invalid(tool, "remove", "true or false")),
                }
            }
            "mode" | "color" | "blend" if preset.allowed().contains(&name.as_str()) => {
                let value = text(doc, *at, 16).ok_or_else(|| {
                    invalid(tool, name, "a short string (see the tool description)")
                })?;
                match name.as_str() {
                    "mode" => spec.mode = Some(value),
                    "color" => spec.color = Some(value),
                    _ => spec.blend = Some(value),
                }
            }
            other => {
                let (key, min, max, int) = PARAMS
                    .iter()
                    .find(|(k, ..)| *k == other)
                    .filter(|(k, ..)| preset.allowed().contains(k))
                    .copied()
                    .ok_or_else(|| {
                        invalid(
                            tool,
                            other,
                            &format!(
                                "one of {} for {}",
                                preset.allowed().join(", "),
                                preset.name()
                            ),
                        )
                    })?;
                let v = doc
                    .token(*at)
                    .filter(|t| t.kind == JsonType::Number)
                    .and_then(|_| number(doc, *at))
                    .filter(|v| {
                        v.is_finite() && (min..=max).contains(v) && (!int || v.fract() == 0.0)
                    })
                    .ok_or_else(|| {
                        invalid(
                            tool,
                            key,
                            &format!(
                                "{} {min} to {max}",
                                if int { "an integer" } else { "a number" }
                            ),
                        )
                    })?;
                spec.nums.push((key, v));
            }
        }
    }
    if spec.remove && spec.prefix.is_none() {
        return Err(invalid(
            tool,
            "idPrefix",
            "the prefix of the effect to remove",
        ));
    }
    Ok(FaceOp::Fx(Box::new(spec)))
}
