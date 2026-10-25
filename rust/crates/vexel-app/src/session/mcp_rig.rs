//! Skeletal rigs: `rig_create` stores the rig in the document as
//! `<desc id="vexel-rig">{json}</desc>` under the root, so it saves, reopens
//! and undoes with the file; `rig_pose` and `clip_apply` (see below) key the
//! bones' groups as `trs` tracks through the bulk timeline path.

use super::settings;
use crate::preview::native_args::rig::{ClipCall, Create, MAX_KEYS, Op, Pivot, Pose};
use crate::project_session::{ProjectTimelineCandidate, SessionEditError};
use crate::session::{AppDocument, McpOutcome, check_room};
use std::cell::Cell;
use std::collections::BTreeMap;
use vexel_json::{JsonDocument, JsonType};
use vexel_timeline::clips::{ClipError, generate};
use vexel_timeline::rig::{Bone, LocalPose, MAX_BONES, Rig, RigError, Role};
use vexel_timeline::{BulkKey, BulkTrack, EditLimits};

/// Element id of the stored rig.
const RIG_ID: &str = "vexel-rig";
const NO_RIG: &str = "no rig in this document; call rig_create first";
const BAD_RIG: &str = "the rig stored in the document is invalid; call rig_create again";

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The text of the stored rig element.
fn stored_text(source: &[u8]) -> Option<&[u8]> {
    let at = find(source, b"id=\"vexel-rig\"")?;
    let open = at + source[at..].iter().position(|b| *b == b'>')? + 1;
    let close = open + find(&source[open..], b"</desc>")?;
    Some(&source[open..close])
}

fn to_json(rig: &Rig) -> String {
    let mut out = String::from("{\"v\":1,\"bones\":[");
    for (i, bone) in rig.bones.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"id\":\"{}\",\"pivot\":[{},{}]",
            bone.id,
            bone.pivot[0] + 0.0,
            bone.pivot[1] + 0.0
        ));
        if let Some(parent) = bone.parent {
            out.push_str(&format!(",\"parent\":\"{}\"", rig.bones[parent].id));
        }
        out.push('}');
    }
    out.push_str("],\"roles\":{");
    for (i, (role, bone)) in rig.roles.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("\"{}\":\"{}\"", role.name(), rig.bones[*bone].id));
    }
    out.push_str("}}");
    out
}

fn from_json(text: &[u8]) -> Option<Rig> {
    let doc = JsonDocument::parse(text).ok()?;
    let root = doc.root();
    if doc.uint(doc.object_get(root, b"v")?)? != 1 {
        return None;
    }
    let list = doc.object_get(root, b"bones")?;
    let count = doc.token(list).filter(|t| t.kind == JsonType::Array)?.child_count;
    if count == 0 || count > MAX_BONES {
        return None;
    }
    let text_of = |at: usize| String::from_utf8(doc.decoded_string(at)?).ok();
    let mut names = Vec::new();
    for i in 0..count {
        let bone = doc.array_get(list, i)?;
        names.push(text_of(doc.object_get(bone, b"id")?)?);
    }
    let mut bones = Vec::new();
    for (i, id) in names.iter().enumerate() {
        let bone = doc.array_get(list, i)?;
        let pivot = doc.object_get(bone, b"pivot")?;
        let num = |n: usize| -> Option<f64> {
            std::str::from_utf8(doc.raw_token(doc.array_get(pivot, n)?)?)
                .ok()?
                .parse()
                .ok()
        };
        let parent = match doc.object_get(bone, b"parent") {
            None => None,
            Some(p) => {
                let name = text_of(p)?;
                Some(names.iter().position(|n| *n == name)?)
            }
        };
        bones.push(Bone {
            id: id.clone(),
            pivot: [num(0)?, num(1)?],
            parent,
        });
    }
    let mut roles = Vec::new();
    let table = doc.object_get(root, b"roles")?;
    for role in Role::ALL {
        if let Some(at) = doc.object_get(table, role.name().as_bytes()) {
            let id = text_of(at)?;
            roles.push((role, names.iter().position(|n| *n == id)?));
        }
    }
    Rig::new(bones, roles).ok()
}

/// The document's rig, if any.
pub(crate) fn load(app: &AppDocument) -> Result<Rig, &'static str> {
    let source = app.source_bytes().map_err(|_| "project history unavailable")?;
    let text = stored_text(&source).ok_or(NO_RIG)?;
    from_json(text).ok_or(BAD_RIG)
}

