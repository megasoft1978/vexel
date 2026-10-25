//! Arguments of the face tools (`mouth_set`, `lip_sync`, `eyes_set`, `blink`):
//! bounded, with messages that name the offending member.

use super::{
    JsonDocument, JsonType, PreviewArgumentError, document, invalid, members, number, pivot, uint32,
};
use crate::preview::native_timeline::mcp_eyes::{
    Blink, Eye, Gaze, Lid, MAX_DURATION, MAX_EYES, MAX_GAZE_KEYS, MAX_PARTS, When,
};
use crate::preview::native_timeline::mcp_face::FaceOp;
use crate::preview::native_timeline::mcp_lipsync::{LipSource, LipSync};
use crate::preview::native_timeline::mcp_mouth::{Mouth, valid_id};
use vexel_timeline::audio_envelope::MAX_TICKS;
use vexel_timeline::blink::MAX_BLINKS;
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

/// 1..=`max` ids of an array.
fn id_list(doc: &JsonDocument<'_>, at: usize, max: usize) -> Option<Vec<String>> {
    let array = doc
        .token(at)
        .filter(|t| t.kind == JsonType::Array && (1..=max).contains(&t.child_count))?;
    let mut out = Vec::new();
    let mut item = array.first_child;
    while let Some(id) = item {
        out.push(id_at(doc, id)?);
        item = doc.token(id).and_then(|t| t.next_sibling);
    }
    Some(out)
}

