//! Arguments for the seeded `noise` modifier and `native_timeline_set_camera`.

use super::{JsonDocument, JsonType, PreviewArgumentError, ease_at, invalid, tick};
use crate::preview::native_timeline::NativeOp;
use crate::preview::native_timeline::mcp_camera::{CameraKey, CameraSpec, Shake};
use vexel_timeline::noise::{MAX_AMPLITUDE, MAX_FREQUENCY, MAX_OCTAVES};

const NOISE_FORM: &str = "an object {amplitude: 0-100000, frequency?: cycles per second above 0 up to 240 (default 4), seed?: integer 0-4294967295 (default 0), octaves?: 1-4 (default 1)}";
const CAMERA_KEYS: usize = 256;
const KEY_FORM: &str = "an array of at most 256 objects {tick, x?, y?, zoom?, rotate?, ease?}";

fn number(doc: &JsonDocument<'_>, token: usize) -> Option<f64> {
    if doc.token(token)?.kind != JsonType::Number {
        return None;
    }
    std::str::from_utf8(doc.raw_token(token)?)
        .ok()?
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
}

/// Members of object `obj` as `(name, value token)`, rejecting names outside `allowed`
/// and duplicates.
fn members(doc: &JsonDocument<'_>, obj: usize, allowed: &[&str]) -> Option<Vec<(String, usize)>> {
    let object = doc.token(obj).filter(|t| t.kind == JsonType::Object)?;
    if object.child_count > allowed.len() * 2 {
        return None;
    }
    let mut out: Vec<(String, usize)> = Vec::new();
    let mut name = object.first_child;
    while let Some(key) = name {
        let text = String::from_utf8(doc.decoded_string(key)?).ok()?;
        let value = doc.token(key)?.next_sibling?;
        if !allowed.contains(&text.as_str()) || out.iter().any(|(k, _)| *k == text) {
            return None;
        }
        out.push((text, value));
        name = doc.token(value)?.next_sibling;
    }
    Some(out)
}

fn whole(n: f64, max: f64) -> Option<u32> {
    (n >= 0.0 && n.fract() == 0.0 && n <= max).then_some(n as u32)
}

/// Parses `{amplitude, frequency?, seed?, octaves?}` (plus `rotate` when `camera`).
fn shake_at(doc: &JsonDocument<'_>, obj: usize, camera: bool) -> Option<(f64, f64, u32, u32, f64)> {
    let allowed: &[&str] = if camera {
        &["amplitude", "frequency", "seed", "octaves", "rotate"]
    } else {
        &["amplitude", "frequency", "seed", "octaves"]
    };
    let (mut amplitude, mut frequency, mut seed, mut octaves, mut rotate) = (None, 4.0, 0, 1, 0.0);
    for (name, token) in members(doc, obj, allowed)? {
        let n = number(doc, token)?;
        match name.as_str() {
            "amplitude" if (0.0..=MAX_AMPLITUDE).contains(&n) => amplitude = Some(n),
            "frequency" if n > 0.0 && n <= MAX_FREQUENCY => frequency = n,
            "seed" => seed = whole(n, f64::from(u32::MAX))?,
            "octaves" => octaves = whole(n, f64::from(MAX_OCTAVES)).filter(|o| *o >= 1)?,
            "rotate" if (0.0..=360.0).contains(&n) => rotate = n,
            _ => return None,
        }
    }
    // A camera shake may be roll only.
    let amplitude = amplitude.or(if camera && rotate > 0.0 {
        Some(0.0)
    } else {
        None
    })?;
    Some((amplitude, frequency, seed, octaves, rotate))
}

