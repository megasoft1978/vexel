//! Arguments of `audio_generate` and `audio_envelope`.

use super::super::ext_structure::document;
use super::super::{JsonType, PreviewArgumentError};
use super::audio_args::{duck, flag, float, invalid, required_text, text, uint};
use crate::session::mcp_audio::{AudioOp, NewTrack, json_string};
use crate::session::mcp_audio_gen::{EnvSource, EnvSpec, GenSpec, MAX_PARAMS, Param};
use vexel_json::JsonDocument;
use vexel_timeline::audio::AudioKind;

const PARAM_FORM: &str =
    "an object of at most 24 members with letters/digits keys and string, number or boolean values";

fn params(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<Vec<Param>, PreviewArgumentError> {
    let Some(object) = doc.object_get(args, b"params") else {
        return Ok(Vec::new());
    };
    let bad = || invalid(tool, "params", PARAM_FORM);
    let object = doc
        .token(object)
        .filter(|t| t.kind == JsonType::Object && t.child_count <= MAX_PARAMS * 2)
        .ok_or_else(bad)?;
    let mut out = Vec::new();
    let mut key = object.first_child;
    while let Some(at) = key {
        let value = doc.token(at).and_then(|t| t.next_sibling).ok_or_else(bad)?;
        let name = doc
            .decoded_string(at)
            .and_then(|b| String::from_utf8(b).ok())
            .filter(|k| {
                !k.is_empty() && k.len() <= 32 && k.bytes().all(|b| b.is_ascii_alphanumeric())
            })
            .ok_or_else(bad)?;
        let json = match doc.token(value).map(|t| t.kind) {
            Some(JsonType::String) => doc
                .decoded_string(value)
                .and_then(|b| String::from_utf8(b).ok())
                .filter(|s| s.len() <= 1000)
                .map(|s| json_string(&s))
                .ok_or_else(bad)?,
            Some(JsonType::Number) => doc
                .raw_token(value)
                .filter(|r| r.len() <= 32)
                .and_then(|r| std::str::from_utf8(r).ok())
                .filter(|r| r.parse::<f64>().is_ok_and(f64::is_finite))
                .map(str::to_owned)
                .ok_or_else(bad)?,
            Some(JsonType::True) => "true".to_owned(),
            Some(JsonType::False) => "false".to_owned(),
            _ => return Err(bad()),
        };
        if out.len() >= MAX_PARAMS || out.iter().any(|p: &Param| p.key == name) {
            return Err(bad());
        }
        out.push(Param { key: name, json });
        key = doc.token(value).and_then(|t| t.next_sibling);
    }
    Ok(out)
}

fn add_track(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    kind: AudioKind,
) -> Result<Option<Box<NewTrack>>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, b"addTrack") else {
        return Ok(None);
    };
    if doc.token(token).map(|t| t.kind) != Some(JsonType::Object) {
        return Err(invalid(tool, "addTrack", "an object with a trackId"));
    }
    Ok(Some(Box::new(NewTrack {
        id: required_text(doc, token, tool, "trackId", 128)?,
        kind,
        path: String::new(),
        start_tick: uint(doc, token, tool, "startTick")?.unwrap_or(0),
        trim_in_ms: uint(doc, token, tool, "trimInMs")?.unwrap_or(0),
        trim_out_ms: uint(doc, token, tool, "trimOutMs")?,
        gain_db: float(doc, token, tool, "gainDb")?.unwrap_or(0.0),
        fade_in_ticks: uint(doc, token, tool, "fadeInTicks")?.unwrap_or(0),
        fade_out_ticks: uint(doc, token, tool, "fadeOutTicks")?.unwrap_or(0),
        looped: flag(doc, token, tool, "loop")?.unwrap_or(false),
        duck: duck(doc, token, tool)?,
    })))
}

/// `None` when `tool` is not one of the two generation tools.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<AudioOp, PreviewArgumentError>> {
    if !matches!(tool, "audio_generate" | "audio_envelope") {
        return None;
    }
    Some((|| {
        document(doc, args, tool)?;
        if tool == "audio_envelope" {
            let path = text(doc, args, tool, "path", 1023)?;
            let track = text(doc, args, tool, "trackId", 128)?;
            let source = match (path, track) {
                (Some(p), None) => EnvSource::Path(p),
                (None, Some(t)) => EnvSource::Track(t),
                _ => return Err(invalid(tool, "path", "exactly one of path or trackId")),
            };
            return Ok(AudioOp::Envelope(Box::new(EnvSpec {
                source,
                ticks: uint(doc, args, tool, "ticks")?,
            })));
        }
        let kind = required_text(doc, args, tool, "kind", 16)?;
        let kind = AudioKind::parse(kind.as_bytes())
            .ok_or_else(|| invalid(tool, "kind", "voice, sfx or music"))?;
        Ok(AudioOp::Generate(Box::new(GenSpec {
            kind,
            params: params(doc, args, tool)?,
            out_path: required_text(doc, args, tool, "outPath", 1023)?,
            consent: flag(doc, args, tool, "consent")?.unwrap_or(false),
            add: add_track(doc, args, tool, kind)?,
        })))
    })())
}
