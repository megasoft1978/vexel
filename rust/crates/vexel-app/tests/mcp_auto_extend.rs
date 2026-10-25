//! Native tracks need no prior `native_timeline_set_duration`: a key past the
//! current duration lengthens the timeline, within the set_duration limits.

mod common;

use common::{Dir, Rig, policy_for};

fn rig() -> (Dir, Rig, String) {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let id = rig.add_rect(10, 10, "#336699");
    (dir, rig, id)
}

fn duration(rig: &mut Rig) -> u64 {
    let get = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    let at = get.find("\"durationTicks\":").expect("durationTicks") + 16;
    get[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap()
}

const TRACK: &str =
    r#""property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#;

#[test]
fn set_property_key_past_the_end_extends_the_duration() {
    let (_dir, mut rig, id) = rig();
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(r#""trackId":"a","targetId":"{id}",{TRACK}"#),
    )
    .ok();
    assert_eq!(duration(&mut rig), 48);
    rig.doc_tool(
        "native_timeline_set_property_key",
        r#""trackId":"a","tick":100,"value":0.5"#,
    )
    .ok();
    assert_eq!(duration(&mut rig), 101);
    rig.doc_tool("native_timeline_undo", "").ok();
    assert_eq!(duration(&mut rig), 48, "one undo step");
    let far = rig.doc_tool(
        "native_timeline_set_property_key",
        r#""trackId":"a","tick":100000,"value":0.5"#,
    );
    assert!(far.is_error, "{}", far.body);
    assert_eq!(duration(&mut rig), 48);
}

#[test]
fn add_property_track_and_tracks_set_keys_extend_the_duration() {
    let (_dir, mut rig, id) = rig();
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(r#""trackId":"a","targetId":"{id}",{TRACK},"keys":[{{"tick":60,"value":0}}]"#),
    )
    .ok();
    assert_eq!(duration(&mut rig), 61);
    rig.doc_tool(
        "native_timeline_tracks_set",
        &format!(
            r#""tracks":[{{"trackId":"b","targetId":"{id}",{TRACK},"keys":[{{"tick":200,"value":0}}]}}]"#
        ),
    )
    .ok();
    assert_eq!(duration(&mut rig), 201);
}
