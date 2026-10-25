//! Face tools (`mouth_set`, `lip_sync`, `eyes_set`, `blink`): the operation
//! type, its routing (lip-sync reads audio, so it needs the host's grant
//! policy) and the shared writer for generated property tracks.

use super::mcp_lipsync::{self, LipSync};
use super::mcp_mouth::{self, Mouth};
use super::settings;
use crate::preview::DocOp;
use crate::preview::native_args::rig::Op;
use crate::project_session::{ProjectTimelineCandidate, ProjectTimelineError};
use crate::session::{AppDocument, McpOutcome};
use std::cell::Cell;
use vexel_mcp_access::AccessPolicy;
use vexel_timeline::{BulkKey, BulkTrack, EditLimits};

/// Most keys one face tool call writes.
pub(crate) const MAX_KEYS: usize = 16_384;

#[derive(Clone, Debug, PartialEq)]
pub enum FaceOp {
    Mouth(Mouth),
    Lip(Box<LipSync>),
}

/// `Operation::Doc` dispatch: face tools get the policy, the rest go on to
/// the plain document families.
pub(crate) fn route(
    app: &mut AppDocument,
    policy: &AccessPolicy,
    op: DocOp,
) -> Result<McpOutcome, &'static str> {
    match op {
        DocOp::Rig(Op::Face(face)) => apply(app, policy, face),
        other => crate::preview::apply_doc(app, other),
    }
}

fn apply(
    app: &mut AppDocument,
    policy: &AccessPolicy,
    op: FaceOp,
) -> Result<McpOutcome, &'static str> {
    match op {
        FaceOp::Mouth(spec) => mcp_mouth::apply(app, spec),
        FaceOp::Lip(spec) => mcp_lipsync::apply(app, policy, *spec),
    }
}

/// A generated property track with its tick-zero value and later keys.
pub(crate) struct OwnTrack {
    pub id: String,
    pub target: String,
    pub property: &'static str,
    pub value_type: &'static str,
    pub interpolation: &'static str,
    pub value: Vec<u8>,
    pub keys: Vec<(u32, Vec<u8>)>,
}

/// `[A-Za-z0-9_-]` form of `id`, usable inside a track id.
pub(crate) fn track_part(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// (tracks, ticks per track id list) facts of the native timeline.
pub(crate) struct TimelineFacts {
    /// (track id, target) of every existing track.
    pub tracks: Vec<(Vec<u8>, Vec<u8>)>,
    pub duration: u32,
}

pub(crate) fn facts(app: &AppDocument) -> Result<TimelineFacts, &'static str> {
    let candidate = app
        .native_timeline_candidate(settings(), EditLimits::default())
        .map_err(|_| "native timeline unavailable")?;
    let view = candidate
        .view()
        .map_err(|_| "native timeline is unsupported")?;
    Ok(TimelineFacts {
        tracks: (0..view.track_count())
            .filter_map(|i| view.track_at(i))
            .map(|t| (t.id.to_vec(), t.target.to_vec()))
            .collect(),
        duration: view.duration_ticks(),
    })
}

/// Error texts of one tool's track writes (each starts with the tool name).
pub(crate) struct Messages {
    pub too_many: &'static str,
    pub ticks: &'static str,
    pub rejected: &'static str,
}

/// Writes `tracks` in ONE timeline edit, first removing the tracks named in
/// `stale`. Returns (tracks, keys). Every key must be below the duration.
pub(crate) fn write_tracks(
    app: &mut AppDocument,
    tracks: &[OwnTrack],
    stale: &[Vec<u8>],
    msg: &Messages,
    duration: u32,
) -> Result<(usize, usize), &'static str> {
    let total: usize = tracks.iter().map(|t| t.keys.len() + 1).sum();
    if tracks.is_empty() || total > MAX_KEYS {
        return Err(msg.too_many);
    }
    if tracks
        .iter()
        .any(|t| t.keys.iter().any(|(tick, _)| *tick >= duration))
    {
        return Err(msg.ticks);
    }
    let names: Vec<&[u8]> = tracks.iter().map(|t| t.id.as_bytes()).collect();
    let key_sets: Vec<Vec<BulkKey<'_>>> = tracks
        .iter()
        .map(|t| {
            t.keys
                .iter()
                .map(|(tick, value)| BulkKey {
                    tick: *tick,
                    value,
                    ease: None,
                })
                .collect()
        })
        .collect();
    let bulk: Vec<BulkTrack<'_>> = tracks
        .iter()
        .zip(&key_sets)
        .zip(names)
        .map(|((t, keys), name)| BulkTrack {
            id: name,
            target: t.target.as_bytes(),
            property: t.property.as_bytes(),
            value_type: t.value_type.as_bytes(),
            interpolation: t.interpolation.as_bytes(),
            value: &t.value,
            name,
            noise: None,
            keys,
        })
        .collect();
    let failure = Cell::new(None);
    app.edit_timeline(settings(), EditLimits::default(), |c| {
        let mut current: Option<ProjectTimelineCandidate> = None;
        for id in stale {
            current = Some(match current {
                None => c.remove_track(id)?,
                Some(prior) => prior.remove_track(id)?,
            });
        }
        Ok::<_, ProjectTimelineError>(match current {
            None => c.add_property_tracks(&bulk, &failure)?,
            Some(prior) => prior.add_property_tracks(&bulk, &failure)?,
        })
    })
    .map_err(|_| msg.rejected)?;
    Ok((tracks.len(), total))
}
