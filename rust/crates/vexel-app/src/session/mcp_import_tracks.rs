//! `document_import` timeline part: the property tracks of the imported
//! file as bulk tracks with prefixed ids and targets.

use crate::preview::native_timeline::{NativeTrack, PropertyKey};
use vexel_json::JsonDocument;
use vexel_project::BundleView;
use vexel_timeline::{AssetRef, CAMERA_TARGET, TimelineContext, decode};

const BAD: &str = "document_import: the file's native timeline is not valid for its scene; retry with includeTimeline false";

fn prefixed(prefix: &str, id: &[u8]) -> Vec<u8> {
    [prefix.as_bytes(), id].concat()
}

/// Raw JSON text of member `key` of the JSON object `object`.
fn member(object: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>, &'static str> {
    let doc = JsonDocument::parse(object).map_err(|_| BAD)?;
    Ok(doc
        .object_get(doc.root(), key)
        .and_then(|at| doc.raw_token(at))
        .map(<[u8]>::to_vec))
}

/// `tick` of a timeline at `from` ticks per second, on one at `to`, rounded
/// to the nearest tick and clamped to `u32::MAX`.
fn rescale(tick: u32, from: u32, to: u32) -> Result<u32, &'static str> {
    if from == 0 {
        return Err(BAD);
    }
    let scaled = (u64::from(tick) * u64::from(to) + u64::from(from) / 2) / u64::from(from);
    Ok(u32::try_from(scaled).unwrap_or(u32::MAX))
}

/// The property tracks of `bundle`'s timeline (drawing, audio and camera
/// tracks are left out), ids and targets prefixed, key ticks rescaled from
/// the file's rate to `host_tps`. Empty when the file has no timeline.
pub(super) fn property_tracks(
    bundle: &BundleView<'_>,
    source_ids: &[String],
    prefix: &str,
    host_tps: u32,
) -> Result<Vec<NativeTrack>, &'static str> {
    let source = bundle.timeline_json;
    if source.is_empty() {
        return Ok(Vec::new());
    }
    let assets = (0..bundle.asset_count)
        .filter_map(|i| bundle.asset_at(i))
        .map(|a| AssetRef {
            name: a.name.to_vec(),
            media_type: a.media_type.to_vec(),
        })
        .collect();
    let mut scene_ids: Vec<Vec<u8>> = source_ids.iter().map(|id| id.as_bytes().to_vec()).collect();
    scene_ids.sort();
    let context = TimelineContext { scene_ids, assets };
    let view = decode(source, &context).map_err(|_| BAD)?;
    let mut out = Vec::new();
    for index in 0..view.track_count() {
        let track = view.track_at(index).ok_or(BAD)?;
        let (Some(property), Some(value_type), Some(interpolation)) =
            (track.property, track.value_type, track.interpolation)
        else {
            continue;
        };
        if track.target == CAMERA_TARGET {
            continue;
        }
        let mut items = Vec::with_capacity(track.item_count);
        for item in 0..track.item_count {
            items.push(view.item_at(index, item).ok_or(BAD)?);
        }
        let Some(first) = items.first() else {
            continue;
        };
        let mut keys: Vec<PropertyKey> = Vec::with_capacity(items.len());
        for (position, item) in items.iter().enumerate() {
            let tick = rescale(item.tick, view.ticks_per_second(), host_tps)?;
            // The tick-zero value carries no ease; a key at the same tick adds it.
            if position == 0 && tick == 0 && item.ease.is_none() {
                continue;
            }
            // Rescaling can fold neighbouring keys onto one tick: the first wins.
            if keys.last().is_some_and(|k| k.tick == tick) {
                continue;
            }
            let ease = if item.ease.is_some() {
                member(&source[item.source.start..item.source.end], b"ease")?
            } else {
                None
            };
            keys.push(PropertyKey {
                tick,
                value: item.value_json.ok_or(BAD)?.to_vec(),
                ease,
            });
        }
        let noise = if view.track_noise(index).is_some() {
            member(track.payload_json, b"noise")?
        } else {
            None
        };
        out.push(NativeTrack {
            track_id: prefixed(prefix, track.id),
            target_id: prefixed(prefix, track.target),
            property: property.to_vec(),
            value_type: value_type.to_vec(),
            interpolation: interpolation.to_vec(),
            value: first.value_json.ok_or(BAD)?.to_vec(),
            name: track.name.map(<[u8]>::to_vec).unwrap_or_default(),
            noise,
            keys,
        });
    }
    Ok(out)
}