fn rig_error(error: RigError) -> &'static str {
    match error {
        RigError::Empty => "rig_create: bones must not be empty",
        RigError::TooMany => "rig_create: at most 128 bones",
        RigError::BadId => "rig_create: bone ids use letters, digits, _ and - (rename the group first)",
        RigError::Duplicate => "rig_create: duplicate bone id",
        RigError::UnknownParent => "rig_create: parent is not a bone of this rig",
        RigError::Cycle => "rig_create: the parent chain has a cycle",
        RigError::BadPivot => "rig_create: pivot must be finite numbers within +-1000000000",
        RigError::UnknownRoleBone => "rig_create: a role names an id that is not a bone",
        RigError::DuplicateRole => "rig_create: a role is given twice",
    }
}

/// Whether `id`'s group lies inside the element `ancestor`.
fn inside(app: &AppDocument, id: &str, ancestor: &str) -> bool {
    let mut current = id.to_owned();
    for _ in 0..4096 {
        let Ok(info) = app.document().element_get(&current) else {
            return false;
        };
        match info.parent {
            Some(parent) if parent == ancestor => return true,
            Some(parent) => current = parent,
            None => return false,
        }
    }
    false
}

fn create(app: &mut AppDocument, spec: Create) -> Result<McpOutcome, &'static str> {
    check_room(app, 1)?;
    let mut bones = Vec::new();
    for bone in &spec.bones {
        let info = app
            .document()
            .element_get(&bone.id)
            .map_err(|_| "rig_create: every bone id must be an element id in the document")?;
        if info.tag != "g" {
            return Err("rig_create: every bone id must name a <g> group (use element_group)");
        }
        let pivot = match bone.pivot {
            Pivot::At(p) => p,
            Pivot::Center => {
                let (x0, y0, x1, y1) = info.bounds.ok_or(
                    "rig_create: cannot take the centre of a group with unsupported transforms; give pivot [x,y]",
                )?;
                [(x0 + x1) / 2.0, (y0 + y1) / 2.0]
            }
        };
        let parent = match &bone.parent {
            None => None,
            Some(id) => {
                let at = spec
                    .bones
                    .iter()
                    .position(|b| b.id == *id)
                    .ok_or("rig_create: parent is not a bone of this rig")?;
                if !inside(app, &bone.id, id) {
                    return Err("rig_create: a bone's group must lie inside its parent bone's group");
                }
                Some(at)
            }
        };
        bones.push(Bone {
            id: bone.id.clone(),
            pivot,
            parent,
        });
    }
    let mut roles = Vec::new();
    for (role, id) in &spec.roles {
        let at = spec
            .bones
            .iter()
            .position(|b| b.id == *id)
            .ok_or("rig_create: a role names an id that is not a bone")?;
        roles.push((*role, at));
    }
    roles.sort_by_key(|(role, _)| Role::ALL.iter().position(|r| r == role));
    let rig = Rig::new(bones, roles).map_err(rig_error)?;
    let xml = format!("<desc id=\"{RIG_ID}\">{}</desc>", to_json(&rig));
    app.edit(|doc| {
        if doc.element_get(RIG_ID).is_ok() {
            doc.delete(RIG_ID)?;
        }
        doc.add_fragment("root", xml.as_bytes()).map(|_| ())
    })
    .map_err(|e| match e {
        SessionEditError::Operation(_) => "rig_create: could not store the rig in the document",
        SessionEditError::History(_) => "project history unavailable",
    })?;
    if load(app).ok().as_ref() != Some(&rig) {
        app.undo().map_err(|_| "project history unavailable")?;
        return Err("rig_create: could not store the rig in the document");
    }
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"rig_create\",\"bones\":{},\"roles\":{}}}",
        rig.bones.len(),
        rig.roles.len()
    )))
}

/// The two writers share one path; this picks their error texts.
#[derive(Clone, Copy)]
enum Tool {
    Pose,
    Clip,
}

impl Tool {
    fn pick(self, pose: &'static str, clip: &'static str) -> &'static str {
        match self {
            Tool::Pose => pose,
            Tool::Clip => clip,
        }
    }
}

/// Keys of one bone, as written by `rig_pose` or `clip_apply`.
struct BoneKeys {
    bone: usize,
    keys: Vec<(u32, LocalPose, Option<Vec<u8>>)>,
}

