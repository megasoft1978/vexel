//! `render_png` through the real JSON-RPC stdio layer and `McpHost` (no display).

mod common;
#[path = "../../vexel-png/tests/common/mod.rs"]
mod png_decode;

use common::{Dir, Rig, no_grants, policy_for};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::AtomicBool;
use vexel_app::animation::frame;

const GENERIC: &str = "invalid preview tool arguments";

fn rig_with_shapes(dir: &Dir) -> Rig {
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    rig.add_rect(40, 40, "#2A7FFF");
    rig.doc_tool(
        "element_add",
        r##""parentId":"root","template":"ellipse","attributes":{"cx":"300","cy":"260","rx":"90","ry":"60","fill":"#FF3300"}"##,
    )
    .ok();
    rig
}

fn args(path: &str, extra: &str) -> String {
    format!(r#""path":"{path}"{extra}"#)
}

fn entries(dir: &Dir) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(&dir.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn in_process(rig: &Rig, size: (u32, u32)) -> Vec<u8> {
    let source = rig.app.source_bytes().unwrap();
    let scene = frame::decode(&source).unwrap();
    scene
        .render_sized(0.0, size, &AtomicBool::new(false))
        .unwrap()
        .rgba()
        .to_vec()
}

#[test]
fn renders_requested_size_matching_the_in_process_render() {
    let dir = Dir::new();
    let mut rig = rig_with_shapes(&dir);
    let target = dir.text("out.png");
    let reply = rig.doc_tool("render_png", &args(&target, r#","width":200,"height":150"#));
    let body = reply.ok().to_owned();
    let canonical = fs::canonicalize(dir.file("out.png")).unwrap();
    let bytes = fs::read(&canonical).unwrap();
    assert_eq!(
        body,
        format!(
            r#"{{"path":"{}","width":200,"height":150,"bytes":{}}}"#,
            canonical.display(),
            bytes.len()
        )
    );
    let decoded = png_decode::decode(&bytes);
    assert_eq!((decoded.width, decoded.height), (200, 150));
    let expected = in_process(&rig, (200, 150));
    assert_eq!(decoded.rgba, expected);
    assert!(
        decoded.rgba.chunks_exact(4).any(|p| p[3] == 255),
        "shapes drew opaque pixels"
    );
    assert_eq!(entries(&dir), ["out.png"], "no temp file left");
}

#[test]
fn defaults_and_aspect_for_a_single_side() {
    let dir = Dir::new();
    let mut rig = rig_with_shapes(&dir);
    let reply = rig.doc_tool("render_png", &args(&dir.text("a.png"), ""));
    assert!(reply.ok().contains(r#""width":512,"height":512,"#));
    let decoded = png_decode::decode(&fs::read(dir.file("a.png")).unwrap());
    assert_eq!(decoded.rgba, in_process(&rig, (512, 512)));
    let reply = rig.doc_tool("render_png", &args(&dir.text("b.png"), r#","width":64"#));
    assert!(reply.ok().contains(r#""width":64,"height":64,"#));
    let reply = rig.doc_tool("render_png", &args(&dir.text("c.png"), r#","height":32"#));
    assert!(reply.ok().contains(r#""width":32,"height":32,"#));
    // An existing file in a grant is replaced.
    let reply = rig.doc_tool("render_png", &args(&dir.text("a.png"), r#","width":16"#));
    reply.ok();
    assert_eq!(
        png_decode::decode(&fs::read(dir.file("a.png")).unwrap()).width,
        16
    );
}

#[test]
fn rejected_calls_return_typed_errors_and_create_no_file() {
    let dir = Dir::new();
    let other = Dir::new();
    dir.sub("folder.png");
    let mut rig = rig_with_shapes(&dir);
    let before = entries(&dir);
    let other_before = entries(&other);

    let cases: Vec<(String, &str)> = vec![
        (args(&other.text("x.png"), ""), "path not granted"),
        (
            args(&format!("{}/../{}", dir.0.display(), "escape.png"), ""),
            "path not granted",
        ),
        (args(&dir.text("x.txt"), ""), "unsupported file type"),
        (args(&dir.text("noext"), ""), "unsupported file type"),
        (args(&dir.text(".png"), ""), "unsupported file type"),
        (args(&dir.text("x.png"), r#","width":0"#), GENERIC),
        (args(&dir.text("x.png"), r#","width":5000"#), GENERIC),
        (args(&dir.text("x.png"), r#","height":0"#), GENERIC),
        (args(&dir.text("x.png"), r#","height":4097"#), GENERIC),
        (args(&dir.text("folder.png"), ""), "path not found"),
        (args(&dir.text("missing/x.png"), ""), "path not found"),
    ];
    for (extra, expected) in &cases {
        let reply = rig.doc_tool("render_png", extra);
        assert_eq!(reply.error(), *expected, "{extra}");
    }
    // Non-integers are refused by the protocol layer before the app.
    for bad in [r#","width":2.5"#, r#","width":-3"#, r#","height":"9""#] {
        let reply = rig.doc_tool("render_png", &args(&dir.text("x.png"), bad));
        assert!(
            reply.is_error && reply.body.contains("invalid_arguments"),
            "{bad}"
        );
    }
    // A wrong document id and missing required fields.
    let reply = rig.tool(
        "render_png",
        &format!(
            r#"{{"documentId":"other",{}}}"#,
            args(&dir.text("x.png"), "")
        ),
    );
    assert_eq!(reply.error(), "unknown preview document");
    assert!(
        rig.tool(
            "render_png",
            &format!("{{{}}}", args(&dir.text("x.png"), ""))
        )
        .is_error
    );
    assert!(
        rig.tool("render_png", r#"{"documentId":"preview-document"}"#)
            .is_error
    );

    assert_eq!(entries(&dir), before);
    assert_eq!(entries(&other), other_before);
}

#[test]
fn no_grants_means_no_file() {
    let dir = Dir::new();
    let mut rig = Rig::new(no_grants());
    let reply = rig.doc_tool("render_png", &args(&dir.text("x.png"), ""));
    assert_eq!(reply.error(), "path not granted");
    assert!(entries(&dir).is_empty());
}

#[test]
fn read_only_directory_keeps_the_existing_file_and_leaves_no_temp() {
    let dir = Dir::new();
    let mut rig = rig_with_shapes(&dir);
    let target = dir.text("keep.png");
    rig.doc_tool("render_png", &args(&target, r#","width":8,"height":8"#))
        .ok();
    let original = fs::read(dir.file("keep.png")).unwrap();

    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o555)).unwrap();
    let reply = rig.doc_tool("render_png", &args(&target, r#","width":24,"height":24"#));
    let failed = reply.is_error;
    let message = reply.body.clone();
    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(failed, "{message}");
    assert_eq!(message, r#"{"error":"file operation failed"}"#);
    assert_eq!(fs::read(dir.file("keep.png")).unwrap(), original);
    assert_eq!(entries(&dir), ["keep.png"]);
}

#[test]
fn quadratic_path_added_via_element_add_renders_pixels() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let d = "M0 330 Q90 250 180 325 Q260 235 355 315 L512 512 L0 512Z";
    let added = rig.doc_tool(
        "element_add",
        &format!(
            r##""parentId":"root","template":"path","attributes":{{"d":"{d}","fill":"#FF0000"}}"##
        ),
    );
    added.ok();
    let target = dir.text("q.png");
    rig.doc_tool("render_png", &args(&target, "")).ok();
    let decoded = png_decode::decode(&fs::read(dir.file("q.png")).unwrap());
    let red = decoded
        .rgba
        .chunks_exact(4)
        .filter(|p| p[0] > 200 && p[1] < 60 && p[2] < 60 && p[3] == 255)
        .count();
    assert!(red > 100, "quadratic path drew {red} red pixels");
    assert_ne!(
        &decoded.rgba[..4],
        &[255, 0, 0, 255],
        "top-left stays background"
    );
}
