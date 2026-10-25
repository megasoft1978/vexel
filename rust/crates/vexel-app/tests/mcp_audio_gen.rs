//! `audio_generate` and `audio_envelope` through the real JSON-RPC layer, with
//! a scripted fake helper standing in for `tools/vexel-audio` (the real helper
//! is exercised by `real_helper_*` when python3 and numpy are installed).

mod common;

use common::{Dir, Rig, policy_for};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;
use vexel_timeline::audio_codec::encode_wav16;

/// Behaviour is chosen by `params.preset`; see the match in the script.
const FAKE: &str = r#"#!/usr/bin/env python3
import json, os, struct, sys, time, wave
def wav(path, seconds=0.5, rate=48000):
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(rate)
        w.writeframes(b"".join(struct.pack("<h", 3000 if (i // 40) % 2 else -3000) for i in range(int(seconds * rate))))
for raw in sys.stdin:
    req = json.loads(raw)
    out, mode, rid = req["outPath"], req["params"].get("preset", "ok"), req["id"]
    d, base = os.path.split(out)
    name = base[1:].split(".vexel-audio-")[0]
    side = os.path.join(d, name)
    open(side + ".pid", "w").write(str(os.getpid()))
    open(side + ".req", "w").write(raw)
    open(side + ".env", "w").write(json.dumps(sorted(os.environ)))
    reply = {"id": rid, "ok": True, "path": out, "durationMs": 500}
    if mode == "ok" or mode == "echo":
        wav(out)
    elif mode == "hang":
        time.sleep(60)
    elif mode == "crash":
        os._exit(1)
    elif mode == "huge":
        wav(out)
        with open(out, "ab") as f: f.truncate(40 * 1024 * 1024)
    elif mode == "badwav":
        open(out, "wb").write(b"RIFFxxxxWAVEnot really")
    elif mode == "symlink":
        target = side + ".real"
        wav(target)
        os.symlink(target, out)
    elif mode == "garbage":
        sys.stdout.write("not json\n"); sys.stdout.flush(); continue
    elif mode == "wrongid":
        wav(out); reply["id"] = rid + 1000
    elif mode == "longline":
        sys.stdout.write("x" * 40000 + "\n"); sys.stdout.flush(); continue
    elif mode == "model":
        reply = {"id": rid, "ok": False, "code": "model_missing", "error": "x"}
    elif mode == "badparams":
        reply = {"id": rid, "ok": False, "code": "bad_params", "error": "x"}
    sys.stdout.write(json.dumps(reply) + "\n"); sys.stdout.flush()
"#;

fn python_ok(imports: &str) -> bool {
    Command::new("python3")
        .args(["-c", imports])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn fake() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("vexel-fake-audio-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fake-helper");
        fs::write(&path, FAKE).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    })
    .clone()
}

fn jstr(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn pid_of(dir: &Dir, name: &str) -> u32 {
    fs::read_to_string(dir.file(&format!("{name}.pid")))
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn gone(pid: u32) -> bool {
    (0..50).any(|_| {
        std::thread::sleep(Duration::from_millis(40));
        !alive(pid)
    })
}

fn saved(dir: &Dir, helper: PathBuf) -> Rig {
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
    rig.host.set_audio_helper(helper, Some(Duration::from_millis(1500)));
    rig
}

fn generate(rig: &mut Rig, dir: &Dir, preset: &str, name: &str, extra: &str) -> common::Reply {
    rig.doc_tool(
        "audio_generate",
        &format!(
            r#""kind":"sfx","params":{{"preset":"{preset}"}},"outPath":{}{extra}"#,
            jstr(&dir.text(name))
        ),
    )
}

#[test]
fn generates_a_file_and_adds_one_undoable_track() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    let reply = generate(
        &mut rig,
        &dir,
        "ok",
        "audio/blip.wav",
        r#","addTrack":{"trackId":"blip","startTick":6,"gainDb":-4}"#,
    )
    .ok()
    .to_owned();
    for part in [
        r#""ok":true"#,
        r#""kind":"sfx""#,
        r#""durationMs":500"#,
        r#""sampleRate":48000"#,
        r#""trackId":"blip""#,
        r#""path":"audio/blip.wav""#,
    ] {
        assert!(reply.contains(part), "{part} in {reply}");
    }
    assert!(dir.file("audio/blip.wav").is_file());
    let list = rig.doc_tool("audio_list", "").ok().to_owned();
    assert!(list.contains(r#""startTick":6"#) && list.contains(r#""gainDb":-4"#), "{list}");
    rig.doc_tool("native_timeline_undo", "").ok();
    assert!(rig.doc_tool("audio_list", "").ok().contains(r#""count":0"#));
    // No temporary files are left behind.
    let stray: Vec<_> = fs::read_dir(dir.file("audio"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("vexel-audio"))
        .collect();
    assert!(stray.is_empty(), "{stray:?}");
}

#[test]
fn user_text_travels_only_as_json_and_consent_is_forwarded() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    let evil = r#"\"; touch /tmp/vexel-pwned; $(touch /tmp/vexel-pwned2) `x`"#;
    let reply = rig.doc_tool(
        "audio_generate",
        &format!(
            r#""kind":"voice","params":{{"preset":"echo","text":"{evil}","speed":1.1,"flag":true}},"outPath":{},"consent":true"#,
            jstr(&dir.text("audio/v.wav"))
        ),
    );
    reply.ok();
    let req = fs::read_to_string(dir.file("audio/v.wav.req")).unwrap();
    assert!(req.contains(r#""consent":true"#) && req.contains(r#""kind":"voice""#), "{req}");
    assert!(req.contains(r#""speed":1.1"#) && req.contains(r#""flag":true"#), "{req}");
    assert!(req.contains(r#"\"; touch /tmp/vexel-pwned;"#), "{req}");
    assert!(!PathBuf::from("/tmp/vexel-pwned").exists());
    assert!(!PathBuf::from("/tmp/vexel-pwned2").exists());
    let again = generate(&mut rig, &dir, "echo", "audio/w.wav", "").ok().to_owned();
    assert!(again.contains(r#""ok":true"#));
    let req = fs::read_to_string(dir.file("audio/w.wav.req")).unwrap();
    assert!(req.contains(r#""consent":false"#), "{req}");
}

#[test]
fn helper_environment_is_scrubbed() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    generate(&mut rig, &dir, "echo", "audio/e.wav", "").ok();
    let env = fs::read_to_string(dir.file("audio/e.wav.env")).unwrap();
    for name in ["USER", "SHELL", "LOGNAME", "TERM", "PWD"] {
        assert!(!env.contains(&format!("\"{name}\"")), "{name} leaked: {env}");
    }
    assert!(env.contains("PATH"), "{env}");
}

#[test]
fn timeout_kills_the_helper() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    rig.host.set_audio_helper(fake(), Some(Duration::from_millis(400)));
    let reply = generate(&mut rig, &dir, "hang", "audio/h.wav", "");
    assert!(reply.error().contains("timed out"), "{}", reply.body);
    assert!(gone(pid_of(&dir, "audio/h.wav")), "helper survived the timeout");
    assert!(rig.host.audio_helper_pid().is_none());
    assert!(!dir.file("audio/h.wav").exists());
}

#[test]
fn crash_is_reported_and_the_helper_restarts() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    generate(&mut rig, &dir, "ok", "audio/a.wav", "").ok();
    let first = rig.host.audio_helper_pid().unwrap();
    let reply = generate(&mut rig, &dir, "crash", "audio/c.wav", "");
    assert!(reply.error().contains("exited unexpectedly"), "{}", reply.body);
    generate(&mut rig, &dir, "ok", "audio/b.wav", "").ok();
    let second = rig.host.audio_helper_pid().unwrap();
    assert_ne!(first, second);
    assert!(dir.file("audio/b.wav").is_file());
}

#[test]
fn bad_helper_output_is_rejected_and_discarded() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    for (mode, expect) in [
        ("huge", "unusable file"),
        ("badwav", "unusable file"),
        ("symlink", "unusable file"),
        ("garbage", "invalid reply"),
        ("wrongid", "invalid reply"),
        ("longline", "invalid reply"),
    ] {
        let name = format!("audio/{mode}.wav");
        let reply = generate(&mut rig, &dir, mode, &name, "");
        assert!(reply.error().contains(expect), "{mode}: {}", reply.body);
        assert!(!dir.file(&name).exists(), "{mode} left an output");
        let temps = fs::read_dir(dir.file("audio"))
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains("vexel-audio")
            })
            .count();
        assert_eq!(temps, 0, "{mode} left a temp file");
    }
    // A malformed reply kills that helper; the next request still works.
    generate(&mut rig, &dir, "ok", "audio/fine.wav", "").ok();
}

#[test]
fn helper_errors_are_typed() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    let reply = generate(&mut rig, &dir, "model", "audio/m.wav", "");
    let message = reply.error();
    assert!(message.contains("model_missing") && message.contains("--download-model"), "{message}");
    assert!(message.contains("consent=true"), "{message}");
    let reply = generate(&mut rig, &dir, "badparams", "audio/p.wav", "");
    assert!(reply.error().contains("whoosh"), "{}", reply.body);
}

#[test]
fn paths_are_confined_to_the_grant_and_the_document() {
    let dir = Dir::new();
    let other = Dir::new();
    let mut rig = saved(&dir, fake());
    let elsewhere = generate(&mut rig, &other, "ok", "x.wav", "");
    assert!(elsewhere.error().contains("not granted"), "{}", elsewhere.body);
    let not_wav = generate(&mut rig, &dir, "ok", "audio/x.mp3", "");
    assert!(not_wav.error().contains(".wav"), "{}", not_wav.body);
    let traversal = rig.doc_tool(
        "audio_generate",
        &format!(
            r#""kind":"sfx","params":{{"preset":"ok"}},"outPath":{}"#,
            jstr(&format!("{}/audio/../../escape.wav", dir.text("")))
        ),
    );
    assert!(traversal.is_error, "{}", traversal.body);
    // Granted but outside the document directory: fine alone, refused with addTrack.
    let wide = Dir::new();
    let mut rig2 = Rig::new(policy_for(&[&dir.0, &wide.0]));
    rig2.host.set_audio_helper(fake(), Some(Duration::from_millis(1500)));
    rig2.doc_tool("save_as", &format!(r#""path":{}"#, jstr(&dir.text("film.vexel"))))
        .ok();
    generate(&mut rig2, &wide, "ok", "free.wav", "").ok();
    let with_track = generate(&mut rig2, &wide, "ok", "tied.wav", r#","addTrack":{"trackId":"t"}"#);
    assert!(with_track.error().contains("inside the document"), "{}", with_track.body);
    assert!(!wide.file("tied.wav").exists());
    assert!(fs::read_dir(other.file("")).unwrap().next().is_none());
}

#[test]
fn failed_add_removes_the_new_file() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    generate(&mut rig, &dir, "ok", "audio/one.wav", r#","addTrack":{"trackId":"dup"}"#).ok();
    let reply = generate(&mut rig, &dir, "ok", "audio/two.wav", r#","addTrack":{"trackId":"dup"}"#);
    assert!(reply.error().contains("already exists"), "{}", reply.body);
    assert!(!dir.file("audio/two.wav").exists());
    assert!(dir.file("audio/one.wav").exists());
}

#[test]
fn absent_helper_is_a_normal_error() {
    let dir = Dir::new();
    let mut rig = saved(&dir, dir.file("no-such-helper"));
    let reply = generate(&mut rig, &dir, "ok", "audio/n.wav", "");
    assert!(reply.error().contains("helper"), "{}", reply.body);
    assert!(rig.host.audio_helper_pid().is_none());
}

#[test]
fn dropping_the_host_kills_the_helper() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    generate(&mut rig, &dir, "ok", "audio/d.wav", "").ok();
    let pid = pid_of(&dir, "audio/d.wav");
    assert!(alive(pid));
    drop(rig);
    assert!(gone(pid), "helper outlived the session");
}

#[test]
fn session_budget_is_enforced() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    rig.host.limit_audio_requests(3);
    for i in 0..3 {
        let r = generate(&mut rig, &dir, "ok", &format!("audio/b{i}.wav"), "");
        assert!(!r.is_error, "request {i}: {}", r.body);
    }
    let over = generate(&mut rig, &dir, "ok", "audio/over.wav", "");
    assert!(over.error().contains("budget"), "{}", over.body);
}

fn tone(seconds: f64, rate: u32, loud_until: f64) -> Vec<u8> {
    let frames = (seconds * f64::from(rate)) as usize;
    let samples: Vec<i16> = (0..frames)
        .map(|i| {
            let t = i as f64 / f64::from(rate);
            let amp = if t < loud_until { 12000.0 } else { 3000.0 };
            ((i as f64 * 440.0 * std::f64::consts::TAU / f64::from(rate)).sin() * amp) as i16
        })
        .collect();
    encode_wav16(rate, 1, &samples)
}

#[test]
fn envelope_is_deterministic_and_per_tick() {
    let dir = Dir::new();
    let mut rig = saved(&dir, fake());
    fs::write(dir.file("audio/line.wav"), tone(2.0, 24_000, 1.0)).unwrap();
    let args = format!(r#""path":{}"#, jstr(&dir.text("audio/line.wav")));
    let a = rig.doc_tool("audio_envelope", &args).ok().to_owned();
    let b = rig.doc_tool("audio_envelope", &args).ok().to_owned();
    assert_eq!(a, b);
    assert!(a.contains(r#""tickRate":24"#) && a.contains(r#""ticks":48"#), "{a}");
    assert!(a.contains(r#""durationMs":2000"#), "{a}");
    let values: Vec<f64> = a
        .split(r#""rms":["#)
        .nth(1)
        .unwrap()
        .trim_end_matches("]}")
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(values.len(), 48);
    assert!(values[..24].iter().all(|v| *v > 0.95), "{values:?}");
    assert!(values[24..].iter().all(|v| (*v - 0.25).abs() < 0.05), "{values:?}");
    // ticks limits the count; a track envelope starts at the track's startTick.
    let few = rig.doc_tool("audio_envelope", &format!("{args},\"ticks\":5")).ok().to_owned();
    assert!(few.contains(r#""ticks":5"#), "{few}");
    rig.doc_tool(
        "audio_add_track",
        r#""trackId":"line","kind":"voice","path":"audio/line.wav","startTick":7,"trimInMs":1000"#,
    )
    .ok();
    let track = rig.doc_tool("audio_envelope", r#""trackId":"line""#).ok().to_owned();
    assert!(track.contains(r#""startTick":7"#) && track.contains(r#""ticks":24"#), "{track}");
    // Argument and path errors are typed and change nothing.
    assert!(rig.doc_tool("audio_envelope", "").is_error);
    assert!(rig.doc_tool("audio_envelope", &format!("{args},\"trackId\":\"line\"")).is_error);
    assert!(rig.doc_tool("audio_envelope", &format!("{args},\"ticks\":0")).is_error);
    assert!(rig.doc_tool("audio_envelope", r#""trackId":"nope""#).is_error);
    let other = Dir::new();
    fs::write(other.file("x.wav"), tone(1.0, 24_000, 1.0)).unwrap();
    let outside = rig.doc_tool("audio_envelope", &format!(r#""path":{}"#, jstr(&other.text("x.wav"))));
    assert!(outside.error().contains("not granted") || outside.error().contains("not found"), "{}", outside.body);
}

fn real_helper() -> PathBuf {
    PathBuf::from(format!("{}/../../../tools/vexel-audio/vexel-audio", env!("CARGO_MANIFEST_DIR")))
}

#[test]
fn real_helper_selftest_and_sfx() {
    if !python_ok("import numpy") {
        eprintln!("skipped: python3 with numpy is not installed");
        return;
    }
    let out = Command::new(real_helper()).arg("--selftest").output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let dir = Dir::new();
    let mut rig = saved(&dir, real_helper());
    rig.host.set_audio_helper(real_helper(), None);
    let a = generate(&mut rig, &dir, "chime", "audio/chime.wav", r#","addTrack":{"trackId":"chime"}"#)
        .ok()
        .to_owned();
    assert!(a.contains(r#""durationMs":1000"#) && a.contains(r#""sampleRate":48000"#), "{a}");
    // Deterministic: the same preset and seed give identical bytes.
    generate(&mut rig, &dir, "chime", "audio/chime2.wav", "").ok();
    assert_eq!(
        fs::read(dir.file("audio/chime.wav")).unwrap(),
        fs::read(dir.file("audio/chime2.wav")).unwrap()
    );
    let music = rig
        .doc_tool(
            "audio_generate",
            &format!(
                r#""kind":"music","params":{{"mood":"calm","key":"C","tempo":120,"bars":4}},"outPath":{}"#,
                jstr(&dir.text("audio/loop.wav"))
            ),
        )
        .ok()
        .to_owned();
    assert!(music.contains(r#""durationMs":8000"#), "{music}");
    // Without the model the real helper reports model_missing and downloads nothing.
    let cache = Dir::new();
    let wrapper = cache.file("helper.sh");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nHF_HUB_CACHE={} exec {} \"$@\"\n",
            cache.text("hub"),
            real_helper().display()
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    rig.host.set_audio_helper(wrapper, None);
    let voice = rig.doc_tool(
        "audio_generate",
        &format!(
            r#""kind":"voice","params":{{"text":"hello"}},"outPath":{}"#,
            jstr(&dir.text("audio/v.wav"))
        ),
    );
    assert!(voice.error().contains("model_missing"), "{}", voice.body);
    assert!(!cache.file("hub").exists(), "nothing may be downloaded without consent");
    assert!(!dir.file("audio/v.wav").exists());
}

#[test]
#[ignore = "needs mlx-audio and the cached Kokoro model; run with --ignored"]
fn real_kokoro_voice() {
    let dir = Dir::new();
    let mut rig = saved(&dir, real_helper());
    rig.host.set_audio_helper(real_helper(), Some(Duration::from_secs(120)));
    let reply = rig
        .doc_tool(
            "audio_generate",
            &format!(
                r#""kind":"voice","params":{{"text":"Hello from Vexel.","voice":"af_heart","effect":"pip"}},"outPath":{},"addTrack":{{"trackId":"hello"}}"#,
                jstr(&dir.text("audio/hello.wav"))
            ),
        )
        .ok()
        .to_owned();
    assert!(reply.contains(r#""ok":true"#), "{reply}");
    let env = rig.doc_tool("audio_envelope", r#""trackId":"hello""#).ok().to_owned();
    assert!(env.contains(r#""peakRms""#), "{env}");
}
