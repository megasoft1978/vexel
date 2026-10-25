//! Timeline audio tracks and the export mux through the real JSON-RPC layer.

mod common;

use common::{Dir, Rig, policy_for};
use std::fs;
use vexel_platform_apple::video_reader::{audio_tracks, decode};
use vexel_timeline::audio_codec::encode_wav16;

fn sine(seconds: f64, rate: u32) -> Vec<u8> {
    let frames = (seconds * f64::from(rate)) as usize;
    let samples: Vec<i16> = (0..frames)
        .map(|i| {
            ((i as f64 * 440.0 * std::f64::consts::TAU / f64::from(rate)).sin() * 9000.0) as i16
        })
        .collect();
    encode_wav16(rate, 1, &samples)
}

fn jstr(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A saved 48-tick animated document in `dir` with `audio/tone.wav` (2 s).
fn saved(dir: &Dir) -> Rig {
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let id = rig.add_rect(40, 40, "#2A7FFF");
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"fade","targetId":"{id}","property":"opacity","valueType":"scalar","interpolation":"linear","value":0"#
        ),
    )
    .ok();
    rig.doc_tool(
        "save_as",
        &format!(r#""path":{}"#, jstr(&dir.text("film.vexel"))),
    )
    .ok();
    fs::create_dir_all(dir.file("audio")).unwrap();
    fs::write(dir.file("audio/tone.wav"), sine(2.0, 24_000)).unwrap();
    rig
}

fn add(rig: &mut Rig, id: &str, path: &str, extra: &str) -> common::Reply {
    rig.doc_tool(
        "audio_add_track",
        &format!(
            r#""trackId":"{id}","kind":"voice","path":{}{extra}"#,
            jstr(path)
        ),
    )
}

fn export(rig: &mut Rig, dir: &Dir, name: &str) -> String {
    rig.doc_tool(
        "export_animation",
        &format!(
            r#""path":{},"format":"mp4","width":64,"height":64"#,
            jstr(&dir.text(name))
        ),
    )
    .ok()
    .to_owned()
}

