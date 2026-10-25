//! `scene_apply`: a whole scene in one atomic, undoable call; every error
//! names the failing member and rolls everything back.

mod common;

use common::{Rig, no_grants};
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use vexel_app::animation::frame;

const SCENE_PRETTY: &str = r##""background":"#87ceeb","duration":48,
"elements":[
 {"template":"group","id":"hero","transform":{"translateX":50,"translateY":60},"children":[
  {"template":"group","id":"hero-torso","children":[
   {"template":"rect","id":"hero-body","attributes":{"x":"10","y":"10","width":"40","height":"60","fill":"#cc3333"}},
   {"template":"ellipse","id":"hero-head","attributes":{"cx":"30","cy":"0","rx":"15","ry":"15","fill":"#fc9"}}]},
  {"template":"group","id":"hero-arm","children":[
   {"template":"path","id":"hero-arm-p","attributes":{"d":"M0 0L40 0","stroke":"#000","stroke-width":"4","stroke-linecap":"round","fill":"none"}}]},
  {"template":"text","id":"hero-label","x":10,"y":-10,"content":"hi","fontSize":14,"fill":"#000000"}]},
 {"template":"fragment","xml":"<defs><symbol id=\"star\"><rect id=\"star-r\" width=\"5\" height=\"5\" fill=\"#ffff00\"/></symbol></defs>"},
 {"template":"use","id":"star-1","refId":"star","x":100,"y":100},
 {"template":"line","attributes":{"x1":"0","y1":"0","x2":"100","y2":"100","stroke":"#000000"}}],
"tracks":[
 {"trackId":"body-fade","targetId":"hero-body","property":"opacity","valueType":"scalar","interpolation":"linear","value":0,"keys":[{"tick":24,"value":1}]},
 {"trackId":"star-fade","targetId":"star-1","property":"opacity","valueType":"scalar","interpolation":"linear","value":1,"keys":[{"tick":40,"value":0.5}]}],
"rig":{"bones":[{"id":"hero-torso","pivot":[30,40]},{"id":"hero-arm","pivot":[10,10]}],"roles":{"torso":"hero-torso","arm_r":"hero-arm"}}"##;

/// JSON-RPC lines cannot hold newlines.
fn scene() -> String {
    SCENE_PRETTY.replace('\n', "")
}

fn rig() -> Rig {
    Rig::new(no_grants())
}

fn source(rig: &Rig) -> String {
    String::from_utf8(rig.app.source_bytes().unwrap()).unwrap()
}

fn rect(i: usize) -> String {
    format!(
        r##"{{"template":"rect","id":"r{i}","attributes":{{"x":"{}","y":"{}","width":"5","height":"5","fill":"#336699"}}}}"##,
        (i % 30) * 6,
        (i / 30) * 6
    )
}

fn renders(rig: &Rig) {
    let scene = frame::decode(&rig.app.source_bytes().unwrap()).expect("decodes");
    scene
        .render_sized(0.0, (128, 128), &AtomicBool::new(false))
        .expect("renders");
}

fn apply(rig: &mut Rig, body: &str) -> common::Reply {
    rig.doc_tool("scene_apply", body)
}

