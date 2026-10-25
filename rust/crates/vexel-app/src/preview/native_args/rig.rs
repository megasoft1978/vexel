//! Arguments of `rig_create`, `rig_pose` and `clip_apply`: bounded, with
//! messages that name the offending member.

use super::super::add_extras::number;
use super::super::ext_structure::document;
use super::{JsonDocument, JsonType, PreviewArgumentError};
use vexel_timeline::clips::{Clip, ClipMode, ClipParams};
use vexel_timeline::rig::{LocalPose, MAX_BONES, MAX_COORD, Role, valid_bone_id};

/// Most poses one `rig_pose` call carries.
pub const MAX_POSES: usize = 256;
/// Most keys (bone poses) one `rig_pose` or `clip_apply` call writes.
pub const MAX_KEYS: usize = 16_384;

/// Where a bone rotates about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pivot {
    At([f64; 2]),
    /// The centre of the group's bounds when the rig is created.
    Center,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoneSpec {
    pub id: String,
    pub pivot: Pivot,
    pub parent: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Create {
    pub bones: Vec<BoneSpec>,
    pub roles: Vec<(Role, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PoseKey {
    pub tick: u32,
    pub ease: Option<Vec<u8>>,
    pub bones: Vec<(String, LocalPose)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub poses: Vec<PoseKey>,
    pub interpolation: Vec<u8>,
    pub noise: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClipCall {
    pub clip: Clip,
    pub params: ClipParams,
    pub ease: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Create(Create),
    Pose(Pose),
    Clip(ClipCall),
}

fn invalid(tool: &str, argument: &str, form: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(format!(
        "{tool} argument {argument} is missing or invalid; use {form}"
    ))
}

/// `(key, value)` token pairs of an object, at most `max`.
fn members(doc: &JsonDocument<'_>, object: usize, max: usize) -> Option<Vec<(String, usize)>> {
    let token = doc.token(object).filter(|t| t.kind == JsonType::Object)?;
    if token.child_count / 2 > max {
        return None;
    }
    let mut out = Vec::new();
    let mut key = token.first_child;
    while let Some(k) = key {
        let value = doc.token(k)?.next_sibling?;
        let name = String::from_utf8(doc.decoded_string(k)?).ok()?;
        out.push((name, value));
        key = doc.token(value)?.next_sibling;
    }
    Some(out)
}

fn id_at(doc: &JsonDocument<'_>, token: usize) -> Option<String> {
    let raw = doc.raw_token(token)?;
    if raw.len() > 6 * 120 + 2 {
        return None;
    }
    String::from_utf8(doc.decoded_string(token)?)
        .ok()
        .filter(|s| valid_bone_id(s))
}

fn pivot(doc: &JsonDocument<'_>, token: usize) -> Option<Pivot> {
    let item = doc.token(token)?;
    match item.kind {
        JsonType::String if doc.string_equals(token, b"center") => Some(Pivot::Center),
        JsonType::Array if item.child_count == 2 => {
            let x = number(doc, doc.array_get(token, 0)?)?;
            let y = number(doc, doc.array_get(token, 1)?)?;
            (x.abs() <= MAX_COORD && y.abs() <= MAX_COORD).then_some(Pivot::At([x, y]))
        }
        _ => None,
    }
}

fn create(doc: &JsonDocument<'_>, args: usize, tool: &str) -> Result<Op, PreviewArgumentError> {
    document(doc, args, tool)?;
    let bones_form = "1-128 objects {id, pivot?: [x,y] or \"center\", parent?: bone id}, ids using letters, digits, _ and -";
    let array = doc
        .object_get(args, b"bones")
        .and_then(|at| doc.token(at))
        .filter(|t| t.kind == JsonType::Array && (1..=MAX_BONES).contains(&t.child_count))
        .ok_or_else(|| invalid(tool, "bones", bones_form))?;
    let mut bones = Vec::new();
    let mut item = array.first_child;
    while let Some(at) = item {
        let bad = |what: &str| invalid(tool, &format!("bones[{}].{what}", bones.len()), bones_form);
        let fields = members(doc, at, 3).ok_or_else(|| bad("(object)"))?;
        let mut spec = BoneSpec {
            id: String::new(),
            pivot: Pivot::Center,
            parent: None,
        };
        for (name, value) in fields {
            match name.as_str() {
                "id" => spec.id = id_at(doc, value).ok_or_else(|| bad("id"))?,
                "pivot" => spec.pivot = pivot(doc, value).ok_or_else(|| bad("pivot"))?,
                "parent" => spec.parent = Some(id_at(doc, value).ok_or_else(|| bad("parent"))?),
                _ => return Err(bad("(unknown member)")),
            }
        }
        if spec.id.is_empty() {
            return Err(bad("id"));
        }
        bones.push(spec);
        item = doc.token(at).and_then(|t| t.next_sibling);
    }
    let mut roles = Vec::new();
    if let Some(at) = doc.object_get(args, b"roles") {
        let form = "an object mapping head, torso, arm_l, arm_r, leg_l, leg_r or tail to a bone id";
        let fields =
            members(doc, at, Role::ALL.len()).ok_or_else(|| invalid(tool, "roles", form))?;
        for (name, value) in fields {
            let role = Role::parse(&name).ok_or_else(|| invalid(tool, "roles", form))?;
            let id = id_at(doc, value).ok_or_else(|| invalid(tool, "roles", form))?;
            roles.push((role, id));
        }
    }
    Ok(Op::Create(Create { bones, roles }))
}

/// The shared parsers name `native_timeline_<tool>`; these tools are not prefixed.
fn plain(error: PreviewArgumentError) -> PreviewArgumentError {
    match error {
        PreviewArgumentError::Actionable(m) => {
            PreviewArgumentError::Actionable(m.replace("native_timeline_", ""))
        }
        other => other,
    }
}

fn uint32(doc: &JsonDocument<'_>, at: usize) -> Option<u32> {
    doc.token(at)
        .filter(|t| t.kind == JsonType::Number)
        .and_then(|_| doc.uint(at))
        .and_then(|v| u32::try_from(v).ok())
}

fn ranged(doc: &JsonDocument<'_>, at: usize, limit: f64) -> Option<f64> {
    number(doc, at).filter(|v| v.abs() <= limit)
}

fn pair(doc: &JsonDocument<'_>, at: usize, limit: f64) -> Option<[f64; 2]> {
    let item = doc
        .token(at)
        .filter(|t| t.kind == JsonType::Array && t.child_count == 2)?;
    let _ = item;
    Some([
        ranged(doc, doc.array_get(at, 0)?, limit)?,
        ranged(doc, doc.array_get(at, 1)?, limit)?,
    ])
}

const POSE_FORM: &str = "{rotate?: degrees, translate?: [x,y], scale?: number or [x,y]}";

fn local_pose(
    doc: &JsonDocument<'_>,
    at: usize,
    tool: &str,
) -> Result<LocalPose, PreviewArgumentError> {
    let bad = |what: &str| invalid(tool, &format!("poses[].bones.{what}"), POSE_FORM);
    let mut pose = LocalPose::REST;
    for (name, value) in members(doc, at, 3).ok_or_else(|| bad("(pose)"))? {
        match name.as_str() {
            "rotate" => pose.rotate = ranged(doc, value, 36_000.0).ok_or_else(|| bad("rotate"))?,
            "translate" => {
                pose.translate = pair(doc, value, 1.0e6).ok_or_else(|| bad("translate"))?
            }
            "scale" => {
                pose.scale = match ranged(doc, value, 1.0e4) {
                    Some(v) => [v, v],
                    None => pair(doc, value, 1.0e4).ok_or_else(|| bad("scale"))?,
                }
            }
            _ => return Err(bad("(unknown member)")),
        }
    }
    Ok(pose)
}

fn pose(doc: &JsonDocument<'_>, args: usize, tool: &str) -> Result<Op, PreviewArgumentError> {
    document(doc, args, tool)?;
    let form = "1-256 objects {tick, bones: {boneId: {rotate?, translate?, scale?}}, ease?}";
    let array = doc
        .object_get(args, b"poses")
        .and_then(|at| doc.token(at))
        .filter(|t| t.kind == JsonType::Array && (1..=MAX_POSES).contains(&t.child_count))
        .ok_or_else(|| invalid(tool, "poses", form))?;
    let mut poses = Vec::new();
    let mut keys = 0usize;
    let mut item = array.first_child;
    while let Some(at) = item {
        let bad = |what: &str| invalid(tool, &format!("poses[{}].{what}", poses.len()), form);
        let mut tick = None;
        let mut bones = Vec::new();
        for (name, value) in members(doc, at, 3).ok_or_else(|| bad("(object)"))? {
            match name.as_str() {
                "tick" => tick = Some(uint32(doc, value).ok_or_else(|| bad("tick"))?),
                "ease" => {}
                "bones" => {
                    for (id, pose_at) in
                        members(doc, value, MAX_BONES).ok_or_else(|| bad("bones"))?
                    {
                        if !valid_bone_id(&id) {
                            return Err(bad("bones (a bone id)"));
                        }
                        bones.push((id, local_pose(doc, pose_at, tool)?));
                    }
                }
                _ => return Err(bad("(unknown member)")),
            }
        }
        keys += bones.len();
        if keys > MAX_KEYS {
            return Err(invalid(tool, "poses", "at most 16384 bone poses per call"));
        }
        poses.push(PoseKey {
            tick: tick.ok_or_else(|| bad("tick"))?,
            ease: super::ease_at(doc, at, tool, "ease").map_err(plain)?,
            bones,
        });
        item = doc.token(at).and_then(|t| t.next_sibling);
    }
    let interpolation = match doc.object_get(args, b"interpolation") {
        None => b"linear".to_vec(),
        Some(at) => doc
            .decoded_string(at)
            .filter(|v| [b"step".as_slice(), b"linear", b"cubic"].contains(&v.as_slice()))
            .ok_or_else(|| invalid(tool, "interpolation", "step, linear or cubic"))?,
    };
    let noise = super::noise::noise_arg(doc, args, tool).map_err(plain)?;
    Ok(Op::Pose(Pose {
        poses,
        interpolation,
        noise,
    }))
}

fn clip(doc: &JsonDocument<'_>, args: usize, tool: &str) -> Result<Op, PreviewArgumentError> {
    document(doc, args, tool)?;
    let name_form = "walk, run, idle, talk_body, blink_bob, wave or jump";
    let clip = doc
        .object_get(args, b"clip")
        .and_then(|at| doc.decoded_string(at))
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|n| Clip::parse(&n))
        .ok_or_else(|| invalid(tool, "clip", name_form))?;
    let int = |key: &'static str, form: &str| {
        doc.object_get(args, key.as_bytes())
            .map(|at| uint32(doc, at).ok_or_else(|| invalid(tool, key, form)))
            .transpose()
    };
    let float = |key: &'static str, form: &str| {
        doc.object_get(args, key.as_bytes())
            .map(|at| number(doc, at).ok_or_else(|| invalid(tool, key, form)))
            .transpose()
    };
    let mut params = ClipParams::default();
    params.duration = int("duration", "an integer of ticks, 1-1000000")?
        .ok_or_else(|| invalid(tool, "duration", "an integer of ticks, 1-1000000"))?;
    if let Some(v) = int("start", "a tick")? {
        params.start = v;
    }
    if let Some(v) = int("loops", "an integer 1-64")? {
        params.loops = v;
    }
    if let Some(v) = float("speed", "a number 0.25-4")? {
        params.speed = v;
    }
    if let Some(v) = float("amplitude", "a number 0-4")? {
        params.amplitude = v;
    }
    if let Some(at) = doc.object_get(args, b"mode") {
        params.mode = match doc.decoded_string(at).as_deref() {
            Some(b"loop") => ClipMode::Loop,
            Some(b"pingpong") => ClipMode::PingPong,
            _ => return Err(invalid(tool, "mode", "loop or pingpong")),
        };
    }
    let ease = super::ease_at(doc, args, tool, "ease").map_err(plain)?;
    Ok(Op::Clip(ClipCall { clip, params, ease }))
}

/// `None` when `tool` is not a rig tool.
pub(in crate::preview) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Op, PreviewArgumentError>> {
    Some(match tool {
        "rig_create" => create(doc, args, tool),
        "rig_pose" => pose(doc, args, tool),
        "clip_apply" => clip(doc, args, tool),
        _ => return None,
    })
}
