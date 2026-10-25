//! Walk and run cycles: opposite legs and arms swing, the torso bobs twice per cycle.

use crate::rig::{LocalPose, Role};
use std::f64::consts::TAU;

fn rot(rotate: f64) -> LocalPose {
    LocalPose { rotate, ..LocalPose::REST }
}

pub(super) fn pose(run: bool, role: Role, phase: f64, a: f64) -> Option<LocalPose> {
    let t = TAU * phase;
    let (leg, arm, bob, lean, tail) = if run {
        (48.0, 40.0, 4.0, 6.0, 15.0)
    } else {
        (28.0, 20.0, 1.5, 0.0, 8.0)
    };
    Some(match role {
        Role::LegL => rot(leg * a * t.sin()),
        Role::LegR => rot(-leg * a * t.sin()),
        Role::ArmL => rot(-arm * a * t.sin()),
        Role::ArmR => rot(arm * a * t.sin()),
        Role::Torso => LocalPose {
            rotate: lean * a + 2.0 * a * t.sin(),
            translate: [0.0, bob * a * (2.0 * t).cos()],
            ..LocalPose::REST
        },
        Role::Head => rot(-2.0 * a * t.sin()),
        Role::Tail => rot(-tail * a * (t - 0.5).cos()),
    })
}
