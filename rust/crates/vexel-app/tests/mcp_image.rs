//! `image_trace`, `image_segment` and `image_inpaint` through the real
//! JSON-RPC layer and `McpHost`: grants, malformed PNGs, limits, one-undo edit.

mod common;

use common::{Dir, Rig, no_grants, policy_for};
use std::fs;
use vexel_png::{Compression, encode_rgba8};

fn disc_png(dir: &Dir, name: &str) -> String {
    let (w, h) = (96usize, 96usize);
    let mut px = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let r2 = (x as f64 - 48.0).powi(2) + (y as f64 - 48.0).powi(2);
            let c: [u8; 3] = if r2 <= 28.0 * 28.0 {
                [220, 40, 40]
            } else if y < 20 {
                [40, 160, 80]
            } else {
                [30, 60, 190]
            };
            let i = (y * w + x) * 4;
            px[i..i + 3].copy_from_slice(&c);
            px[i + 3] = 255;
        }
    }
    let png = encode_rgba8(w as u32, h as u32, &px, Compression::Fixed).unwrap();
    fs::write(dir.file(name), png).unwrap();
    dir.text(name)
}

fn rig(dir: &Dir) -> Rig {
    Rig::new(policy_for(&[&dir.0]))
}

fn quoted(s: &str) -> String {
    format!("\"{s}\"")
}

