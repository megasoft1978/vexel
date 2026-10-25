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

pub struct Eyes {
    pub eye: String,
    pub pupil: String,
    pub lid: String,
    pub open: String,
    pub closed: String,
}

fn shape(rig: &mut Rig, template: &str, attrs: &str) -> String {
    element_id(
        rig.doc_tool(
            "element_add",
            &format!(r#""parentId":"root","template":"{template}","attributes":{{{attrs}}}"#),
        )
        .ok(),
    )
}

fn wrap(rig: &mut Rig, id: &str) -> String {
    element_id(
        rig.doc_tool("element_group", &format!(r#""elementId":"{id}""#))
            .ok(),
    )
}

/// White page, a grey elliptical eye at (100,100), a black pupil square in it
/// and a skin-coloured lid box over it; plus open (green) and closed (red)
/// eye pictures elsewhere. With `pupil` false the pupil is left out.
pub fn build_eyes(rig: &mut Rig, pupil: bool) -> Eyes {
    shape(
        rig,
        "rect",
        r##""x":"0","y":"0","width":"512","height":"512","fill":"#ffffff""##,
    );
    let eye = shape(
        rig,
        "ellipse",
        r##""cx":"100","cy":"100","rx":"30","ry":"20","fill":"#cccccc""##,
    );
    let eye = wrap(rig, &eye);
    let p = shape(
        rig,
        "rect",
        r##""x":"90","y":"90","width":"20","height":"20","fill":"#000000""##,
    );
    let pupil_id = if pupil {
        wrap(rig, &p)
    } else {
        rig.doc_tool("element_delete", &format!(r#""elementId":"{p}""#))
            .ok();
        String::new()
    };
    let lid = shape(
        rig,
        "rect",
        r##""x":"70","y":"80","width":"60","height":"40","fill":"#ffcc99""##,
    );
    let lid = wrap(rig, &lid);
    let open = shape(
        rig,
        "rect",
        r##""x":"300","y":"100","width":"40","height":"40","fill":"#00aa00""##,
    );
    let open = wrap(rig, &open);
    let closed = shape(
        rig,
        "rect",
        r##""x":"300","y":"100","width":"40","height":"10","fill":"#dd0000""##,
    );
    let closed = wrap(rig, &closed);
    Eyes {
        eye,
        pupil: pupil_id,
        lid,
        open,
        closed,
    }
}
