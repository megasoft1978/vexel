//! A character group with three mouth groups, shared by the face tests.
#![allow(dead_code)]

use super::common::{Rig, element_id};

pub struct Face {
    pub character: String,
    pub rest: String,
    pub a: String,
    pub o: String,
}

fn rect_group(rig: &mut Rig, (x, y, w, h): (u32, u32, u32, u32), fill: &str) -> String {
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

/// Rest is a thin green line, A a tall red box and O a blue square, all in one
/// character group on a white page.
pub fn build(rig: &mut Rig) -> Face {
    rect_group(rig, (0, 0, 512, 512), "#ffffff");
    let rest = rect_group(rig, (200, 300, 80, 4), "#00aa00");
    let a = rect_group(rig, (200, 280, 80, 60), "#dd0000");
    let o = rect_group(rig, (210, 280, 60, 60), "#0000dd");
    let character = element_id(
        rig.doc_tool(
            "element_group",
            &format!(r#""elementIds":["{rest}","{a}","{o}"]"#),
        )
        .ok(),
    );
    Face {
        character,
        rest,
        a,
        o,
    }
}

pub fn shapes(f: &Face) -> String {
    format!(
        r#""character":"{}","shapes":{{"rest":"{}","A":"{}","O":"{}"}}"#,
        f.character, f.rest, f.a, f.o
    )
}
