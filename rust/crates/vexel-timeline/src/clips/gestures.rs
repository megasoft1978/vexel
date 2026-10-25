//! Idle breathing, body talk, blink bob, wave and jump.

use super::Clip;
use crate::rig::{LocalPose, Role};
use std::f64::consts::{PI, TAU};

fn rot(rotate: f64) -> LocalPose {
    LocalPose { rotate, ..LocalPose::REST }
}

fn shift(y: f64) -> LocalPose {
    LocalPose { translate: [0.0, y], ..LocalPose::REST }
}

pub(super) fn pose(clip: Clip, role: Role, phase: f64, a: f64) -> Option<LocalPose> {
    let t = TAU * phase;
    let up = (PI * phase).sin(); // 0 at both ends
    let bump = 0.5 - 0.5 * t.cos(); // 0 at both ends
    Some(match (clip, role) {
        (Clip::Idle, Role::Torso) => LocalPose {
            rotate: 0.0,
            translate: [0.0, -0.5 * a * t.sin()],
            scale: [1.0 - 0.01 * a * t.sin(), 1.0 + 0.02 * a * t.sin()],
        },
        (Clip::Idle, Role::Head) => rot(1.5 * a * (t - 0.8).sin()),
        (Clip::Idle, Role::ArmL) => rot(2.0 * a * (t - 0.4).sin()),
        (Clip::Idle, Role::ArmR) => rot(-2.0 * a * (t - 0.4).sin()),
        (Clip::Idle, Role::Tail) => rot(5.0 * a * (t - 1.2).sin()),
        (Clip::TalkBody, Role::Head) => LocalPose {
            rotate: 4.0 * a * t.sin(),
            translate: [0.0, 0.8 * a * (2.0 * t).cos()],
            ..LocalPose::REST
        },
        (Clip::TalkBody, Role::Torso) => rot(1.5 * a * (t - 0.5).sin()),
        (Clip::TalkBody, Role::ArmL) => rot(10.0 * a * (t + 1.0).sin()),
        (Clip::TalkBody, Role::ArmR) => rot(-10.0 * a * (t + 2.0).sin()),
        (Clip::BlinkBob, Role::Head) => shift(2.5 * a * bump),
        (Clip::BlinkBob, Role::Torso) => LocalPose {
            translate: [0.0, a * bump],
            scale: [1.0, 1.0 - 0.02 * a * bump],
            ..LocalPose::REST
        },
        (Clip::Wave, Role::ArmR) => rot(-110.0 * a * up + 18.0 * a * (2.0 * t).sin() * up),
        (Clip::Wave, Role::Head) => rot(3.0 * a * up),
        (Clip::Wave, Role::Torso) => rot(-2.0 * a * up),
        (Clip::Jump, Role::Torso) => LocalPose {
            translate: [0.0, -36.0 * a * up],
            scale: [1.0, 1.0 + 0.06 * a * (2.0 * t).sin()],
            ..LocalPose::REST
        },
        (Clip::Jump, Role::LegL) => rot(18.0 * a * (2.0 * t).sin()),
        (Clip::Jump, Role::LegR) => rot(-18.0 * a * (2.0 * t).sin()),
        (Clip::Jump, Role::ArmL) => rot(-70.0 * a * up),
        (Clip::Jump, Role::ArmR) => rot(70.0 * a * up),
        _ => return None,
    })
}