fn eyes_set(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<FaceOp, PreviewArgumentError> {
    document(doc, args, tool)?;
    let form = "1-8 objects {eye: shape or group id, pupils: 1-8 sibling <g> group ids}";
    let array = doc
        .object_get(args, b"eyes")
        .and_then(|at| doc.token(at))
        .filter(|t| t.kind == JsonType::Array && (1..=MAX_EYES).contains(&t.child_count))
        .ok_or_else(|| invalid(tool, "eyes", form))?;
    let mut eyes = Vec::new();
    let mut item = array.first_child;
    while let Some(at) = item {
        let bad = |what: &str| invalid(tool, &format!("eyes[{}].{what}", eyes.len()), form);
        let mut eye = None;
        let mut pupils = None;
        for (name, value) in members(doc, at, 2).ok_or_else(|| bad("(object)"))? {
            match name.as_str() {
                "eye" => eye = Some(id_at(doc, value).ok_or_else(|| bad("eye"))?),
                "pupils" => {
                    pupils = Some(id_list(doc, value, MAX_EYES).ok_or_else(|| bad("pupils"))?)
                }
                _ => return Err(bad("(unknown member)")),
            }
        }
        eyes.push(Eye {
            eye: eye.ok_or_else(|| bad("eye"))?,
            pupils: pupils.ok_or_else(|| bad("pupils"))?,
        });
        item = doc.token(at).and_then(|t| t.next_sibling);
    }
    Ok(FaceOp::Eyes(eyes))
}

fn lids(doc: &JsonDocument<'_>, at: usize, tool: &str) -> Result<Vec<Lid>, PreviewArgumentError> {
    let form = "1-16 objects {id: <g> group id, pivot?: [x,y] or \"center\", closedScale?: 0.01-1}";
    let array = doc
        .token(at)
        .filter(|t| t.kind == JsonType::Array && (1..=MAX_PARTS).contains(&t.child_count))
        .ok_or_else(|| invalid(tool, "lids", form))?;
    let mut out = Vec::new();
    let mut item = array.first_child;
    while let Some(at) = item {
        let bad = |what: &str| invalid(tool, &format!("lids[{}].{what}", out.len()), form);
        let mut lid = Lid {
            id: String::new(),
            pivot: super::Pivot::Center,
            closed_scale: 0.1,
        };
        for (name, value) in members(doc, at, 3).ok_or_else(|| bad("(object)"))? {
            match name.as_str() {
                "id" => lid.id = id_at(doc, value).ok_or_else(|| bad("id"))?,
                "pivot" => lid.pivot = pivot(doc, value).ok_or_else(|| bad("pivot"))?,
                "closedScale" => {
                    lid.closed_scale = number(doc, value)
                        .filter(|v| (0.01..=1.0).contains(v))
                        .ok_or_else(|| bad("closedScale"))?
                }
                _ => return Err(bad("(unknown member)")),
            }
        }
        if lid.id.is_empty() {
            return Err(bad("id"));
        }
        out.push(lid);
        item = doc.token(at).and_then(|t| t.next_sibling);
    }
    Ok(out)
}

fn when(doc: &JsonDocument<'_>, args: usize, tool: &str) -> Result<When, PreviewArgumentError> {
    let form =
        "exactly one of at (1-256 start ticks) or random {seed?, start?, end?, minGap?, maxGap?}";
    match (doc.object_get(args, b"at"), doc.object_get(args, b"random")) {
        (Some(at), None) => {
            let bad = || invalid(tool, "at", form);
            let array = doc
                .token(at)
                .filter(|t| t.kind == JsonType::Array && (1..=MAX_BLINKS).contains(&t.child_count))
                .ok_or_else(bad)?;
            let mut ticks = Vec::new();
            let mut item = array.first_child;
            while let Some(tick) = item {
                ticks.push(uint32(doc, tick).ok_or_else(bad)?);
                item = doc.token(tick).and_then(|t| t.next_sibling);
            }
            Ok(When::At(ticks))
        }
        (None, Some(at)) => {
            let bad = || invalid(tool, "random", form);
            let mut random = When::Random {
                seed: 0,
                start: 0,
                end: None,
                min_gap: 24,
                max_gap: 96,
            };
            let When::Random {
                seed,
                start,
                end,
                min_gap,
                max_gap,
            } = &mut random
            else {
                unreachable!()
            };
            for (name, value) in members(doc, at, 5).ok_or_else(bad)? {
                let v = uint32(doc, value).ok_or_else(bad)?;
                match name.as_str() {
                    "seed" => *seed = v,
                    "start" => *start = v,
                    "end" => *end = Some(v),
                    "minGap" => *min_gap = v,
                    "maxGap" => *max_gap = v,
                    _ => return Err(bad()),
                }
            }
            if *min_gap == 0 || *min_gap > *max_gap || *max_gap > 1_000_000 {
                return Err(bad());
            }
            Ok(random)
        }
        _ => Err(invalid(tool, "at", form)),
    }
}

fn gaze(doc: &JsonDocument<'_>, at: usize, tool: &str) -> Result<Gaze, PreviewArgumentError> {
    let form = "{pupils: 1-16 <g> group ids, keys: 1-256 objects {tick, x, y} with offsets within +-1000000}";
    let bad = |what: &str| invalid(tool, &format!("gaze.{what}"), form);
    let mut pupils = None;
    let mut keys = Vec::new();
    for (name, value) in members(doc, at, 2).ok_or_else(|| bad("(object)"))? {
        match name.as_str() {
            "pupils" => pupils = Some(id_list(doc, value, MAX_PARTS).ok_or_else(|| bad("pupils"))?),
            "keys" => {
                let array = doc
                    .token(value)
                    .filter(|t| {
                        t.kind == JsonType::Array && (1..=MAX_GAZE_KEYS).contains(&t.child_count)
                    })
                    .ok_or_else(|| bad("keys"))?;
                let mut item = array.first_child;
                while let Some(key) = item {
                    let (mut tick, mut x, mut y) = (None, None, None);
                    for (field, v) in members(doc, key, 3).ok_or_else(|| bad("keys"))? {
                        let offset = || number(doc, v).filter(|n| n.abs() <= 1.0e6);
                        match field.as_str() {
                            "tick" => tick = uint32(doc, v),
                            "x" => x = offset(),
                            "y" => y = offset(),
                            _ => return Err(bad("keys")),
                        }
                    }
                    keys.push((
                        tick.ok_or_else(|| bad("keys.tick"))?,
                        [
                            x.ok_or_else(|| bad("keys.x"))?,
                            y.ok_or_else(|| bad("keys.y"))?,
                        ],
                    ));
                    item = doc.token(key).and_then(|t| t.next_sibling);
                }
            }
            _ => return Err(bad("(unknown member)")),
        }
    }
    Ok(Gaze {
        pupils: pupils.ok_or_else(|| bad("pupils"))?,
        keys: if keys.is_empty() {
            return Err(bad("keys"));
        } else {
            keys
        },
    })
}

fn blink(doc: &JsonDocument<'_>, args: usize, tool: &str) -> Result<FaceOp, PreviewArgumentError> {
    document(doc, args, tool)?;
    let lids = match doc.object_get(args, b"lids") {
        Some(at) => lids(doc, at, tool)?,
        None => Vec::new(),
    };
    let (mut open, mut closed) = (Vec::new(), Vec::new());
    if let Some(at) = doc.object_get(args, b"swap") {
        let form = "{open?: 1-16 <g> group ids shown between blinks, closed?: 1-16 <g> group ids shown during a blink}";
        let bad = || invalid(tool, "swap", form);
        for (name, value) in members(doc, at, 2).ok_or_else(bad)? {
            let ids = id_list(doc, value, MAX_PARTS).ok_or_else(bad)?;
            match name.as_str() {
                "open" => open = ids,
                "closed" => closed = ids,
                _ => return Err(bad()),
            }
        }
    }
    let gaze = match doc.object_get(args, b"gaze") {
        Some(at) => Some(gaze(doc, at, tool)?),
        None => None,
    };
    if lids.is_empty() && open.is_empty() && closed.is_empty() && gaze.is_none() {
        return Err(invalid(tool, "lids", "lids, swap or gaze (at least one)"));
    }
    let duration = match doc.object_get(args, b"duration") {
        None => 4,
        Some(at) => uint32(doc, at)
            .filter(|d| (2..=MAX_DURATION).contains(d))
            .ok_or_else(|| invalid(tool, "duration", "an integer of ticks, 2-48"))?,
    };
    let animated = !(lids.is_empty() && open.is_empty() && closed.is_empty());
    let when = if animated
        || doc.object_get(args, b"at").is_some()
        || doc.object_get(args, b"random").is_some()
    {
        when(doc, args, tool)?
    } else {
        When::At(Vec::new())
    };
    Ok(FaceOp::Blink(Box::new(Blink {
        lids,
        open,
        closed,
        when,
        duration,
        gaze,
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
        "eyes_set" => eyes_set(doc, args, tool),
        "blink" => blink(doc, args, tool),
        _ => return None,
    })
}
