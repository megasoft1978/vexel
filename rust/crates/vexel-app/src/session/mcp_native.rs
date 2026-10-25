//! `native_timeline_*` MCP tools routed onto the live project session.
//!
//! Each edit is one journal transition through
//! [`AppDocument::edit_timeline`]; undo and redo use the shared project
//! history. Payload-only edits advance `state_revision()`, which the native
//! animation host observes, so the UI refreshes without extra wiring.

use crate::animation::native_drawing::SvgGeometry;
use crate::animation::native_properties::{element_tag, property_fits_tag};
use crate::preview::Operation;
use crate::project_session::{NativeCommandError, ProjectTimelineError, SessionError};
use crate::session::{AppDocument, McpOutcome};
use vexel_json::JsonDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, Error as TimelineError, LoopMode};

/// One key of a bulk `native_timeline_add_property_track` request.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyKey {
    pub tick: u32,
    pub value: Vec<u8>,
    pub ease: Option<Vec<u8>>,
}

/// One validated native timeline request.
#[derive(Clone, Debug, PartialEq)]
pub enum NativeOp {
    Get,
    AddTrack {
        track_id: Vec<u8>,
        target_id: Vec<u8>,
        asset_name: Vec<u8>,
        name: Vec<u8>,
    },
    AddDrawingAsset {
        asset_name: Vec<u8>,
        svg: Vec<u8>,
    },
    AddPropertyTrack {
        track_id: Vec<u8>,
        target_id: Vec<u8>,
        property: Vec<u8>,
        value_type: Vec<u8>,
        interpolation: Vec<u8>,
        value: Vec<u8>,
        name: Vec<u8>,
        /// Extra keys applied in the same edit as the track.
        keys: Vec<PropertyKey>,
    },
    SetPropertyKey {
        track_id: Vec<u8>,
        tick: u32,
        value: Vec<u8>,
        ease: Option<Vec<u8>>,
    },
    RemovePropertyKey {
        track_id: Vec<u8>,
        tick: u32,
    },
    MovePropertyKey {
        track_id: Vec<u8>,
        from_tick: u32,
        to_tick: u32,
    },
    RemoveTrack {
        track_id: Vec<u8>,
    },
    RenameTrack {
        track_id: Vec<u8>,
        name: Vec<u8>,
    },
    DuplicateTrack {
        track_id: Vec<u8>,
        new_track_id: Vec<u8>,
    },
    SetTrackInterpolation {
        track_id: Vec<u8>,
        interpolation: Vec<u8>,
    },
    ReorderTrack {
        track_id: Vec<u8>,
        index: usize,
    },
    SetCel {
        track_id: Vec<u8>,
        tick: u32,
        asset_name: Vec<u8>,
        label: Vec<u8>,
    },
    RemoveCel {
        track_id: Vec<u8>,
        tick: u32,
    },
    MoveItem {
        track_id: Vec<u8>,
        from_tick: u32,
        to_tick: u32,
    },
    SetLoop(LoopMode),
    SetDuration {
        duration_ticks: u32,
        ticks_per_second: Option<u32>,
    },
    Undo,
    Redo,
}

/// Parses the argument-free `native_timeline_*` tools (without the prefix).
/// Tools with arguments are parsed with specific messages in
/// `preview/native_args.rs`. Returns `None` for tools this route does not
/// handle.
pub(crate) fn parse(
    _doc: &JsonDocument<'_>,
    _args: usize,
    tool: &str,
) -> Option<Result<Operation, &'static str>> {
    let op = match tool {
        "get" => Ok(NativeOp::Get),
        "undo" => Ok(NativeOp::Undo),
        "redo" => Ok(NativeOp::Redo),
        _ => return None,
    };
    Some(op.map(Operation::Native))
}

