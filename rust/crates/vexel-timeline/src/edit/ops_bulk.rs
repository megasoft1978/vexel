//! Bulk property-track insertion: many tracks with all their keys are built as
//! one payload, spliced once and validated once, instead of one splice and one
//! full decode per track and per key.

use super::ops::{json_string, make_key, value_json};
use super::{valid_id, valid_target, validate_utf8, EditFailure, TimelineEdit};
use crate::json::{array_spans, object_fields, Reader};
use crate::schema::field;
use crate::{Error, TimelineContext, MAX_SOURCE_BYTES, MAX_TRACKS};
use vexel_base::Span;

/// One key of a bulk track: the same inputs as `set_property_key_with_ease`.
#[derive(Debug, Clone, Copy)]
pub struct BulkKey<'a> {
    pub tick: u32,
    pub value: &'a [u8],
    pub ease: Option<&'a [u8]>,
}

/// One property track of a bulk request: the inputs of `add_property_track`
/// (`value` is the tick-zero value) plus its later keys.
#[derive(Debug, Clone, Copy)]
pub struct BulkTrack<'a> {
    pub id: &'a [u8],
    pub target: &'a [u8],
    pub property: &'a [u8],
    pub value_type: &'a [u8],
    pub interpolation: &'a [u8],
    pub value: &'a [u8],
    pub name: &'a [u8],
    /// Optional `noise` member as JSON text (see `crate::noise`).
    pub noise: Option<&'a [u8]>,
    pub keys: &'a [BulkKey<'a>],
}

/// A rejected bulk request; `index` names the offending track when known and
/// `in_keys` tells whether the failure lies in its later keys (the work of
/// `set_property_key`) rather than in the track itself or its tick-zero value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BulkFailure {
    pub index: Option<usize>,
    pub in_keys: bool,
    pub failure: EditFailure,
}

fn fail_at(index: Option<usize>, error: Error, offset: usize) -> BulkFailure {
    BulkFailure {
        index,
        in_keys: false,
        failure: super::fail(error, offset),
    }
}

fn append(out: &mut Vec<u8>, part: &[u8], index: usize) -> Result<(), BulkFailure> {
    out.try_reserve(part.len())
        .map_err(|_| fail_at(Some(index), Error::Limit, 0))?;
    out.extend_from_slice(part);
    Ok(())
}

/// Tick-zero value then keys: a key at an existing tick (tick 0 included)
/// replaces that item, others are inserted in tick order, exactly as
/// sequential `set_property_key_with_ease` calls would leave them.
fn merged<'a>(track: &BulkTrack<'a>) -> Vec<BulkKey<'a>> {
    let mut items = Vec::with_capacity(track.keys.len() + 1);
    items.push(BulkKey {
        tick: 0,
        value: track.value,
        ease: None,
    });
    for key in track.keys {
        match items.binary_search_by_key(&key.tick, |item| item.tick) {
            Ok(at) => items[at] = *key,
            Err(at) => items.insert(at, *key),
        }
    }
    items
}

/// The track JSON and the offset in it where the later keys (after the
/// tick-zero item) start.
fn payload(track: &BulkTrack<'_>, index: usize) -> Result<(Vec<u8>, usize), BulkFailure> {
    let at = |e: EditFailure| BulkFailure {
        index: Some(index),
        in_keys: false,
        failure: e,
    };
    let strings = [
        track.id,
        track.target,
        track.property,
        track.value_type,
        track.interpolation,
        track.name,
    ];
    if strings
        .iter()
        .any(|s| !validate_utf8(s) || s.len() > crate::MAX_STRING_BYTES)
        || track.property.is_empty()
        || track.value_type.is_empty()
        || track.interpolation.is_empty()
        || !valid_id(track.id)
        || track.target.is_empty()
    {
        return Err(fail_at(Some(index), Error::InvalidArgument, 0));
    }
    value_json(track.value).map_err(at)?;
    let items = merged(track);
    let mut out = Vec::new();
    append(&mut out, b"{\"id\":", index)?;
    append(&mut out, &json_string(track.id).map_err(at)?, index)?;
    append(&mut out, b",\"kind\":\"property\",\"target\":", index)?;
    append(&mut out, &json_string(track.target).map_err(at)?, index)?;
    append(&mut out, b",\"visible\":true,\"property\":", index)?;
    append(&mut out, &json_string(track.property).map_err(at)?, index)?;
    append(&mut out, b",\"value_type\":", index)?;
    append(&mut out, &json_string(track.value_type).map_err(at)?, index)?;
    append(&mut out, b",\"interpolation\":", index)?;
    append(&mut out, &json_string(track.interpolation).map_err(at)?, index)?;
    append(&mut out, b",\"keys\":[", index)?;
    let mut keys_from = 0;
    for (n, item) in items.iter().enumerate() {
        if n > 0 {
            append(&mut out, b",", index)?;
        }
        append(&mut out, &make_key(item.tick, item.value, item.ease).map_err(at)?, index)?;
        if n == 0 {
            keys_from = out.len();
        }
    }
    append(&mut out, b"]", index)?;
    if let Some(noise) = track.noise {
        let mut reader = Reader::new(noise, None);
        let whole = noise.first() == Some(&b'{')
            && reader.scan(0).is_ok()
            && reader.cursor == noise.len()
            && crate::noise::parse(noise, Span { start: 0, end: noise.len() }).is_ok();
        if !whole {
            return Err(fail_at(Some(index), Error::InvalidValue, 0));
        }
        append(&mut out, b",\"noise\":", index)?;
        append(&mut out, noise, index)?;
    }
    if !track.name.is_empty() {
        append(&mut out, b",\"name\":", index)?;
        append(&mut out, &json_string(track.name).map_err(at)?, index)?;
    }
    append(&mut out, b"}", index)?;
    Ok((out, keys_from))
}

