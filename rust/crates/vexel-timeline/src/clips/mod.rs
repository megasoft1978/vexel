//! Parametric procedural clips: pure functions from parameters to per-role
//! local pose keys. Deterministic (no clocks, no randomness); a looped clip
//! ends on exactly the pose it starts on.

mod gestures;
mod locomotion;
#[cfg(test)]
mod tests;

use crate::rig::{LocalPose, Role};

/// Most cycles one clip may span.
pub const MAX_CYCLES: u32 = 64;
/// Longest clip in ticks.
pub const MAX_DURATION: u32 = 1_000_000;
/// Largest amplitude multiplier.
pub const MAX_AMPLITUDE: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clip {
    Walk,
    Run,
    Idle,
    TalkBody,
    BlinkBob,
    Wave,
    Jump,
}

impl Clip {
    pub const ALL: [Clip; 7] = [
        Clip::Walk,
        Clip::Run,
        Clip::Idle,
        Clip::TalkBody,
        Clip::BlinkBob,
        Clip::Wave,
        Clip::Jump,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Clip::Walk => "walk",
            Clip::Run => "run",
            Clip::Idle => "idle",
            Clip::TalkBody => "talk_body",
            Clip::BlinkBob => "blink_bob",
            Clip::Wave => "wave",
            Clip::Jump => "jump",
        }
    }

    pub fn parse(name: &str) -> Option<Clip> {
        match name {
            "blink-body-bob" => Some(Clip::BlinkBob),
            other => Clip::ALL.into_iter().find(|c| c.name() == other),
        }
    }

    /// Roles the clip cannot run without.
    pub const fn required(self) -> &'static [Role] {
        match self {
            Clip::Walk | Clip::Run => &[Role::LegL, Role::LegR],
            Clip::Idle | Clip::Jump => &[Role::Torso],
            Clip::TalkBody | Clip::BlinkBob => &[Role::Head],
            Clip::Wave => &[Role::ArmR],
        }
    }

    /// Samples per cycle.
    const fn samples(self) -> u32 {
        match self {
            Clip::Wave => 16,
            Clip::Jump => 12,
            _ => 8,
        }
    }

    /// The pose of `role` at `phase` (0..=1) or `None` when the clip leaves it alone.
    fn pose(self, role: Role, phase: f64, amp: f64) -> Option<LocalPose> {
        match self {
            Clip::Walk | Clip::Run => locomotion::pose(self == Clip::Run, role, phase, amp),
            other => gestures::pose(other, role, phase, amp),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipMode {
    /// The cycle repeats; the last key equals the first.
    Loop,
    /// Each cycle plays forward then backward.
    PingPong,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipParams {
    pub start: u32,
    pub duration: u32,
    /// Cycles over the duration.
    pub loops: u32,
    /// Multiplies `loops`; cycles = round(loops * speed), at least 1.
    pub speed: f64,
    pub amplitude: f64,
    pub mode: ClipMode,
}

impl Default for ClipParams {
    fn default() -> Self {
        ClipParams {
            start: 0,
            duration: 24,
            loops: 1,
            speed: 1.0,
            amplitude: 1.0,
            mode: ClipMode::Loop,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipKey {
    pub tick: u32,
    pub pose: LocalPose,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoleKeys {
    pub role: Role,
    pub keys: Vec<ClipKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipError {
    BadParams,
    /// A role the clip needs is not in the rig.
    MissingRole(Role),
}

/// Number of cycles the parameters describe.
pub fn cycles(params: &ClipParams) -> u32 {
    let n = (f64::from(params.loops) * params.speed).round();
    (n as u32).clamp(1, MAX_CYCLES)
}

fn round4(v: f64) -> f64 {
    (v * 1.0e4).round() / 1.0e4 + 0.0
}

fn clean(pose: LocalPose) -> LocalPose {
    LocalPose {
        rotate: round4(pose.rotate),
        translate: [round4(pose.translate[0]), round4(pose.translate[1])],
        scale: [round4(pose.scale[0]), round4(pose.scale[1])],
    }
}

/// Keys of every role the clip animates and `has` reports present, in
/// `Role::ALL` order. Keys are strictly increasing in tick.
pub fn generate(
    clip: Clip,
    params: &ClipParams,
    has: &dyn Fn(Role) -> bool,
) -> Result<Vec<RoleKeys>, ClipError> {
    let ok = (1..=MAX_DURATION).contains(&params.duration)
        && (1..=MAX_CYCLES).contains(&params.loops)
        && params.speed.is_finite()
        && (0.25..=4.0).contains(&params.speed)
        && params.amplitude.is_finite()
        && (0.0..=MAX_AMPLITUDE).contains(&params.amplitude)
        && params.start.checked_add(params.duration).is_some();
    if !ok {
        return Err(ClipError::BadParams);
    }
    if let Some(missing) = clip.required().iter().find(|r| !has(**r)) {
        return Err(ClipError::MissingRole(*missing));
    }
    let samples = clip.samples();
    let half = u64::from(cycles(params)) * u64::from(samples);
    let per_cycle = match params.mode {
        ClipMode::Loop => samples,
        ClipMode::PingPong => samples * 2,
    };
    let total = half * u64::from(per_cycle / samples);
    let mut out = Vec::new();
    for role in Role::ALL {
        if !has(role) || clip.pose(role, 0.0, 1.0).is_none() {
            continue;
        }
        let mut keys: Vec<ClipKey> = Vec::new();
        for i in 0..=total {
            let tick = params.start
                + ((u64::from(params.duration) * i + total / 2) / total) as u32;
            if keys.last().is_some_and(|k| k.tick == tick) {
                continue;
            }
            let j = (i % u64::from(per_cycle)) as u32;
            let phase = match params.mode {
                ClipMode::Loop => f64::from(j) / f64::from(samples),
                ClipMode::PingPong => f64::from(if j <= samples { j } else { per_cycle - j })
                    / f64::from(samples),
            };
            let pose = clip
                .pose(role, phase, params.amplitude)
                .unwrap_or(LocalPose::REST);
            keys.push(ClipKey {
                tick,
                pose: clean(pose),
            });
        }
        out.push(RoleKeys { role, keys });
    }
    Ok(out)
}
