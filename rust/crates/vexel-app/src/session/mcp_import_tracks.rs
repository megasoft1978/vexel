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

/// The property tracks of `bundle`'s timeline (drawing, audio and camera
/// tracks are left out), ids and targets prefixed. Empty when the file has
/// no timeline.
pub(super) fn property_tracks(
    bundle: &BundleView<'_>,
    source_ids: &[String],
    prefix: &str,
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
        let Some((first, rest)) = items.split_first() else {
            continue;
        };
        let key = |item: &vexel_timeline::ItemView<'_>| -> Result<PropertyKey, &'static str> {
            let ease = if item.ease.is_some() {
                member(&source[item.source.start..item.source.end], b"ease")?
            } else {
                None
            };
            Ok(PropertyKey {
                tick: item.tick,
                value: item.value_json.ok_or(BAD)?.to_vec(),
                ease,
            })
        };
        let mut keys = Vec::with_capacity(rest.len() + 1);
        // The tick-zero value carries no ease; a key at the same tick adds it.
        if first.ease.is_some() || first.tick != 0 {
            keys.push(key(first)?);
        }
        for item in rest {
            keys.push(key(item)?);
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