/// Longest timeline `native_timeline_set_duration` accepts, in seconds.
const MAX_DURATION_SECONDS: u64 = 3600;

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
        NativeOp::SetDuration {
            duration_ticks,
            ticks_per_second,
        } => {
            let tps = match ticks_per_second {
                Some(tps) => tps,
                None => app
                    .native_timeline_candidate(settings(), limits)
                    .map_err(error)?
                    .view()
                    .map_err(|_| "unsupported native timeline")?
                    .ticks_per_second(),
            };
            if tps != 0 && u64::from(duration_ticks) > u64::from(tps) * MAX_DURATION_SECONDS {
                return Err(
                    "native_timeline_set_duration duration exceeds 3600 seconds; reduce durationTicks or raise ticksPerSecond (durationTicks / ticksPerSecond must be at most 3600)",
                );
            }
            app.edit_timeline(settings(), limits, |c| {
                c.set_timing(duration_ticks, ticks_per_second)
            })
            .map_err(|e| operation_error(e, "set_duration", Refs::default()))?
        }
        NativeOp::AddPropertyTrack {
            track_id,
            target_id,
            property,
            value_type,
            interpolation,
            value,
            name,
            keys,
        } => {
            if let Some(message) = property_target_mismatch(app, &target_id, &property) {
                return Err(message);
            }
            let in_keys = std::cell::Cell::new(false);
            app.edit_timeline(settings(), limits, |c| {
                let mut edited = c.add_property_track(
                    &track_id,
                    &target_id,
                    &property,
                    &value_type,
                    &interpolation,
                    &value,
                    &name,
                )?;
                in_keys.set(true);
                for key in &keys {
                    edited = edited.set_property_key_with_ease(
                        &track_id,
                        key.tick,
                        &key.value,
                        key.ease.as_deref(),
                    )?;
                }
                Ok(edited)
            })
            .map_err(|error| {
                if in_keys.get() {
                    return operation_error(error, "set_property_key", Refs::default());
                }
                let refs = Refs {
                    asset: true,
                    target: known_target(app, &target_id),
                };
                operation_error(error, "add_property_track", refs)
            })?
        }
        NativeOp::SetPropertyKey {
            track_id,
            tick,
            value,
            ease,
        } => app
            .edit_timeline(settings(), limits, |c| {
                c.set_property_key_with_ease(&track_id, tick, &value, ease.as_deref())
            })
            .map_err(|error| operation_error(error, "set_property_key", Refs::default()))?,
        NativeOp::AddTrack {
            track_id,
            target_id,
            asset_name,
            name,
        } => app
            .edit_timeline(settings(), limits, |c| {
                c.add_drawing_track(&track_id, &target_id, &asset_name, &name)
            })
            .map_err(|error| {
                let refs = Refs {
                    asset: known_asset(app, &asset_name),
                    target: known_target(app, &target_id),
                };
                operation_error(error, "add_track", refs)
            })?,
        NativeOp::AddDrawingAsset { asset_name, svg } => {
            let (width, height) = app.size();
            let geometry = SvgGeometry {
                width: f64::from(width),
                height: f64::from(height),
                view_box: [0.0, 0.0, f64::from(width), f64::from(height)],
            };
            app.edit_timeline(settings(), limits, |c| {
                c.add_drawing_asset(&asset_name, b"image/svg+xml", &svg, geometry)
            })
            .map_err(|error| asset_error(error, &svg))?
        }
        NativeOp::RemovePropertyKey { track_id, tick } => app
            .edit_timeline(settings(), limits, |c| {
                c.remove_property_key(&track_id, tick)
            })
            .map_err(|e| operation_error(e, "remove_key", Refs::default()))?,
        NativeOp::MovePropertyKey {
            track_id,
            from_tick,
            to_tick,
        } => app
            .edit_timeline(settings(), limits, |c| {
                c.move_property_key(&track_id, from_tick, to_tick)
            })
            .map_err(|e| operation_error(e, "move_key", Refs::default()))?,
        NativeOp::RemoveTrack { track_id } => app
            .edit_timeline(settings(), limits, |c| c.remove_track(&track_id))
            .map_err(|e| operation_error(e, "remove_track", Refs::default()))?,
        NativeOp::RenameTrack { track_id, name } => app
            .edit_timeline(settings(), limits, |c| c.rename_track(&track_id, &name))
            .map_err(|e| operation_error(e, "rename_track", Refs::default()))?,
        NativeOp::DuplicateTrack {
            track_id,
            new_track_id,
        } => app
            .edit_timeline(settings(), limits, |c| {
                c.duplicate_track(&track_id, &new_track_id)
            })
            .map_err(|e| operation_error(e, "duplicate_track", Refs::default()))?,
        NativeOp::SetTrackInterpolation {
            track_id,
            interpolation,
        } => app
            .edit_timeline(settings(), limits, |c| {
                c.set_track_interpolation(&track_id, &interpolation)
            })
            .map_err(|e| operation_error(e, "set_interpolation", Refs::default()))?,
        NativeOp::ReorderTrack { track_id, index } => app
            .edit_timeline(settings(), limits, |c| c.reorder_track(&track_id, index))
            .map_err(|e| operation_error(e, "reorder_track", Refs::default()))?,
        NativeOp::SetCel {
            track_id,
            tick,
            asset_name,
            label,
        } => app
            .edit_timeline(settings(), limits, |c| {
                c.set_cel(&track_id, tick, &asset_name, &label)
            })
            .map_err(|error| {
                let refs = Refs {
                    asset: known_asset(app, &asset_name),
                    target: true,
                };
                operation_error(error, "set_cel", refs)
            })?,
        NativeOp::RemoveCel { track_id, tick } => app
            .edit_timeline(settings(), limits, |c| c.remove_cel(&track_id, tick))
            .map_err(|error| operation_error(error, "remove_cel", Refs::default()))?,
        NativeOp::MoveItem {
            track_id,
            from_tick,
            to_tick,
        } => app
            .edit_timeline(settings(), limits, |c| {
                c.move_item(&track_id, from_tick, to_tick)
            })
            .map_err(|error| operation_error(error, "move_item", Refs::default()))?,
    };
    Ok(done(if edited {
        "Timeline updated"
    } else {
        "Timeline unchanged"
    }))
}

