//! `export_animation` as apng and PNG frame sequences through the real
//! JSON-RPC layer and `McpHost`. APNG output is checked by walking its chunks.

mod common;

use common::{Dir, Rig, policy_for};
use std::fs;

/// A 512x512 document with one rect fading in over a 48-tick (2 s at 24 tps)
/// native timeline: opacity 0 at tick 0, 1 at tick 20.
fn animated(dir: &Dir) -> Rig {
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
        "native_timeline_set_property_key",
        r#""trackId":"fade","tick":20,"value":1"#,
    )
    .ok();
    rig
}

fn export(rig: &mut Rig, path: &str, extra: &str) -> common::Reply {
    rig.doc_tool("export_animation", &format!(r#""path":"{path}"{extra}"#))
}

fn entries(dir: &Dir) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(&dir.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

struct Apng {
    width: u32,
    height: u32,
    frames: u32,
    plays: u32,
    /// `fcTL` entries: (delay_num, delay_den).
    delays: Vec<(u16, u16)>,
    /// Compressed pixel data of each frame (IDAT, then fdAT without sequence).
    data: Vec<Vec<u8>>,
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes(b[..4].try_into().unwrap())
}

fn parse_apng(bytes: &[u8]) -> Apng {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut at = 8;
    let mut out = Apng {
        width: 0,
        height: 0,
        frames: 0,
        plays: 0,
        delays: Vec::new(),
        data: Vec::new(),
    };
    let mut ended = false;
    while at < bytes.len() {
        let len = be32(&bytes[at..]) as usize;
        let kind = &bytes[at + 4..at + 8];
        let body = &bytes[at + 8..at + 8 + len];
        match kind {
            b"IHDR" => {
                out.width = be32(body);
                out.height = be32(&body[4..]);
            }
            b"acTL" => {
                out.frames = be32(body);
                out.plays = be32(&body[4..]);
            }
            b"fcTL" => {
                out.delays.push((
                    u16::from_be_bytes([body[20], body[21]]),
                    u16::from_be_bytes([body[22], body[23]]),
                ));
                out.data.push(Vec::new());
            }
            b"IDAT" => out.data.last_mut().unwrap().extend_from_slice(body),
            b"fdAT" => out.data.last_mut().unwrap().extend_from_slice(&body[4..]),
            b"IEND" => ended = true,
            _ => {}
        }
        at += 12 + len;
    }
    assert!(ended && at == bytes.len());
    out
}

#[test]
fn apng_of_a_three_key_timeline_decodes_back() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    rig.doc_tool(
        "native_timeline_set_property_key",
        r#""trackId":"fade","tick":40,"value":0.2"#,
    )
    .ok();
    let reply = export(
        &mut rig,
        &dir.text("loop.apng"),
        r#","format":"apng","fps":12,"width":64,"height":48,"loops":3"#,
    );
    let body = reply.ok().to_owned();
    for part in [
        r#""format":"apng""#,
        r#""width":64"#,
        r#""height":48"#,
        r#""frames":24"#,
        r#""fps":12.000"#,
        r#""durationSeconds":2.000"#,
        r#""loops":3"#,
    ] {
        assert!(body.contains(part), "{part} in {body}");
    }
    let apng = parse_apng(&fs::read(dir.file("loop.apng")).unwrap());
    assert_eq!((apng.width, apng.height), (64, 48));
    assert_eq!((apng.frames, apng.plays), (24, 3));
    assert_eq!(apng.data.len(), 24);
    assert!(
        apng.delays.iter().all(|&d| d == (1, 12)),
        "{:?}",
        apng.delays
    );
    assert_ne!(apng.data[0], apng.data[23], "first and last frame differ");
    assert_ne!(apng.data[0], apng.data[10]);
    assert_eq!(entries(&dir), ["loop.apng"], "no temp file left");
}

#[test]
fn apng_defaults_to_one_frame_per_tick_looping_forever_and_overwrites() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    export(
        &mut rig,
        &dir.text("a.png"),
        r#","format":"apng","width":32,"height":32"#,
    )
    .ok();
    let apng = parse_apng(&fs::read(dir.file("a.png")).unwrap());
    assert_eq!((apng.frames, apng.plays), (48, 0));
    assert!(apng.delays.iter().all(|&d| d == (1, 24)));
    // The same path is replaced atomically, with a background colour this time.
    export(
        &mut rig,
        &dir.text("a.png"),
        r##","format":"apng","fps":6,"width":32,"height":32,"background":"#102030""##,
    )
    .ok();
    let apng = parse_apng(&fs::read(dir.file("a.png")).unwrap());
    assert_eq!(apng.frames, 12);
    assert_eq!(entries(&dir), ["a.png"]);
}

#[test]
fn frames_write_one_numbered_png_per_tick() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    let out = dir.sub("out");
    let pattern = format!("{}/shot_####.png", out.display());
    let body = export(
        &mut rig,
        &pattern,
        r#","format":"frames","fps":4,"width":16,"height":16"#,
    )
    .ok()
    .to_owned();
    for part in [
        r#""format":"frames""#,
        r#""first":"shot_0000.png""#,
        r#""last":"shot_0007.png""#,
        r#""frames":8"#,
        r#""fps":4.000"#,
    ] {
        assert!(body.contains(part), "{part} in {body}");
    }
    let names: Vec<String> = (0..8).map(|i| format!("shot_{i:04}.png")).collect();
    let mut found: Vec<String> = fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    assert_eq!(found, names);
    let first = fs::read(out.join("shot_0000.png")).unwrap();
    let last = fs::read(out.join("shot_0007.png")).unwrap();
    assert_eq!(&first[..8], b"\x89PNG\r\n\x1a\n");
    assert_ne!(first, last);
    // A prefix without extension names files prefix_0000.png.
    let prefix = format!("{}/clip", out.display());
    export(
        &mut rig,
        &prefix,
        r#","format":"frames","fps":24,"width":8,"height":8"#,
    )
    .ok();
    assert!(out.join("clip_0000.png").is_file() && out.join("clip_0047.png").is_file());
    assert_eq!(fs::read_dir(&out).unwrap().count(), 8 + 48);
}

