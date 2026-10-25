//! Per-key `ease` forms (presets, aliases, bezier string/array) on set key, bulk keys and
//! `native_timeline_tracks_set`; bad forms rejected.

mod common;

use common::{Dir, Rig, policy_for};

fn track(rig: &mut Rig, id: &str, target: &str, keys: &str) -> common::Reply {
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"{id}","targetId":"{target}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1{keys}"#
        ),
    )
}

#[test]
fn ease_forms_on_keys_and_bulk_keys() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let a = rig.add_rect(10, 10, "#336699");
    track(
        &mut rig,
        "t",
        &a,
        r#","keys":[{"tick":5,"value":0.2,"ease":"bounce"},{"tick":10,"value":0.4,"ease":[0.1,0.2,0.3,1]},{"tick":15,"value":0.5,"interp":"ease_in_out_cubic"},{"tick":20,"value":0.6,"ease":"bezier:0.2,0,0.4,1"}]"#,
    )
    .ok();
    let get = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    for want in [
        r#""ease":"ease_out_bounce""#,
        r#""ease":{"bezier":[0.1,0.2,0.3,1]}"#,
        r#""ease":"ease_in_out_cubic""#,
        r#""ease":{"bezier":[0.2,0,0.4,1]}"#,
    ] {
        assert!(get.contains(want), "{want} in {get}");
    }
    for (tick, ease) in [
        (25, r#""elastic""#),
        (30, r#"[0,0,1,1]"#),
        (35, r#""ease_in_out_back""#),
    ] {
        rig.doc_tool(
            "native_timeline_set_property_key",
            &format!(r#""trackId":"t","tick":{tick},"value":0.5,"ease":{ease}"#),
        )
        .ok();
    }
    let get = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(
        get.contains("ease_out_elastic") && get.contains("ease_in_out_back"),
        "{get}"
    );
}

#[test]
fn bad_ease_forms_are_rejected() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let a = rig.add_rect(10, 10, "#336699");
    track(&mut rig, "t", &a, "").ok();
    for ease in [
        r#""wobble""#,
        "[0,0,1]",
        "[0,0,1,1,1]",
        r#"["a",0,1,1]"#,
        "[2,0,1,1]",
        "7",
        "{}",
    ] {
        let reply = rig.doc_tool(
            "native_timeline_set_property_key",
            &format!(r#""trackId":"t","tick":3,"value":0.5,"ease":{ease}"#),
        );
        reply.error();
        let get = rig.doc_tool("native_timeline_get", "").ok().to_owned();
        assert!(!get.contains("\"tick\":3,"), "{ease} applied: {get}");
    }
}
