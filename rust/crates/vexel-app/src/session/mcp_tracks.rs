//! Property tracks written in bulk: one splice and one validation for a track
//! with all its keys (`native_timeline_add_property_track` with `keys`) and for
//! many tracks (`native_timeline_tracks_set`), instead of one full timeline
//! decode per track and per key. A rejected fast path leaves the project
//! untouched; a single track then replays the sequential edits so its error
//! message is exactly the one those tools always gave.

use super::{
    NativeTrack, Refs, edit_message, history_error, known_target, mismatch_message,
    operation_error, settings,
};
use crate::animation::native_properties::{element_tags, property_fits_tag};
use crate::project_session::{NativeCommandError, ProjectTimelineError};
use crate::session::{AppDocument, McpOutcome};
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Mutex;
use vexel_timeline::{BulkFailure, BulkKey, BulkTrack, EditLimits};

type Failed = (
    Option<BulkFailure>,
    NativeCommandError<ProjectTimelineError>,
);

fn key_sets(tracks: &[NativeTrack]) -> Vec<Vec<BulkKey<'_>>> {
    tracks
        .iter()
        .map(|t| {
            t.keys
                .iter()
                .map(|k| BulkKey {
                    tick: k.tick,
                    value: &k.value,
                    ease: k.ease.as_deref(),
                })
                .collect()
        })
        .collect()
}

/// Adds `tracks` in one edit. On rejection nothing changed.
fn add_many(app: &mut AppDocument, tracks: &[NativeTrack]) -> Result<bool, Failed> {
    let keys = key_sets(tracks);
    let bulk: Vec<BulkTrack<'_>> = tracks
        .iter()
        .zip(&keys)
        .map(|(t, keys)| BulkTrack {
            id: &t.track_id,
            target: &t.target_id,
            property: &t.property,
            value_type: &t.value_type,
            interpolation: &t.interpolation,
            value: &t.value,
            name: &t.name,
            noise: t.noise.as_deref(),
            keys,
        })
        .collect();
    let failure = Cell::new(None);
    app.edit_timeline(settings(), EditLimits::default(), |c| {
        c.add_property_tracks(&bulk, &failure)
    })
    .map_err(|e| (failure.get(), e))
}

/// One track and its keys as a single edit; falls back to the sequential edits
/// (one per key) when rejected, for their exact error messages.
pub(super) fn add_one(app: &mut AppDocument, track: &NativeTrack) -> Result<bool, &'static str> {
    let fast = add_many(app, std::slice::from_ref(track));
    if fast.as_ref().is_ok_and(|edited| *edited) {
        return Ok(true);
    }
    if track.noise.is_some() {
        // The sequential replay below cannot carry `noise`; report the bulk rejection.
        return fast.map_err(rejection);
    }
    let in_keys = Cell::new(false);
    app.edit_timeline(settings(), EditLimits::default(), |c| {
        let mut edited = c.add_property_track(
            &track.track_id,
            &track.target_id,
            &track.property,
            &track.value_type,
            &track.interpolation,
            &track.value,
            &track.name,
        )?;
        in_keys.set(true);
        for key in &track.keys {
            edited = edited.set_property_key_with_ease(
                &track.track_id,
                key.tick,
                &key.value,
                key.ease.as_deref(),
            )?;
        }
        Ok(edited)
    })
    .map_err(|failure| {
        if in_keys.get() {
            return operation_error(failure, "set_property_key", Refs::default());
        }
        let refs = Refs {
            asset: true,
            target: known_target(app, &track.target_id),
        };
        operation_error(failure, "add_property_track", refs)
    })
}

/// Most distinct indexed messages ever interned: 256 indices times
/// the handful of bulk error texts.
const MAX_INTERNED: usize = 8192;

/// `native_timeline_tracks_set tracks[index]: message` as a `'static` string. Errors travel as
/// `&'static str`; the index is below 256 and the texts come from a fixed set,
/// so the interned table is bounded (and stops growing at `MAX_INTERNED`).
fn indexed(index: usize, message: &'static str) -> &'static str {
    static TABLE: Mutex<Option<HashMap<(usize, usize), &'static str>>> = Mutex::new(None);
    let Ok(mut guard) = TABLE.lock() else {
        return message;
    };
    let table = guard.get_or_insert_with(HashMap::new);
    let key = (index, message.as_ptr() as usize);
    if let Some(text) = table.get(&key) {
        return text;
    }
    if table.len() >= MAX_INTERNED {
        return message;
    }
    let text: &'static str = Box::leak(
        format!("native_timeline_tracks_set tracks[{index}]: {message}").into_boxed_str(),
    );
    table.insert(key, text);
    text
}

fn rejection(failure: Failed) -> &'static str {
    let (bulk, command) = failure;
    let Some(bulk) = bulk else {
        return operation_error(command, "add_property_track", Refs::default());
    };
    let operation = if bulk.in_keys {
        "set_property_key"
    } else {
        "add_property_track"
    };
    let refs = Refs {
        asset: true,
        target: bulk.in_keys,
    };
    let mut message = edit_message(operation, bulk.failure.error, refs);
    if message.is_empty() {
        message = "native timeline edit rejected";
    }
    match bulk.index {
        Some(index) => indexed(index, message),
        None => message,
    }
}

pub(super) fn tracks_set(
    app: &mut AppDocument,
    tracks: &[NativeTrack],
) -> Result<McpOutcome, &'static str> {
    // One parse of the scene finds every target's element type.
    let source = app.source_bytes().map_err(history_error)?;
    let targets: Vec<&[u8]> = tracks.iter().map(|t| t.target_id.as_slice()).collect();
    for (index, (track, tag)) in tracks
        .iter()
        .zip(element_tags(&source, &targets))
        .enumerate()
    {
        if tag.is_some_and(|tag| !property_fits_tag(&tag, &track.property)) {
            return Err(indexed(index, mismatch_message(&track.property)));
        }
    }
    drop(source);
    add_many(app, tracks).map_err(rejection)?;
    let keys: usize = tracks.iter().map(|t| t.keys.len() + 1).sum();
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"message\":\"Timeline updated\",\"tracks\":{},\"keys\":{keys}}}",
        tracks.len()
    )))
}
