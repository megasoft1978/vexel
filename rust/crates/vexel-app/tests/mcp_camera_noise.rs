//! `noise` on tracks and `native_timeline_set_camera` through MCP: rendered PNGs are
//! deterministic, the camera changes the view, and clearing it restores the exact bytes.

mod common;

use common::{Dir, Rig, policy_for};
use std::fs;

fn png(rig: &mut Rig, dir: &Dir, name: &str, time: &str) -> Vec<u8> {
    let path = dir.0.join(name);
    rig.doc_tool(
        "render_png",
        &format!(r#""path":"{}","time":{time}"#, path.display()),
    )
    .ok();
    fs::read(path).unwrap()
}

fn scene(dir: &Dir) -> (Rig, String) {
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let id = rig.add_rect(40, 40, "#2A7FFF");
    (rig, id)
}

#[test]
fn camera_changes_view_and_clear_restores_exact_bytes() {
    let dir = Dir::new();
    let (mut rig, _) = scene(&dir);
    let base = png(&mut rig, &dir, "base.png", "0.5");
    rig.doc_tool(
        "native_timeline_set_camera",
        r#""keys":[{"tick":0,"zoom":1},{"tick":24,"zoom":3,"x":-20,"rotate":10,"ease":"ease_in_out_cubic"}]"#,
    )
    .ok();
    let tl = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(
        tl.contains("@camera") && tl.contains("ease_in_out_cubic"),
        "{tl}"
    );
    let moved = png(&mut rig, &dir, "moved.png", "0.5");
    assert_ne!(base, moved, "camera must change the frame");
    assert_eq!(
        moved,
        png(&mut rig, &dir, "moved2.png", "0.5"),
        "deterministic"
    );
    rig.doc_tool("native_timeline_undo", "").ok();
    assert_eq!(base, png(&mut rig, &dir, "undone.png", "0.5"));
    rig.doc_tool("native_timeline_redo", "").ok();
    rig.doc_tool("native_timeline_set_camera", r#""clear":true"#)
        .ok();
    assert_eq!(
        base,
        png(&mut rig, &dir, "cleared.png", "0.5"),
        "no camera, same bytes"
    );
    assert!(
        !rig.doc_tool("native_timeline_get", "")
            .ok()
            .contains("@camera")
    );
}

#[test]
fn camera_shake_is_seeded_and_replacing_is_one_edit() {
    let dir = Dir::new();
    let (mut rig, _) = scene(&dir);
    let shake =
        |seed: u32| format!(r#""shake":{{"amplitude":6,"frequency":8,"seed":{seed},"rotate":2}}"#);
    rig.doc_tool("native_timeline_set_camera", &shake(1)).ok();
    let a1 = png(&mut rig, &dir, "a1.png", "0.25");
    let a2 = png(&mut rig, &dir, "a2.png", "0.25");
    let later = png(&mut rig, &dir, "later.png", "0.75");
    assert_eq!(a1, a2);
    assert_ne!(a1, later, "shake moves over time");
    rig.doc_tool("native_timeline_set_camera", &shake(2)).ok();
    assert_ne!(a1, png(&mut rig, &dir, "b.png", "0.25"), "seed matters");
    rig.doc_tool("native_timeline_undo", "").ok();
    assert_eq!(
        a1,
        png(&mut rig, &dir, "back.png", "0.25"),
        "one undo restores seed 1"
    );
}

#[test]
fn camera_argument_errors() {
    let dir = Dir::new();
    let (mut rig, _) = scene(&dir);
    for args in [
        "",
        r#""clear":true,"shake":{"amplitude":1}"#,
        r#""keys":[{"tick":0}]"#,
        r#""keys":[{"tick":0,"zoom":0}]"#,
        r#""keys":[{"tick":0,"zoom":1,"ease":"wobble"}]"#,
        r#""keys":[{"tick":0,"zoom":1,"extra":1}]"#,
        r#""shake":{"amplitude":-1}"#,
        r#""shake":{"frequency":2}"#,
        r#""shake":{"amplitude":1,"octaves":9}"#,
        r#""keys":[{"tick":9999,"zoom":2}]"#,
    ] {
        rig.doc_tool("native_timeline_set_camera", args).error();
    }
    let many: Vec<String> = (0..257)
        .map(|i| format!(r#"{{"tick":{},"zoom":1}}"#, i % 40))
        .collect();
    rig.doc_tool(
        "native_timeline_set_camera",
        &format!(r#""keys":[{}]"#, many.join(",")),
    )
    .error();
    assert!(
        !rig.doc_tool("native_timeline_get", "")
            .ok()
            .contains("@camera")
    );
}

#[test]
fn noise_on_tracks_is_deterministic_and_validated() {
    let dir = Dir::new();
    let (mut rig, id) = scene(&dir);
    let track = |extra: &str| {
        format!(
            r#""trackId":"sway","targetId":"{id}","property":"x","valueType":"scalar","interpolation":"linear","value":40{extra}"#
        )
    };
    rig.doc_tool(
        "native_timeline_add_property_track",
        &track(r#","noise":{"amplitude":20,"frequency":3,"seed":5,"octaves":2}"#),
    )
    .ok();
    assert!(
        rig.doc_tool("native_timeline_get", "")
            .ok()
            .contains("noise")
    );
    let a = png(&mut rig, &dir, "n1.png", "0.4");
    assert_eq!(a, png(&mut rig, &dir, "n2.png", "0.4"));
    assert_ne!(a, png(&mut rig, &dir, "n3.png", "0.9"));
    for bad in [
        r#","noise":{}"#,
        r#","noise":{"amplitude":-1}"#,
        r#","noise":{"amplitude":1,"frequency":0}"#,
        r#","noise":{"amplitude":1,"octaves":5}"#,
        r#","noise":{"amplitude":1,"seed":-1}"#,
        r#","noise":{"amplitude":1,"x":1}"#,
        r#","noise":[1]"#,
        r#","noise":3"#,
    ] {
        let id2 = bad.len();
        rig.doc_tool(
            "native_timeline_add_property_track",
            &track(bad).replace("sway", &format!("bad{id2}")),
        )
        .error();
    }
    let reply = rig.doc_tool(
        "native_timeline_tracks_set",
        &format!(
            r#""tracks":[{{"trackId":"b1","targetId":"{id}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1,"noise":{{"amplitude":0.2}},"keys":[{{"tick":10,"value":0.5,"ease":"bounce"}}]}}]"#
        ),
    );
    reply.ok();
    // Noise is rejected on a boolean track.
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"vis","targetId":"{id}","property":"visibility","valueType":"boolean","interpolation":"step","value":true,"noise":{{"amplitude":1}}"#
        ),
    )
    .error();
}
