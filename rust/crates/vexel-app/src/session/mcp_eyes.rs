//! `eyes_set` (pupils clipped to the eye shape) and `blink` (squash or
//! open/closed swap tracks, plus gaze keys).

use super::mcp_face::{Messages, OwnTrack, facts, track_part, write_tracks};
use super::mcp_rig::transformed;
use crate::preview::native_args::rig::Pivot;
use crate::project_session::SessionEditError;
use crate::session::{AppDocument, McpOutcome, check_room};
use std::collections::BTreeMap;
use vexel_timeline::blink::{MAX_BLINKS, blink_ticks};
use vexel_timeline::rig::LocalPose;

/// Most eyes per `eyes_set` call, pupil groups per eye, and parts per `blink`.
pub(crate) const MAX_EYES: usize = 8;
pub(crate) const MAX_PARTS: usize = 16;
pub(crate) const MAX_GAZE_KEYS: usize = 256;
pub(crate) const MAX_DURATION: u32 = 48;

#[derive(Clone, Debug, PartialEq)]
pub struct Eye {
    pub eye: String,
    pub pupils: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lid {
    pub id: String,
    pub pivot: Pivot,
    /// Vertical scale of the closed lid.
    pub closed_scale: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum When {
    At(Vec<u32>),
    Random {
        seed: u32,
        start: u32,
        end: Option<u32>,
        min_gap: u32,
        max_gap: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gaze {
    pub pupils: Vec<String>,
    pub keys: Vec<(u32, [f64; 2])>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Blink {
    pub lids: Vec<Lid>,
    pub open: Vec<String>,
    pub closed: Vec<String>,
    pub when: When,
    pub duration: u32,
    pub gaze: Option<Gaze>,
}

/// Group `id` exists and is a `<g>`.
fn group(app: &AppDocument, id: &str, tool: &'static str) -> Result<(), &'static str> {
    let info = app.document().element_get(id).map_err(|_| match tool {
        "eyes_set" => "eyes_set: every id must be an element id in the document",
        _ => "blink: every id must be an element id in the document",
    })?;
    if info.tag == "g" {
        return Ok(());
    }
    Err(match tool {
        "eyes_set" => "eyes_set: pupils must be <g> groups (use element_group)",
        _ => "blink: every lid, eye and pupil id must name a <g> group (use element_group)",
    })
}

pub(crate) fn eyes_set(app: &mut AppDocument, eyes: Vec<Eye>) -> Result<McpOutcome, &'static str> {
    check_room(app, eyes.len() * 2)?;
    // (eye, clip is a rect, x, y, w, h)
    let mut plans = Vec::new();
    for eye in &eyes {
        let info = app
            .document()
            .element_get(&eye.eye)
            .map_err(|_| "eyes_set: eye must be the id of an element in the document")?;
        if !matches!(
            info.tag.as_str(),
            "ellipse" | "circle" | "rect" | "path" | "g"
        ) {
            return Err(
                "eyes_set: eye must be a shape or group whose outline the pupils stay inside",
            );
        }
        if transformed(app, &eye.eye) {
            return Err("eyes_set: the eye shape cannot sit inside a transform");
        }
        let (x0, y0, x1, y1) = info
            .bounds
            .ok_or("eyes_set: the eye shape has no usable bounds")?;
        if !((x1 - x0) > 0.0 && (y1 - y0) > 0.0) {
            return Err("eyes_set: the eye shape has no area");
        }
        let mut parent = None;
        for pupil in &eye.pupils {
            group(app, pupil, "eyes_set")?;
            let info = app
                .document()
                .element_get(pupil)
                .map_err(|_| "eyes_set: every id must be an element id in the document")?;
            if info.tag != "g" || *pupil == eye.eye {
                return Err("eyes_set: pupils must be <g> groups other than the eye");
            }
            if parent.get_or_insert(info.parent.clone()) != &info.parent {
                return Err("eyes_set: the pupil groups of one eye must be siblings");
            }
            if let Some(p) = &info.parent
                && transformed(app, p)
            {
                return Err("eyes_set: pupils cannot sit inside a transformed group");
            }
        }
        plans.push((info.tag == "rect", x0, y0, x1 - x0, y1 - y0));
    }
    let mut made: Vec<(String, String)> = Vec::new();
    app.edit(|doc| {
        for (eye, (rect, x, y, w, h)) in eyes.iter().zip(&plans) {
            let wrapper = doc.group(&eye.pupils)?;
            let clip = if *rect {
                doc.add_clip_rect(*x, *y, *w, *h)?
            } else {
                doc.add_clip_ellipse(x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0)?
            };
            doc.set_clip(&wrapper, Some(&clip))?;
            made.push((wrapper, clip));
        }
        Ok::<(), vexel_v1_document::V1DocumentError>(())
    })
    .map_err(|e| match e {
        SessionEditError::Operation(_) => {
            "eyes_set: could not clip the pupils (are they siblings under one parent?)"
        }
        SessionEditError::History(_) => "project history unavailable",
    })?;
    let items: Vec<String> = eyes
        .iter()
        .zip(&made)
        .map(|(eye, (wrapper, clip))| {
            format!(
                "{{\"eye\":\"{}\",\"groupId\":\"{wrapper}\",\"clipId\":\"{clip}\"}}",
                eye.eye
            )
        })
        .collect();
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"eyes_set\",\"eyes\":[{}]}}",
        items.join(",")
    )))
}

const MESSAGES: Messages = Messages {
    too_many: "blink: too many keys for one call (at most 16384); use fewer blinks or a shorter range",
    ticks: "blink: every blink and gaze tick must be below the timeline duration (lengthen it with native_timeline_set_duration)",
    rejected: "blink: the timeline rejected the tracks (track limit of 256, size limit reached, or a part already has a transform track)",
};

fn pose(scale_y: f64, tx: f64, ty: f64) -> LocalPose {
    LocalPose {
        rotate: 0.0,
        translate: [tx, ty],
        scale: [1.0, scale_y],
    }
}

pub(crate) fn blink(app: &mut AppDocument, spec: Blink) -> Result<McpOutcome, &'static str> {
    let timeline = facts(app)?;
    let d = spec.duration;
    let mut ticks = match &spec.when {
        When::At(at) => at.clone(),
        When::Random {
            seed,
            start,
            end,
            min_gap,
            max_gap,
        } => {
            let end = end.unwrap_or(timeline.duration).min(timeline.duration);
            blink_ticks(*seed, *start, end, d, *min_gap, *max_gap)
        }
    };
    ticks.sort_unstable();
    ticks.dedup();
    if ticks.len() > MAX_BLINKS || ticks.windows(2).any(|w| w[1] - w[0] < d) {
        return Err("blink: at most 256 blinks, each at least duration ticks after the previous");
    }
    let mut tracks: Vec<OwnTrack> = Vec::new();
    let mut used: Vec<String> = Vec::new();
    let mut unique = |prefix: &str, id: &str| {
        let mut name = format!("{prefix}_{}", track_part(id));
        while used.contains(&name) {
            name.push('_');
        }
        used.push(name.clone());
        name
    };
    let half = (d / 2).max(1);
    for lid in &spec.lids {
        group(app, &lid.id, "blink")?;
        let pivot = match lid.pivot {
            Pivot::At(p) => p,
            Pivot::Center => {
                let info = app
                    .document()
                    .element_get(&lid.id)
                    .map_err(|_| "blink: every id must be an element id in the document")?;
                if transformed(app, &lid.id) {
                    return Err(
                        "blink: cannot take the centre of a lid inside a transform; give pivot [x,y]",
                    );
                }
                let (x0, y0, x1, y1) = info
                    .bounds
                    .ok_or("blink: cannot take the centre of this lid; give pivot [x,y]")?;
                [(x0 + x1) / 2.0, (y0 + y1) / 2.0]
            }
        };
        let json = |p: LocalPose| p.trs_json(pivot).into_bytes();
        let mut keys: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
        for t in &ticks {
            keys.insert(*t, json(pose(1.0, 0.0, 0.0)));
            keys.insert(*t + half, json(pose(lid.closed_scale, 0.0, 0.0)));
            keys.insert(*t + d, json(pose(1.0, 0.0, 0.0)));
        }
        keys.remove(&0);
        tracks.push(OwnTrack {
            id: unique("blink", &lid.id),
            target: lid.id.clone(),
            property: "transform",
            value_type: "trs",
            interpolation: "linear",
            value: json(pose(1.0, 0.0, 0.0)),
            keys: keys.into_iter().collect(),
        });
    }
    for (ids, shown_during, prefix) in [
        (&spec.open, false, "blink_open"),
        (&spec.closed, true, "blink_closed"),
    ] {
        for id in ids {
            group(app, id, "blink")?;
            let flag = |on: bool| {
                if on {
                    b"true".to_vec()
                } else {
                    b"false".to_vec()
                }
            };
            let mut keys: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
            for t in &ticks {
                keys.insert(*t, flag(shown_during));
                keys.insert(*t + d, flag(!shown_during));
            }
            let first = keys.remove(&0);
            tracks.push(OwnTrack {
                id: unique(prefix, id),
                target: id.clone(),
                property: "visibility",
                value_type: "boolean",
                interpolation: "step",
                value: first.unwrap_or_else(|| flag(!shown_during)),
                keys: keys.into_iter().collect(),
            });
        }
    }
    if let Some(gaze) = &spec.gaze {
        for id in &gaze.pupils {
            group(app, id, "blink")?;
            let mut keys: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
            for (tick, [x, y]) in &gaze.keys {
                keys.insert(*tick, pose(1.0, *x, *y).trs_json([0.0, 0.0]).into_bytes());
            }
            let first = keys.remove(&0);
            tracks.push(OwnTrack {
                id: unique("gaze", id),
                target: id.clone(),
                property: "transform",
                value_type: "trs",
                interpolation: "linear",
                value: first
                    .unwrap_or_else(|| pose(1.0, 0.0, 0.0).trs_json([0.0, 0.0]).into_bytes()),
                keys: keys.into_iter().collect(),
            });
        }
    }
    if tracks.is_empty() {
        return Err("blink: nothing to animate (give lids, open/closed groups or gaze)");
    }
    let stale: Vec<Vec<u8>> = timeline
        .tracks
        .iter()
        .filter(|(id, _)| tracks.iter().any(|t| t.id.as_bytes() == id.as_slice()))
        .map(|(id, _)| id.clone())
        .collect();
    let (count, keys) = write_tracks(app, &tracks, &stale, &MESSAGES, timeline.duration)?;
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"blink\",\"blinks\":{},\"tracks\":{count},\"keys\":{keys}}}",
        ticks.len()
    )))
}
