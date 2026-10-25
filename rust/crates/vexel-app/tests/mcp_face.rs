//! Face tools through MCP: `mouth_set`, `lip_sync`, `eyes_set`, `blink`.

mod common;
mod face_support;

use common::{Dir, Rig, no_grants, policy_for};
use face_support::{Eyes, Face, build, build_eyes, shapes};
use std::fs;
use vexel_timeline::audio_codec::encode_wav16;

fn source(rig: &Rig) -> String {
    String::from_utf8(rig.app.source_bytes().unwrap()).unwrap()
}

#[test]
fn mouth_set_stores_in_the_document_and_undoes() {
    let mut rig = Rig::new(no_grants());
    let face = build(&mut rig);
    let before = source(&rig);
    let reply = rig.doc_tool("mouth_set", &shapes(&face)).ok().to_owned();
    assert!(reply.contains("\"shapes\":3"), "{reply}");
    let stored = source(&rig);
    assert_eq!(stored.matches("id=\"vexel-mouths\"").count(), 1);
    assert!(stored.contains(&format!("\"rest\":\"{}\"", face.rest)));
    // Replacing keeps one element and one undo restores the first set.
    rig.doc_tool(
        "mouth_set",
        &format!(
            r#""character":"{}","shapes":{{"rest":"{}"}}"#,
            face.character, face.rest
        ),
    )
    .ok();
    assert_eq!(source(&rig).matches("vexel-mouths").count(), 1);
    rig.doc_tool("undo", "").ok();
    assert_eq!(source(&rig), stored);
    rig.doc_tool("undo", "").ok();
    assert_eq!(source(&rig), before);
}

