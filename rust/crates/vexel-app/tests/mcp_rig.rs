//! `rig_create` through MCP: stored in the document, one undo step, validated.

mod common;
mod rig_support;

use common::{Rig, no_grants};
use rig_support::{build, rig_args, rig_args_with};

fn setup() -> (Rig, rig_support::Stick) {
    let mut rig = Rig::new(no_grants());
    let stick = build(&mut rig);
    (rig, stick)
}

fn source(rig: &Rig) -> String {
    String::from_utf8(rig.app.source_bytes().unwrap()).unwrap()
}

#[test]
fn create_stores_the_rig_and_undoes_in_one_step() {
    let (mut rig, stick) = setup();
    let before = source(&rig);
    let reply = rig.doc_tool("rig_create", &rig_args(&stick)).ok().to_owned();
    assert!(reply.contains("\"bones\":6") && reply.contains("\"roles\":6"), "{reply}");
    assert!(source(&rig).contains("id=\"vexel-rig\""));
    // Re-creating replaces the rig in one edit, and one undo restores the first rig.
    let first = source(&rig);
    rig.doc_tool("rig_create", &format!(r#""bones":[{{"id":"{}"}}]"#, stick.torso))
        .ok();
    assert_eq!(source(&rig).matches("vexel-rig").count(), 1);
    rig.doc_tool("undo", "").ok();
    assert_eq!(source(&rig), first);
    rig.doc_tool("undo", "").ok();
    assert_eq!(source(&rig), before);
}

#[test]
fn center_pivot_uses_group_bounds() {
    let (mut rig, stick) = setup();
    rig.doc_tool("rig_create", &format!(r#""bones":[{{"id":"{}"}}]"#, stick.head))
        .ok();
    assert!(source(&rig).contains(r#""pivot":[150,80]"#), "{}", source(&rig));
}

#[test]
fn bad_rigs_are_rejected_without_changes() {
    let (mut rig, stick) = setup();
    let (t, h, al) = (&stick.torso, &stick.head, &stick.arm_l);
    let rect = {
        let s = source(&rig);
        let at = s.find("<rect").unwrap();
        let id = s[at..].split("id=\"").nth(1).unwrap();
        id[..id.find('"').unwrap()].to_owned()
    };
    let many: Vec<String> = (0..129).map(|i| format!(r#"{{"id":"b{i}"}}"#)).collect();
    let cases = [
        r#""bones":[]"#.to_owned(),
        format!(r#""bones":[{{"id":"nope"}}]"#),
        format!(r#""bones":[{{"id":"{rect}"}}]"#),
        format!(r#""bones":[{{"id":"{t}"}},{{"id":"{t}"}}]"#),
        format!(r#""bones":[{{"id":"{t}","parent":"{t}"}}]"#),
        format!(r#""bones":[{{"id":"{t}","parent":"{h}"}},{{"id":"{h}","parent":"{t}"}}]"#),
        format!(r#""bones":[{{"id":"{t}"}},{{"id":"{al}","parent":"{t}"}},{{"id":"{h}","parent":"{al}"}}]"#),
        format!(r#""bones":[{{"id":"{t}","parent":"missing"}}]"#),
        format!(r#""bones":[{{"id":"{t}","pivot":[1e999,0]}}]"#),
        format!(r#""bones":[{{"id":"{t}","pivot":[2000000000,0]}}]"#),
        format!(r#""bones":[{{"id":"{t}","pivot":"left"}}]"#),
        format!(r#""bones":[{{"id":"{t}","pivot":[1]}}]"#),
        format!(r#""bones":[{{"id":"{t}","extra":1}}]"#),
        format!(r#""bones":[{{"id":"{t}"}}],"roles":{{"head":"{h}"}}"#),
        format!(r#""bones":[{{"id":"{t}"}}],"roles":{{"tentacle":"{t}"}}"#),
        format!(r#""bones":[{{"id":"{t}"}}],"roles":{{"head":3}}"#),
        format!(r#""bones":[{{"id":"bad.id"}}]"#),
        format!(r#""bones":[{}]"#, many.join(",")),
        format!(r#""bones":{{"id":"{t}"}}"#),
    ];
    let state = rig.state();
    for args in &cases {
        let message = rig.doc_tool("rig_create", args).error().to_owned();
        assert!(message.contains("rig_create") || message.contains("`bones`"), "{args}: {message}");
    }
    assert_eq!(rig.state(), state, "rejected calls changed nothing");
    assert!(!source(&rig).contains("vexel-rig"));
}

fn png(rig: &mut Rig, dir: &common::Dir, name: &str, time: &str) -> Vec<u8> {
    let path = dir.text(name);
    rig.doc_tool("render_png", &format!(r#""path":"{path}","time":{time}"#))
        .ok();
    std::fs::read(dir.file(name)).unwrap()
}

fn timeline(rig: &mut Rig) -> String {
    rig.doc_tool("native_timeline_get", "").ok().to_owned()
}

#[test]
fn pose_keys_all_bones_in_one_undoable_edit() {
    let dir = common::Dir::new();
    let mut rig = Rig::new(common::policy_for(&[&dir.0]));
    let stick = build(&mut rig);
    rig.doc_tool("rig_create", &rig_args_with(&stick, true)).ok();
    let (arm, leg) = (&stick.arm_r, &stick.leg_l);
    let rest = png(&mut rig, &dir, "rest.png", "0.5");
    let before = rig.state();
    let args = format!(
        r#""poses":[{{"tick":0,"bones":{{"{arm}":{{"rotate":0}},"{leg}":{{"rotate":0}}}}}},{{"tick":12,"bones":{{"{arm}":{{"rotate":-70,"translate":[0,2]}},"{leg}":{{"rotate":25,"scale":[1,0.9]}}}},"ease":"ease_in_out"}}]"#
    );
    let reply = rig.doc_tool("rig_pose", &args).ok().to_owned();
    assert!(reply.contains("\"tracks\":2") && reply.contains("\"keys\":4"), "{reply}");
    let tl = timeline(&mut rig);
    assert!(tl.contains(&format!("rig_{arm}")) && tl.contains(&format!("rig_{leg}")), "{tl}");
    assert!(tl.contains("ease_in_out"), "{tl}");
    let posed = png(&mut rig, &dir, "posed.png", "0.5");
    assert_ne!(rest, posed, "the pose moves limbs");
    assert_eq!(posed, png(&mut rig, &dir, "posed2.png", "0.5"), "deterministic");
    // Second call extends the existing tracks (no new ones) and replaces a key.
    rig.doc_tool(
        "rig_pose",
        &format!(r#""poses":[{{"tick":12,"bones":{{"{arm}":{{"rotate":-10}}}}}},{{"tick":30,"bones":{{"{arm}":{{"rotate":40}}}}}}]"#),
    )
    .ok();
    assert_eq!(timeline(&mut rig).matches("\"kind\":\"property\"").count(), 2);
    rig.doc_tool("native_timeline_undo", "").ok();
    rig.doc_tool("native_timeline_undo", "").ok();
    assert_eq!(rig.state().history, before.history - 0, "undo leaves the rig edit");
    assert!(!timeline(&mut rig).contains("rig_"), "{}", timeline(&mut rig));
    assert_eq!(rest, png(&mut rig, &dir, "back.png", "0.5"));
}

#[test]
fn pose_rejects_bad_input_without_changes() {
    let mut rig = Rig::new(common::no_grants());
    let stick = build(&mut rig);
    let a = &stick.arm_l;
    rig.doc_tool("rig_pose", &format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{}}}}}}]"#))
        .error();
    rig.doc_tool("rig_create", &rig_args(&stick)).ok();
    let state = rig.state();
    let many: Vec<String> = (0..257).map(|i| format!(r#"{{"tick":{},"bones":{{}}}}"#, i % 40)).collect();
    for args in [
        r#""poses":[]"#.to_owned(),
        format!(r#""poses":[{{"tick":0,"bones":{{"ghost":{{"rotate":1}}}}}}]"#),
        format!(r#""poses":[{{"tick":48,"bones":{{"{a}":{{"rotate":1}}}}}}]"#),
        format!(r#""poses":[{{"tick":4294967295,"bones":{{"{a}":{{"rotate":1}}}}}}]"#),
        format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"rotate":"x"}}}}}}]"#),
        format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"rotate":1e999}}}}}}]"#),
        format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"spin":1}}}}}}]"#),
        format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"translate":[1]}}}}}}]"#),
        format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"scale":"big"}}}}}}]"#),
        format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"rotate":1}}}},"ease":"wobble"}}]"#),
        format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"rotate":1}}}},"extra":1}}]"#),
        format!(r#""poses":[{{"tick":-1,"bones":{{"{a}":{{"rotate":1}}}}}}]"#),
        format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"rotate":1}}}}}}],"interpolation":"bouncy""#),
        format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"rotate":1}}}}}}],"noise":{{"amplitude":-1}}"#),
        format!(r#""poses":[{}]"#, many.join(",")),
        r#""poses":{"tick":0}"#.to_owned(),
    ] {
        rig.doc_tool("rig_pose", &args).error();
    }
    assert_eq!(rig.state(), state, "rejected calls changed nothing");
    // Noise needs fresh tracks.
    let ok = format!(r#""poses":[{{"tick":0,"bones":{{"{a}":{{"rotate":1}}}}}}],"noise":{{"amplitude":2,"seed":3}}"#);
    rig.doc_tool("rig_pose", &ok).ok();
    rig.doc_tool("rig_pose", &ok).error();
}

