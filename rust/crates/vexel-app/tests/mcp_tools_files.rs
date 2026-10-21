//! Routed file tools and launch grants over the real JSON-RPC stdio layer
//! (AC 4-6).

mod common;

use common::{Dir, Rig, fixture, no_grants, policy_for};
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use vexel_app::animation::job::ExportJob;
use vexel_app::compose::{Launch, parse_launch_with_home};
use vexel_app::session::{AppDocument, GrantError, McpGrants};

const GENERIC: &str = "invalid preview tool arguments";

fn path_args(path: &str) -> String {
    format!(r#""path":{}"#, json_string(path))
}

fn json_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn granted(dir: &Dir) -> Rig {
    Rig::new(policy_for(&[&dir.0]))
}

fn rect_rig(dir: &Dir) -> Rig {
    let mut rig = granted(dir);
    rig.add_rect(10, 10, "#2A7FFF");
    rig
}

#[test]
fn save_as_then_open_in_a_fresh_document_is_byte_identical() {
    let dir = Dir::new();
    let mut rig = rect_rig(&dir);
    rig.doc_tool(
        "animation_add",
        r#""targetId":"vexel-object-1","xml":"<animate attributeName=\"x\" values=\"10;60\" dur=\"2s\"/>""#,
    )
    .ok();
    let saved = rig.state();
    let target = dir.text("walk.vexel");
    let reply = rig.doc_tool("save_as", &path_args(&target));
    assert_eq!(
        reply.ok(),
        r#"{"ok":true,"operation":"save_as","dirty":false}"#
    );
    let after = rig.state();
    assert!(!after.dirty);
    assert_eq!(after.source, saved.source);
    assert_eq!(after.revision, saved.revision, "save changes no bytes");
    assert_eq!(after.history, saved.history, "save keeps history");
    assert!(dir.file("walk.vexel").is_file());

    let mut fresh = granted(&dir);
    let fresh_before = fresh.state();
    let reply = fresh.tool("document_open", &format!("{{{}}}", path_args(&target)));
    assert_eq!(
        reply.ok(),
        r#"{"ok":true,"operation":"document_open","documentId":"preview-document","dirty":false}"#
    );
    let opened = fresh.state();
    assert_eq!(opened.source, saved.source);
    assert_eq!(opened.revision, fresh_before.revision + 1);
    assert_eq!(opened.history, 0, "open clears history");
    assert!(!opened.dirty);
    assert_eq!(
        fresh.app.path(),
        Some(
            fs::canonicalize(dir.file("walk.vexel"))
                .unwrap()
                .as_os_str()
                .as_bytes()
        )
    );

    // A further edit works, undoes, and `save` writes it back.
    fresh
        .doc_tool(
            "attribute_set",
            &format!(
                r#""elementId":"vexel-object-1","name":"fill","value":"{}""#,
                "#00ff00"
            ),
        )
        .ok();
    assert!(fresh.state().dirty);
    let edited = fresh.state().source;
    let reply = fresh.doc_tool("save", "");
    assert_eq!(
        reply.ok(),
        r#"{"ok":true,"operation":"save","dirty":false}"#
    );
    assert!(!fresh.state().dirty);
    let mut again = AppDocument::new(10, 10).unwrap();
    again.open_path(&dir.file("walk.vexel")).unwrap();
    assert_eq!(again.source_bytes().unwrap(), edited);
    fresh.doc_tool("undo", "").ok();
    assert_ne!(fresh.state().source, edited);
}

#[test]
fn open_svg_replaces_the_document_and_ids_continue_after_it() {
    let dir = Dir::new();
    fs::copy(fixture("open.svg"), dir.file("open.svg")).unwrap();
    let mut rig = rect_rig(&dir);
    rig.doc_tool("undo", "").ok();
    let before = rig.state();
    let reply = rig.tool(
        "document_open",
        &format!("{{{}}}", path_args(&dir.text("open.svg"))),
    );
    assert!(reply.ok().contains(r#""operation":"document_open""#));
    let after = rig.state();
    assert_eq!(after.source, fs::read(fixture("open.svg")).unwrap());
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.history, 0);
    assert!(!after.dirty);
    assert_eq!(rig.app.size(), (200, 120));
    // An imported SVG is untitled: `save` has nowhere to write.
    let reply = rig.doc_tool("save", "");
    assert_eq!(reply.error(), "document has no path");

    // Id allocation restarted from the opened ids: 1 and 2 are taken.
    assert_eq!(rig.add_rect(100, 80, "#445566"), "vexel-object-3");
    let edited = rig.state();
    assert!(edited.dirty);
    rig.doc_tool("undo", "").ok();
    assert_eq!(rig.state().source, after.source);
}

#[test]
fn path_tool_failures_are_typed_and_leave_the_document_untouched() {
    let dir = Dir::new();
    let outside = Dir::new();
    let mut rig = rect_rig(&dir);

    // A valid project to truncate, plus a corrupt one.
    let good = dir.text("good.vexel");
    rig.doc_tool("save_as", &path_args(&good)).ok();
    let bytes = fs::read(&good).unwrap();
    fs::write(dir.file("truncated.vexel"), &bytes[..bytes.len() / 2]).unwrap();
    fs::write(dir.file("corrupt.vexel"), b"not a vexel project at all").unwrap();
    let mut flipped = bytes.clone();
    let middle = flipped.len() / 2;
    flipped[middle] ^= 0xff;
    fs::write(dir.file("flipped.vexel"), &flipped).unwrap();
    fs::write(dir.file("bad.svg"), b"<svg").unwrap();
    fs::write(dir.file("notes.txt"), b"hello").unwrap();
    rig.add_rect(60, 60, "#445566");
    let outside_file = outside.file("far.vexel");

    let missing = dir.text("missing.vexel");
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "document_open",
            dir.text("truncated.vexel"),
            "invalid project file",
        ),
        (
            "document_open",
            dir.text("corrupt.vexel"),
            "invalid project file",
        ),
        (
            "document_open",
            dir.text("flipped.vexel"),
            "invalid project file",
        ),
        ("document_open", dir.text("bad.svg"), "invalid project file"),
        (
            "document_open",
            dir.text("notes.txt"),
            "unsupported file type",
        ),
        ("document_open", missing.clone(), "path not found"),
        (
            "document_open",
            outside_file.to_str().unwrap().to_owned(),
            "path not granted",
        ),
        (
            "document_open",
            dir.text("sub-missing/x.vexel"),
            "path not found",
        ),
        ("document_open", "x".repeat(1024), GENERIC),
        ("document_open", String::new(), GENERIC),
        ("save_as", dir.text("notes.txt"), "unsupported file type"),
        ("save_as", dir.text("noext"), "unsupported file type"),
        (
            "save_as",
            outside_file.to_str().unwrap().to_owned(),
            "path not granted",
        ),
        ("save_as", dir.text("sub-missing/x.vexel"), "path not found"),
        (
            "save_as",
            format!("{}/{}.vexel", dir.0.display(), "a".repeat(1100)),
            GENERIC,
        ),
        ("save_as", String::new(), GENERIC),
    ];
    for (tool, path, error) in &cases {
        let before = rig.state();
        let args = if *tool == "save_as" {
            format!(r#"{{"documentId":"preview-document",{}}}"#, path_args(path))
        } else {
            format!("{{{}}}", path_args(path))
        };
        let reply = rig.tool(tool, &args);
        assert_eq!(reply.error(), *error, "{tool} {path}");
        assert_eq!(rig.state(), before, "{tool} {path} must not change state");
    }
    assert!(!outside_file.exists());

    // A document opened by the user from outside every grant cannot be saved
    // through MCP; neither can an untitled one.
    let mut rig2 = granted(&dir);
    rig2.add_rect(5, 5, "#111111");
    let before = rig2.state();
    assert_eq!(rig2.doc_tool("save", "").error(), "document has no path");
    assert_eq!(rig2.state(), before);
    rig2.app
        .save_as(outside_file.as_os_str().as_bytes())
        .unwrap();
    rig2.add_rect(50, 50, "#222222");
    let before = rig2.state();
    let outside_bytes = fs::read(&outside_file).unwrap();
    assert_eq!(rig2.doc_tool("save", "").error(), "path not granted");
    assert_eq!(rig2.state(), before);
    assert_eq!(fs::read(&outside_file).unwrap(), outside_bytes);
    // A document whose path was deleted can be saved again inside the grant.
    let mut rig3 = rect_rig(&dir);
    rig3.doc_tool("save_as", &path_args(&dir.text("gone.vexel")))
        .ok();
    fs::remove_file(dir.file("gone.vexel")).unwrap();
    rig3.add_rect(70, 70, "#333333");
    rig3.doc_tool("save", "").ok();
    assert!(dir.file("gone.vexel").is_file());
}

#[test]
fn missing_grant_refuses_every_path_tool() {
    let dir = Dir::new();
    fs::copy(fixture("open.svg"), dir.file("open.svg")).unwrap();
    let mut rig = Rig::new(no_grants());
    rig.add_rect(10, 10, "#2A7FFF");
    let before = rig.state();
    for (tool, args) in [
        (
            "save_as",
            format!(
                r#"{{"documentId":"preview-document",{}}}"#,
                path_args(&dir.text("a.vexel"))
            ),
        ),
        (
            "document_open",
            format!("{{{}}}", path_args(&dir.text("open.svg"))),
        ),
        (
            "export_animation",
            format!(
                r#"{{"documentId":"preview-document",{},"format":"gif","fps":10,"duration":0.5}}"#,
                path_args(&dir.text("a.gif"))
            ),
        ),
    ] {
        let reply = rig.tool(tool, &args);
        let error = reply.error().to_owned();
        assert!(
            error == "path not granted" || error == "export path is not in a granted directory",
            "{tool}: {error}"
        );
        assert_eq!(rig.state(), before);
    }
    assert!(!dir.file("a.vexel").exists());
    // `save` with a path the user chose still needs a grant.
    rig.app
        .save_as(dir.file("user.vexel").as_os_str().as_bytes())
        .unwrap();
    rig.add_rect(80, 80, "#222222");
    let before = rig.state();
    assert_eq!(rig.doc_tool("save", "").error(), "path not granted");
    assert_eq!(rig.state(), before);
}

#[test]
fn symlinks_cannot_escape_the_grant() {
    let grant = Dir::new();
    let outside = Dir::new();
    let mut rig = rect_rig(&grant);
    // An outside project, reachable only through links inside the grant.
    rig.app
        .save_as(outside.file("secret.vexel").as_os_str().as_bytes())
        .unwrap();
    let secret = fs::read(outside.file("secret.vexel")).unwrap();
    symlink(outside.file("secret.vexel"), grant.file("link.vexel")).unwrap();
    symlink(&outside.0, grant.file("dirlink")).unwrap();
    symlink(outside.file("nothing.vexel"), grant.file("dangling.vexel")).unwrap();

    let mut rig = Rig::new(policy_for(&[&grant.0]));
    rig.add_rect(10, 10, "#2A7FFF");
    let before = rig.state();
    let open = |rig: &mut Rig, path: String| {
        rig.tool("document_open", &format!("{{{}}}", path_args(&path)))
            .error()
            .to_owned()
    };
    assert_eq!(open(&mut rig, grant.text("link.vexel")), "path not granted");
    assert_eq!(
        open(&mut rig, grant.text("dirlink/secret.vexel")),
        "path not granted"
    );
    assert_eq!(
        open(&mut rig, grant.text("dangling.vexel")),
        "path not found"
    );
    for target in ["link.vexel", "dirlink/new.vexel", "dirlink/secret.vexel"] {
        let reply = rig.doc_tool("save_as", &path_args(&grant.text(target)));
        assert_eq!(reply.error(), "path not granted", "{target}");
    }
    assert_eq!(rig.state(), before);
    assert_eq!(fs::read(outside.file("secret.vexel")).unwrap(), secret);
    assert!(!outside.file("new.vexel").exists());
}

fn launch(args: &[&str], home: Option<&Path>) -> Launch {
    parse_launch_with_home(args.iter().map(OsString::from), home)
}

#[test]
fn grant_dir_flag_parsing() {
    let home = Dir::new();
    let dir = Dir::new();
    let second = Dir::new();
    let file = dir.file("plain.txt");
    fs::write(&file, b"x").unwrap();
    let d = dir.0.to_str().unwrap();
    let d2 = second.0.to_str().unwrap();
    let canonical = |p: &Path| fs::canonicalize(p).unwrap();

    match launch(&["--mcp-preview", "--grant-dir", d], Some(&home.0)) {
        Launch::McpPreview { open: None, grants } => {
            assert_eq!(grants.dirs(), [canonical(&dir.0)]);
        }
        other => panic!("{other:?}"),
    }
    // Order independent and repeatable.
    match launch(
        &["--grant-dir", d, "--mcp-preview", "--grant-dir", d2],
        Some(&home.0),
    ) {
        Launch::McpPreview { open: None, grants } => {
            assert_eq!(grants.dirs(), [canonical(&dir.0), canonical(&second.0)]);
        }
        other => panic!("{other:?}"),
    }
    // Without grants the launch is unchanged; the flag value is not a file.
    assert_eq!(
        launch(&["--mcp-preview"], Some(&home.0)),
        Launch::McpPreview {
            open: None,
            grants: McpGrants::none()
        }
    );
    assert_eq!(
        launch(&["--grant-dir", d, "a.svg"], Some(&home.0)),
        Launch::Shell {
            open: Some(PathBuf::from("a.svg"))
        }
    );

    let refused = |args: &[&str], error: GrantError| {
        assert_eq!(
            launch(args, Some(&home.0)),
            Launch::Usage(error.message()),
            "{args:?}"
        );
    };
    let missing = dir.text("no-such-dir");
    refused(
        &["--mcp-preview", "--grant-dir", &missing],
        GrantError::NotFound,
    );
    refused(
        &["--mcp-preview", "--grant-dir", file.to_str().unwrap()],
        GrantError::NotDirectory,
    );
    refused(&["--mcp-preview", "--grant-dir", "/"], GrantError::TooBroad);
    refused(
        &["--mcp-preview", "--grant-dir", home.0.to_str().unwrap()],
        GrantError::TooBroad,
    );
    refused(&["--mcp-preview", "--grant-dir", ""], GrantError::NotFound);
    // The home directory itself is refused, one of its children is fine.
    let child = home.sub("projects");
    assert!(matches!(
        launch(
            &["--mcp-preview", "--grant-dir", child.to_str().unwrap()],
            Some(&home.0)
        ),
        Launch::McpPreview { .. }
    ));
    assert_eq!(
        launch(&["--mcp-preview", "--grant-dir"], Some(&home.0)),
        Launch::Usage("usage: --grant-dir <directory>")
    );
}

#[test]
fn explicit_grants_combine_with_environment_and_config_file() {
    let (a, env, config, other) = (Dir::new(), Dir::new(), Dir::new(), Dir::new());
    let home = Dir::new();
    let config_dir = home.sub(".config/vexel");
    fs::write(
        config_dir.join("mcp-allowed-dirs.txt"),
        format!("{}\n", config.0.display()),
    )
    .unwrap();
    let grants = McpGrants::from_dirs([&a.0], Some(&home.0)).unwrap();
    let policy = grants.policy_with(
        Some(OsString::from(env.0.as_os_str())),
        Some(home.0.clone()),
    );
    for dir in [&a, &env, &config] {
        assert!(
            policy.resolve_save_path(dir.file("x.vexel")).is_ok(),
            "{:?}",
            dir.0
        );
    }
    assert!(policy.resolve_save_path(other.file("x.vexel")).is_err());
    // Without explicit grants the environment and config still apply.
    let plain = McpGrants::none().policy_with(
        Some(OsString::from(env.0.as_os_str())),
        Some(home.0.clone()),
    );
    assert!(plain.resolve_save_path(env.file("x.vexel")).is_ok());
    assert!(plain.resolve_save_path(config.file("x.vexel")).is_ok());
    assert!(plain.resolve_save_path(a.file("x.vexel")).is_err());
}

fn animated(dir: &Dir) -> Rig {
    let mut rig = rect_rig(dir);
    rig.doc_tool(
        "animation_add",
        r#""targetId":"vexel-object-1","xml":"<animate attributeName=\"x\" values=\"10;60\" dur=\"2s\"/>""#,
    )
    .ok();
    rig
}

#[test]
fn export_job_and_tool_share_the_grant_policy() {
    let grant = Dir::new();
    let outside = Dir::new();
    let mut rig = animated(&grant);
    let export = |rig: &mut Rig, path: &str| {
        rig.doc_tool(
            "export_animation",
            &format!(
                r#"{},"format":"gif","fps":10,"duration":0.5"#,
                path_args(path)
            ),
        )
    };
    let ok = export(&mut rig, &grant.text("loop.gif"));
    assert!(ok.ok().contains(r#""frames":5"#), "{}", ok.body);
    assert!(grant.file("loop.gif").is_file());
    let denied = export(&mut rig, &outside.text("loop.gif"));
    assert_eq!(denied.error(), "export path is not in a granted directory");
    assert!(!outside.file("loop.gif").exists());

    // Spawning the job directly with the host's policy refuses the same path.
    let source = rig.app.source_bytes().unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    let mut job = ExportJob::spawn(
        source,
        rig.host.policy().clone(),
        outside.file("direct.gif").into_os_string(),
        10,
        0.5,
        tx,
    )
    .unwrap();
    let status = loop {
        if let Some(status) = job.poll() {
            break status;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    };
    assert_eq!(status, "export path is not in a granted directory");
    assert_eq!(
        rx.recv().unwrap(),
        Err("export path is not in a granted directory")
    );
    assert!(!outside.file("direct.gif").exists());
}

#[test]
fn export_svg_decodes_and_equals_the_source() {
    let dir = Dir::new();
    let mut rig = rect_rig(&dir);
    rig.doc_tool(
        "element_add",
        r##""parentId":"root","template":"path","attributes":{"d":"M 20 20 L 90 20 L 90 90 Z","fill":"#336699","stroke":"#000000","stroke-width":"2"}"##,
    )
    .ok();
    let reply = rig.doc_tool("export_svg", "");
    let body = reply.ok().to_owned();
    let doc = vexel_json::JsonDocument::parse(body.as_bytes()).unwrap();
    let svg = doc
        .decoded_string(doc.object_get(doc.root(), b"svg").unwrap())
        .unwrap();
    assert_eq!(svg, rig.app.source_bytes().unwrap());
    // Decodes through the SVG decoder and renders to the same pixels.
    let from_export = vexel_app::compose::compose_svg(&svg, 512, 512).unwrap();
    let from_source =
        vexel_app::compose::compose_svg(&rig.app.source_bytes().unwrap(), 512, 512).unwrap();
    assert_eq!(from_export.rgba, from_source.rgba);
    assert!(from_export.rgba.iter().any(|b| *b != 255));
}

#[test]
fn startup_file_opens_through_document_open_under_its_grant() {
    let dir = Dir::new();
    fs::copy(fixture("open.svg"), dir.file("open.svg")).unwrap();
    let mut rig = Rig::with_open(policy_for(&[&dir.0]), &dir.file("open.svg")).unwrap();
    let opened = rig.state();
    assert_eq!(opened.source, fs::read(fixture("open.svg")).unwrap());
    assert_eq!(opened.history, 0);
    assert!(!opened.dirty);
    assert_eq!(rig.app.size(), (200, 120));
    let listed = rig.tool("document_list", "{}");
    assert!(
        listed.ok().contains(r#""width":200,"height":120"#),
        "{}",
        listed.body
    );
    let found = rig.doc_tool("element_query", r#""tag":"ellipse""#);
    assert!(found.ok().contains("vexel-object-2"), "{}", found.body);
    // An opened SVG is untitled: `save` has nowhere to write, `save_as`
    // into a granted directory does.
    rig.add_rect(100, 80, "#445566");
    assert_eq!(rig.doc_tool("save", "").error(), "document has no path");
    let target = dir.text("copy.vexel");
    let saved = rig.doc_tool("save_as", &path_args(&target));
    assert!(
        saved.ok().contains(r#""operation":"save_as""#),
        "{}",
        saved.body
    );
    assert!(!rig.state().dirty);
    let bytes = fs::read(dir.file("copy.vexel")).unwrap();
    assert!(bytes.windows(7).any(|w| w == b"#445566"));
}

#[test]
fn startup_file_outside_every_grant_is_refused() {
    let dir = Dir::new();
    let other = Dir::new();
    fs::copy(fixture("open.svg"), dir.file("open.svg")).unwrap();
    let path = dir.file("open.svg");
    assert_eq!(
        Rig::with_open(policy_for(&[&other.0]), &path).err(),
        Some("path not granted")
    );
    assert_eq!(
        Rig::with_open(no_grants(), &path).err(),
        Some("path not granted")
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        fs::read(fixture("open.svg")).unwrap()
    );
}
