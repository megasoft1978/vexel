//! Nested groups, native timeline duration and `render_png` time through the
//! real JSON-RPC layer: a child added into a group rotates about its pivot
//! under a TRS track, and stills at two times differ.

mod common;

use common::{Dir, Rig, element_id, policy_for};
use std::fs;

fn rect(rig: &mut Rig, parent: &str, x: u32, fill: &str) -> String {
    let reply = rig.doc_tool(
        "element_add",
        &format!(
            r#""parentId":"{parent}","template":"rect","attributes":{{"x":"{x}","y":"100","width":"200","height":"40","fill":"{fill}"}}"#
        ),
    );
    element_id(reply.ok())
}

fn png(rig: &mut Rig, dir: &Dir, name: &str, time: &str) -> Vec<u8> {
    let path = dir.text(name);
    let reply = rig.doc_tool("render_png", &format!(r#""path":"{path}","time":{time}"#));
    reply.ok();
    fs::read(dir.file(name)).unwrap()
}

#[test]
fn child_in_group_rotates_about_pivot_and_renders_differently() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let anchor = rect(&mut rig, "root", 20, "#336699");
    let group = element_id(
        rig.doc_tool("element_group", &format!(r#""elementId":"{anchor}""#))
            .ok(),
    );
    let child = rect(&mut rig, &group, 150, "#cc3300");
    let source = String::from_utf8(rig.app.source_bytes().unwrap()).unwrap();
    let open = source.find(&format!(r#"id="{group}""#)).unwrap();
    let close = source.find("</g>").unwrap();
    let at = source.find(&format!(r#"id="{child}""#)).unwrap();
    assert!(
        open < at && at < close,
        "child is inside the group: {source}"
    );

    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"swing","targetId":"{child}","property":"transform","valueType":"trs","interpolation":"linear","value":[0,0,0,1,1,250,120]"#
        ),
    )
    .ok();
    rig.doc_tool(
        "native_timeline_set_property_key",
        r#""trackId":"swing","tick":24,"value":[0,0,60,1,1,250,120]"#,
    )
    .ok();
    let start = png(&mut rig, &dir, "a.png", "0");
    let mid = png(&mut rig, &dir, "b.png", "1");
    assert_ne!(start, mid, "rotation changes the still");
}

#[test]
fn element_add_rejects_non_group_and_unknown_parents() {
    let mut rig = Rig::new(policy_for(&[]));
    let shape = rect(&mut rig, "root", 20, "#336699");
    for parent in [shape.as_str(), "vexel-object-99"] {
        let before = rig.state();
        let reply = rig.doc_tool(
            "element_add",
            &format!(
                r##""parentId":"{parent}","template":"rect","attributes":{{"x":"1","y":"1","width":"9","height":"9","fill":"#123456"}}"##
            ),
        );
        assert!(reply.is_error, "{parent}");
        assert_eq!(rig.state(), before, "{parent}");
    }
}

#[test]
fn opacity_track_stills_differ_by_seconds_and_far_time_is_rejected() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let shape = rect(&mut rig, "root", 20, "#cc3300");
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"fade","targetId":"{shape}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#
        ),
    )
    .ok();
    rig.doc_tool(
        "native_timeline_set_property_key",
        r#""trackId":"fade","tick":24,"value":0.2"#,
    )
    .ok();
    let start = png(&mut rig, &dir, "a.png", "0");
    let mid = png(&mut rig, &dir, "b.png", "1");
    assert_ne!(start, mid, "time is in seconds: 1 s is tick 24");

    // 24 is seconds, far past the 2 s timeline: an error, not a silent clamp.
    let path = dir.text("c.png");
    let reply = rig.doc_tool("render_png", &format!(r#""path":"{path}","time":24"#));
    assert!(reply.is_error);
    assert!(reply.error().contains("seconds"), "{}", reply.error());
    assert!(!dir.file("c.png").exists());

    // A longer timeline accepts it and reports the tick rendered.
    rig.doc_tool("native_timeline_set_duration", r#""durationTicks":600"#)
        .ok();
    rig.doc_tool(
        "native_timeline_set_property_key",
        r#""trackId":"fade","tick":500,"value":1"#,
    )
    .ok();
    let reply = rig.doc_tool("render_png", &format!(r#""path":"{path}","time":10"#));
    assert!(reply.ok().contains(r#""tick":240"#), "{}", reply.body);
}