#[test]
fn add_list_set_remove_undo_and_persist() {
    let dir = Dir::new();
    let mut rig = saved(&dir);
    let reply = add(
        &mut rig,
        "line1",
        "audio/tone.wav",
        r#","startTick":6,"gainDb":-3,"fadeInTicks":2"#,
    )
    .ok()
    .to_owned();
    for part in [
        r#""trackId":"line1""#,
        r#""path":"audio/tone.wav""#,
        r#""durationMs":2000"#,
        r#""sampleRate":24000"#,
    ] {
        assert!(reply.contains(part), "{part} in {reply}");
    }
    // Absolute paths inside the document directory are stored relative.
    let abs = dir.text("audio/tone.wav");
    let reply = add(
        &mut rig,
        "line2",
        &abs,
        r#","loop":true,"duck":{"under":"line1","amountDb":-9}"#,
    )
    .ok()
    .to_owned();
    assert!(reply.contains(r#""path":"audio/tone.wav""#), "{reply}");
    let list = rig.doc_tool("audio_list", "").ok().to_owned();
    assert!(
        list.contains(r#""count":2"#) && list.contains(r#""startTick":6"#),
        "{list}"
    );
    assert!(
        list.contains(r#""duck":{"under":"line1","amountDb":-9}"#),
        "{list}"
    );

    rig.doc_tool(
        "audio_set",
        r#""trackId":"line1","gainDb":-12,"startTick":0,"trimOutMs":1500"#,
    )
    .ok();
    let list = rig.doc_tool("audio_list", "").ok().to_owned();
    assert!(
        list.contains(r#""gainDb":-12"#) && list.contains(r#""trimOutMs":1500"#),
        "{list}"
    );
    // Undo the set, then redo it, through the timeline history.
    rig.doc_tool("native_timeline_undo", "").ok();
    assert!(
        rig.doc_tool("audio_list", "")
            .ok()
            .contains(r#""gainDb":-3"#)
    );
    rig.doc_tool("native_timeline_redo", "").ok();
    assert!(
        rig.doc_tool("audio_list", "")
            .ok()
            .contains(r#""gainDb":-12"#)
    );

    // line1 cannot go while line2 ducks under it; clearing the duck allows it.
    assert!(
        rig.doc_tool("audio_set", r#""trackId":"line1","remove":true"#)
            .error()
            .contains("duck")
    );
    rig.doc_tool("audio_set", r#""trackId":"line2","clearDuck":true"#)
        .ok();
    rig.doc_tool("audio_set", r#""trackId":"line1","remove":true"#)
        .ok();
    assert!(rig.doc_tool("audio_list", "").ok().contains(r#""count":1"#));

    // Saved with the document and read back after reopening.
    rig.doc_tool("save", "").ok();
    let mut fresh = Rig::with_open(policy_for(&[&dir.0]), &dir.file("film.vexel")).unwrap();
    let list = fresh.doc_tool("audio_list", "").ok().to_owned();
    assert!(
        list.contains(r#""trackId":"line2""#) && list.contains(r#""loop":true"#),
        "{list}"
    );
}

#[test]
fn rejects_bad_requests_and_changes_nothing() {
    let dir = Dir::new();
    let other = Dir::new();
    let mut rig = saved(&dir);
    fs::write(other.file("elsewhere.wav"), sine(1.0, 24_000)).unwrap();
    fs::write(dir.file("audio/bad.wav"), b"RIFFxxxxWAVEnot really").unwrap();
    fs::write(dir.file("audio/text.wav"), b"hello world, not audio").unwrap();
    let mut truncated = sine(1.0, 24_000);
    truncated.truncate(truncated.len() - 100);
    fs::write(dir.file("audio/short.wav"), truncated).unwrap();
    let stereo3 = encode_wav16(24_000, 3, &[0; 300]);
    fs::write(dir.file("audio/three.wav"), stereo3).unwrap();
    std::os::unix::fs::symlink(other.file("elsewhere.wav"), dir.file("audio/link.wav")).unwrap();
    let big = fs::File::create(dir.file("audio/huge.wav")).unwrap();
    big.set_len(256 * 1024 * 1024 + 1).unwrap();
    let before = rig.state();
    let cases = [
        (
            add(&mut rig, "a", "audio/missing.wav", ""),
            "audio file not found",
        ),
        (
            add(&mut rig, "a", "audio/bad.wav", ""),
            "audio file is not a valid WAV, AIFF or CAF",
        ),
        (
            add(&mut rig, "a", "audio/text.wav", ""),
            "audio file is not a valid WAV, AIFF or CAF",
        ),
        (
            add(&mut rig, "a", "audio/short.wav", ""),
            "audio file is truncated",
        ),
        (
            add(&mut rig, "a", "audio/three.wav", ""),
            "audio encoding is unsupported; use PCM16, PCM24, PCM32 or float32, mono or stereo, 8-192 kHz",
        ),
        (
            add(&mut rig, "a", "audio/huge.wav", ""),
            "audio file is larger than 256 MiB",
        ),
        (add(&mut rig, "a", "audio/link.wav", ""), "path not granted"),
        (
            add(&mut rig, "a", &other.text("elsewhere.wav"), ""),
            "path not granted",
        ),
        (
            add(&mut rig, "a", "../escape.wav", ""),
            "audio file must be inside the document's directory",
        ),
        (
            add(&mut rig, "voice", "audio/tone.wav", ""),
            "audio edit rejected: trackId is 1-128",
        ),
        (
            add(&mut rig, "a", "audio/tone.wav", r#","startTick":48"#),
            "audio edit rejected: trackId is 1-128",
        ),
        (
            add(&mut rig, "a", "audio/tone.wav", r#","gainDb":40"#),
            "audio edit rejected: trackId is 1-128",
        ),
        (
            add(
                &mut rig,
                "a",
                "audio/tone.wav",
                r#","duck":{"under":"ghost","amountDb":-6}"#,
            ),
            "audio edit rejected: duck.under",
        ),
        (
            rig.doc_tool("audio_set", r#""trackId":"nope","gainDb":1"#),
            "audio_set trackId is not an audio track",
        ),
    ];
    for (reply, want) in cases {
        let got = reply.error().to_owned();
        assert!(got.starts_with(want), "wanted {want:?}, got {got:?}");
    }
    assert_eq!(
        rig.state(),
        before,
        "rejected calls must not change the document"
    );

    // An unsaved document has no directory to resolve against.
    let mut unsaved = Rig::new(policy_for(&[&dir.0]));
    let id = unsaved.add_rect(10, 10, "#FF0000");
    unsaved
        .doc_tool(
            "native_timeline_add_property_track",
            &format!(r#""trackId":"t","targetId":"{id}","property":"opacity","valueType":"scalar","interpolation":"linear","value":0"#),
        )
        .ok();
    let err = add(&mut unsaved, "a", &dir.text("audio/tone.wav"), "")
        .error()
        .to_owned();
    assert!(err.contains("save it with save_as"), "{err}");
}

#[test]
fn track_limit_is_64() {
    let dir = Dir::new();
    let mut rig = saved(&dir);
    for i in 0..64 {
        add(&mut rig, &format!("t{i}"), "audio/tone.wav", "").ok();
    }
    let err = add(&mut rig, "t64", "audio/tone.wav", "")
        .error()
        .to_owned();
    assert_eq!(err, "audio edit rejected: at most 64 audio tracks");
}

#[test]
fn export_muxes_an_aac_track_of_the_video_length() {
    let dir = Dir::new();
    let mut rig = saved(&dir);
    let plain = export(&mut rig, &dir, "plain.mp4");
    assert!(plain.contains(r#""frames":48"#), "{plain}");
    assert_eq!(audio_tracks(&dir.file("plain.mp4")).unwrap().0, 0);

    add(&mut rig, "line1", "audio/tone.wav", r#","fadeOutTicks":4"#).ok();
    add(
        &mut rig,
        "bed",
        "audio/tone.wav",
        r#","loop":true,"gainDb":-12,"duck":{"under":"voice","amountDb":-10}"#,
    )
    .ok();
    let body = export(&mut rig, &dir, "with.mp4");
    assert!(body.contains(r#""frames":48"#), "{body}");
    let (count, seconds) = audio_tracks(&dir.file("with.mp4")).unwrap();
    assert_eq!(count, 1);
    assert!((seconds - 2.0).abs() < 0.15, "audio duration {seconds}");
    let video = decode(&dir.file("with.mp4")).unwrap();
    assert_eq!(video.frames.len(), 48);

    // mov works too, and no scratch or temp files are left behind.
    rig.doc_tool(
        "export_animation",
        &format!(
            r#""path":{},"format":"mov","width":32,"height":32"#,
            jstr(&dir.text("with.mov"))
        ),
    )
    .ok();
    assert_eq!(audio_tracks(&dir.file("with.mov")).unwrap().0, 1);
    let mut names: Vec<String> = fs::read_dir(&dir.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["audio", "film.vexel", "plain.mp4", "with.mov", "with.mp4"]
    );

    // A missing source file fails the export cleanly and writes nothing.
    fs::remove_file(dir.file("audio/tone.wav")).unwrap();
    let err = rig
        .doc_tool(
            "export_animation",
            &format!(r#""path":{},"format":"mp4""#, jstr(&dir.text("broken.mp4"))),
        )
        .error()
        .to_owned();
    assert!(
        err.contains("export audio track's file is missing"),
        "{err}"
    );
    assert!(!dir.file("broken.mp4").exists());
    // Removing every track restores the plain export.
    rig.doc_tool("audio_set", r#""trackId":"bed","remove":true"#)
        .ok();
    rig.doc_tool("audio_set", r#""trackId":"line1","remove":true"#)
        .ok();
    export(&mut rig, &dir, "again.mp4");
    assert_eq!(audio_tracks(&dir.file("again.mp4")).unwrap().0, 0);
}
