//! Arguments of the face tools (`mouth_set`, `lip_sync`, `eyes_set`, `blink`):
//! bounded, with messages that name the offending member.

use super::{
    JsonDocument, JsonType, PreviewArgumentError, document, invalid, members, number, uint32,
};
use crate::preview::native_timeline::mcp_face::FaceOp;
use crate::preview::native_timeline::mcp_lipsync::{LipSource, LipSync};
use crate::preview::native_timeline::mcp_mouth::{Mouth, valid_id};
use vexel_timeline::audio_envelope::MAX_TICKS;
use vexel_timeline::lipsync::{self, Mapping};

fn id_at(doc: &JsonDocument<'_>, at: usize) -> Option<String> {
    let raw = doc.raw_token(at)?;
    if raw.len() > 6 * 128 + 2 {
        return None;
    }
    String::from_utf8(doc.decoded_string(at)?)
        .ok()
        .filter(|s| valid_id(s))
}

fn mouth_set(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<FaceOp, PreviewArgumentError> {
    document(doc, args, tool)?;
    let character = doc
        .object_get(args, b"character")
        .and_then(|at| id_at(doc, at))
        .ok_or_else(|| invalid(tool, "character", "a <g> group id"))?;
    let form = "an object mapping rest, A, E, I, O, U, MBP, FV, L, WQ, CDGK or TH to <g> group ids; rest is required";
    let table = doc
        .object_get(args, b"shapes")
        .and_then(|at| members(doc, at, lipsync::VISEMES.len()))
        .ok_or_else(|| invalid(tool, "shapes", form))?;
    let mut shapes: Vec<(u8, String)> = Vec::new();
    for (name, value) in table {
        let viseme = lipsync::index(&name).ok_or_else(|| invalid(tool, "shapes", form))?;
        let group = id_at(doc, value).ok_or_else(|| invalid(tool, "shapes", form))?;
        if shapes.iter().any(|(v, _)| *v == viseme) {
            return Err(invalid(tool, "shapes", form));
        }
        shapes.push((viseme, group));
    }
    shapes.sort();
    if shapes.first().map(|s| s.0) != Some(lipsync::REST) {
        return Err(invalid(tool, "shapes.rest", form));
    }
    Ok(FaceOp::Mouth(Mouth { character, shapes }))
}

fn text_at(doc: &JsonDocument<'_>, at: usize, max: usize) -> Option<String> {
    let raw = doc.raw_token(at)?;
    if raw.len() > max * 6 + 2 {
        return None;
    }
    String::from_utf8(doc.decoded_string(at)?)
        .ok()
        .filter(|s| !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control))
}

fn source(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<LipSource, PreviewArgumentError> {
    let form = "an object with exactly one of trackId (audio track id), path (WAV, AIFF or CAF file) or envelope (1-100000 loudness numbers 0-1, one per tick)";
    let bad = || invalid(tool, "source", form);
    let fields = doc
        .object_get(args, b"source")
        .and_then(|at| members(doc, at, 3))
        .ok_or_else(bad)?;
    let [(name, value)] = fields.as_slice() else {
        return Err(bad());
    };
    match name.as_str() {
        "trackId" => text_at(doc, *value, 128)
            .map(LipSource::Track)
            .ok_or_else(bad),
        "path" => text_at(doc, *value, 1023)
            .map(LipSource::Path)
            .ok_or_else(bad),
        "envelope" => {
            let array = doc
                .token(*value)
                .filter(|t| t.kind == JsonType::Array && (1..=MAX_TICKS).contains(&t.child_count))
                .ok_or_else(bad)?;
            let mut values = Vec::with_capacity(array.child_count);
            let mut item = array.first_child;
            while let Some(at) = item {
                let v = number(doc, at)
                    .filter(|v| (0.0..=1.0).contains(v))
                    .ok_or_else(bad)?;
                values.push(v as f32);
                item = doc.token(at).and_then(|t| t.next_sibling);
            }
            Ok(LipSource::Envelope(values))
        }
        _ => Err(bad()),
    }
}

fn lip_sync(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<FaceOp, PreviewArgumentError> {
    document(doc, args, tool)?;
    let character = doc
        .object_get(args, b"character")
        .and_then(|at| id_at(doc, at))
        .ok_or_else(|| invalid(tool, "character", "a <g> group id with a mouth_set"))?;
    let source = source(doc, args, tool)?;
    let int = |key: &'static str, form: &str| {
        doc.object_get(args, key.as_bytes())
            .map(|at| uint32(doc, at).ok_or_else(|| invalid(tool, key, form)))
            .transpose()
    };
    let start_tick = int("startTick", "a tick, 0 to 4294967295")?;
    let ticks = int("ticks", "an integer 1-100000")?;
    if ticks.is_some_and(|t| t == 0 || t as usize > MAX_TICKS) {
        return Err(invalid(tool, "ticks", "an integer 1-100000"));
    }
    let smoothing = int("smoothing", "an integer 0-8")?.unwrap_or(1);
    if smoothing > lipsync::MAX_SMOOTHING {
        return Err(invalid(tool, "smoothing", "an integer 0-8"));
    }
    let seed = int("seed", "an integer 0 to 4294967295")?.unwrap_or(0);
    let mut mapping = Mapping::default();
    if let Some(at) = doc.object_get(args, b"mapping") {
        let form = "{close?, mid?, wide?} numbers with 0 <= close < mid < wide <= 1";
        let bad = || invalid(tool, "mapping", form);
        for (name, value) in members(doc, at, 3).ok_or_else(bad)? {
            let v = number(doc, value).ok_or_else(bad)? as f32;
            match name.as_str() {
                "close" => mapping.close = v,
                "mid" => mapping.mid = v,
                "wide" => mapping.wide = v,
                _ => return Err(bad()),
            }
        }
        if !mapping.valid() {
            return Err(bad());
        }
    }
    let mut text = Vec::new();
    if let Some(at) = doc.object_get(args, b"text") {
        let form = "at most 256 word strings of 1-32 characters";
        let bad = || invalid(tool, "text", form);
        let array = doc
            .token(at)
            .filter(|t| t.kind == JsonType::Array && t.child_count <= lipsync::MAX_WORDS)
            .ok_or_else(bad)?;
        let mut item = array.first_child;
        while let Some(word) = item {
            text.push(text_at(doc, word, lipsync::MAX_WORD_LEN).ok_or_else(bad)?);
            item = doc.token(word).and_then(|t| t.next_sibling);
        }
    }
    Ok(FaceOp::Lip(Box::new(LipSync {
        character,
        source,
        start_tick,
        ticks,
        mapping,
        smoothing,
        seed,
        text,
    })))
}

/// `None` when `tool` is not a face tool.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<FaceOp, PreviewArgumentError>> {
    Some(match tool {
        "mouth_set" => mouth_set(doc, args, tool),
        "lip_sync" => lip_sync(doc, args, tool),
        _ => return None,
    })
}
