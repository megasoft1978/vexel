//! `attribute_set` name `id`: renames an element (groups included), its
//! references and the native tracks that target it, in one undo step.

mod common;

use common::{Rig, element_id, no_grants};

fn set_id(rig: &mut Rig, id: &str, value: &str) -> common::Reply {
    rig.doc_tool(
        "attribute_set",
        &format!(r#""elementId":"{id}","name":"id","value":"{value}""#),
    )
}

fn svg(rig: &mut Rig) -> String {
    rig.doc_tool("export_svg", "").ok().replace("\\\"", "\"")
}

fn group_with_track(rig: &mut Rig) -> String {
    let rect = rig.add_rect(10, 10, "#ff0000");
    let group = element_id(
        rig.doc_tool("element_group", &format!(r#""elementId":"{rect}""#))
            .ok(),
    );
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"fade","targetId":"{group}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#
        ),
    )
    .ok();
    group
}

#[test]
fn renaming_a_group_retargets_tracks_and_undoes_in_one_step() {
    let mut rig = Rig::new(no_grants());
    let group = group_with_track(&mut rig);
    let before = svg(&mut rig);
    let timeline_before = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    let reply = set_id(&mut rig, &group, "head");
    assert!(
        reply.ok().contains(r#""elementId":"head""#),
        "{}",
        reply.body
    );
    assert!(svg(&mut rig).contains(r#"<g id="head""#));
    let timeline = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(timeline.contains(r#""target":"head""#), "{timeline}");
    // The track still edits and the renamed group still takes new tracks.
    rig.doc_tool(
        "native_timeline_set_property_key",
        r#""trackId":"fade","tick":6,"value":0.5"#,
    )
    .ok();
    // One undo of the (key) edit, one of the rename: the old id comes back
    // in both the source and the timeline.
    rig.doc_tool("undo", "").ok();
    rig.doc_tool("undo", "").ok();
    assert_eq!(svg(&mut rig), before);
    assert_eq!(
        rig.doc_tool("native_timeline_get", "").ok(),
        timeline_before
    );
}

#[test]
fn rename_rejects_used_and_invalid_ids_without_changes() {
    let mut rig = Rig::new(no_grants());
    let group = group_with_track(&mut rig);
    let other = rig.add_rect(50, 50, "#00ff00");
    let state = rig.state();
    for value in [other.as_str(), "bad id", "1abc", ""] {
        let reply = set_id(&mut rig, &group, value);
        assert!(reply.is_error, "{value}: {}", reply.body);
    }
    assert!(set_id(&mut rig, "nope", "fresh").is_error);
    assert_eq!(rig.state(), state);
}

#[test]
fn rename_works_inside_a_batch() {
    let mut rig = Rig::new(no_grants());
    let group = group_with_track(&mut rig);
    let reply = rig.doc_tool(
        "batch",
        &format!(
            r#""ops":[{{"tool":"attribute_set","arguments":{{"documentId":"preview-document","elementId":"{group}","name":"id","value":"torso"}}}}]"#
        ),
    );
    assert!(reply.ok().contains(r#""failedCount":0"#), "{}", reply.body);
    let timeline = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(timeline.contains(r#""target":"torso""#), "{timeline}");
}
