//! `native_timeline_tracks_set`: an array of `add_property_track` requests
//! applied as one undoable edit. Every entry is parsed with the same code as a
//! single `native_timeline_add_property_track` call, so its value forms,
//! ease names and size limits are identical; this adds the count caps.

use super::{JsonDocument, JsonType, Operation, PreviewArgumentError, invalid, parse_one};
use crate::preview::native_timeline::{NativeOp, NativeTrack};

/// Most tracks one call may carry (the timeline holds at most 256).
pub(crate) const MAX_TRACKS: usize = 256;
/// Most keys (tick-zero values included) one call may carry.
pub(crate) const MAX_KEYS: usize = 16_384;

const TOOL: &str = "tracks_set";
const FORM: &str = "an array of 1-256 objects {trackId, targetId, property, valueType, interpolation, value, name?, keys?}, each the arguments of native_timeline_add_property_track without documentId";
const MEMBERS: [&[u8]; 8] = [
    b"trackId",
    b"targetId",
    b"property",
    b"valueType",
    b"interpolation",
    b"value",
    b"name",
    b"keys",
];

pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
) -> Result<Operation, PreviewArgumentError> {
    let array = doc
        .object_get(args, b"tracks")
        .and_then(|at| doc.token(at))
        .filter(|t| t.kind == JsonType::Array && (1..=MAX_TRACKS).contains(&t.child_count))
        .ok_or_else(|| invalid(TOOL, "tracks", FORM, None))?;
    let mut tracks = Vec::new();
    tracks
        .try_reserve_exact(array.child_count)
        .map_err(|_| PreviewArgumentError::Generic("preview allocation failed"))?;
    let mut keys = 0usize;
    let mut item = array.first_child;
    while let Some(at) = item {
        let index = tracks.len();
        let wrap = |error: PreviewArgumentError| match error {
            PreviewArgumentError::Actionable(message) => {
                PreviewArgumentError::Actionable(format!("tracks[{index}]: {message}"))
            }
            PreviewArgumentError::Generic(message) => {
                PreviewArgumentError::Actionable(format!("tracks[{index}]: {message}"))
            }
        };
        let object = doc.token(at).filter(|t| t.kind == JsonType::Object);
        let Some(object) = object else {
            return Err(wrap(invalid(TOOL, "tracks", FORM, None)));
        };
        let mut name = object.first_child;
        while let Some(member) = name {
            let known = doc
                .decoded_string(member)
                .is_some_and(|k| MEMBERS.contains(&k.as_slice()));
            if !known {
                return Err(wrap(invalid(TOOL, "tracks", FORM, None)));
            }
            name = doc
                .token(member)
                .and_then(|t| t.next_sibling)
                .and_then(|v| doc.token(v))
                .and_then(|t| t.next_sibling);
        }
        let NativeOp::AddPropertyTrack {
            track_id,
            target_id,
            property,
            value_type,
            interpolation,
            value,
            name,
            keys: track_keys,
        } = parse_one(doc, at, "add_property_track").map_err(wrap)?
        else {
            return Err(wrap(invalid(TOOL, "tracks", FORM, None)));
        };
        keys += track_keys.len() + 1;
        if keys > MAX_KEYS {
            return Err(PreviewArgumentError::Actionable(format!(
                "native_timeline_tracks_set carries more than {MAX_KEYS} keys; split it into several calls"
            )));
        }
        tracks.push(NativeTrack {
            track_id,
            target_id,
            property,
            value_type,
            interpolation,
            value,
            name,
            keys: track_keys,
        });
        item = doc.token(at).and_then(|t| t.next_sibling);
    }
    Ok(Operation::Native(NativeOp::TracksSet(tracks)))
}