#[test]
fn walk_for_two_seconds_moves_limbs_and_survives_save_and_reopen() {
    let dir = common::Dir::new();
    let mut rig = Rig::new(common::policy_for(&[&dir.0]));
    let stick = build(&mut rig);
    // Only limbs have roles, so every pixel that changes is a limb pixel.
    rig.doc_tool("rig_create", &rig_args_with(&stick, false)).ok();
    rig.doc_tool("native_timeline_set_duration", r#""durationTicks":72"#).ok();
    let rest = png(&mut rig, &dir, "rest.png", "0");
    let reply = rig
        .doc_tool("clip_apply", r#""clip":"walk","start":0,"duration":48,"loops":2"#)
        .ok()
        .to_owned();
    assert!(reply.contains("\"tracks\":4") && reply.contains("\"keys\":68"), "{reply}");
    let frames: Vec<Vec<u8>> = ["0", "0.125", "0.25", "0.2917"]
        .iter()
        .enumerate()
        .map(|(i, t)| png(&mut rig, &dir, &format!("f{i}.png"), t))
        .collect();
    // Tick 0 is the rest pose (sin 0); later ticks swing the limbs.
    assert_eq!(rest, frames[0]);
    for (i, a) in frames.iter().enumerate() {
        for (j, b) in frames.iter().enumerate().skip(i + 1) {
            assert_ne!(a, b, "limb pixels differ between ticks {i} and {j}");
        }
    }
    // One undo removes the whole clip.
    rig.doc_tool("native_timeline_undo", "").ok();
    assert!(!timeline(&mut rig).contains("rig_"));
    assert_eq!(rest, png(&mut rig, &dir, "undone.png", "0.25"));
    rig.doc_tool("native_timeline_redo", "").ok();
    let swing = png(&mut rig, &dir, "swing.png", "0.25");
    assert_eq!(swing, frames[2]);

    let target = dir.file("walker.vexel");
    rig.doc_tool("save_as", &format!(r#""path":"{}""#, target.display())).ok();
    let mut reopened = Rig::with_open(common::policy_for(&[&dir.0]), &target).unwrap();
    assert_eq!(swing, png(&mut reopened, &dir, "reopened.png", "0.25"));
    assert!(String::from_utf8(reopened.app.source_bytes().unwrap()).unwrap().contains("vexel-rig"));
    // The reopened rig still drives clips: chain an idle at the end.
    reopened
        .doc_tool("clip_apply", r#""clip":"wave","start":48,"duration":20,"mode":"pingpong""#)
        .ok();
    let idle = reopened
        .doc_tool("clip_apply", r#""clip":"jump","start":48,"duration":20,"ease":"ease_out""#)
        .error()
        .to_owned();
    assert!(idle.contains("torso"), "{idle}");
    reopened
        .doc_tool("clip_apply", r#""clip":"walk","start":48,"duration":20,"amplitude":2,"mode":"pingpong""#)
        .ok();
}

#[test]
fn clip_apply_validates_and_is_deterministic() {
    let dir = common::Dir::new();
    let mut rig = Rig::new(common::policy_for(&[&dir.0]));
    let stick = build(&mut rig);
    rig.doc_tool("clip_apply", r#""clip":"walk","duration":24"#).error();
    rig.doc_tool("rig_create", &rig_args(&stick)).ok();
    let state = rig.state();
    for args in [
        r#""clip":"dance","duration":24"#,
        r#""clip":"walk""#,
        r#""clip":"walk","duration":0"#,
        r#""clip":"walk","duration":1000001"#,
        r#""clip":"walk","duration":24,"loops":0"#,
        r#""clip":"walk","duration":24,"loops":65"#,
        r#""clip":"walk","duration":24,"speed":0.1"#,
        r#""clip":"walk","duration":24,"speed":9"#,
        r#""clip":"walk","duration":24,"amplitude":-1"#,
        r#""clip":"walk","duration":24,"amplitude":5"#,
        r#""clip":"walk","duration":24,"mode":"zigzag""#,
        r#""clip":"walk","duration":24,"ease":"wobble""#,
        r#""clip":"walk","duration":24,"start":-1"#,
        r#""clip":"walk","duration":24,"start":4294967295"#,
        r#""clip":"walk","duration":24,"start":40"#,
        r#""clip":"walk","duration":24.5"#,
        r#""clip":"walk","duration":24,"noise":{"amplitude":1}"#,
        r#""clip":"walk","duration":24,"bones":["a"]"#,
    ] {
        rig.doc_tool("clip_apply", args).error();
    }
    assert_eq!(rig.state(), state, "rejected calls changed nothing");
    // A rig without the needed role is refused with a message naming it.
    rig.doc_tool("rig_create", &format!(r#""bones":[{{"id":"{}"}}],"roles":{{"torso":"{}"}}"#, stick.torso, stick.torso)).ok();
    let message = rig.doc_tool("clip_apply", r#""clip":"walk","duration":24"#).error().to_owned();
    assert!(message.contains("leg_l"), "{message}");
    // Same call, same keys.
    rig.doc_tool("rig_create", &rig_args(&stick)).ok();
    rig.doc_tool("clip_apply", r#""clip":"idle","duration":40,"loops":2,"amplitude":3"#).ok();
    let first = timeline(&mut rig);
    rig.doc_tool("native_timeline_undo", "").ok();
    rig.doc_tool("clip_apply", r#""clip":"idle","duration":40,"loops":2,"amplitude":3"#).ok();
    assert_eq!(first, timeline(&mut rig));
    for clip in ["run", "talk_body", "blink_bob", "blink-body-bob", "wave", "jump"] {
        rig.doc_tool("clip_apply", &format!(r#""clip":"{clip}","start":1,"duration":30,"loops":3"#)).ok();
    }
}
