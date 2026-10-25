//! `document_import`: a saved .vexel becomes one prefixed group with its
//! tracks (and rig) retargeted; hostile files are refused untouched.

mod common;
mod rig_support;

use common::{Dir, Rig, fixture, policy_for};
use rig_support::{Stick, build, rig_args};
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use vexel_app::animation::frame;
use vexel_project::{BundleInput, encode};

fn ids(rig: &Rig) -> Vec<String> {
    let source = String::from_utf8(rig.app.source_bytes().unwrap()).unwrap();
    source
        .split(" id=\"")
        .skip(1)
        .map(|rest| rest[..rest.find('"').unwrap()].to_owned())
        .collect()
}

fn assert_unique_and_renders(rig: &Rig) {
    let all = ids(rig);
    let unique: HashSet<&String> = all.iter().collect();
    assert_eq!(all.len(), unique.len(), "duplicate ids in {all:?}");
    let source = rig.app.source_bytes().unwrap();
    let scene = frame::decode(&source).expect("decodes");
    scene
        .render_sized(0.0, (128, 128), &AtomicBool::new(false))
        .expect("renders");
}

fn path_arg(path: &str) -> String {
    format!(r#""path":"{path}""#)
}

/// A stick figure with a rig and rig_pose tracks, saved as `stick.vexel`.
fn saved_stick(dir: &Dir) -> (String, Stick) {
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let stick = build(&mut rig);
    rig.doc_tool("rig_create", &rig_args(&stick)).ok();
    rig.doc_tool(
        "rig_pose",
        &format!(
            r#""poses":[{{"tick":0,"bones":{{"{a}":{{"rotate":0}}}}}},{{"tick":12,"bones":{{"{a}":{{"rotate":45}}}},"ease":"ease_in_out"}}]"#,
            a = stick.arm_r
        ),
    )
    .ok();
    let path = dir.text("stick.vexel");
    rig.doc_tool("save_as", &path_arg(&path)).ok();
    (path, stick)
}

fn bundle(svg: &str) -> Vec<u8> {
    encode(&BundleInput {
        manifest_json: br#"{"format":"vexel-project","width":100,"height":100}"#,
        scene_svg: svg.as_bytes(),
        timeline_json: b"",
        assets: &[],
    })
    .unwrap()
}

#[test]
fn two_imports_get_distinct_ids_tracks_and_rigs_in_one_undo_step() {
    let dir = Dir::new();
    let (path, stick) = saved_stick(&dir);
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let blank = rig.state();
    let args =
        |prefix: &str, extra: &str| format!(r#"{},"idPrefix":"{prefix}"{extra}"#, path_arg(&path));
    let first = rig
        .doc_tool(
            "document_import",
            &args("a_", r#","includeRig":true,"transform":[10,20,0,1,1]"#),
        )
        .ok()
        .to_owned();
    assert!(
        first.contains(r#""groupId":"a""#) && first.contains(r#""tracks":1"#),
        "{first}"
    );
    let after_first = rig.state();
    rig.doc_tool("document_import", &args("b_", r#","includeRig":true"#))
        .ok();
    assert_unique_and_renders(&rig);
    let source = String::from_utf8(rig.app.source_bytes().unwrap()).unwrap();
    assert!(source.contains(r#"<g id="a" transform="matrix(1 0 0 1 10 20)">"#));
    let torso = &stick.torso;
    assert!(
        source.contains(&format!(r#"id="a_{torso}""#))
            && source.contains(&format!(r#"id="b_{torso}""#))
    );
    // The rig now holds both characters' bones, prefixed.
    assert_eq!(source.matches(r#""pivot""#).count(), 12, "{source}");
    assert!(source.contains(&format!(r#""torso":"{}""#, format!("a_{torso}"))));
    let timeline = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    for prefix in ["a_", "b_"] {
        let track = format!("{prefix}rig_{}", stick.arm_r);
        assert!(
            timeline.contains(&format!("\"{track}\"")),
            "{track} in {timeline}"
        );
        assert!(
            timeline.contains(&format!("\"{prefix}{}\"", stick.arm_r)),
            "{timeline}"
        );
    }
    // One undo removes the whole second import: elements, tracks and rig.
    rig.doc_tool("undo", "").ok();
    assert_eq!(rig.state().source, after_first.source);
    let timeline = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(!timeline.contains("b_rig_"), "{timeline}");
    rig.doc_tool("undo", "").ok();
    assert_eq!(rig.state().source, blank.source);
}

#[test]
fn defaults_derive_the_prefix_and_skip_the_rig() {
    let dir = Dir::new();
    let (path, stick) = saved_stick(&dir);
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let reply = rig
        .doc_tool("document_import", &path_arg(&path))
        .ok()
        .to_owned();
    assert!(
        reply.contains(r#""groupId":"stick""#) && reply.contains(r#""idPrefix":"stick_""#),
        "{reply}"
    );
    let source = String::from_utf8(rig.app.source_bytes().unwrap()).unwrap();
    assert!(source.contains(&format!(r#"id="stick_{}""#, stick.head)));
    assert!(!source.contains("vexel-rig"), "rig is opt-in");
    // includeTimeline false copies elements only.
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    rig.doc_tool(
        "document_import",
        &format!(r#"{},"includeTimeline":false"#, path_arg(&path)),
    )
    .ok();
    let timeline = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(!timeline.contains("rig_"), "{timeline}");
    // The same prefix twice clashes and changes nothing.
    let before = rig.state();
    let again = rig.doc_tool("document_import", &path_arg(&path));
    assert!(again.error().contains("idPrefix"), "{}", again.body);
    assert_eq!(rig.state(), before);
}

#[test]
fn hostile_files_and_paths_are_refused_without_changes() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    rig.add_rect(1, 1, "#112233");
    let before = rig.state();
    let wrap = |inner: &str| {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">{inner}</svg>"#)
    };
    let hostile = [
        ("script", wrap("<script>alert(1)</script>")),
        ("onclick", wrap(r#"<rect id="r" onclick="x()" width="1" height="1"/>"#)),
        ("external", wrap(r#"<image id="i" href="https://example.com/a.png"/>"#)),
        ("urlext", wrap(r#"<rect id="r" fill="url(file:///etc/passwd)" width="1" height="1"/>"#)),
        ("style", wrap("<style>rect{fill:red}</style>")),
        ("entity", r#"<!DOCTYPE svg [<!ENTITY a "b">]><svg xmlns="http://www.w3.org/2000/svg"><rect id="r"/></svg>"#.to_owned()),
        ("notsvg", "hello".to_owned()),
    ];
    for (name, svg) in hostile {
        let file = dir.text(&format!("{name}.vexel"));
        std::fs::write(&file, bundle(&svg)).unwrap();
        let reply = rig.doc_tool("document_import", &path_arg(&file));
        assert!(
            reply.error().starts_with("document_import"),
            "{name}: {}",
            reply.body
        );
        assert_eq!(rig.state(), before, "{name} changed the document");
    }
    // Oversize scene (over 4 MiB of source) is refused.
    let big = wrap(&" ".repeat(4 * 1024 * 1024 + 1));
    let file = dir.text("big.vexel");
    std::fs::write(&file, bundle(&big)).unwrap();
    let reply = rig.doc_tool("document_import", &path_arg(&file));
    assert!(
        reply.error().starts_with("document_import"),
        "{}",
        reply.body
    );
    assert_eq!(rig.state(), before);
    // Not a project, wrong extension, missing and ungranted paths.
    std::fs::write(dir.file("junk.vexel"), b"not a bundle").unwrap();
    for (path, expect) in [
        (dir.text("junk.vexel"), "document_import"),
        (dir.text("a.svg"), "document_import"),
        (dir.text("missing.vexel"), "path not found"),
        ("/etc/hosts.vexel".to_owned(), "path not granted"),
    ] {
        let reply = rig.doc_tool("document_import", &path_arg(&path));
        assert!(reply.error().contains(expect), "{path}: {}", reply.body);
        assert_eq!(rig.state(), before);
    }
    for extra in [
        r#","idPrefix":"1bad""#,
        r#","idPrefix":"a b""#,
        r#","transform":[1,2,3]"#,
        r#","parentId":"nope""#,
        r#","includeRig":"yes""#,
    ] {
        let reply = rig.doc_tool(
            "document_import",
            &format!("{}{extra}", path_arg(&dir.text("big.vexel"))),
        );
        assert!(reply.is_error, "{extra}: {}", reply.body);
        assert_eq!(rig.state(), before);
    }
}

#[test]
fn a_saved_character_file_imports_twice_and_renders() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let file = dir.file("pip_rig.vexel");
    std::fs::copy(fixture("pip_rig.vexel"), &file).unwrap();
    let path = file.to_str().unwrap();
    for (prefix, x) in [("p1_", 100), ("p2_", 300)] {
        rig.doc_tool(
            "document_import",
            &format!(
                r#"{},"idPrefix":"{prefix}","transform":[{x},0,0,1,1]"#,
                path_arg(path)
            ),
        )
        .ok();
    }
    assert_unique_and_renders(&rig);
    let all = ids(&rig);
    assert!(
        all.iter().any(|id| id.starts_with("p1_")) && all.iter().any(|id| id.starts_with("p2_"))
    );
}
