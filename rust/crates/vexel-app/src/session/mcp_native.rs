//! `native_timeline_*` MCP tools routed onto the live project session.
//!
//! Each edit is one journal transition through
//! [`AppDocument::edit_timeline`]; undo and redo use the shared project
//! history. Payload-only edits advance `state_revision()`, which the native
//! animation host observes, so the UI refreshes without extra wiring.

use crate::preview::Operation;
use crate::project_session::{NativeCommandError, ProjectTimelineError, SessionError};
use crate::session::{AppDocument, McpOutcome};
use vexel_json::{JsonDocument, JsonType};
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

const BAD_ARGS: &str = "invalid preview tool arguments";
const ID_LIMIT: usize = 256;
const VALUE_LIMIT: usize = 4096;

/// One validated native timeline request.
#[derive(Clone, Debug, PartialEq)]
pub enum NativeOp {
    Get,
    AddPropertyTrack {
        track_id: Vec<u8>,
        target_id: Vec<u8>,
        property: Vec<u8>,
        value_type: Vec<u8>,
        interpolation: Vec<u8>,
        value: Vec<u8>,
        name: Vec<u8>,
    },
    SetPropertyKey {
        track_id: Vec<u8>,
        tick: u32,
        value: Vec<u8>,
    },
    SetLoop(LoopMode),
    Undo,
    Redo,
}

fn text(
    doc: &JsonDocument<'_>,
    args: usize,
    key: &[u8],
    min: usize,
) -> Result<Vec<u8>, &'static str> {
    let token = doc.object_get(args, key).ok_or(BAD_ARGS)?;
    if doc.raw_token(token).ok_or(BAD_ARGS)?.len() > ID_LIMIT * 6 + 2 {
        return Err(BAD_ARGS);
    }
    let bytes = doc.decoded_string(token).ok_or(BAD_ARGS)?;
    if bytes.len() < min || bytes.len() > ID_LIMIT || std::str::from_utf8(&bytes).is_err() {
        return Err(BAD_ARGS);
    }
    Ok(bytes)
}

fn value(doc: &JsonDocument<'_>, args: usize) -> Result<Vec<u8>, &'static str> {
    let token = doc.object_get(args, b"value").ok_or(BAD_ARGS)?;
    let kind = doc.token(token).ok_or(BAD_ARGS)?.kind;
    if !matches!(
        kind,
        JsonType::Number | JsonType::True | JsonType::False | JsonType::Array
    ) {
        return Err(BAD_ARGS);
    }
    let raw = doc.raw_token(token).ok_or(BAD_ARGS)?;
    if raw.len() > VALUE_LIMIT {
        return Err(BAD_ARGS);
    }
    Ok(raw.to_vec())
}

/// Validates the arguments of one `native_timeline_*` tool (without the
/// prefix). Returns `None` for tools this route does not handle.
pub(crate) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Operation, &'static str>> {
    let op = match tool {
        "get" => Ok(NativeOp::Get),
        "undo" => Ok(NativeOp::Undo),
        "redo" => Ok(NativeOp::Redo),
        "set_loop" => text(doc, args, b"mode", 1).and_then(|mode| match mode.as_slice() {
            b"once" => Ok(NativeOp::SetLoop(LoopMode::Once)),
            b"loop" => Ok(NativeOp::SetLoop(LoopMode::Repeat)),
            b"ping_pong" => Ok(NativeOp::SetLoop(LoopMode::PingPong)),
            _ => Err(BAD_ARGS),
        }),
        "add_property_track" => (|| {
            Ok(NativeOp::AddPropertyTrack {
                track_id: text(doc, args, b"trackId", 1)?,
                target_id: text(doc, args, b"targetId", 1)?,
                property: text(doc, args, b"property", 1)?,
                value_type: text(doc, args, b"valueType", 1)?,
                interpolation: text(doc, args, b"interpolation", 1)?,
                value: value(doc, args)?,
                name: if doc.object_get(args, b"name").is_some() {
                    text(doc, args, b"name", 0)?
                } else {
                    Vec::new()
                },
            })
        })(),
        "set_property_key" => (|| {
            let tick = doc
                .object_get(args, b"tick")
                .and_then(|t| doc.uint(t))
                .and_then(|t| u32::try_from(t).ok())
                .ok_or(BAD_ARGS)?;
            Ok(NativeOp::SetPropertyKey {
                track_id: text(doc, args, b"trackId", 1)?,
                tick,
                value: value(doc, args)?,
            })
        })(),
        _ => return None,
    };
    Some(op.map(Operation::Native))
}

fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 48,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Repeat,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    }
}

