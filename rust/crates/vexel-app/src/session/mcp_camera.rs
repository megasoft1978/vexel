//! `native_timeline_set_camera`: the camera as one set of reserved `@camera` tracks
//! (`camera_x`, `camera_y`, `camera_zoom`, `camera_rotate`), replaced in a single
//! undoable edit. The renderer applies it as a view transform (see
//! `animation/native_properties/camera.rs`): one `<g transform>` around the root's content
//! in each per-tick snapshot, so the document's scene itself is unchanged.

use super::{NativeTrack, PropertyKey, done, error, settings};
use crate::project_session::ProjectTimelineCandidate;
use crate::session::{AppDocument, McpOutcome};
use std::cell::Cell;
use vexel_timeline::{BulkKey, BulkTrack, CAMERA_TARGET, EditLimits};

/// One camera key; omitted channels are not keyed at this tick.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraKey {
    pub tick: u32,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub zoom: Option<f64>,
    pub rotate: Option<f64>,
    pub ease: Option<Vec<u8>>,
}

/// Seeded handheld shake added on top of the keyed camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shake {
    pub amplitude: f64,
    pub frequency: f64,
    pub seed: u32,
    pub octaves: u32,
    /// Roll amplitude in degrees (0 = none).
    pub rotate: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraSpec {
    pub keys: Vec<CameraKey>,
    pub shake: Option<Shake>,
    pub clear: bool,
}

const REJECTED: &str = "native_timeline_set_camera rejected: every key tick must be below durationTicks (lengthen with native_timeline_set_duration); x and y within +-1000000, zoom 0.01-100, rotate within +-36000 degrees";

fn noise_json(amplitude: f64, shake: &Shake, seed: u32) -> Vec<u8> {
    format!(
        "{{\"amplitude\":{amplitude},\"frequency\":{},\"seed\":{seed},\"octaves\":{}}}",
        shake.frequency, shake.octaves
    )
    .into_bytes()
}

/// The camera tracks for `spec`, in a fixed order.
pub(crate) fn tracks(spec: &CameraSpec) -> Vec<NativeTrack> {
    type Channel = (
        &'static str,
        &'static str,
        f64,
        fn(&CameraKey) -> Option<f64>,
    );
    let channels: [Channel; 4] = [
        ("x", "Camera x", 0.0, |k| k.x),
        ("y", "Camera y", 0.0, |k| k.y),
        ("zoom", "Camera zoom", 1.0, |k| k.zoom),
        ("rotate", "Camera rotate", 0.0, |k| k.rotate),
    ];
    let mut out = Vec::new();
    for (index, (property, name, rest, pick)) in channels.into_iter().enumerate() {
        let shake = spec.shake.filter(|s| match property {
            "x" | "y" => s.amplitude > 0.0,
            "rotate" => s.rotate > 0.0,
            _ => false,
        });
        let keyed: Vec<&CameraKey> = spec.keys.iter().filter(|k| pick(k).is_some()).collect();
        if keyed.is_empty() && shake.is_none() {
            continue;
        }
        let first = keyed.iter().find(|k| k.tick == 0).and_then(|k| pick(k));
        let keys = keyed
            .iter()
            .filter(|k| k.tick != 0)
            .map(|k| PropertyKey {
                tick: k.tick,
                value: pick(k).unwrap_or(rest).to_string().into_bytes(),
                ease: k.ease.clone(),
            })
            .collect();
        let tick0_ease = keyed
            .iter()
            .find(|k| k.tick == 0)
            .and_then(|k| k.ease.clone());
        let noise = shake.map(|s| {
            let amplitude = if property == "rotate" {
                s.rotate
            } else {
                s.amplitude
            };
            noise_json(amplitude, &s, s.seed.wrapping_add(index as u32))
        });
        out.push(NativeTrack {
            track_id: format!("camera_{property}").into_bytes(),
            target_id: CAMERA_TARGET.to_vec(),
            property: property.as_bytes().to_vec(),
            value_type: b"scalar".to_vec(),
            interpolation: b"linear".to_vec(),
            value: first.unwrap_or(rest).to_string().into_bytes(),
            name: name.as_bytes().to_vec(),
            noise,
            keys,
        });
        // A tick-zero ease cannot be carried by the track's initial value; re-key it.
        if let (Some(ease), Some(track)) = (tick0_ease, out.last_mut()) {
            track.keys.insert(
                0,
                PropertyKey {
                    tick: 0,
                    value: track.value.clone(),
                    ease: Some(ease),
                },
            );
        }
    }
    out
}

pub(crate) fn set_camera(
    app: &mut AppDocument,
    spec: &CameraSpec,
) -> Result<McpOutcome, &'static str> {
    let limits = EditLimits::default();
    let existing: Vec<Vec<u8>> = {
        let candidate = app
            .native_timeline_candidate(settings(), limits)
            .map_err(error)?;
        let view = candidate
            .view()
            .map_err(|_| "unsupported native timeline")?;
        (0..view.track_count())
            .filter_map(|i| view.track_at(i))
            .filter(|t| t.target == CAMERA_TARGET)
            .map(|t| t.id.to_vec())
            .collect()
    };
    let new = if spec.clear { Vec::new() } else { tracks(spec) };
    if existing.is_empty() && new.is_empty() {
        return Ok(done("Timeline unchanged"));
    }
    let key_sets: Vec<Vec<BulkKey<'_>>> = new
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
        .collect();
    let bulk: Vec<BulkTrack<'_>> = new
        .iter()
        .zip(&key_sets)
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
    app.edit_timeline(settings(), limits, |c| {
        let mut current: Option<ProjectTimelineCandidate> = None;
        for id in &existing {
            current = Some(match current {
                None => c.remove_track(id)?,
                Some(prior) => prior.remove_track(id)?,
            });
        }
        match current {
            Some(prior) if bulk.is_empty() => Ok(prior),
            Some(prior) => prior.add_property_tracks(&bulk, &failure),
            None => c.add_property_tracks(&bulk, &failure),
        }
    })
    .map_err(|_| REJECTED)?;
    Ok(done("Timeline updated"))
}
