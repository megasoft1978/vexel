//! `native_timeline_set_camera` reaches GIF, mp4 and PNG frame exports and
//! works inside `batch`. A camera panning x by 20 shows the same pixels as a
//! scene whose rect sits 20 units further left, so every export of the camera
//! scene must equal the export of the moved scene and differ from the rect at
//! its original place.

mod common;

use common::{Dir, Rig, policy_for};
use std::fs;
use vexel_platform_apple::video_reader::decode;

const PAN: &str = r#""keys":[{"tick":0,"x":20}]"#;

/// A rect at `x` plus an opacity track that makes the timeline native.
fn scene(dir: &Dir, x: u32, camera: bool) -> Rig {
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let id = rig.add_rect(x, 40, "#2A7FFF");
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"fade","targetId":"{id}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#
        ),
    )
    .ok();
    if camera {
        rig.doc_tool("native_timeline_set_camera", PAN).ok();
    }
    rig
}

fn export(rig: &mut Rig, path: &str, extra: &str) {
    rig.doc_tool("export_animation", &format!(r#""path":"{path}"{extra}"#))
        .ok();
}

/// Compressed image data of every GIF frame; the encoder is deterministic, so
/// equal pixels give equal bytes.
fn gif_frame_streams(gif: &[u8]) -> Vec<Vec<u8>> {
    assert_eq!(&gif[..6], b"GIF89a");
    let mut at = 13 + (3usize << ((gif[10] & 7) + 1));
    let mut frames = Vec::new();
    let sub_blocks = |at: &mut usize| {
        let mut data = Vec::new();
        while gif[*at] != 0 {
            let len = gif[*at] as usize;
            data.extend_from_slice(&gif[*at + 1..*at + 1 + len]);
            *at += 1 + len;
        }
        *at += 1;
        data
    };
    loop {
        match gif[at] {
            0x3B => return frames,
            0x21 => {
                at += 2;
                sub_blocks(&mut at);
            }
            0x2C => {
                at += 10 + 1;
                frames.push(sub_blocks(&mut at));
            }
            other => panic!("bad GIF block {other:#x}"),
        }
    }
}

#[test]
fn gif_export_applies_the_camera() {
    let dir = Dir::new();
    let gif = |x: u32, camera: bool, name: &str| {
        let mut rig = scene(&dir, x, camera);
        export(
            &mut rig,
            &dir.text(name),
            r#","format":"gif","fps":4,"duration":0.5,"width":128,"height":128"#,
        );
        gif_frame_streams(&fs::read(dir.file(name)).unwrap())
    };
    let cam = gif(100, true, "cam.gif");
    assert!(!cam.is_empty());
    assert_eq!(cam, gif(80, false, "moved.gif"), "camera == moved rect");
    assert_ne!(cam, gif(100, false, "plain.gif"), "camera moves pixels");
}

#[test]
fn frames_export_applies_the_camera() {
    let dir = Dir::new();
    let first = |x: u32, camera: bool, name: &str| {
        let mut rig = scene(&dir, x, camera);
        let out = dir.sub(name);
        export(
            &mut rig,
            &format!("{}/f_####.png", out.display()),
            r#","format":"frames","fps":4,"width":128,"height":128"#,
        );
        (
            fs::read(out.join("f_0000.png")).unwrap(),
            fs::read(out.join("f_0003.png")).unwrap(),
        )
    };
    let cam = first(100, true, "cam");
    assert_eq!(cam, first(80, false, "moved"), "camera == moved rect");
    assert_ne!(cam.0, first(100, false, "plain").0, "camera moves pixels");
}

#[test]
fn mp4_export_applies_the_camera() {
    let dir = Dir::new();
    let frames = |x: u32, camera: bool, name: &str| {
        let mut rig = scene(&dir, x, camera);
        export(
            &mut rig,
            &dir.text(name),
            r#","format":"mp4","width":128,"height":128"#,
        );
        decode(&dir.file(name)).unwrap().frames
    };
    let cam = frames(100, true, "cam.mp4");
    let moved = frames(80, false, "moved.mp4");
    let plain = frames(100, false, "plain.mp4");
    assert_eq!(cam.len(), 48);
    assert_eq!(cam[0].rgba, moved[0].rgba, "camera == moved rect");
    assert_ne!(cam[0].rgba, plain[0].rgba, "camera moves pixels");
    // Rect covers x 80-120, y 40-70 under the camera: pixels (24, 14) at 1/4 scale.
    let pixel = |f: &vexel_platform_apple::video_reader::DecodedFrame, x: usize, y: usize| {
        let at = (y * f.width + x) * 4;
        [
            f.rgba[at] as i32,
            f.rgba[at + 1] as i32,
            f.rgba[at + 2] as i32,
        ]
    };
    let [r, _, b] = pixel(&cam[0], 24, 14);
    assert!(
        b > r + 60,
        "camera frame shows the rect at the panned place {r} {b}"
    );
    let [r, _, b] = pixel(&plain[0], 24, 14);
    assert!(b < r + 60, "no camera: that place is background {r} {b}");
}

fn op(tool: &str, arguments: &str) -> String {
    format!(r#"{{"tool":"{tool}","arguments":{{{arguments}}}}}"#)
}

fn batch(rig: &mut Rig, atomic: bool, ops: &[String]) -> common::Reply {
    rig.tool(
        "batch",
        &format!(
            r#"{{"documentId":"preview-document","atomic":{atomic},"ops":[{}]}}"#,
            ops.join(",")
        ),
    )
}

#[test]
fn set_camera_works_inside_batch_and_rolls_back() {
    let dir = Dir::new();
    let mut rig = scene(&dir, 100, false);
    let ok = batch(
        &mut rig,
        true,
        &[
            op("native_timeline_set_camera", PAN),
            op(
                "element_add",
                r##""documentId":"preview-document","parentId":"root","template":"rect","attributes":{"x":"5","y":"5","width":"10","height":"10","fill":"#336699"}"##,
            ),
        ],
    );
    assert!(ok.ok().contains(r#""ok":true"#), "{}", ok.body);
    assert!(
        rig.doc_tool("native_timeline_get", "")
            .ok()
            .contains("@camera")
    );
    rig.doc_tool("undo", "").ok();
    assert!(
        !rig.doc_tool("native_timeline_get", "")
            .ok()
            .contains("@camera"),
        "one undo step"
    );
    let failed = batch(
        &mut rig,
        true,
        &[
            op("native_timeline_set_camera", PAN),
            op(
                "native_timeline_set_property_key",
                r#""trackId":"missing","tick":1,"value":1"#,
            ),
        ],
    );
    assert!(failed.ok().contains(r#""ok":false"#), "{}", failed.body);
    assert!(
        !rig.doc_tool("native_timeline_get", "")
            .ok()
            .contains("@camera"),
        "atomic rollback"
    );
}
