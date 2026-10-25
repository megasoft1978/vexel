//! Arguments of `audio_add_track`, `audio_set` and `audio_list`.

use super::super::add_extras::number;
use super::super::ext_structure::document;
use super::super::{JsonType, PreviewArgumentError};
use crate::session::mcp_audio::{AudioOp, DuckSpec, NewTrack, SetTrack};
use vexel_json::JsonDocument;
use vexel_timeline::audio::AudioKind;

pub(super) fn invalid(tool: &str, argument: &str, form: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!(
        "{tool} argument {argument} is missing or invalid; use {form}"
    ))
}

pub(super) fn text(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
    max: usize,
) -> Result<Option<String>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, key.as_bytes()) else {
        return Ok(None);
    };
    doc.decoded_string(token)
        .and_then(|b| String::from_utf8(b).ok())
        .filter(|s| !s.is_empty() && s.len() <= max)
        .map(Some)
        .ok_or_else(|| {
            invalid(
                tool,
                key,
                &format!("a non-empty UTF-8 string of at most {max} bytes"),
            )
        })
}

pub(super) fn required_text(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
    max: usize,
) -> Result<String, PreviewArgumentError> {
    text(doc, args, tool, key, max)?.ok_or_else(|| invalid(tool, key, "a string"))
}

pub(super) fn uint(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
) -> Result<Option<u32>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, key.as_bytes()) else {
        return Ok(None);
    };
    doc.uint(token)
        .and_then(|v| u32::try_from(v).ok())
        .map(Some)
        .ok_or_else(|| invalid(tool, key, "a non-negative integer"))
}

pub(super) fn float(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
) -> Result<Option<f64>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, key.as_bytes()) else {
        return Ok(None);
    };
    number(doc, token)
        .map(Some)
        .ok_or_else(|| invalid(tool, key, "a finite number"))
}

pub(super) fn flag(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
) -> Result<Option<bool>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, key.as_bytes()) else {
        return Ok(None);
    };
    match doc.token(token).map(|t| t.kind) {
        Some(JsonType::True) => Ok(Some(true)),
        Some(JsonType::False) => Ok(Some(false)),
        _ => Err(invalid(tool, key, "true or false")),
    }
}

pub(super) fn duck(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<Option<DuckSpec>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, b"duck") else {
        return Ok(None);
    };
    let form = "{under: a track id or voice|sfx|music, amountDb: -60..0}";
    let under = doc
        .object_get(token, b"under")
        .and_then(|t| doc.decoded_string(t))
        .and_then(|b| String::from_utf8(b).ok())
        .filter(|s| !s.is_empty() && s.len() <= 128);
    let amount = doc
        .object_get(token, b"amountDb")
        .and_then(|t| number(doc, t));
    match (under, amount) {
        (Some(under), Some(amount_db)) => Ok(Some(DuckSpec { under, amount_db })),
        _ => Err(invalid(tool, "duck", form)),
    }
}

/// `None` when `tool` is not an audio tool.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<AudioOp, PreviewArgumentError>> {
    if let Some(parsed) = super::audio_gen_args::parse(doc, args, tool) {
        return Some(parsed);
    }
    if !matches!(tool, "audio_add_track" | "audio_set" | "audio_list") {
        return None;
    }
    Some((|| {
        document(doc, args, tool)?;
        let id = |args| required_text(doc, args, tool, "trackId", 128);
        match tool {
            "audio_list" => Ok(AudioOp::List),
            "audio_add_track" => {
                let kind = required_text(doc, args, tool, "kind", 16)?;
                let kind = AudioKind::parse(kind.as_bytes())
                    .ok_or_else(|| invalid(tool, "kind", "voice, sfx or music"))?;
                Ok(AudioOp::Add(Box::new(NewTrack {
                    id: id(args)?,
                    kind,
                    path: required_text(doc, args, tool, "path", 1023)?,
                    start_tick: uint(doc, args, tool, "startTick")?.unwrap_or(0),
                    trim_in_ms: uint(doc, args, tool, "trimInMs")?.unwrap_or(0),
                    trim_out_ms: uint(doc, args, tool, "trimOutMs")?,
                    gain_db: float(doc, args, tool, "gainDb")?.unwrap_or(0.0),
                    fade_in_ticks: uint(doc, args, tool, "fadeInTicks")?.unwrap_or(0),
                    fade_out_ticks: uint(doc, args, tool, "fadeOutTicks")?.unwrap_or(0),
                    looped: flag(doc, args, tool, "loop")?.unwrap_or(false),
                    duck: duck(doc, args, tool)?,
                })))
            }
            _ => Ok(AudioOp::Set(Box::new(SetTrack {
                id: id(args)?,
                remove: flag(doc, args, tool, "remove")?.unwrap_or(false),
                gain_db: float(doc, args, tool, "gainDb")?,
                fade_in_ticks: uint(doc, args, tool, "fadeInTicks")?,
                fade_out_ticks: uint(doc, args, tool, "fadeOutTicks")?,
                start_tick: uint(doc, args, tool, "startTick")?,
                trim_in_ms: uint(doc, args, tool, "trimInMs")?,
                trim_out_ms: uint(doc, args, tool, "trimOutMs")?,
                looped: flag(doc, args, tool, "loop")?,
                duck: duck(doc, args, tool)?,
                clear_duck: flag(doc, args, tool, "clearDuck")?.unwrap_or(false),
            }))),
        }
    })())
}