#[test]
fn mouth_set_follows_a_renamed_shape_group() {
    let mut rig = Rig::new(no_grants());
    let face = build(&mut rig);
    rig.doc_tool("mouth_set", &shapes(&face)).ok();
    rig.doc_tool(
        "attribute_set",
        &format!(r#""elementId":"{}","name":"id","value":"mouth_o""#, face.o),
    )
    .ok();
    let text = source(&rig);
    assert!(text.contains("\"O\":\"mouth_o\""), "{text}");
    assert!(!text.contains(&format!("\"{}\"", face.o)));
    let reply = rig.doc_tool(
        "attribute_set",
        r#""elementId":"vexel-mouths","name":"id","value":"x""#,
    );
    assert!(reply.is_error);
}

#[test]
fn mouth_set_rejects_bad_input_without_changes() {
    let mut rig = Rig::new(no_grants());
    let face = build(&mut rig);
    let (c, r, a) = (&face.character, &face.rest, &face.a);
    let stray = {
        let g = common::element_id(
            rig.doc_tool(
                "element_add",
                r##""parentId":"root","template":"rect","attributes":{"x":"1","y":"1","width":"5","height":"5","fill":"#000000"}"##,
            )
            .ok(),
        );
        common::element_id(
            rig.doc_tool("element_group", &format!(r#""elementId":"{g}""#))
                .ok(),
        )
    };
    let cases = [
        format!(r#""character":"{c}","shapes":{{}}"#),
        format!(r#""character":"{c}","shapes":{{"A":"{a}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","Q":"{a}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","A":"nope"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","A":"{stray}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","A":"{c}"}}"#),
        format!(r#""character":"missing","shapes":{{"rest":"{r}"}}"#),
        format!(r#""character":"{r}","shapes":{{"rest":"{a}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","A":5}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","rest":"{a}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"bad id"}}"#),
        format!(r#""character":"{c}","shapes":[1]"#),
        format!(r#""character":3,"shapes":{{"rest":"{r}"}}"#),
        format!(r#""character":"{c}""#),
    ];
    let state = rig.state();
    for args in &cases {
        let reply = rig.doc_tool("mouth_set", args);
        assert!(reply.is_error, "{args}: {}", reply.body);
    }
    assert_eq!(rig.state(), state, "rejected calls changed nothing");
}

#[test]
fn a_forged_mouth_element_is_not_trusted() {
    let mut rig = Rig::new(no_grants());
    let face = build(&mut rig);
    let reply = rig.doc_tool(
        "element_add_fragment",
        r#""parentId":"root","xml":"<rect id=\"vexel-mouths\" x=\"0\" y=\"0\" width=\"1\" height=\"1\"/>""#,
    );
    if reply.is_error {
        return; // the id is refused outright
    }
    let state = rig.state();
    let reply = rig.doc_tool("mouth_set", &shapes(&face));
    assert!(reply.is_error, "{}", reply.body);
    assert_eq!(rig.state(), state);
}

/// 2 s at 24 kHz: silence, a loud 440 Hz burst from 0.5 s to 1.25 s, silence.
fn burst() -> Vec<u8> {
    let rate = 24_000u32;
    let samples: Vec<i16> = (0..2 * rate as usize)
        .map(|i| {
            if (rate as usize / 2..rate as usize * 5 / 4).contains(&i) {
                ((i as f64 * 440.0 * std::f64::consts::TAU / f64::from(rate)).sin() * 20_000.0)
                    as i16
            } else {
                0
            }
        })
        .collect();
    encode_wav16(rate, 1, &samples)
}

fn jstr(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A face with its mouth set and a native timeline (48 ticks at 24/s).
fn talking(dir: &Dir) -> (Rig, Face) {
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let face = build(&mut rig);
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"fade","targetId":"{}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#,
            face.character
        ),
    )
    .ok();
    rig.doc_tool("mouth_set", &shapes(&face)).ok();
    fs::write(dir.file("burst.wav"), burst()).unwrap();
    (rig, face)
}

fn png(rig: &mut Rig, dir: &Dir, name: &str, time: &str) -> Vec<u8> {
    let path = dir.text(name);
    rig.doc_tool("render_png", &format!(r#""path":"{path}","time":{time}"#))
        .ok();
    fs::read(dir.file(name)).unwrap()
}

#[test]
fn lip_sync_opens_the_mouth_inside_the_burst_only() {
    let dir = Dir::new();
    let (mut rig, face) = talking(&dir);
    let before = rig.state();
    let args = format!(
        r#""character":"{}","source":{{"path":{}}},"seed":3"#,
        face.character,
        jstr(&dir.text("burst.wav"))
    );
    let reply = rig.doc_tool("lip_sync", &args).ok().to_owned();
    assert!(
        reply.contains("\"tracks\":3") && reply.contains("\"droppedTicks\":0"),
        "{reply}"
    );
    let quiet = png(&mut rig, &dir, "quiet.png", "0.1");
    let loud = png(&mut rig, &dir, "loud.png", "0.9");
    let after = png(&mut rig, &dir, "after.png", "1.75");
    assert_ne!(quiet, loud, "the mouth opens inside the burst");
    assert_eq!(quiet, after, "and closes again after it");
    assert_eq!(
        loud,
        png(&mut rig, &dir, "loud2.png", "0.9"),
        "deterministic"
    );
    let tl = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(
        tl.contains(&format!("lip_{}", face.rest)) && tl.contains("\"visibility\""),
        "{tl}"
    );
    // Running again replaces the lip tracks instead of failing.
    rig.doc_tool("lip_sync", &args).ok();
    let again = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert_eq!(again.matches("lip_").count(), tl.matches("lip_").count());
    // One undo per call; two undos leave only the mouth set.
    rig.doc_tool("native_timeline_undo", "").ok();
    rig.doc_tool("native_timeline_undo", "").ok();
    let bare = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(!bare.contains("lip_"), "{bare}");
    assert_ne!(before.source, rig.state().source, "the mouth set stays");
}

#[test]
fn lip_sync_takes_a_given_envelope_and_text_hints() {
    let dir = Dir::new();
    let (mut rig, face) = talking(&dir);
    let c = &face.character;
    let envelope = "0,0,0,0.9,0.9,0.9,0.9,0.9,0.9,0,0,0";
    let reply = rig
        .doc_tool(
            "lip_sync",
            &format!(
                r#""character":"{c}","source":{{"envelope":[{envelope}]}},"startTick":4,"text":["oh"],"smoothing":0"#
            ),
        )
        .ok()
        .to_owned();
    assert!(reply.contains("\"startTick\":4"), "{reply}");
    let mouthless = png(&mut rig, &dir, "m0.png", "0.05");
    let speaking = png(&mut rig, &dir, "m1.png", "0.4");
    assert_ne!(mouthless, speaking);
    // Ticks past the duration are dropped, not an error.
    let reply = rig
        .doc_tool(
            "lip_sync",
            &format!(r#""character":"{c}","source":{{"envelope":[{envelope}]}},"startTick":44"#),
        )
        .ok()
        .to_owned();
    assert!(reply.contains("\"droppedTicks\":8"), "{reply}");
}

#[test]
fn lip_sync_rejects_bad_input_without_changes() {
    let dir = Dir::new();
    let (mut rig, face) = talking(&dir);
    let c = &face.character;
    let wav = jstr(&dir.text("burst.wav"));
    let many = vec!["0.5"; 100_001].join(",");
    let words = vec!["\"ha\""; 257].join(",");
    let cases = [
        format!(r#""character":"nobody","source":{{"envelope":[0.5]}}"#),
        format!(r#""character":"{c}""#),
        format!(r#""character":"{c}","source":{{}}"#),
        format!(r#""character":"{c}","source":{{"path":{wav},"trackId":"x"}}"#),
        format!(r#""character":"{c}","source":{{"trackId":"missing"}}"#),
        format!(r#""character":"{c}","source":{{"path":"/etc/hosts"}}"#),
        format!(r#""character":"{c}","source":{{"path":"nope.wav"}}"#),
        format!(r#""character":"{c}","source":{{"envelope":[]}}"#),
        format!(r#""character":"{c}","source":{{"envelope":[2]}}"#),
        format!(r#""character":"{c}","source":{{"envelope":["x"]}}"#),
        format!(r#""character":"{c}","source":{{"envelope":[1e999]}}"#),
        format!(r#""character":"{c}","source":{{"envelope":[0.5]}},"startTick":48"#),
        format!(r#""character":"{c}","source":{{"envelope":[0.5]}},"ticks":5"#),
        format!(r#""character":"{c}","source":{{"path":{wav}}},"ticks":0"#),
        format!(r#""character":"{c}","source":{{"envelope":[0.5]}},"smoothing":9"#),
        format!(r#""character":"{c}","source":{{"envelope":[0.5]}},"seed":-1"#),
        format!(
            r#""character":"{c}","source":{{"envelope":[0.5]}},"mapping":{{"close":0.9,"mid":0.2}}"#
        ),
        format!(r#""character":"{c}","source":{{"envelope":[0.5]}},"mapping":{{"loud":1}}"#),
        format!(r#""character":"{c}","source":{{"envelope":[0.5]}},"text":[3]"#),
        format!(r#""character":"{c}","source":{{"envelope":[0.5]}},"text":[{words}]"#),
    ];
    let state = rig.state();
    for args in &cases {
        let reply = rig.doc_tool("lip_sync", args);
        let short: String = args.chars().take(90).collect();
        assert!(reply.is_error, "{short}: {}", reply.body);
    }
    // An oversized envelope is refused by the parser or the tool, never run.
    let huge = format!(
        r#"{{"jsonrpc":"2.0","id":900,"method":"tools/call","params":{{"name":"lip_sync","arguments":{{"documentId":"preview-document","character":"{c}","source":{{"envelope":[{many}]}}}}}}}}"#
    );
    let out = rig.line(&huge);
    assert!(
        out.contains("\"error\"") || out.contains("\"isError\":true"),
        "{}",
        &out[..out.len().min(200)]
    );
    assert_eq!(rig.state(), state, "rejected calls changed nothing");
    // A character without a mouth set is a clear error.
    let mut bare = Rig::new(no_grants());
    let other = build(&mut bare);
    let reply = bare.doc_tool(
        "lip_sync",
        &format!(
            r#""character":"{}","source":{{"envelope":[0.5]}}"#,
            other.character
        ),
    );
    assert!(reply.error().contains("mouth_set"), "{}", reply.body);
}

fn timeline(rig: &mut Rig) -> String {
    rig.doc_tool("native_timeline_get", "").ok().to_owned()
}

fn timeline_for(rig: &mut Rig, anchor: &str) {
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"anchor","targetId":"{anchor}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#
        ),
    )
    .ok();
}

fn eyes_rig(pupil: bool) -> (Dir, Rig, Eyes) {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let eyes = build_eyes(&mut rig, pupil);
    timeline_for(&mut rig, &eyes.open);
    (dir, rig, eyes)
}

const FAR: &str =
    r#""gaze":{"pupils":["P"],"keys":[{"tick":0,"x":0,"y":0},{"tick":6,"x":300,"y":0}]}"#;

#[test]
fn eyes_set_keeps_a_wandering_pupil_inside_the_eye() {
    let (dir, mut clipped, eyes) = eyes_rig(true);
    let state = clipped.state();
    let reply = clipped
        .doc_tool(
            "eyes_set",
            &format!(
                r#""eyes":[{{"eye":"{}","pupils":["{}"]}}]"#,
                eyes.eye, eyes.pupil
            ),
        )
        .ok()
        .to_owned();
    assert!(
        reply.contains("clipId") && reply.contains("groupId"),
        "{reply}"
    );
    assert!(
        source(&clipped).contains("clip-path=\"url(#vexel-clip-"),
        "{}",
        source(&clipped)
    );
    clipped
        .doc_tool(
            "blink",
            &FAR.replace('P', &eyes.pupil)
                .replace("\"gaze\"", "\"gaze\""),
        )
        .ok();
    let held = png(&mut clipped, &dir, "held.png", "0.25");
    // One undo reverts the gaze tracks, the next the clip wrapper.
    clipped.doc_tool("native_timeline_undo", "").ok();
    clipped.doc_tool("undo", "").ok();
    assert_eq!(clipped.state().source, state.source);

    // Reference: the same scene with no pupil at all.
    let (rdir, mut without, _) = eyes_rig(false);
    let nothing = png(&mut without, &rdir, "none.png", "0.25");
    assert_eq!(held, nothing, "a pupil outside the eye is clipped away");

    // Without eyes_set the wandering pupil shows.
    let (udir, mut free, ueyes) = eyes_rig(true);
    free.doc_tool("blink", &FAR.replace('P', &ueyes.pupil)).ok();
    let shown = png(&mut free, &udir, "free.png", "0.25");
    assert_ne!(shown, nothing);
}

#[test]
fn eyes_set_rejects_bad_input_without_changes() {
    let (_dir, mut rig, e) = eyes_rig(true);
    let (eye, pupil, lid) = (&e.eye, &e.pupil, &e.lid);
    let many: Vec<String> = (0..9)
        .map(|_| format!(r#"{{"eye":"{eye}","pupils":["{pupil}"]}}"#))
        .collect();
    let cases = [
        r#""eyes":[]"#.to_owned(),
        format!(r#""eyes":[{}]"#, many.join(",")),
        format!(r#""eyes":[{{"eye":"nope","pupils":["{pupil}"]}}]"#),
        format!(r#""eyes":[{{"eye":"{eye}","pupils":["nope"]}}]"#),
        format!(r#""eyes":[{{"eye":"{eye}","pupils":[]}}]"#),
        format!(r#""eyes":[{{"eye":"{eye}","pupils":["{eye}"]}}]"#),
        format!(r#""eyes":[{{"eye":"{eye}"}}]"#),
        format!(r#""eyes":[{{"pupils":["{pupil}"]}}]"#),
        format!(r#""eyes":[{{"eye":"{eye}","pupils":["{pupil}"],"x":1}}]"#),
        format!(r#""eyes":[{{"eye":"{eye}","pupils":[5]}}]"#),
        format!(r#""eyes":{{"eye":"{eye}"}}"#),
        format!(r#""eyes":[{{"eye":"{eye}","pupils":["{pupil}","{lid}","{pupil}"]}}]"#),
    ];
    let state = rig.state();
    for args in &cases {
        let reply = rig.doc_tool("eyes_set", args);
        let short: String = args.chars().take(80).collect();
        assert!(reply.is_error, "{short}: {}", reply.body);
    }
    assert_eq!(rig.state(), state, "rejected calls changed nothing");
}

#[test]
fn blink_squashes_a_lid_and_is_deterministic_from_the_seed() {
    let (dir, mut rig, e) = eyes_rig(true);
    let lid = &e.lid;
    let before = rig.state();
    let rest = png(&mut rig, &dir, "rest.png", "0.1");
    let reply = rig
        .doc_tool(
            "blink",
            &format!(r#""lids":[{{"id":"{lid}","closedScale":0.1}}],"at":[10],"duration":4"#),
        )
        .ok()
        .to_owned();
    assert!(
        reply.contains("\"blinks\":1") && reply.contains("\"tracks\":1"),
        "{reply}"
    );
    let closed = png(&mut rig, &dir, "closed.png", "0.5");
    assert_ne!(rest, closed, "the lid squashes at the blink peak");
    assert_eq!(rest, png(&mut rig, &dir, "after.png", "0.9"), "and reopens");
    // Running again replaces the track; one undo then restores the start.
    rig.doc_tool(
        "blink",
        &format!(r#""lids":[{{"id":"{lid}"}}],"at":[10],"duration":4"#),
    )
    .ok();
    // The anchor track plus one blink track, not two.
    assert_eq!(
        timeline(&mut rig).matches("\"kind\":\"property\"").count(),
        2
    );
    rig.doc_tool("native_timeline_undo", "").ok();
    rig.doc_tool("native_timeline_undo", "").ok();
    assert_eq!(rig.state().source, before.source);

    let random = |rig: &mut Rig, seed: u32| {
        rig.doc_tool(
            "blink",
            &format!(
                r#""lids":[{{"id":"{lid}"}}],"random":{{"seed":{seed},"minGap":3,"maxGap":9}},"duration":2"#
            ),
        )
        .ok();
        timeline(rig)
    };
    let a = random(&mut rig, 11);
    assert_eq!(a, random(&mut rig, 11), "same seed, same blinks");
    assert_ne!(a, random(&mut rig, 12), "another seed, other blinks");
}

#[test]
fn blink_swaps_eye_pictures() {
    let (dir, mut rig, e) = eyes_rig(true);
    let reply = rig
        .doc_tool(
            "blink",
            &format!(
                r#""swap":{{"open":["{}"],"closed":["{}"]}},"at":[10],"duration":4"#,
                e.open, e.closed
            ),
        )
        .ok()
        .to_owned();
    assert!(reply.contains("\"tracks\":2"), "{reply}");
    let wide = png(&mut rig, &dir, "wide.png", "0.1");
    let shut = png(&mut rig, &dir, "shut.png", "0.5");
    assert_ne!(wide, shut);
    assert_eq!(wide, png(&mut rig, &dir, "wide2.png", "0.9"));
}

#[test]
fn blink_rejects_bad_input_without_changes() {
    let (_dir, mut rig, e) = eyes_rig(true);
    let lid = &e.lid;
    let ok_lid = format!(r#""lids":[{{"id":"{lid}"}}]"#);
    let many: Vec<String> = (0..257).map(|i| (i * 5).to_string()).collect();
    let cases = [
        r#""at":[1]"#.to_owned(),
        format!(r#"{ok_lid}"#),
        format!(r#"{ok_lid},"at":[]"#),
        format!(r#"{ok_lid},"at":[10],"random":{{}}"#),
        format!(r#"{ok_lid},"at":[10,11],"duration":4"#),
        format!(r#"{ok_lid},"at":[48],"duration":4"#),
        format!(r#"{ok_lid},"at":[-1]"#),
        format!(r#"{ok_lid},"at":["x"]"#),
        format!(r#"{ok_lid},"at":[{}]"#, many.join(",")),
        format!(r#"{ok_lid},"at":[10],"duration":1"#),
        format!(r#"{ok_lid},"at":[10],"duration":49"#),
        format!(r#"{ok_lid},"random":{{"minGap":9,"maxGap":3}}"#),
        format!(r#"{ok_lid},"random":{{"minGap":0}}"#),
        format!(r#"{ok_lid},"random":{{"bogus":1}}"#),
        format!(r#""lids":[{{"id":"nope"}}],"at":[10]"#),
        format!(r#""lids":[{{"id":"{lid}","closedScale":0}}],"at":[10]"#),
        format!(r#""lids":[{{"id":"{lid}","closedScale":2}}],"at":[10]"#),
        format!(r#""lids":[{{"id":"{lid}","pivot":"left"}}],"at":[10]"#),
        format!(r#""lids":[{{"id":"{lid}","extra":1}}],"at":[10]"#),
        format!(r#""lids":[],"at":[10]"#),
        format!(r#""swap":{{"open":["nope"]}},"at":[10]"#),
        format!(r#""swap":{{"shut":["{lid}"]}},"at":[10]"#),
        format!(r#""gaze":{{"pupils":["{lid}"],"keys":[]}}"#),
        format!(r#""gaze":{{"pupils":["{lid}"],"keys":[{{"tick":1,"x":1e9,"y":0}}]}}"#),
        format!(r#""gaze":{{"pupils":["{lid}"],"keys":[{{"tick":99,"x":1,"y":0}}]}}"#),
        format!(r#""gaze":{{"pupils":["{lid}"],"keys":[{{"tick":1,"x":1}}]}}"#),
        format!(r#""gaze":{{"keys":[{{"tick":1,"x":1,"y":1}}]}}"#),
    ];
    let state = rig.state();
    for args in &cases {
        let reply = rig.doc_tool("blink", args);
        let short: String = args.chars().take(80).collect();
        assert!(reply.is_error, "{short}: {}", reply.body);
    }
    assert_eq!(rig.state(), state, "rejected calls changed nothing");
}