#[test]
fn trace_returns_paths_without_editing() {
    let dir = Dir::new();
    let path = disc_png(&dir, "disc.png");
    let mut rig = rig(&dir);
    let before = rig.state();
    let reply = rig.doc_tool("image_trace", &format!(r#""path":{},"k":3"#, quoted(&path)));
    let body = reply.ok();
    assert!(
        body.contains(r#""width":96"#) && body.contains(r#""layers":3"#),
        "{body}"
    );
    assert!(
        body.contains(r#""paths":[{"fill":"#) && body.contains(r#""d":"M"#),
        "{body}"
    );
    assert_eq!(rig.state(), before);
}

#[test]
fn trace_into_root_is_one_undoable_edit() {
    let dir = Dir::new();
    let path = disc_png(&dir, "disc.png");
    let mut rig = rig(&dir);
    let before = rig.state();
    let reply = rig.doc_tool(
        "image_trace",
        &format!(
            r#""path":{},"k":3,"groupId":"root","idPrefix":"scene""#,
            quoted(&path)
        ),
    );
    let body = reply.ok().to_owned();
    assert!(
        body.contains(r#""elementIds":["scene-0","scene-1","scene-2"]"#),
        "{body}"
    );
    let source = String::from_utf8(rig.app.source_bytes().unwrap()).unwrap();
    assert_eq!(source.matches("<path").count(), 3, "{source}");
    assert_ne!(rig.state().source, before.source);
    rig.doc_tool("undo", "").ok();
    assert_eq!(rig.state().source, before.source);
    // The same prefix again collides only while the first set exists.
    let again = format!(
        r#""path":{},"k":3,"groupId":"root","idPrefix":"scene""#,
        quoted(&path)
    );
    rig.doc_tool("image_trace", &again).ok();
    assert!(rig.doc_tool("image_trace", &again).is_error);
}

#[test]
fn trace_rejects_bad_group_region_and_arguments() {
    let dir = Dir::new();
    let path = disc_png(&dir, "disc.png");
    let mut rig = rig(&dir);
    let before = rig.state();
    for extra in [
        r#","groupId":"nope""#,
        r#","region":[10,10,500,500]"#,
        r#","region":[10,10,5,5]"#,
        r#","k":65"#,
        r#","k":0"#,
        r#","tolerance":99"#,
        r#","idPrefix":"a b""#,
        r#","minArea":0"#,
    ] {
        let reply = rig.doc_tool(
            "image_trace",
            &format!(r#""path":{}{extra}"#, quoted(&path)),
        );
        assert!(reply.is_error, "{extra}: {}", reply.body);
    }
    assert_eq!(rig.state(), before);
}

#[test]
fn paths_outside_the_grants_and_bad_files_fail_cleanly() {
    let dir = Dir::new();
    let path = disc_png(&dir, "disc.png");
    let mut denied = Rig::new(no_grants());
    assert_eq!(
        denied
            .doc_tool("image_trace", &format!(r#""path":{}"#, quoted(&path)))
            .error(),
        "path not granted"
    );
    let mut rig = rig(&dir);
    assert_eq!(
        rig.doc_tool(
            "image_trace",
            &format!(r#""path":{}"#, quoted(&dir.text("missing.png")))
        )
        .error(),
        "image not found"
    );
    let png = fs::read(dir.file("disc.png")).unwrap();
    let mut huge = png.clone();
    huge[16..24].copy_from_slice(&[0, 0, 0xea, 0x60, 0, 0, 0xea, 0x60]); // 60000 x 60000
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty.png", vec![]),
        ("trunc.png", png[..png.len() / 2].to_vec()),
        (
            "garbage.png",
            (0..2000u32).map(|i| (i * 31 % 251) as u8).collect(),
        ),
        ("huge.png", huge),
        (
            "jpeg.png",
            vec![0xff, 0xd8, 0xff, 0xe0, 0, 16, b'J', b'F', b'I', b'F'],
        ),
    ];
    for (name, bytes) in cases {
        fs::write(dir.file(name), bytes).unwrap();
        let reply = rig.doc_tool(
            "image_trace",
            &format!(r#""path":{}"#, quoted(&dir.text(name))),
        );
        assert!(reply.is_error, "{name}: {}", reply.body);
    }
    let jpeg = rig.doc_tool(
        "image_trace",
        &format!(r#""path":{}"#, quoted(&dir.text("jpeg.png"))),
    );
    assert!(jpeg.error().contains("JPEG"), "{}", jpeg.body);
}

#[test]
fn segment_cuts_the_disc_writes_a_mask_and_traces_it() {
    let dir = Dir::new();
    let path = disc_png(&dir, "disc.png");
    let mut rig = rig(&dir);
    let mask = dir.text("mask.png");
    let args = format!(
        r#""path":{},"seeds":[[48,48]],"outMask":{},"trace":true"#,
        quoted(&path),
        quoted(&mask)
    );
    let reply = rig.doc_tool("image_segment", &args);
    let body = reply.ok().to_owned();
    let truth = std::f64::consts::PI * 28.0 * 28.0;
    let area: f64 = body
        .split("\"area\":")
        .nth(1)
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!((area - truth).abs() / truth < 0.05, "{body}");
    assert!(
        body.contains(r#""bbox":[20,20,77,77]"#) || body.contains(r#""bbox":[2"#),
        "{body}"
    );
    assert!(
        body.contains(r#""d":"M"#) && body.contains("mask.png"),
        "{body}"
    );
    assert!(fs::metadata(dir.file("mask.png")).unwrap().len() > 8);
    // Never overwrites.
    assert!(rig.doc_tool("image_segment", &args).is_error);
    // Bad seeds / polygons.
    for extra in [
        r#""seeds":[[500,5]]"#,
        r#""seeds":[]"#,
        r#""seeds":[[1.5,2]]"#,
        r#""seeds":[[48,48]],"exclude":[[[0,0],[1,1]]]"#,
    ] {
        assert!(
            rig.doc_tool(
                "image_segment",
                &format!(r#""path":{},{extra}"#, quoted(&path))
            )
            .is_error,
            "{extra}"
        );
    }
}

#[test]
fn inpaint_fills_from_a_mask_png_or_a_polygon_into_a_new_file() {
    let dir = Dir::new();
    let path = disc_png(&dir, "disc.png");
    let mut rig = rig(&dir);
    let mask = dir.text("mask.png");
    rig.doc_tool(
        "image_segment",
        &format!(
            r#""path":{},"seeds":[[48,48]],"outMask":{}"#,
            quoted(&path),
            quoted(&mask)
        ),
    )
    .ok();
    let out = dir.text("clean.png");
    let body = rig
        .doc_tool(
            "image_inpaint",
            &format!(
                r#""path":{},"maskPath":{},"outPath":{}"#,
                quoted(&path),
                quoted(&mask),
                quoted(&out)
            ),
        )
        .ok()
        .to_owned();
    assert!(
        body.contains(r#""width":96"#) && body.contains(r#""filled":"#),
        "{body}"
    );
    let png = fs::read(dir.file("clean.png")).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    // Exists now: refuse. Wrong extension and missing mask source: refuse.
    let again = format!(
        r#""path":{},"maskPath":{},"outPath":{}"#,
        quoted(&path),
        quoted(&mask),
        quoted(&out)
    );
    assert!(rig.doc_tool("image_inpaint", &again).is_error);
    let poly = r#""polygon":[[10,10],[40,10],[40,40],[10,40]]"#;
    let txt = dir.text("clean.txt");
    assert!(
        rig.doc_tool(
            "image_inpaint",
            &format!(
                r#""path":{},{poly},"outPath":{}"#,
                quoted(&path),
                quoted(&txt)
            )
        )
        .is_error
    );
    let out2 = dir.text("clean2.png");
    rig.doc_tool(
        "image_inpaint",
        &format!(
            r#""path":{},{poly},"outPath":{}"#,
            quoted(&path),
            quoted(&out2)
        ),
    )
    .ok();
    assert!(
        rig.doc_tool(
            "image_inpaint",
            &format!(
                r#""path":{},"outPath":{}"#,
                quoted(&path),
                quoted(&dir.text("n.png"))
            )
        )
        .is_error
    );
    // A mask covering the whole image leaves nothing to sample.
    let all = r#""polygon":[[-5,-5],[200,-5],[200,200],[-5,200]]"#;
    assert!(
        rig.doc_tool(
            "image_inpaint",
            &format!(
                r#""path":{},{all},"outPath":{}"#,
                quoted(&path),
                quoted(&dir.text("all.png"))
            )
        )
        .is_error
    );
}

#[test]
fn tools_are_listed() {
    let dir = Dir::new();
    let mut rig = rig(&dir);
    let tools = rig.list_tools();
    for name in ["image_trace", "image_segment", "image_inpaint"] {
        assert!(tools.iter().any(|t| t == name), "{name}");
    }
}
