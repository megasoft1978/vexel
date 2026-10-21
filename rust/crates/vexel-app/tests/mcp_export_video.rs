//! `export_animation` as mp4/mov through the real JSON-RPC layer and
//! `McpHost`, decoded back with the AVAssetReader test helper.

mod common;

use common::{Dir, Rig, policy_for};
use std::fs;
use vexel_platform_apple::video_reader::decode;

/// A 512x512 document with one rect fading in over a 48-tick (2 s at 24 tps)
/// native timeline.
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

#[test]
fn mp4_exports_one_frame_per_tick_at_the_requested_size() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    let reply = export(
        &mut rig,
        &dir.text("loop.mp4"),
        r#","format":"mp4","width":64,"height":48"#,
    );
    let body = reply.ok().to_owned();
    for part in [
        r#""format":"mp4""#,
        r#""codec":"h264""#,
        r#""width":64"#,
        r#""height":48"#,
        r#""frames":48"#,
        r#""fps":24.000"#,
    ] {
        assert!(body.contains(part), "{part} in {body}");
    }
    let video = decode(&dir.file("loop.mp4")).unwrap();
    assert_eq!(video.frames.len(), 48);
    assert!(
        (video.duration_seconds - 2.0).abs() < 0.05,
        "{}",
        video.duration_seconds
    );
    assert_eq!(video.natural_size, (64.0, 48.0));
    assert_eq!((video.frames[0].width, video.frames[0].height), (64, 48));
    assert_eq!(entries(&dir), ["loop.mp4"], "no temp file left");
}

