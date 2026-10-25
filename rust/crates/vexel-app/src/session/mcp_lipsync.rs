//! `lip_sync`: a viseme per tick from an audio envelope, written as one
//! `visibility` step track per mouth group in ONE undoable timeline edit.

use super::mcp_face::{Messages, OwnTrack, facts, track_part, write_tracks};
use super::mcp_mouth;
use crate::session::mcp_audio_gen::{EnvSource, envelope_of};
use crate::session::{AppDocument, McpOutcome};
use vexel_mcp_access::AccessPolicy;
use vexel_timeline::audio_envelope::MAX_TICKS;
use vexel_timeline::lipsync::{self, Config, Mapping};

#[derive(Clone, Debug, PartialEq)]
pub enum LipSource {
    Track(String),
    Path(String),
    /// One 0..1 loudness per tick, taken as given.
    Envelope(Vec<f32>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LipSync {
    pub character: String,
    pub source: LipSource,
    pub start_tick: Option<u32>,
    pub ticks: Option<u32>,
    pub mapping: Mapping,
    pub smoothing: u32,
    pub seed: u32,
    pub text: Vec<String>,
}

const MESSAGES: Messages = Messages {
    too_many: "lip_sync: too many mouth changes for one call (at most 16384 keys); pass ticks to take a shorter part",
    ticks: "lip_sync: every key must be below the timeline duration (lengthen it with native_timeline_set_duration)",
    rejected: "lip_sync: the timeline rejected the tracks (track limit of 256 or size limit reached)",
};

pub(crate) fn apply(
    app: &mut AppDocument,
    policy: &AccessPolicy,
    spec: LipSync,
) -> Result<McpOutcome, &'static str> {
    let mouth = mcp_mouth::load(app, &spec.character)?;
    for (_, group) in &mouth.shapes {
        if app.document().element_get(group).is_err() {
            return Err("lip_sync: a mouth group no longer exists; call mouth_set again");
        }
    }
    let timeline = facts(app)?;
    let (rms, from) = match &spec.source {
        LipSource::Envelope(values) => {
            if spec.ticks.is_some() {
                return Err("lip_sync: ticks applies to path and trackId sources only");
            }
            (values.clone(), spec.start_tick.unwrap_or(0))
        }
        LipSource::Track(id) => {
            let data = envelope_of(app, policy, &EnvSource::Track(id.clone()), spec.ticks)?;
            (data.env.rms, spec.start_tick.unwrap_or(data.start_tick))
        }
        LipSource::Path(path) => {
            let data = envelope_of(app, policy, &EnvSource::Path(path.clone()), spec.ticks)?;
            (data.env.rms, spec.start_tick.unwrap_or(0))
        }
    };
    if rms.is_empty() || rms.len() > MAX_TICKS {
        return Err("lip_sync: the envelope must hold 1-100000 ticks");
    }
    if from >= timeline.duration {
        return Err(
            "lip_sync: startTick must be below the timeline duration (lengthen it with native_timeline_set_duration)",
        );
    }
    let cfg = Config {
        mapping: spec.mapping,
        smoothing: spec.smoothing,
        seed: spec.seed,
    };
    let words: Vec<Vec<u8>> = spec.text.iter().map(|w| lipsync::word_visemes(w)).collect();
    let visemes = lipsync::derive(&rms, &cfg, &words);
    // Group shown for each viseme: its own, else the nearest, else rest.
    let rest = mouth.shapes[0].1.clone();
    let group_for = |v: u8| -> &str {
        let find = |w: u8| mouth.shapes.iter().find(|s| s.0 == w).map(|s| s.1.as_str());
        find(v)
            .or_else(|| lipsync::fallback(v).iter().find_map(|w| find(*w)))
            .unwrap_or(&rest)
    };
    // Change points: (tick, group shown from there), starting at rest.
    let mut points: Vec<(u32, &str)> = vec![(0, rest.as_str())];
    let mut dropped = 0usize;
    for (i, v) in visemes.iter().enumerate() {
        let tick = u64::from(from) + i as u64;
        if tick >= u64::from(timeline.duration) {
            dropped += 1;
            continue;
        }
        let tick = tick as u32;
        let group = group_for(*v);
        match points.last_mut() {
            Some(last) if last.0 == tick => last.1 = group,
            Some(last) if last.1 == group => {}
            _ => points.push((tick, group)),
        }
    }
    let end = u64::from(from) + visemes.len() as u64;
    if end < u64::from(timeline.duration) && points.last().is_some_and(|p| p.1 != rest) {
        points.push((end as u32, rest.as_str()));
    }
    let mut groups: Vec<&str> = Vec::new();
    for (_, group) in &mouth.shapes {
        if !groups.contains(&group.as_str()) {
            groups.push(group);
        }
    }
    let mut tracks = Vec::new();
    let mut used: Vec<String> = Vec::new();
    for group in &groups {
        let mut id = format!("lip_{}", track_part(group));
        while used.contains(&id) {
            id.push('_');
        }
        used.push(id.clone());
        let shown = |g: &str| {
            if g == *group {
                &b"true"[..]
            } else {
                &b"false"[..]
            }
        };
        let mut keys: Vec<(u32, Vec<u8>)> = Vec::new();
        let mut state = shown(points[0].1);
        for (tick, g) in points.iter().skip(1) {
            if shown(g) != state {
                state = shown(g);
                keys.push((*tick, state.to_vec()));
            }
        }
        tracks.push(OwnTrack {
            id,
            target: (*group).to_owned(),
            property: "visibility",
            value_type: "boolean",
            interpolation: "step",
            value: shown(points[0].1).to_vec(),
            keys,
        });
    }
    // Earlier lip tracks of this character's groups are replaced.
    let stale: Vec<Vec<u8>> = timeline
        .tracks
        .iter()
        .filter(|(id, target)| {
            id.starts_with(b"lip_") && groups.iter().any(|g| g.as_bytes() == target.as_slice())
        })
        .map(|(id, _)| id.clone())
        .collect();
    let (count, keys) = write_tracks(app, &tracks, &stale, &MESSAGES, timeline.duration)?;
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"lip_sync\",\"tracks\":{count},\"keys\":{keys},\"startTick\":{from},\"ticks\":{},\"droppedTicks\":{dropped},\"changes\":{}}}",
        visemes.len(),
        points.len() - 1
    )))
}