#[test]
fn ping_pong_loops_write_the_reflected_ticks() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    rig.doc_tool("native_timeline_set_loop", r#""mode":"ping_pong""#)
        .ok();
    // 48 ticks ping-pong = 94 steps (end ticks not repeated), all written.
    let out = dir.sub("pp");
    let body = export(
        &mut rig,
        &format!("{}/f_####.png", out.display()),
        r#","format":"frames","width":4,"height":4"#,
    )
    .ok()
    .to_owned();
    assert!(body.contains(r#""frames":94"#), "{body}");
}

#[test]
fn rejected_calls_return_clear_errors_and_write_nothing() {
    let dir = Dir::new();
    let other = Dir::new();
    let mut rig = animated(&dir);
    let out = dir.sub("out");
    let before = rig.state();
    let pat = |name: &str| format!("{}/{name}", out.display());
    let cases: Vec<(String, &str)> = vec![
        (
            export(&mut rig, &dir.text("a.gif"), r#","format":"apng""#)
                .error()
                .to_owned(),
            "export apng path must end in .png or .apng",
        ),
        (
            export(&mut rig, &other.text("a.png"), r#","format":"apng""#)
                .error()
                .to_owned(),
            "path not granted",
        ),
        (
            export(&mut rig, &other.text("f_####.png"), r#","format":"frames""#)
                .error()
                .to_owned(),
            "path not granted",
        ),
        (
            export(&mut rig, &pat("f_###.png"), r#","format":"frames""#)
                .error()
                .to_owned(),
            "export frames path must be name_####.png (four or more # for the frame number) or a name prefix without extension",
        ),
        (
            export(&mut rig, &pat("f_####.jpg"), r#","format":"frames""#)
                .error()
                .to_owned(),
            "export frames path must be name_####.png (four or more # for the frame number) or a name prefix without extension",
        ),
        (
            export(&mut rig, &pat("f.png"), r#","format":"frames""#)
                .error()
                .to_owned(),
            "export frames path must be name_####.png (four or more # for the frame number) or a name prefix without extension",
        ),
        (
            export(
                &mut rig,
                &dir.text("missing/f_####.png"),
                r#","format":"frames""#,
            )
            .error()
            .to_owned(),
            "path not found",
        ),
        (
            export(
                &mut rig,
                &pat("f_####.png"),
                r#","format":"frames","loops":2"#,
            )
            .error()
            .to_owned(),
            "export loops applies to apng only",
        ),
        (
            export(
                &mut rig,
                &pat("f_####.png"),
                r#","format":"frames","codec":"h264""#,
            )
            .error()
            .to_owned(),
            "export codec applies to mp4 and mov only, not frames",
        ),
        (
            export(
                &mut rig,
                &dir.text("a.apng"),
                r#","format":"apng","width":64"#,
            )
            .error()
            .to_owned(),
            "export width and height must be given together",
        ),
        (
            export(
                &mut rig,
                &dir.text("a.apng"),
                r#","format":"apng","duration":1"#,
            )
            .error()
            .to_owned(),
            "export duration applies to gif only; apng export the whole timeline loop",
        ),
        (
            export(
                &mut rig,
                &dir.text("a.apng"),
                r#","format":"apng","background":"red""#,
            )
            .error()
            .to_owned(),
            "export background must be a #rgb or #rrggbb colour",
        ),
    ];
    for (got, want) in cases {
        assert_eq!(got, want);
    }
    assert_eq!(rig.state(), before);
    assert!(entries(&dir) == ["out"] && entries(&Dir(out.clone())).is_empty());
    assert!(entries(&other).is_empty());
}

#[test]
fn frames_refuse_to_overwrite_and_leave_the_directory_untouched() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    let out = dir.sub("out");
    fs::write(out.join("f_0003.png"), b"keep").unwrap();
    let error = export(
        &mut rig,
        &format!("{}/f_####.png", out.display()),
        r#","format":"frames","fps":12,"width":8,"height":8"#,
    )
    .error()
    .to_owned();
    assert_eq!(
        error,
        "export frames would overwrite existing files; use a new name or an empty directory"
    );
    assert_eq!(fs::read(out.join("f_0003.png")).unwrap(), b"keep");
    assert_eq!(fs::read_dir(&out).unwrap().count(), 1);
}

#[test]
fn oversized_exports_are_refused() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    // 48 frames of 4096x4096 RGBA is 3 GiB, past the in-memory APNG cap.
    let error = export(
        &mut rig,
        &dir.text("big.apng"),
        r#","format":"apng","width":4096,"height":4096"#,
    )
    .error()
    .to_owned();
    assert_eq!(error, "export too large; reduce size, fps or frame count");
    assert!(entries(&dir).is_empty());
}

#[test]
fn apng_and_frames_need_a_native_timeline() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    rig.add_rect(10, 10, "#336699");
    for (path, format) in [("a.apng", "apng"), ("f_####.png", "frames")] {
        let error = export(
            &mut rig,
            &dir.text(path),
            &format!(r#","format":"{format}""#),
        )
        .error()
        .to_owned();
        assert!(
            error.starts_with("export apng and frames need a native timeline"),
            "{error}"
        );
    }
    assert!(entries(&dir).is_empty());
}
