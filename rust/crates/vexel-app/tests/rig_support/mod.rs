//! A six-group stick character built through MCP, shared by the rig tests.
#![allow(dead_code)]

use super::common::{Rig, element_id};

pub struct Stick {
    pub torso: String,
    pub head: String,
    pub arm_l: String,
    pub arm_r: String,
    pub leg_l: String,
    pub leg_r: String,
}

fn part(rig: &mut Rig, (x, y, w, h): (u32, u32, u32, u32), fill: &str) -> String {
    let rect = element_id(
        rig.doc_tool(
            "element_add",
            &format!(
                r#""parentId":"root","template":"rect","attributes":{{"x":"{x}","y":"{y}","width":"{w}","height":"{h}","fill":"{fill}"}}"#
            ),
        )
        .ok(),
    );
    element_id(
        rig.doc_tool("element_group", &format!(r#""elementId":"{rect}""#))
            .ok(),
    )
}

/// Torso group holding head, arms and legs groups.
pub fn build(rig: &mut Rig) -> Stick {
    let torso = part(rig, (140, 100, 20, 80), "#336699");
    let head = part(rig, (140, 70, 20, 20), "#cc9966");
    let arm_l = part(rig, (100, 105, 40, 8), "#cc3300");
    let arm_r = part(rig, (160, 105, 40, 8), "#cc3300");
    let leg_l = part(rig, (140, 180, 8, 60), "#228822");
    let leg_r = part(rig, (152, 180, 8, 60), "#228822");
    for child in [&head, &arm_l, &arm_r, &leg_l, &leg_r] {
        rig.doc_tool(
            "element_reparent",
            &format!(r#""elementId":"{child}","parentId":"{torso}""#),
        )
        .ok();
    }
    Stick {
        torso,
        head,
        arm_l,
        arm_r,
        leg_l,
        leg_r,
    }
}

/// `bones` and `roles` members for `rig_create`.
pub fn rig_args(s: &Stick) -> String {
    rig_args_with(s, true)
}

/// With `body` false only the four limbs get roles, so a clip moves limbs alone.
pub fn rig_args_with(s: &Stick, body: bool) -> String {
    let all = rig_args_full(s);
    if body {
        return all;
    }
    let cut = all.find(",\"roles\"").unwrap();
    format!(
        r#"{}, "roles":{{"arm_l":"{}","arm_r":"{}","leg_l":"{}","leg_r":"{}"}}"#,
        &all[..cut],
        s.arm_l,
        s.arm_r,
        s.leg_l,
        s.leg_r
    )
}

fn rig_args_full(s: &Stick) -> String {
    format!(
        r#""bones":[{{"id":"{t}"}},{{"id":"{h}","parent":"{t}"}},{{"id":"{al}","pivot":[140,109],"parent":"{t}"}},{{"id":"{ar}","pivot":[160,109],"parent":"{t}"}},{{"id":"{ll}","pivot":[144,180],"parent":"{t}"}},{{"id":"{lr}","pivot":[156,180],"parent":"{t}"}}],"roles":{{"torso":"{t}","head":"{h}","arm_l":"{al}","arm_r":"{ar}","leg_l":"{ll}","leg_r":"{lr}"}}"#,
        t = s.torso,
        h = s.head,
        al = s.arm_l,
        ar = s.arm_r,
        ll = s.leg_l,
        lr = s.leg_r
    )
}