pub(super) fn add_property_tracks(
    edit: &mut TimelineEdit,
    context: &TimelineContext,
    tracks: &[BulkTrack<'_>],
) -> Result<(), BulkFailure> {
    let whole = |e: EditFailure| BulkFailure {
        index: None,
        in_keys: false,
        failure: e,
    };
    if tracks.is_empty() || tracks.len() > MAX_TRACKS {
        return Err(fail_at(None, Error::InvalidArgument, 0));
    }
    // The editor's source was decoded when it was created or committed, so only
    // the tracks array is located here; the single decode is the commit below.
    let source = edit.source();
    let mut reader = Reader::new(source, None);
    reader.scan(0).map_err(|e| whole(e.into()))?;
    let root = Span {
        start: 0,
        end: reader.cursor,
    };
    let root_fields = object_fields(source, root, None).map_err(|e| whole(e.into()))?;
    let array = field(&root_fields, b"tracks")
        .ok_or_else(|| fail_at(None, Error::InvalidSchema, 0))?
        .value;
    let spans = array_spans(source, array, MAX_TRACKS, None).map_err(|e| whole(e.into()))?;
    if spans.len() + tracks.len() > MAX_TRACKS {
        return Err(fail_at(None, Error::Limit, array.start));
    }
    let insert_at = spans.last().map_or(array.end - 1, |s| s.end);
    let mut insertion = Vec::new();
    let mut starts = Vec::new();
    starts
        .try_reserve_exact(tracks.len())
        .map_err(|_| fail_at(None, Error::Limit, 0))?;
    for (index, track) in tracks.iter().enumerate() {
        if track.target != crate::CAMERA_TARGET && !valid_target(context, track.target) {
            return Err(fail_at(Some(index), Error::InvalidReference, 0));
        }
        if !spans.is_empty() || index > 0 {
            append(&mut insertion, b",", index)?;
        }
        let (bytes, keys_from) = payload(track, index)?;
        starts.push((insertion.len(), insertion.len() + keys_from));
        append(&mut insertion, &bytes, index)?;
        if source.len() + insertion.len() > MAX_SOURCE_BYTES {
            return Err(fail_at(Some(index), Error::Limit, insert_at));
        }
    }
    let mut candidate = Vec::new();
    candidate
        .try_reserve_exact(source.len() + insertion.len())
        .map_err(|_| fail_at(None, Error::Limit, insert_at))?;
    candidate.extend_from_slice(&source[..insert_at]);
    candidate.extend_from_slice(&insertion);
    candidate.extend_from_slice(&source[insert_at..]);
    edit.commit(context, candidate).map_err(|failure| {
        // Map the offset in the candidate back to the track it falls in.
        let rel = failure.offset.checked_sub(insert_at);
        let index = rel.map(|rel| starts.partition_point(|s| s.0 <= rel).saturating_sub(1));
        BulkFailure {
            index,
            in_keys: rel.zip(index).is_some_and(|(rel, i)| rel >= starts[i].1),
            failure,
        }
    })
}