#[test]
fn builds_a_whole_scene_in_one_undoable_call() {
    let mut rig = rig();
    let before = rig.state();
    let timeline_before = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    let reply = apply(&mut rig, &scene()).ok().to_owned();
    assert!(
        reply.contains(r#""elements":10"#)
            && reply.contains(r#""tracks":2"#)
            && reply.contains(r#""rigBones":2"#),
        "{reply}"
    );
    let text = source(&rig);
    for id in [
        "scene-background",
        "hero",
        "hero-torso",
        "hero-body",
        "hero-label",
        "star-1",
        "star-r",
        "vexel-rig",
    ] {
        assert!(text.contains(&format!("id=\"{id}\"")), "{id} in {text}");
    }
    let all: Vec<&str> = text
        .split(" id=\"")
        .skip(1)
        .map(|r| &r[..r.find('"').unwrap()])
        .collect();
    assert_eq!(
        all.len(),
        all.iter().collect::<HashSet<_>>().len(),
        "{all:?}"
    );
    // Nested elements sit inside their group.
    let info = rig
        .doc_tool("element_get", r#""elementId":"hero-head""#)
        .ok()
        .to_owned();
    assert!(info.contains("hero-torso"), "{info}");
    renders(&rig);
    let timeline = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(
        timeline.contains("body-fade") && timeline.contains("star-fade"),
        "{timeline}"
    );
    rig.doc_tool(
        "rig_pose",
        r#""poses":[{"tick":0,"bones":{"hero-arm":{"rotate":10}}}]"#,
    )
    .ok();
    rig.doc_tool("native_timeline_undo", "").ok();
    // One undo removes elements, tracks and rig together.
    rig.doc_tool("undo", "").ok();
    assert_eq!(
        source(&rig),
        String::from_utf8(before.source.clone()).unwrap()
    );
    let timeline_after = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert_eq!(timeline_after, timeline_before);
}

#[test]
fn chunks_are_additive() {
    let mut rig = rig();
    let first: Vec<String> = (0..5).map(rect).collect();
    let second: Vec<String> = (5..10).map(rect).collect();
    for part in [&first, &second] {
        apply(&mut rig, &format!(r#""elements":[{}]"#, part.join(","))).ok();
    }
    let tracks = r##""tracks":[{"trackId":"t","targetId":"r9","property":"opacity","valueType":"scalar","interpolation":"linear","value":1}]"##;
    apply(&mut rig, tracks).ok();
    assert!(source(&rig).contains("id=\"r9\""));
    // A repeated id (an overlapping chunk) is refused and changes nothing.
    let before = rig.state();
    let again = apply(&mut rig, &format!(r#""elements":[{}]"#, rect(9)));
    assert!(
        again.error().contains("elements[0]") && again.error().contains("r9"),
        "{}",
        again.body
    );
    assert_eq!(rig.state(), before);
}

fn assert_rolled_back(rig: &mut Rig, body: &str, expect: &[&str]) {
    let before = rig.state();
    let timeline = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    let reply = apply(rig, body);
    let message = reply.error().to_owned();
    for want in expect {
        assert!(message.contains(want), "{want} not in {message}");
    }
    let after = rig.state();
    assert_eq!(after.source, before.source, "{message}");
    assert_eq!(
        rig.doc_tool("native_timeline_get", "").ok(),
        timeline,
        "{message}"
    );
    // The failed call leaves the next one working.
    assert!(!rig.doc_tool("document_info", "").is_error);
}

#[test]
fn every_error_names_its_member_and_rolls_everything_back() {
    let mut rig = rig();
    rig.add_rect(1, 1, "#112233");
    let mut items: Vec<String> = (0..20).map(rect).collect();
    let good = items.clone();
    let join = |items: &[String]| format!(r#""elements":[{}]"#, items.join(","));
    // Parse-time errors.
    items[12] =
        r##"{"template":"path","id":"bad","attributes":{"d":"M0 0 nonsense","fill":"#000000"}}"##
            .into();
    assert_rolled_back(
        &mut rig,
        &join(&items),
        &["scene_apply", "elements[12]", "path"],
    );
    items = good.clone();
    items[3] = r#"{"template":"blob"}"#.into();
    assert_rolled_back(&mut rig, &join(&items), &["elements[3]", "template"]);
    items[3] = r#"{"template":"rect","attributes":{"x":"1"},"bogus":1}"#.into();
    assert_rolled_back(&mut rig, &join(&items), &["elements[3]", "bogus"]);
    // Apply-time errors after earlier elements were added.
    items = good.clone();
    items[7] = rect(3);
    assert_rolled_back(&mut rig, &join(&items), &["elements[7]", "r3"]);
    items[7] = rect(7).replace(r#""id":"r7""#, r#""id":"vexel-object-1""#);
    assert_rolled_back(&mut rig, &join(&items), &["elements[7]", "vexel-object-1"]);
    items[7] = r##"{"template":"rect","id":"r7","parentId":"nope","attributes":{"x":"1","y":"1","width":"2","height":"2","fill":"#000000"}}"##.into();
    assert_rolled_back(&mut rig, &join(&items), &["elements[7]"]);
    // Nested.
    let group = r##"{"template":"group","id":"g","children":[CHILDREN]}"##;
    let kids = [
        rect(100),
        rect(101),
        r#"{"template":"rect","attributes":{"x":"1"}}"#.to_owned(),
    ];
    let nested = group.replace("CHILDREN", &kids.join(","));
    assert_rolled_back(
        &mut rig,
        &format!(r#""elements":[{},{nested}]"#, rect(0)),
        &["elements[1].children[2]"],
    );
    // Track and rig errors roll the elements back too.
    let tracks = |target: &str| {
        format!(
            r#","tracks":[{{"trackId":"ok","targetId":"r1","property":"opacity","valueType":"scalar","interpolation":"linear","value":1}},{{"trackId":"bad","targetId":"{target}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1}}]"#
        )
    };
    assert_rolled_back(
        &mut rig,
        &format!("{}{}", join(&good), tracks("missing")),
        &["tracks[1]"],
    );
    assert_rolled_back(
        &mut rig,
        &format!(r#"{},"rig":{{"bones":[{{"id":"nothere"}}]}}"#, join(&good)),
        &["rig"],
    );
    assert_rolled_back(
        &mut rig,
        &format!(r#"{},"duration":99999999999"#, join(&good)),
        &["duration"],
    );
    assert_rolled_back(
        &mut rig,
        &format!(r#"{},"background":"red""#, join(&good)),
        &["background"],
    );
    // Malformed requests.
    for body in [
        r#""elements":[]"#,
        r#""elements":"x""#,
        r#""elements":[1]"#,
        r#""tracks":[]"#,
        r#""duration":0"#,
    ] {
        let before = rig.state();
        assert!(apply(&mut rig, body).is_error, "{body}");
        assert_eq!(rig.state(), before, "{body}");
    }
    let empty = apply(&mut rig, "");
    assert!(empty.is_error);
}

#[test]
fn is_not_allowed_in_a_batch_and_obeys_the_scene_limit() {
    let mut rig = rig();
    let reply = rig.doc_tool(
        "batch",
        r#""ops":[{"tool":"scene_apply","arguments":{"elements":[{"template":"group","id":"g"}]}}]"#,
    );
    assert!(
        reply.error().contains("not allowed in a batch"),
        "{}",
        reply.body
    );
    let many: Vec<String> = (0..4097)
        .map(|i| format!(r#"{{"template":"group","id":"g{i}"}}"#))
        .collect();
    let before = rig.state();
    assert!(apply(&mut rig, &format!(r#""elements":[{}]"#, many.join(","))).is_error);
    assert_eq!(rig.state(), before);
}
