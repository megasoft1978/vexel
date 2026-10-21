//! Live-preview JSON-RPC acceptance scenario for a walking character at sunset.

mod common;

use common::{Dir, Rig, element_id, policy_for};
use std::fs;

fn args(members: &str) -> String {
    let comma = if members.is_empty() { "" } else { "," };
    format!("{{\"documentId\":\"preview-document\"{comma}{members}}}")
}

fn call(rig: &mut Rig, tool: &str, members: &str) -> String {
    let reply = rig.tool(tool, &args(members));
    assert!(!reply.is_error, "{tool} {members}: {}", reply.body);
    reply.body
}

fn add(rig: &mut Rig, parent: &str, template: &str, attributes: &str) -> String {
    let body = call(
        rig,
        "element_add",
        &format!(
            "\"parentId\":\"{parent}\",\"template\":\"{template}\",\"attributes\":{{{attributes}}}"
        ),
    );
    element_id(&body)
}

fn native(rig: &mut Rig, operation: &str, members: &str) -> String {
    call(rig, &format!("native_timeline_{operation}"), members)
}

#[test]
fn walking_character_at_sunset_live_preview_native_gif() {
    let dir = Dir::new();
    let project = dir.text("sunset.vexel");
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let names = rig.list_tools();
    assert_eq!(names.len(), 52);

    let sky = add(
        &mut rig,
        "root",
        "rect",
        "\"x\":\"0\",\"y\":\"0\",\"width\":\"512\",\"height\":\"512\",\"fill\":\"#000000\"",
    );
    let gradient = call(
        &mut rig,
        "gradient_add",
        "\"kind\":\"radial\",\"units\":\"userSpace\",\"cx\":256,\"cy\":190,\"r\":320,\"stops\":\"0:#ffd36a:1,1:#44205f:1\"",
    );
    let gradient_id = gradient
        .split("\"gradientId\":\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let stop_id = gradient
        .split("\"stopIds\":[\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    call(
        &mut rig,
        "element_set_paint",
        &format!("\"elementId\":\"{sky}\",\"property\":\"fill\",\"gradientId\":\"{gradient_id}\""),
    );
    let sun = add(
        &mut rig,
        "root",
        "ellipse",
        "\"cx\":\"360\",\"cy\":\"190\",\"rx\":\"58\",\"ry\":\"58\",\"fill\":\"#fff1a8\"",
    );
    call(
        &mut rig,
        "element_set_paint",
        &format!("\"elementId\":\"{sun}\",\"property\":\"fill\",\"gradientId\":\"{gradient_id}\""),
    );
    let ground = add(
        &mut rig,
        "root",
        "rect",
        "\"x\":\"0\",\"y\":\"350\",\"width\":\"512\",\"height\":\"162\",\"fill\":\"#28533c\"",
    );
    let clip = call(
        &mut rig,
        "clip_add",
        "\"shape\":\"rect\",\"a\":0,\"b\":0,\"c\":512,\"d\":512",
    );
    let clip_id = clip
        .split("\"clipId\":\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    call(
        &mut rig,
        "element_set_clip",
        &format!("\"elementId\":\"{ground}\",\"clipId\":\"{clip_id}\""),
    );

    let body = add(
        &mut rig,
        "root",
        "rect",
        "\"x\":\"226\",\"y\":\"265\",\"width\":\"56\",\"height\":\"83\",\"fill\":\"#3568a8\"",
    );
    let head = add(
        &mut rig,
        "root",
        "ellipse",
        "\"cx\":\"254\",\"cy\":\"237\",\"rx\":\"24\",\"ry\":\"25\",\"fill\":\"#efb18a\"",
    );
    let arm = add(
        &mut rig,
        "root",
        "path",
        "\"d\":\"M230 280 L207 315\",\"stroke\":\"#efb18a\",\"stroke-width\":\"12\",\"fill\":\"none\"",
    );
    let leg = add(
        &mut rig,
        "root",
        "path",
        "\"d\":\"M240 345 L225 393\",\"stroke\":\"#273b58\",\"stroke-width\":\"14\",\"fill\":\"none\"",
    );
    let group = element_id(
        &call(
            &mut rig,
            "element_group",
            &format!("\"elementId\":\"{body}\""),
        )
        .to_owned(),
    );
    let title = call(
        &mut rig,
        "text_add",
        "\"x\":24,\"y\":54,\"content\":\"A walk at dusk\",\"fontSize\":28,\"fill\":\"#fff3cb\"",
    );
    let title_id = element_id(&title);
    call(
        &mut rig,
        "text_set",
        &format!("\"elementId\":\"{title_id}\",\"text\":\"A walk at dusk — Vexel\""),
    );
    let cloud = add(
        &mut rig,
        "root",
        "ellipse",
        "\"cx\":\"104\",\"cy\":\"104\",\"rx\":\"46\",\"ry\":\"15\",\"fill\":\"#fff4dd\"",
    );
    let copy = element_id(
        &call(
            &mut rig,
            "element_duplicate",
            &format!("\"elementId\":\"{cloud}\""),
        )
        .to_owned(),
    );
    call(
        &mut rig,
        "element_reorder",
        &format!("\"elementId\":\"{copy}\",\"targetId\":\"{sun}\",\"before\":true"),
    );
    assert!(
        call(
            &mut rig,
            "element_get",
            &format!("\"elementId\":\"{group}\"")
        )
        .contains(&group)
    );
    assert!(
        call(
            &mut rig,
            "element_query",
            "\"tag\":\"ellipse\",\"limit\":32"
        )
        .contains(&head)
    );
    assert!(call(&mut rig, "tree_get", "\"limit\":64").contains(&group));

    native(
        &mut rig,
        "add_property_track",
        &format!(
            "\"trackId\":\"sun-color\",\"targetId\":\"{stop_id}\",\"property\":\"stop-color\",\"valueType\":\"rgba\",\"interpolation\":\"linear\",\"value\":[1,0.82,0.42,1]"
        ),
    );
    native(
        &mut rig,
        "set_property_key",
        "\"trackId\":\"sun-color\",\"tick\":24,\"value\":[0.96,0.32,0.14,1]",
    );
    native(
        &mut rig,
        "add_property_track",
        &format!(
            "\"trackId\":\"blink\",\"targetId\":\"{head}\",\"property\":\"opacity\",\"valueType\":\"scalar\",\"interpolation\":\"linear\",\"value\":1"
        ),
    );
    native(
        &mut rig,
        "set_property_key",
        "\"trackId\":\"blink\",\"tick\":12,\"value\":0.1",
    );
    native(
        &mut rig,
        "set_property_key",
        "\"trackId\":\"blink\",\"tick\":14,\"value\":1",
    );
    native(
        &mut rig,
        "move_key",
        "\"trackId\":\"blink\",\"fromTick\":14,\"toTick\":15",
    );
    native(&mut rig, "remove_key", "\"trackId\":\"blink\",\"tick\":15");
    native(
        &mut rig,
        "duplicate_track",
        "\"trackId\":\"blink\",\"newTrackId\":\"blink-copy\"",
    );
    native(
        &mut rig,
        "rename_track",
        "\"trackId\":\"blink-copy\",\"name\":\"Blink backup\"",
    );
    native(
        &mut rig,
        "set_interpolation",
        "\"trackId\":\"blink-copy\",\"interpolation\":\"step\"",
    );
    native(
        &mut rig,
        "reorder_track",
        "\"trackId\":\"blink\",\"index\":0",
    );
    assert!(native(&mut rig, "get", "").contains("sun-color"));

    let saved_source = rig.state().source;
    let source = String::from_utf8(saved_source.clone()).unwrap();
    for fact in [
        "<radialGradient",
        "<clipPath",
        "stop-color",
        "A walk at dusk",
        &group,
        &arm,
        &leg,
    ] {
        assert!(source.contains(fact), "source missing {fact}");
    }
    native(&mut rig, "undo", "");
    native(&mut rig, "redo", "");
    assert_eq!(rig.state().source, saved_source);
    call(&mut rig, "save_as", &format!("\"path\":\"{project}\""));
    let mut reopened = Rig::new(policy_for(&[&dir.0]));
    reopened
        .tool("document_open", &format!("{{\"path\":\"{project}\"}}"))
        .ok();
    assert_eq!(reopened.state().source, saved_source);

    let png = dir.text("sunset.png");
    call(
        &mut rig,
        "render_png",
        &format!("\"path\":\"{png}\",\"width\":512,\"height\":512"),
    );
    assert_eq!(&fs::read(png).unwrap()[..8], b"\x89PNG\r\n\x1a\n");
}

/// Compressed image data of every GIF frame (sub-blocks concatenated). The
/// palette and encoder are deterministic, so equal pixels give equal bytes.
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

fn walking_arm(rig: &mut Rig) {
    let arm = add(
        rig,
        "root",
        "path",
        "\"d\":\"M60 60 L200 200\",\"stroke\":\"#111111\",\"stroke-width\":\"12\"",
    );
    native(
        rig,
        "add_property_track",
        &format!(
            "\"trackId\":\"arm\",\"targetId\":\"{arm}\",\"property\":\"transform\",\"valueType\":\"trs\",\"interpolation\":\"linear\",\"value\":[0,0,0,1,1,60,60]"
        ),
    );
    native(
        rig,
        "set_property_key",
        "\"trackId\":\"arm\",\"tick\":24,\"value\":[40,0,70,1,1,60,60]",
    );
}

#[test]
fn gradient_scene_native_gif_export() {
    let dir = Dir::new();
    let gif = dir.text("sunset.gif");
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let sky = add(
        &mut rig,
        "root",
        "rect",
        "\"x\":\"0\",\"y\":\"0\",\"width\":\"256\",\"height\":\"256\",\"fill\":\"#000000\"",
    );
    let gradient = call(
        &mut rig,
        "gradient_add",
        "\"kind\":\"radial\",\"units\":\"userSpace\",\"cx\":128,\"cy\":100,\"r\":160,\"stops\":\"0:#ffd36a:1,1:#44205f:1\"",
    );
    let gradient_id = gradient
        .split("\"gradientId\":\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    call(
        &mut rig,
        "element_set_paint",
        &format!("\"elementId\":\"{sky}\",\"property\":\"fill\",\"gradientId\":\"{gradient_id}\""),
    );
    walking_arm(&mut rig);
    let started = std::time::Instant::now();
    let reply = call(
        &mut rig,
        "export_animation",
        &format!("\"path\":\"{gif}\",\"format\":\"gif\",\"fps\":2,\"duration\":0.5"),
    );
    assert!(started.elapsed().as_secs() < 120, "export too slow");
    assert!(reply.contains("\"frames\":48"), "{reply}");
    let streams = gif_frame_streams(&fs::read(&gif).unwrap());
    assert_eq!(streams.len(), 48);
    assert_ne!(streams[0], streams[24], "frames 0 and 24 are identical");
    assert_ne!(streams[0], streams[12], "frames 0 and 12 are identical");
}

#[test]
fn trs_pivot_rotation_tracks_render_differently() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    walking_arm(&mut rig);
    let render = |rig: &mut Rig, name: &str, time: &str| {
        let png = dir.text(name);
        call(
            rig,
            "render_png",
            &format!("\"path\":\"{png}\",\"time\":{time}"),
        );
        let bytes = fs::read(png).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        bytes
    };
    let start = render(&mut rig, "t0.png", "0");
    let middle = render(&mut rig, "t1.png", "0.5");
    assert_ne!(start, middle, "TRS track did not change the rendered frame");
}