/// What a failed reference check can still be narrowed to.
#[derive(Clone, Copy)]
struct Refs {
    asset: bool,
    target: bool,
}

impl Default for Refs {
    fn default() -> Self {
        Self {
            asset: true,
            target: true,
        }
    }
}

fn known_asset(app: &AppDocument, name: &[u8]) -> bool {
    app.native_timeline_candidate(settings(), EditLimits::default())
        .map(|c| c.assets().iter().any(|asset| asset.name == name))
        .unwrap_or(true)
}

/// A geometry property the target's element type does not have, which would
/// make every later render fail.
fn property_target_mismatch(
    app: &AppDocument,
    target_id: &[u8],
    property: &[u8],
) -> Option<&'static str> {
    let source = app.source_bytes().ok()?;
    let tag = element_tag(&source, target_id)?;
    if property_fits_tag(&tag, property) {
        return None;
    }
    Some(match property {
        b"d" => "native_timeline_add_property_track property d morphs path data and needs a <path> target (polygon and polyline use points)",
        b"points" => "native_timeline_add_property_track property points needs a <polygon> or <polyline> target (paths use d)",
        b"stdDeviation" => "native_timeline_add_property_track property stdDeviation needs the id of a feGaussianBlur or feDropShadow filter primitive",
        b"dx" | b"dy" => "native_timeline_add_property_track properties dx and dy need the id of a feOffset or feDropShadow filter primitive",
        b"flood-opacity" => "native_timeline_add_property_track property flood-opacity needs the id of a feFlood or feDropShadow filter primitive",
        b"values" => "native_timeline_add_property_track property values needs the id of a feColorMatrix filter primitive",
        _ => "native_timeline_add_property_track property is a geometry attribute (x, y, width, height, cx, cy, r, rx, ry) that the target element type does not have; paths, lines, polygons and groups move with a transform track (valueType trs), circles use cx cy r, ellipses cx cy rx ry, rects x y width height rx ry",
    })
}

fn known_target(app: &AppDocument, id: &[u8]) -> bool {
    let Ok(source) = app.source_bytes() else {
        return true;
    };
    [b'"', b'\''].iter().any(|quote| {
        let needle = [b"id=".as_slice(), &[*quote], id, &[*quote]].concat();
        source.windows(needle.len()).any(|part| part == needle)
    })
}