/// Keys every bone of `items` in ONE timeline edit: a new `rig_<bone>` trs
/// track per bone (bulk path) or keys added to the bone's existing track.
/// Returns (tracks, keys).
fn write(
    app: &mut AppDocument,
    rig: &Rig,
    items: Vec<BoneKeys>,
    interpolation: &[u8],
    noise: Option<&[u8]>,
    tool: Tool,
) -> Result<(usize, usize), &'static str> {
    let mut merged: BTreeMap<usize, BTreeMap<u32, (LocalPose, Option<Vec<u8>>)>> = BTreeMap::new();
    for item in items {
        let keys = merged.entry(item.bone).or_default();
        for (tick, pose, ease) in item.keys {
            keys.insert(tick, (pose, ease));
        }
    }
    let total: usize = merged.values().map(BTreeMap::len).sum();
    if total == 0 {
        return Err(tool.pick(
            "rig_pose: poses name no bones",
            "clip_apply: the rig has no bones for this clip",
        ));
    }
    if total > MAX_KEYS {
        return Err(tool.pick(
            "rig_pose: at most 16384 keys per call",
            "clip_apply: at most 16384 keys per call; shorten the clip or split it",
        ));
    }
    for bone in merged.keys() {
        if app.document().element_get(&rig.bones[*bone].id).is_err() {
            return Err(tool.pick(
                "rig_pose: a bone's group no longer exists; call rig_create again",
                "clip_apply: a bone's group no longer exists; call rig_create again",
            ));
        }
    }
    let limits = EditLimits::default();
    let (existing, duration) = {
        let candidate = app
            .native_timeline_candidate(settings(), limits)
            .map_err(|_| "native timeline unavailable")?;
        let view = candidate.view().map_err(|_| "unsupported native timeline")?;
        let ids: Vec<Vec<u8>> = (0..view.track_count())
            .filter_map(|i| view.track_at(i))
            .map(|t| t.id.to_vec())
            .collect();
        (ids, view.duration_ticks())
    };
    if merged.values().any(|keys| keys.keys().any(|t| *t >= duration)) {
        return Err(tool.pick(
            "rig_pose: every tick must be below the timeline duration (lengthen it with native_timeline_set_duration)",
            "clip_apply: start + duration must be below the timeline duration (lengthen it with native_timeline_set_duration)",
        ));
    }
    struct New {
        id: Vec<u8>,
        name: Vec<u8>,
        target: Vec<u8>,
        value: Vec<u8>,
        keys: Vec<(u32, Vec<u8>, Option<Vec<u8>>)>,
    }
    let mut fresh: Vec<New> = Vec::new();
    let mut extend: Vec<(Vec<u8>, u32, Vec<u8>, Option<Vec<u8>>)> = Vec::new();
    for (bone, keys) in &merged {
        let pivot = rig.bones[*bone].pivot;
        let id = rig.track_id(*bone).into_bytes();
        let json = |pose: &LocalPose| pose.trs_json(pivot).into_bytes();
        if existing.contains(&id) {
            if noise.is_some() {
                return Err(tool.pick(
                    "rig_pose: noise only applies to bones whose track is created by this call",
                    "clip_apply: clips take no noise",
                ));
            }
            for (tick, (pose, ease)) in keys {
                extend.push((id.clone(), *tick, json(pose), ease.clone()));
            }
        } else {
            fresh.push(New {
                id,
                name: rig.bones[*bone].id.clone().into_bytes(),
                target: rig.bones[*bone].id.clone().into_bytes(),
                value: json(&keys.get(&0).map_or(LocalPose::REST, |k| k.0)),
                keys: keys
                    .iter()
                    .map(|(tick, (pose, ease))| (*tick, json(pose), ease.clone()))
                    .collect(),
            });
        }
    }
    let key_sets: Vec<Vec<BulkKey<'_>>> = fresh
        .iter()
        .map(|t| {
            t.keys
                .iter()
                .map(|(tick, value, ease)| BulkKey {
                    tick: *tick,
                    value,
                    ease: ease.as_deref(),
                })
                .collect()
        })
        .collect();
    let bulk: Vec<BulkTrack<'_>> = fresh
        .iter()
        .zip(&key_sets)
        .map(|(t, keys)| BulkTrack {
            id: &t.id,
            target: &t.target,
            property: b"transform",
            value_type: b"trs",
            interpolation,
            value: &t.value,
            name: &t.name,
            noise,
            keys,
        })
        .collect();
    let failure = Cell::new(None);
    app.edit_timeline(settings(), limits, |c| {
        let mut current: Option<ProjectTimelineCandidate> = None;
        if !bulk.is_empty() {
            current = Some(c.add_property_tracks(&bulk, &failure)?);
        }
        for (id, tick, value, ease) in &extend {
            current = Some(match current {
                None => c.set_property_key_with_ease(id, *tick, value, ease.as_deref())?,
                Some(prior) => prior.set_property_key_with_ease(id, *tick, value, ease.as_deref())?,
            });
        }
        current.ok_or(crate::project_session::ProjectTimelineError::Limit)
    })
    .map_err(|_| {
        tool.pick(
            "rig_pose: the timeline rejected the keys (track limit of 256 or size limit reached)",
            "clip_apply: the timeline rejected the keys (track limit of 256 or size limit reached)",
        )
    })?;
    Ok((merged.len(), total))
}

