//! TRS keys over MCP: every documented value form is accepted on tracks that
//! target a group or an element nested in a group, bad values name the
//! offending field, and scenes with nested opacity (group opacity plus a child
//! opacity) render and export, with stills that differ by time.

mod common;

use common::{Dir, Rig, element_id, policy_for};
use std::fs;

fn rect(rig: &mut Rig, parent: &str, x: u32, extra: &str) -> String {
    element_id(
        rig.doc_tool(
            "element_add",
            &format!(
                r##""parentId":"{parent}","template":"rect","attributes":{{"x":"{x}","y":"100","width":"200","height":"40","fill":"#cc3300"{extra}}}"##
            ),
        )
        .ok(),
    )
}

fn png(rig: &mut Rig, dir: &Dir, name: &str, time: &str) -> Vec<u8> {
    let path = dir.text(name);
    rig.doc_tool("render_png", &format!(r#""path":"{path}","time":{time}"#))
        .ok();
    fs::read(dir.file(name)).unwrap()
}

fn add_trs(rig: &mut Rig, track: &str, target: &str) {
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"{track}","targetId":"{target}","property":"transform","valueType":"trs","interpolation":"linear","value":{{"pivot":[250,120]}}"#
        ),
    )
    .ok();
}

fn key(rig: &mut Rig, track: &str, tick: u32, value: &str) {
    rig.doc_tool(
        "native_timeline_set_property_key",
        &format!(r#""trackId":"{track}","tick":{tick},"value":{value}"#),
    )
    .ok();
}

#[test]
fn trs_object_and_array_forms_work_on_group_and_nested_targets() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let anchor = rect(&mut rig, "root", 20, "");
    let outer = element_id(
        rig.doc_tool("element_group", &format!(r#""elementId":"{anchor}""#))
            .ok(),
    );
    let child = rect(&mut rig, &outer, 150, "");
    let inner = element_id(
        rig.doc_tool("element_group", &format!(r#""elementId":"{child}""#))
            .ok(),
    );
    let leaf = rect(&mut rig, &inner, 90, "");
    for (track, target) in [("outer", &outer), ("inner", &inner), ("leaf", &leaf)] {
        add_trs(&mut rig, track, target);
    }
    // Object forms: nested pairs, flat names, `{x,y}` pairs and scalar scale.
    key(
        &mut rig,
        "outer",
        24,
        r#"{"translate":[10,-4],"rotate":15,"pivot":[250,120]}"#,
    );
    key(
        &mut rig,
        "inner",
        24,
        r#"{"translateX":5,"translateY":2,"rotation":30,"scaleX":1.1,"scaleY":0.9,"pivotX":250,"pivotY":120}"#,
    );
    key(
        &mut rig,
        "leaf",
        24,
        r#"{"translate":{"x":3,"y":3},"scale":1.5,"skewX":5,"pivot":{"x":200,"y":120}}"#,
    );
    // Array forms (7 and 8 numbers) still work, with and without valueType.
    key(&mut rig, "outer", 40, "[0,0,0,1,1,250,120]");
    key(&mut rig, "leaf", 40, "[0,0,0,1,1,200,120,0]");
    let start = png(&mut rig, &dir, "a.png", "0");
    let mid = png(&mut rig, &dir, "b.png", "1");
    assert_ne!(start, mid, "TRS keys change the still");
    // An object with no parts is the identity transform.
    key(&mut rig, "outer", 44, "{}");
}

#[test]
fn bad_trs_values_name_the_offending_field() {
    let mut rig = Rig::new(policy_for(&[]));
    let shape = rect(&mut rig, "root", 20, "");
    add_trs(&mut rig, "swing", &shape);
    let cases = [
        (r#"{"bogus":1}"#, "value.bogus"),
        (r#"{"rotate":"x"}"#, "value.rotate"),
        (r#"{"scale":[1]}"#, "value.scale"),
        (r#"{"pivot":{"x":1}}"#, "value.pivot"),
        (r#"[1,2,3]"#, "value"),
        (r#""text""#, "`value`"),
    ];
    for (value, field) in cases {
        let reply = rig.doc_tool(
            "native_timeline_set_property_key",
            &format!(r#""trackId":"swing","tick":4,"value":{value}"#),
        );
        assert!(reply.is_error, "{value}");
        assert!(reply.body.contains(field), "{value}: {}", reply.body);
    }
}

#[test]
fn nested_opacity_renders_exports_and_differs_by_time() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let plain = rect(&mut rig, "root", 20, "");
    let group = element_id(
        rig.doc_tool("element_group", &format!(r#""elementId":"{plain}""#))
            .ok(),
    );
    // A plain child first, then one with its own opacity: nested opacity.
    let faded = rect(&mut rig, &group, 150, "");
    for (track, target, property, value) in [
        ("g-fade", &group, "opacity", "1"),
        ("c-fade", &faded, "opacity", "0.5"),
        ("c-move", &faded, "x", "150"),
    ] {
        rig.doc_tool(
            "native_timeline_add_property_track",
            &format!(
                r#""trackId":"{track}","targetId":"{target}","property":"{property}","valueType":"scalar","interpolation":"linear","value":{value}"#
            ),
        )
        .ok();
    }
    key(&mut rig, "g-fade", 24, "0.965");
    key(&mut rig, "c-fade", 24, "0.2");
    key(&mut rig, "c-move", 24, "190");
    let a = png(&mut rig, &dir, "a.png", "0");
    let b = png(&mut rig, &dir, "b.png", "0.5");
    let c = png(&mut rig, &dir, "c.png", "1");
    assert_ne!(a, b);
    assert_ne!(b, c);
    let gif = dir.text("out.gif");
    rig.doc_tool(
        "export_animation",
        &format!(r#""path":"{gif}","format":"gif","fps":12,"duration":1"#),
    )
    .ok();
    assert!(fs::metadata(dir.file("out.gif")).unwrap().len() > 0);
}
