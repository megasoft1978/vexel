//! `native_timeline_add_property_track` with `keys`: one edit, same result as
//! per-key calls, TRS forms accepted, bad keys named by index, count capped.

mod common;

use common::{Dir, Rig, policy_for};

fn rect(rig: &mut Rig) -> String {
    rig.add_rect(10, 10, "#336699")
}

fn track(rig: &mut Rig, id: &str, target: &str, keys: &str) -> common::Reply {
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"{id}","targetId":"{target}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1{keys}"#
        ),
    )
}

#[test]
fn bulk_keys_match_individual_keys_in_one_edit() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let a = rect(&mut rig);
    let b = rect(&mut rig);
    track(
        &mut rig,
        "bulk",
        &a,
        r#","keys":[{"tick":5,"value":0.25},{"tick":20,"value":0.5,"interp":"ease_in_out"},{"tick":30,"value":1}]"#,
    )
    .ok();
    let full = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(full.contains("\"bulk\""), "{full}");
    rig.doc_tool("native_timeline_undo", "").ok();
    let gone = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(
        !gone.contains("\"bulk\""),
        "one undo removes track and keys: {gone}"
    );
    rig.doc_tool("native_timeline_redo", "").ok();
    track(&mut rig, "single", &b, "").ok();
    for (tick, value, ease) in [
        (5, "0.25", ""),
        (20, "0.5", r#","ease":"ease_in_out""#),
        (30, "1", ""),
    ] {
        rig.doc_tool(
            "native_timeline_set_property_key",
            &format!(r#""trackId":"single","tick":{tick},"value":{value}{ease}"#),
        )
        .ok();
    }
    let get = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert_eq!(
        get.matches("0.25").count(),
        2,
        "both tracks carry the key: {get}"
    );
    assert_eq!(get.matches("ease_in_out").count(), 2, "{get}");
}

#[test]
fn trs_keys_and_errors() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let a = rect(&mut rig);
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"move","targetId":"{a}","property":"transform","valueType":"trs","interpolation":"linear","value":{{"pivot":[20,20]}},"keys":[{{"tick":10,"value":{{"rotate":30,"pivot":[20,20]}}}},{{"tick":20,"value":[5,5,0,1,1,20,20]}}]"#
        ),
    )
    .ok();
    let state = rig.state();
    let bad = track(
        &mut rig,
        "bad",
        &a,
        r#","keys":[{"tick":1,"value":1},{"tick":2,"value":1,"extra":1}]"#,
    );
    assert!(bad.error().contains("keys[1]"), "{}", bad.body);
    let bad = track(&mut rig, "bad", &a, r#","keys":[{"tick":900,"value":1}]"#);
    assert!(bad.is_error, "{}", bad.body);
    let many = vec![r#"{"tick":1,"value":1}"#; 257].join(",");
    let bad = track(&mut rig, "bad", &a, &format!(r#","keys":[{many}]"#));
    assert!(bad.error().contains("keys"), "{}", bad.body);
    assert_eq!(rig.state(), state, "rejected calls change nothing");
}