fn pose(app: &mut AppDocument, spec: Pose) -> Result<McpOutcome, &'static str> {
    let rig = load(app)?;
    let mut items: Vec<BoneKeys> = Vec::new();
    for key in spec.poses {
        for (id, local) in key.bones {
            let bone = rig
                .index(&id)
                .ok_or("rig_pose: a pose names a bone that is not in the rig (see rig_create)")?;
            match items.iter_mut().find(|i| i.bone == bone) {
                Some(item) => item.keys.push((key.tick, local, key.ease.clone())),
                None => items.push(BoneKeys {
                    bone,
                    keys: vec![(key.tick, local, key.ease.clone())],
                }),
            }
        }
    }
    let (tracks, keys) = write(app, &rig, items, &spec.interpolation, spec.noise.as_deref(), Tool::Pose)?;
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"rig_pose\",\"tracks\":{tracks},\"keys\":{keys}}}"
    )))
}

fn missing(role: Role) -> &'static str {
    match role {
        Role::Head => "clip_apply: this clip needs a head role (set roles.head in rig_create)",
        Role::Torso => "clip_apply: this clip needs a torso role (set roles.torso in rig_create)",
        Role::ArmL => "clip_apply: this clip needs an arm_l role (set roles.arm_l in rig_create)",
        Role::ArmR => "clip_apply: this clip needs an arm_r role (set roles.arm_r in rig_create)",
        Role::LegL => "clip_apply: this clip needs a leg_l role (set roles.leg_l in rig_create)",
        Role::LegR => "clip_apply: this clip needs a leg_r role (set roles.leg_r in rig_create)",
        Role::Tail => "clip_apply: this clip needs a tail role (set roles.tail in rig_create)",
    }
}

fn clip(app: &mut AppDocument, call: ClipCall) -> Result<McpOutcome, &'static str> {
    let rig = load(app)?;
    let tracks = generate(call.clip, &call.params, &|role| rig.role(role).is_some()).map_err(
        |e| match e {
            ClipError::BadParams => {
                "clip_apply: duration 1-1000000, loops 1-64, speed 0.25-4, amplitude 0-4 and start + duration within 32 bits are required"
            }
            ClipError::MissingRole(role) => missing(role),
        },
    )?;
    let items = tracks
        .into_iter()
        .filter_map(|t| {
            Some(BoneKeys {
                bone: rig.role(t.role)?,
                keys: t
                    .keys
                    .into_iter()
                    .map(|k| (k.tick, k.pose, call.ease.clone()))
                    .collect(),
            })
        })
        .collect();
    let (tracks, keys) = write(app, &rig, items, b"linear", None, Tool::Clip)?;
    Ok(McpOutcome::Reply(format!(
        "{{\"ok\":true,\"operation\":\"clip_apply\",\"clip\":\"{}\",\"tracks\":{tracks},\"keys\":{keys}}}",
        call.clip.name()
    )))
}

pub(crate) fn apply(app: &mut AppDocument, op: Op) -> Result<McpOutcome, &'static str> {
    match op {
        Op::Create(spec) => create(app, spec),
        Op::Pose(spec) => pose(app, spec),
        Op::Clip(call) => clip(app, call),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trips() {
        let rig = Rig::new(
            vec![
                Bone { id: "body".into(), pivot: [1.5, -2.0], parent: None },
                Bone { id: "arm".into(), pivot: [0.0, 4.0], parent: Some(0) },
            ],
            vec![(Role::Torso, 0), (Role::ArmR, 1)],
        )
        .unwrap();
        let text = to_json(&rig);
        assert_eq!(from_json(text.as_bytes()), Some(rig));
        let xml = format!("<svg><desc id=\"vexel-rig\">{text}</desc></svg>");
        assert_eq!(stored_text(xml.as_bytes()), Some(text.as_bytes()));
    }

    #[test]
    fn malformed_stored_rigs_are_rejected() {
        for bad in [
            "",
            "{}",
            "{\"v\":2,\"bones\":[],\"roles\":{}}",
            "{\"v\":1,\"bones\":[],\"roles\":{}}",
            "{\"v\":1,\"bones\":[{\"id\":\"a\",\"pivot\":[0]}],\"roles\":{}}",
            "{\"v\":1,\"bones\":[{\"id\":\"a\",\"pivot\":[0,0],\"parent\":\"a\"}],\"roles\":{}}",
            "{\"v\":1,\"bones\":[{\"id\":\"a\",\"pivot\":[0,0]}],\"roles\":{\"head\":\"zz\"}}",
        ] {
            assert_eq!(from_json(bad.as_bytes()), None, "{bad}");
        }
    }
}