fn edit_message(operation: &str, kind: TimelineError, refs: Refs) -> &'static str {
    match (operation, kind) {
        ("add_track", TimelineError::InvalidReference) if !refs.asset => {
            "native_timeline_add_track assetName is not an existing asset; add it first with native_timeline_add_drawing_asset"
        }
        ("add_track", TimelineError::InvalidReference) if !refs.target => {
            "native_timeline_add_track targetId is not an element id in the document; use an id such as vexel-object-1 (the elementId returned by element_add)"
        }
        ("add_track", TimelineError::InvalidReference) => {
            "native_timeline_add_track targetId or assetName is invalid; use an existing element id and an asset added with native_timeline_add_drawing_asset"
        }
        ("add_track", TimelineError::Duplicate) => {
            "native_timeline_add_track trackId already exists; use a unique 1-128 character ID of ASCII letters, digits, `_` or `-`"
        }
        ("add_track", _) => {
            "native_timeline_add_track trackId must be 1-128 ASCII letters, digits, `_` or `-` and name at most 256 UTF-8 bytes"
        }
        ("set_cel", TimelineError::InvalidReference) if !refs.asset => {
            "native_timeline_set_cel assetName is not an existing asset; add it first with native_timeline_add_drawing_asset"
        }
        ("set_cel", TimelineError::InvalidReference) => {
            "native_timeline_set_cel trackId is not an existing track; create a drawing track with native_timeline_add_track"
        }
        ("set_cel", TimelineError::InvalidArgument) => {
            "native_timeline_set_cel trackId must be a drawing track (property tracks use set_property_key) and label at most 256 UTF-8 bytes"
        }
        ("set_cel", TimelineError::InvalidValue) => {
            "native_timeline_set_cel tick must be an integer from 0 through durationTicks-1 (0-47 for the default 48-tick timeline; lengthen with native_timeline_set_duration)"
        }
        ("remove_cel", TimelineError::InvalidReference) => {
            "native_timeline_remove_cel trackId is not a track or tick has no cel; read cel ticks with native_timeline_get"
        }
        ("remove_cel", TimelineError::InvalidArgument) => {
            "native_timeline_remove_cel trackId must be a drawing track"
        }
        ("remove_cel", TimelineError::InvalidValue) => {
            "native_timeline_remove_cel cannot remove the last cel of a track; add another cel first"
        }
        ("move_item", TimelineError::InvalidReference) => {
            "native_timeline_move_item trackId is not a track or fromTick has no item; read item ticks with native_timeline_get"
        }
        ("move_item", TimelineError::Duplicate) => {
            "native_timeline_move_item toTick is occupied; use an unused tick"
        }
        ("move_item", TimelineError::InvalidValue) => {
            "native_timeline_move_item fromTick 0 cannot be moved, and toTick must be an integer from 0 through durationTicks-1 (0-47 for the default 48-tick timeline; lengthen with native_timeline_set_duration)"
        }
        ("add_property_track", TimelineError::InvalidReference) if !refs.target => {
            "native_timeline_add_property_track targetId is not an element id in the document; use an id such as vexel-object-1 (the elementId returned by element_add)"
        }
        ("add_property_track", TimelineError::Duplicate) => {
            "native_timeline_add_property_track trackId already exists; use a unique 1-128 character ID of ASCII letters, digits, `_` or `-`"
        }
        ("add_property_track", _) => {
            "native_timeline_add_property_track rejected: trackId must be ASCII letters, digits, `_` or `-`; property/valueType pairs are x,y,width,height,cx,cy,r,rx,ry,opacity,fill-opacity,stroke-opacity,stroke-width,stroke-dashoffset, stdDeviation (0-32), dx, dy, flood-opacity, values (hueRotate or saturate feColorMatrix), offset, stop-opacity with scalar; d or points with path (identical command structure in every key, arcs unsupported); fill, stroke or stop-color with rgba (4 numbers 0-1); transform with transform2d (6 numbers); visibility with boolean (interpolation step); interpolation is step, linear or cubic"
        }
        ("set_property_key", TimelineError::InvalidReference) => {
            "native_timeline_set_property_key trackId is not an existing track"
        }
        ("set_property_key", TimelineError::InvalidArgument) => {
            "native_timeline_set_property_key trackId must be a property track (drawing tracks use set_cel)"
        }
        ("set_property_key", _) => {
            "native_timeline_set_property_key tick must be an integer from 0 through durationTicks-1 (0-47 for the default 48-tick timeline; lengthen with native_timeline_set_duration) and value must match the track valueType (a d or points track needs the same path commands as its base path, arcs unsupported)"
        }
        ("remove_key", TimelineError::InvalidReference) => {
            "native_timeline_remove_key trackId must identify a property track and tick must identify an existing key"
        }
        ("remove_key", _) => {
            "native_timeline_remove_key tick zero cannot be removed; tick must identify an existing nonzero key"
        }
        ("move_key", TimelineError::InvalidReference) => {
            "native_timeline_move_key trackId and fromTick must identify an existing property key"
        }
        ("move_key", TimelineError::Duplicate) => {
            "native_timeline_move_key toTick is occupied; use an unused tick"
        }
        ("move_key", _) => {
            "native_timeline_move_key fromTick zero cannot be moved and toTick must be within durationTicks"
        }
        ("remove_track", _) => {
            "native_timeline_remove_track trackId must identify an existing track"
        }
        ("rename_track", _) => {
            "native_timeline_rename_track trackId must exist and name must be valid UTF-8 of at most 4096 bytes"
        }
        ("duplicate_track", TimelineError::Duplicate) => {
            "native_timeline_duplicate_track newTrackId already exists; use a unique track ID"
        }
        ("duplicate_track", _) => {
            "native_timeline_duplicate_track trackId must exist and newTrackId must be 1-128 ASCII letters, digits, `_` or `-`"
        }
        ("set_interpolation", _) => {
            "native_timeline_set_interpolation trackId must identify a property track and interpolation must be step, linear or cubic"
        }
        ("set_duration", _) => {
            "native_timeline_set_duration durationTicks must be 1-14400000 and greater than the last key or cel tick (see native_timeline_get), and ticksPerSecond 1-240"
        }
        ("reorder_track", _) => {
            "native_timeline_reorder_track trackId must exist and index must be within the track list"
        }
        (_, TimelineError::Limit) => "native timeline limit reached; reduce timeline or asset size",
        (_, _) => "",
    }
}