/// The optional `noise` argument as canonical JSON text for the track.
pub(in crate::preview) fn noise_arg(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<Option<Vec<u8>>, PreviewArgumentError> {
    let Some(obj) = doc.object_get(args, b"noise") else {
        return Ok(None);
    };
    let (amplitude, frequency, seed, octaves, _) =
        shake_at(doc, obj, false).ok_or_else(|| invalid(tool, "noise", NOISE_FORM, None))?;
    Ok(Some(
        format!(
            "{{\"amplitude\":{amplitude},\"frequency\":{frequency},\"seed\":{seed},\"octaves\":{octaves}}}"
        )
        .into_bytes(),
    ))
}

fn channel(
    doc: &JsonDocument<'_>,
    key: usize,
    name: &'static str,
    range: (f64, f64),
    tool: &str,
) -> Result<Option<f64>, PreviewArgumentError> {
    let Some(token) = doc.object_get(key, name.as_bytes()) else {
        return Ok(None);
    };
    number(doc, token)
        .filter(|n| (range.0..=range.1).contains(n))
        .map(Some)
        .ok_or_else(|| {
            invalid(
                tool,
                name,
                &format!("a number from {} to {}", range.0, range.1),
                None,
            )
        })
}

pub(in crate::preview) fn camera(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<NativeOp, PreviewArgumentError> {
    let clear = match doc.object_get(args, b"clear").and_then(|t| doc.token(t)) {
        None => false,
        Some(t) if t.kind == JsonType::True => true,
        Some(t) if t.kind == JsonType::False => false,
        Some(_) => return Err(invalid(tool, "clear", "true or false", None)),
    };
    let mut keys: Vec<CameraKey> = Vec::new();
    if let Some(array) = doc.object_get(args, b"keys") {
        let token = doc
            .token(array)
            .filter(|t| t.kind == JsonType::Array && t.child_count <= CAMERA_KEYS)
            .ok_or_else(|| invalid(tool, "keys", KEY_FORM, None))?;
        let mut item = token.first_child;
        while let Some(at) = item {
            let index = keys.len();
            let wrap = |e: PreviewArgumentError| match e {
                PreviewArgumentError::Actionable(m) => {
                    PreviewArgumentError::Actionable(format!("keys[{index}]: {m}"))
                }
                PreviewArgumentError::Generic(m) => {
                    PreviewArgumentError::Actionable(format!("keys[{index}]: {m}"))
                }
            };
            members(doc, at, &["tick", "x", "y", "zoom", "rotate", "ease"])
                .ok_or_else(|| wrap(invalid(tool, "keys", KEY_FORM, None)))?;
            let key = CameraKey {
                tick: tick(doc, at, tool, "tick").map_err(wrap)?,
                x: channel(doc, at, "x", (-1e6, 1e6), tool).map_err(wrap)?,
                y: channel(doc, at, "y", (-1e6, 1e6), tool).map_err(wrap)?,
                zoom: channel(doc, at, "zoom", (0.01, 100.0), tool).map_err(wrap)?,
                rotate: channel(doc, at, "rotate", (-36000.0, 36000.0), tool).map_err(wrap)?,
                ease: ease_at(doc, at, tool, "ease").map_err(wrap)?,
            };
            if key.x.is_none() && key.y.is_none() && key.zoom.is_none() && key.rotate.is_none() {
                return Err(wrap(invalid(tool, "keys", KEY_FORM, None)));
            }
            if keys.iter().any(|k| k.tick == key.tick) {
                return Err(wrap(PreviewArgumentError::Actionable(format!(
                    "duplicate tick {}; each camera key needs its own tick",
                    key.tick
                ))));
            }
            keys.push(key);
            item = doc.token(at).and_then(|t| t.next_sibling);
        }
    }
    let shake = match doc.object_get(args, b"shake") {
        None => None,
        Some(obj) => {
            let (amplitude, frequency, seed, octaves, rotate) = shake_at(doc, obj, true)
                .ok_or_else(|| {
                    invalid(
                        tool,
                        "shake",
                        "an object {amplitude: pixels, frequency?: Hz, seed?, octaves?, rotate?: degrees of roll}",
                        None,
                    )
                })?;
            Some(Shake {
                amplitude,
                frequency,
                seed,
                octaves,
                rotate,
            })
        }
    };
    if clear == (!keys.is_empty() || shake.is_some()) {
        return Err(invalid(
            tool,
            "keys",
            "keys and/or shake to set the camera, or clear:true to remove it (not both)",
            None,
        ));
    }
    Ok(NativeOp::SetCamera(CameraSpec { keys, shake, clear }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(keys: &str) -> Result<NativeOp, PreviewArgumentError> {
        let json = format!("{{\"keys\":[{keys}]}}");
        let doc = JsonDocument::parse_owned(json.as_bytes()).unwrap();
        camera(&doc, doc.root(), "set_camera")
    }

    #[test]
    fn duplicate_camera_ticks_are_rejected_by_name() {
        match parse(r#"{"tick":0,"zoom":1},{"tick":24,"zoom":2},{"tick":24,"x":5}"#) {
            Err(PreviewArgumentError::Actionable(m)) => {
                assert!(
                    m.contains("keys[2]") && m.contains("duplicate tick 24"),
                    "{m}"
                );
            }
            _ => panic!("duplicate tick must be rejected"),
        }
        match parse(r#"{"tick":0,"zoom":1},{"tick":0,"zoom":2}"#) {
            Err(PreviewArgumentError::Actionable(m)) => {
                assert!(m.contains("duplicate tick 0"), "{m}")
            }
            _ => panic!("tick-zero duplicate must be rejected"),
        }
        assert!(parse(r#"{"tick":0,"zoom":1},{"tick":1,"zoom":2}"#).is_ok());
    }
}