fn error<E>(error: NativeCommandError<E>) -> &'static str {
    match error {
        NativeCommandError::Session(SessionError::HistoryUnavailable) => {
            "project history unavailable"
        }
        NativeCommandError::Timeline(ProjectTimelineError::UnsupportedTimeline) => {
            "unsupported native timeline"
        }
        NativeCommandError::Timeline(ProjectTimelineError::Limit) => {
            "native timeline limit reached"
        }
        NativeCommandError::Timeline(ProjectTimelineError::OutOfMemory) => {
            "preview allocation failed"
        }
        NativeCommandError::Operation(_) | NativeCommandError::Timeline(_) => {
            "native timeline edit rejected"
        }
        _ => "native timeline unavailable",
    }
}

fn history_error(error: SessionError) -> &'static str {
    match error {
        SessionError::HistoryUnavailable => "project history unavailable",
        _ => "file operation failed",
    }
}

pub(crate) fn apply(app: &mut AppDocument, op: NativeOp) -> Result<McpOutcome, &'static str> {
    let limits = EditLimits::default();
    let edited = match op {
        NativeOp::Get => return get(app).map(McpOutcome::Reply),
        NativeOp::Undo => {
            if !app.can_undo() {
                return Err("nothing to undo");
            }
            app.undo().map_err(history_error)?;
            return Ok(done("Undo complete"));
        }
        NativeOp::Redo => {
            if !app.can_redo() {
                return Err("nothing to redo");
            }
            app.redo().map_err(history_error)?;
            return Ok(done("Redo complete"));
        }
        NativeOp::SetLoop(mode) => app
            .edit_timeline(settings(), limits, |c| c.set_loop(mode))
            .map_err(error)?,
        NativeOp::AddPropertyTrack {
            track_id,
            target_id,
            property,
            value_type,
            interpolation,
            value,
            name,
        } => app
            .edit_timeline(settings(), limits, |c| {
                c.add_property_track(
                    &track_id,
                    &target_id,
                    &property,
                    &value_type,
                    &interpolation,
                    &value,
                    &name,
                )
            })
            .map_err(error)?,
        NativeOp::SetPropertyKey {
            track_id,
            tick,
            value,
        } => app
            .edit_timeline(settings(), limits, |c| {
                c.set_property_key(&track_id, tick, &value)
            })
            .map_err(error)?,
    };
    Ok(done(if edited {
        "Timeline updated"
    } else {
        "Timeline unchanged"
    }))
}

fn done(message: &str) -> McpOutcome {
    McpOutcome::Reply(format!("{{\"ok\":true,\"message\":\"{message}\"}}"))
}

fn esc(out: &mut String, bytes: &[u8]) {
    out.push('"');
    for c in String::from_utf8_lossy(bytes).chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn opt(out: &mut String, key: &str, bytes: Option<&[u8]>) {
    if let Some(bytes) = bytes {
        out.push_str(&format!(",\"{key}\":"));
        esc(out, bytes);
    }
}

fn get(app: &AppDocument) -> Result<String, &'static str> {
    let candidate = app
        .native_timeline_candidate(settings(), EditLimits::default())
        .map_err(error)?;
    let view = candidate
        .view()
        .map_err(|_| "unsupported native timeline")?;
    let (num, den) = view.fps();
    let mode = match view.loop_mode() {
        LoopMode::Once => "once",
        LoopMode::Repeat => "loop",
        LoopMode::PingPong => "ping_pong",
    };
    let mut out = format!(
        "{{\"ticksPerSecond\":{},\"durationTicks\":{},\"fps\":{{\"numerator\":{num},\"denominator\":{den}}},\"loop\":\"{mode}\",\"tracks\":[",
        view.ticks_per_second(),
        view.duration_ticks()
    );
    for index in 0..view.track_count() {
        let track = view.track_at(index).ok_or("unsupported native timeline")?;
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"id\":");
        esc(&mut out, track.id);
        out.push_str(",\"kind\":");
        esc(&mut out, track.kind);
        out.push_str(",\"target\":");
        esc(&mut out, track.target);
        opt(&mut out, "name", track.name);
        opt(&mut out, "property", track.property);
        opt(&mut out, "valueType", track.value_type);
        opt(&mut out, "interpolation", track.interpolation);
        out.push_str(&format!(",\"visible\":{},\"items\":[", track.visible));
        for item_index in 0..track.item_count {
            let item = view
                .item_at(index, item_index)
                .ok_or("unsupported native timeline")?;
            if item_index > 0 {
                out.push(',');
            }
            out.push_str(&format!("{{\"tick\":{}", item.tick));
            opt(&mut out, "asset", item.asset);
            if let Some(v) = item.value_json {
                out.push_str(",\"value\":");
                out.push_str(&String::from_utf8_lossy(v));
            }
            out.push('}');
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    Ok(out)
}

#[cfg(test)]
#[path = "tests_mcp_native.rs"]
mod tests;