/// The timeline error of a failed edit: closures report it as
/// `Operation`, candidate construction as `Timeline`.
fn inner(failure: &NativeCommandError<ProjectTimelineError>) -> Option<&ProjectTimelineError> {
    match failure {
        NativeCommandError::Operation(e) | NativeCommandError::Timeline(e) => Some(e),
        _ => None,
    }
}

fn operation_error(
    failure: NativeCommandError<ProjectTimelineError>,
    operation: &str,
    refs: Refs,
) -> &'static str {
    match inner(&failure) {
        Some(ProjectTimelineError::Edit(edit)) => {
            let msg = edit_message(operation, edit.error, refs);
            if !msg.is_empty() {
                return msg;
            }
        }
        Some(ProjectTimelineError::Limit) => {
            return "native timeline limit reached; reduce timeline or asset size";
        }
        Some(ProjectTimelineError::OutOfMemory) => return "preview allocation failed",
        Some(ProjectTimelineError::UnsupportedTimeline) => return "unsupported native timeline",
        _ => {}
    }
    error(failure)
}

fn asset_error(failure: NativeCommandError<ProjectTimelineError>, svg: &[u8]) -> &'static str {
    use crate::animation::native_drawing::DrawingError;
    use crate::project_session::SvgAssetError;
    let Some(ProjectTimelineError::Asset(asset)) = inner(&failure) else {
        return operation_error(failure, "add_drawing_asset", Refs::default());
    };
    match asset {
        SvgAssetError::InvalidName => {
            "native_timeline_add_drawing_asset assetName must be 1-128 ASCII letters, digits, `.`, `_` or `-`, and not `.` or `..`"
        }
        SvgAssetError::InvalidMediaType => {
            "native_timeline_add_drawing_asset only accepts SVG (image/svg+xml)"
        }
        SvgAssetError::DuplicateName => {
            "native_timeline_add_drawing_asset assetName already exists; use a unique SVG asset name"
        }
        SvgAssetError::Limit => {
            "native_timeline_add_drawing_asset asset count or size limit reached; reduce or reuse assets"
        }
        SvgAssetError::OutOfMemory => "preview allocation failed",
        SvgAssetError::Drawing(DrawingError::GeometryMismatch) => super::native_svg::GEOMETRY,
        SvgAssetError::Drawing(DrawingError::InvalidSvg | DrawingError::UnsafeSvg) => {
            super::native_svg::reason(svg)
        }
        SvgAssetError::Drawing(_) => {
            "native_timeline_add_drawing_asset svg is invalid; send a self-contained SVG whose root width, height and viewBox equal the document size"
        }
    }
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
    let timeline = candidate.timeline_bytes();
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
                if let Some(raw) = timeline.get(item.source.start..item.source.end) {
                    if let Ok(key) = JsonDocument::parse(raw) {
                        if let Some(ease) = key
                            .object_get(key.root(), b"ease")
                            .and_then(|at| key.raw_token(at))
                        {
                            out.push_str(",\"ease\":");
                            out.push_str(&String::from_utf8_lossy(ease));
                        }
                    }
                }
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