#[test]
fn mov_hevc_with_fps_loops_and_background() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    // 48 ticks x 2 loops at 12 fps: every second tick, 48 frames, 4 s.
    let reply = export(
        &mut rig,
        &dir.text("clip.mov"),
        r##","format":"mov","codec":"hevc","fps":12,"loops":2,"width":64,"height":64,"quality":0.5,"background":"#102030""##,
    );
    let body = reply.ok().to_owned();
    assert!(
        body.contains(r#""codec":"hevc""#) && body.contains(r#""frames":48"#),
        "{body}"
    );
    assert!(body.contains(r#""fps":12.000"#), "{body}");
    let video = decode(&dir.file("clip.mov")).unwrap();
    assert_eq!(video.frames.len(), 48);
    assert!(
        (video.duration_seconds - 4.0).abs() < 0.05,
        "{}",
        video.duration_seconds
    );
    assert_eq!(video.natural_size, (64.0, 64.0));
    // The first frame (rect at opacity 0) is the background, not transparent.
    let px = &video.frames[0].rgba[..4];
    assert!(px[3] == 255 && px[2] > px[0], "{px:?}");
    let reply = export(
        &mut rig,
        &dir.text("rate.mp4"),
        r#","format":"mp4","bitrate":800,"width":32,"height":32"#,
    );
    reply.ok();
    assert_eq!(decode(&dir.file("rate.mp4")).unwrap().frames.len(), 48);
}

#[test]
fn default_size_is_the_document_size() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    export(
        &mut rig,
        &dir.text("full.mp4"),
        r#","format":"mp4","fps":6"#,
    )
    .ok();
    let video = decode(&dir.file("full.mp4")).unwrap();
    assert_eq!(video.natural_size, (512.0, 512.0));
    assert_eq!(video.frames.len(), 12);
}

#[test]
fn rejected_calls_return_clear_errors_and_write_nothing() {
    let dir = Dir::new();
    let other = Dir::new();
    let mut rig = animated(&dir);
    let before = rig.state();
    let cases: Vec<(String, String)> = vec![
        (
            export(&mut rig, &dir.text("a.mov"), r#","format":"mp4""#).error().to_owned(),
            "export path extension must match the format (.mp4 or .mov)".to_owned(),
        ),
        (
            export(&mut rig, &dir.text("a.mp4"), r#","format":"mov""#).error().to_owned(),
            "export path extension must match the format (.mp4 or .mov)".to_owned(),
        ),
        (
            export(&mut rig, &dir.text("a.gif"), r#","format":"mp4""#).error().to_owned(),
            "export path extension must match the format (.mp4 or .mov)".to_owned(),
        ),
        (
            export(&mut rig, &other.text("a.mp4"), r#","format":"mp4""#).error().to_owned(),
            "path not granted".to_owned(),
        ),
        (
            export(&mut rig, &dir.text("a.mp4"), r#","format":"mp4","width":5000,"height":64"#)
                .error()
                .to_owned(),
            "export width and height must be integers from 1 to 4096".to_owned(),
        ),
        (
            export(&mut rig, &dir.text("a.mp4"), r#","format":"mp4","width":64"#).error().to_owned(),
            "export width and height must be given together".to_owned(),
        ),
        (
            export(&mut rig, &dir.text("a.mp4"), r#","format":"mp4","loops":101"#).error().to_owned(),
            "export loops must be an integer from 1 to 100".to_owned(),
        ),
        (
            export(&mut rig, &dir.text("a.mp4"), r#","format":"mp4","codec":"vp9""#).error().to_owned(),
            "export codec must be h264 or hevc".to_owned(),
        ),
        (
            export(&mut rig, &dir.text("a.mp4"), r#","format":"mp4","quality":0.5,"bitrate":900"#)
                .error()
                .to_owned(),
            "export takes quality or bitrate, not both".to_owned(),
        ),
        (
            export(&mut rig, &dir.text("a.mp4"), r#","format":"mp4","quality":2"#).error().to_owned(),
            "export quality must be a number from 0 to 1".to_owned(),
        ),
        (
            export(&mut rig, &dir.text("a.mp4"), r#","format":"mp4","duration":1"#).error().to_owned(),
            "export duration applies to gif only; mp4 and mov export the whole timeline loop (use loops to repeat it)".to_owned(),
        ),
    ];
    for (got, want) in cases {
        assert_eq!(got, want);
    }
    assert_eq!(rig.state(), before);
    assert!(
        entries(&dir).iter().all(|n| !n.contains("a.")),
        "{:?}",
        entries(&dir)
    );
    assert!(entries(&other).is_empty());
}

#[test]
fn video_needs_a_native_timeline_and_gif_still_works() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    rig.add_rect(10, 10, "#336699");
    let error = export(&mut rig, &dir.text("a.mp4"), r#","format":"mp4""#)
        .error()
        .to_owned();
    assert!(
        error.starts_with("export mp4 and mov need a native timeline"),
        "{error}"
    );
    assert!(entries(&dir).is_empty());
    // GIF without fps/duration is still rejected by the protocol layer.
    let gif = export(&mut rig, &dir.text("a.gif"), r#","format":"gif""#).body;
    assert!(gif.contains("export requires a path"), "{gif}");
}

#[test]
fn hevc_alpha_mov_keeps_the_transparent_first_frame() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    let reply = export(
        &mut rig,
        &dir.text("alpha.mov"),
        r##","format":"mov","codec":"hevc","alpha":true,"width":64,"height":64,"background":"#102030""##,
    );
    let body = reply.ok().to_owned();
    assert!(body.contains(r#""codec":"hevc""#), "{body}");
    let video = decode(&dir.file("alpha.mov")).unwrap();
    assert_eq!(video.frames.len(), 48);
    // Frame 0 has the rect at opacity 0: corners stay transparent, not background.
    let px = &video.frames[0].rgba[..4];
    assert!(px[3] < 255, "{px:?}");
    // Last frame: rect fully visible, so some pixel is opaque.
    let last = video.frames.last().unwrap();
    assert!(last.rgba.chunks_exact(4).any(|p| p[3] > 200));
}

#[test]
fn alpha_is_rejected_without_mov_hevc() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    let cases = [
        (
            r#","format":"mp4","codec":"hevc","alpha":true"#,
            "a.mp4",
            "export alpha needs format mov (mp4 cannot carry alpha)",
        ),
        (
            r#","format":"mov","alpha":true"#,
            "a.mov",
            "export alpha needs codec hevc (h264 cannot carry alpha)",
        ),
        (
            r#","format":"mov","codec":"h264","alpha":true"#,
            "a.mov",
            "export alpha needs codec hevc (h264 cannot carry alpha)",
        ),
    ];
    for (extra, file, want) in cases {
        assert_eq!(export(&mut rig, &dir.text(file), extra).error(), want);
    }
    let typed = export(
        &mut rig,
        &dir.text("a.mov"),
        r#","format":"mov","codec":"hevc","alpha":"yes""#,
    )
    .body;
    assert!(typed.contains("`alpha` has the wrong type"), "{typed}");
    assert!(entries(&dir).is_empty(), "{:?}", entries(&dir));
    // alpha:false behaves like omitting it.
    export(
        &mut rig,
        &dir.text("ok.mp4"),
        r#","format":"mp4","alpha":false,"width":32,"height":32"#,
    )
    .ok();
}

#[test]
fn schema_lists_video_formats_and_options() {
    let dir = Dir::new();
    let mut rig = animated(&dir);
    let listing = rig.line(r#"{"jsonrpc":"2.0","id":7,"method":"tools/list","params":{}}"#);
    for part in [
        r#""enum":["gif","mp4","mov"]"#,
        r#""codec":{"type":"string","enum":["h264","hevc"]}"#,
        r#""loops":{"type":"integer","minimum":1,"maximum":100}"#,
        r#""width":{"type":"integer","minimum":1,"maximum":4096}"#,
        r#""alpha":{"type":"boolean"}"#,
    ] {
        assert!(listing.contains(part), "{part}");
    }
}
