//! Scripted Codex-over-MCP V1 walkthrough (ADR-0029 cutline, WALKTHROUGH-02).
//!
//! A deterministic JSON-RPC client speaks the real `vexel_mcp_stdio::serve`
//! protocol to the app's `LiveBackend`; the test thread plays the UI loop on
//! one `AppDocument` through `McpHost` and the shared `Driver`. No window, no
//! network. Native appearance is covered by the manual checklist only.

mod common;
#[path = "../../vexel-gif/tests/support/mod.rs"]
mod gif_decoder;

use common::{Dir, Reply, Rig, State, element_id, policy_for};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use vexel_app::compose::compose_svg;
use vexel_app::session::AppDocument;
use vexel_json::JsonDocument;

const FPS: u32 = 10;
const SIZE: u32 = 512;
const EYE: (usize, usize) = (296, 180);
const BODY: (usize, usize) = (256, 400);

/// Everything the determinism check compares across runs.
struct Outputs {
    project: Vec<u8>,
    gif: Vec<u8>,
    svg: Vec<u8>,
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../../fixtures/v1/walkthrough/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn path_arg(path: &str) -> String {
    format!(r#""path":"{path}""#)
}

fn add(rig: &mut Rig, template: &str, attributes: &str) -> String {
    let before = rig.state();
    let reply = rig.doc_tool(
        "element_add",
        &format!(r#""parentId":"root","template":"{template}","attributes":{{{attributes}}}"#),
    );
    let id = element_id(reply.ok());
    assert!(reply.body.contains(r#""elementId":"vexel-object-"#));
    let after = rig.state();
    assert!(after.dirty, "{template} marks dirty");
    assert!(after.history >= 1, "one undo entry per edit");
    assert_eq!(after.revision, before.revision + 1);
    assert!(String::from_utf8_lossy(&after.source).contains(&format!(r#"id="{id}""#)));
    id
}

fn ellipse(rig: &mut Rig, cx: u32, cy: u32, rx: u32, ry: u32, fill: &str) -> String {
    add(
        rig,
        "ellipse",
        &format!(r#""cx":"{cx}","cy":"{cy}","rx":"{rx}","ry":"{ry}","fill":"{fill}""#),
    )
}

/// The byte range of the element tag carrying `id="<id>"`.
fn tag(source: &[u8], id: &str) -> (usize, usize) {
    let text = std::str::from_utf8(source).unwrap();
    let at = text.find(&format!(r#"id="{id}""#)).expect(id);
    let start = text[..at].rfind('<').unwrap();
    let end = at + text[at..].find('>').unwrap() + 1;
    (start, end)
}

/// Asserts `before` and `after` differ only inside the tag of `id`.
fn only_tag_changed(before: &[u8], after: &[u8], id: &str) {
    assert_ne!(before, after);
    let (start, end) = tag(before, id);
    let (start2, end2) = tag(after, id);
    assert_eq!(start, start2, "{id} tag moved");
    assert_eq!(before[..start], after[..start2], "bytes before {id}");
    assert_eq!(before[end..], after[end2..], "bytes after {id}");
    assert_ne!(before[start..end], after[start2..end2]);
}

fn named(reply: &Reply, operation: &str, id: Option<&str>, dirty: bool) {
    let body = reply.ok();
    assert!(
        body.contains(&format!(r#""operation":"{operation}""#)),
        "{body}"
    );
    if let Some(id) = id {
        assert!(body.contains(&format!(r#""elementId":"{id}""#)), "{body}");
    }
    assert!(body.contains(&format!(r#""dirty":{dirty}"#)), "{body}");
}

fn rgba_at(frame: &[u8], (x, y): (usize, usize)) -> [u8; 3] {
    let at = (y * SIZE as usize + x) * 4;
    [frame[at], frame[at + 1], frame[at + 2]]
}

fn json_svg(body: &str) -> Vec<u8> {
    let doc = JsonDocument::parse(body.as_bytes()).unwrap();
    doc.decoded_string(doc.object_get(doc.root(), b"svg").unwrap())
        .unwrap()
}

fn walk() -> Outputs {
    let grant = Dir::new();
    let grant_path = grant.0.to_str().unwrap().to_owned();

    // a. initialize (done by the rig), tools/list, launch grant.
    let mut rig = Rig::new(policy_for(&[&grant.0]));
    let tools = rig.list_tools();
    for name in [
        "document_open",
        "attribute_set",
        "element_delete",
        "save",
        "save_as",
        "export_svg",
        "element_add",
        "animation_add",
        "animation_control",
        "export_animation",
        "undo",
        "redo",
    ] {
        assert!(tools.iter().any(|n| n == name), "tools/list lacks {name}");
    }
    assert!(tools.iter().any(|n| n == "render_png"), "render_png routed since WO-RUST-V1-PNG-01");
    assert!(!tools.iter().any(|n| n.contains("grant")), "no grant tool");
    assert!(rig.state().source.windows(4).any(|w| w == b"<svg"));

    // b. a text-free character: six shapes with distinct ids.
    let stroke = r##""stroke":"#202020","stroke-width":"3""##;
    let body = add(
        &mut rig,
        "path",
        &format!(r##""d":"M 156 300 L 356 300 L 336 480 L 176 480 Z","fill":"#3a7bd5",{stroke}"##),
    );
    let head = ellipse(&mut rig, 256, 200, 90, 80, "#ffe0b0");
    let eye_left = ellipse(&mut rig, 216, 180, 14, 14, "#000000");
    let eye_right = ellipse(&mut rig, 296, 180, 14, 14, "#000000");
    let mouth = add(
        &mut rig,
        "path",
        &format!(r##""d":"M 216 230 L 256 250 L 296 230","fill":"#ffe0b0",{stroke}"##),
    );
    let scratch = add(
        &mut rig,
        "line",
        r##""x1":"20","y1":"20","x2":"120","y2":"60","stroke":"#ff0000","stroke-width":"4""##,
    );
    let ids = [&body, &head, &eye_left, &eye_right, &mouth, &scratch];
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(**id, format!("vexel-object-{}", i + 1));
    }
    let src = String::from_utf8(rig.state().source).unwrap();
    assert!(!src.contains("<text"), "text-free character");
    assert!(src.contains("<path") && src.contains("<ellipse") && src.contains("<line"));

    // c. revise the body: fill, then translate; only that element changes.
    let before = rig.state();
    let reply = rig.doc_tool(
        "attribute_set",
        &format!(r##""elementId":"{body}","name":"fill","value":"#2a9d5c""##),
    );
    named(&reply, "attribute_set", Some(&body), true);
    let after_fill = rig.state();
    only_tag_changed(&before.source, &after_fill.source, &body);
    assert!(
        String::from_utf8_lossy(&after_fill.source).contains("#2a9d5c")
            || String::from_utf8_lossy(&after_fill.source).contains("#2A9D5C")
    );
    let reply = rig.doc_tool(
        "attribute_set",
        &format!(r#""elementId":"{body}","name":"transform","value":"translate(10 5)""#),
    );
    named(&reply, "attribute_set", Some(&body), true);
    let after_move = rig.state();
    only_tag_changed(&after_fill.source, &after_move.source, &body);

    // d. delete the scratch element.
    let reply = rig.doc_tool("element_delete", &format!(r#""elementId":"{scratch}""#));
    named(&reply, "element_delete", Some(&scratch), true);
    let after_delete = rig.state();
    assert!(!String::from_utf8_lossy(&after_delete.source).contains(&scratch));
    assert_eq!(rig.state().history, after_move.history + 1);

    // e. keyframed loop: three opacity keys on the right eye; play/seek/pause.
    let reply = rig.doc_tool(
        "animation_add",
        &format!(
            r#""targetId":"{eye_right}","xml":"<animate attributeName=\"opacity\" values=\"1;0;1\" dur=\"1s\"/>""#
        ),
    );
    assert!(reply.ok().contains("Animation added"), "{}", reply.body);
    let listed = rig.doc_tool("animation_list", "");
    assert!(
        listed
            .ok()
            .contains(&format!(r#""targetId":"{eye_right}""#))
    );
    assert!(listed.body.contains(r#""attributeName":"opacity""#));
    assert!(listed.body.contains(r#""durationMs":1000"#));
    let animated = rig.state();
    let reply = rig.doc_tool("animation_control", r#""action":"play""#);
    assert!(
        reply
            .ok()
            .starts_with(r#"{"ok":true,"playing":true,"timeSeconds":"#)
    );
    let reply = rig.doc_tool("animation_control", r#""action":"seek","timeMs":500"#);
    assert_eq!(
        reply.ok(),
        r#"{"ok":true,"playing":true,"timeSeconds":0.500}"#
    );
    let reply = rig.doc_tool("animation_control", r#""action":"pause""#);
    assert!(reply.ok().contains(r#""playing":false"#), "{}", reply.body);
    let reply = rig.doc_tool("animation_control", r#""action":"seek","timeMs":250"#);
    assert_eq!(
        reply.ok(),
        r#"{"ok":true,"playing":false,"timeSeconds":0.250}"#
    );
    assert_eq!(rig.state().source, animated.source, "playback is read-only");

    // f. undo then redo of an edit, full source bytes each time.
    let before = rig.state();
    let reply = rig.doc_tool(
        "attribute_set",
        &format!(r##""elementId":"{head}","name":"fill","value":"#ffd0a0""##),
    );
    named(&reply, "attribute_set", Some(&head), true);
    let edited = rig.state();
    only_tag_changed(&before.source, &edited.source, &head);
    let reply = rig.doc_tool("undo", "");
    assert!(reply.ok().contains("Undo complete"), "{}", reply.body);
    assert_eq!(rig.state().source, before.source, "undo restores bytes");
    let reply = rig.doc_tool("redo", "");
    assert!(reply.ok().contains("Redo complete"), "{}", reply.body);
    assert_eq!(rig.state().source, edited.source, "redo reapplies bytes");

    // g. save_as, reopen in a fresh document + host, edit, undo, save.
    let saved = rig.state();
    let project = format!("{grant_path}/walk.vexel");
    let reply = rig.doc_tool("save_as", &path_arg(&project));
    assert_eq!(
        reply.ok(),
        r#"{"ok":true,"operation":"save_as","dirty":false}"#
    );
    assert_eq!(rig.state().source, saved.source);
    assert!(!rig.state().dirty);
    let animation_before = rig.doc_tool("animation_list", "").body;
    let mut reopened = AppDocument::new(10, 10).unwrap();
    reopened.open_path(&grant.file("walk.vexel")).unwrap();
    assert_eq!(reopened.source_bytes().unwrap(), saved.source);

    let mut fresh = Rig::new(policy_for(&[&grant.0]));
    let reply = fresh.tool("document_open", &format!("{{{}}}", path_arg(&project)));
    assert_eq!(
        reply.ok(),
        r#"{"ok":true,"operation":"document_open","documentId":"preview-document","dirty":false}"#
    );
    let opened = fresh.state();
    assert_eq!(opened.source, saved.source, "reopened equals saved");
    assert!(!opened.dirty);
    assert_eq!(opened.history, 0);
    assert!(
        String::from_utf8_lossy(&opened.source).contains("<animate"),
        "animation fragment intact"
    );
    assert_eq!(fresh.doc_tool("animation_list", "").body, animation_before);
    let reply = fresh.doc_tool(
        "attribute_set",
        &format!(r##""elementId":"{eye_left}","name":"fill","value":"#103060""##),
    );
    named(&reply, "attribute_set", Some(&eye_left), true);
    let revised = fresh.state();
    only_tag_changed(&opened.source, &revised.source, &eye_left);
    fresh.doc_tool("undo", "").ok();
    assert_eq!(fresh.state().source, opened.source, "undo reverts the edit");
    fresh.doc_tool("redo", "").ok();
    assert_eq!(fresh.state().source, revised.source);
    let reply = fresh.doc_tool("save", "");
    assert_eq!(
        reply.ok(),
        r#"{"ok":true,"operation":"save","dirty":false}"#
    );
    assert!(!fresh.state().dirty);
    let mut again = AppDocument::new(10, 10).unwrap();
    again.open_path(&grant.file("walk.vexel")).unwrap();
    assert_eq!(again.source_bytes().unwrap(), revised.source);

    // Keep the exported artifacts independent of the further eye edit: export
    // from the first document, whose bytes are the saved ones.
    // h. GIF (fixed fps/duration) and SVG.
    let gif_path = format!("{grant_path}/walk.gif");
    let reply = rig.doc_tool(
        "export_animation",
        &format!(
            r#"{},"format":"gif","fps":{FPS},"duration":1"#,
            path_arg(&gif_path)
        ),
    );
    assert_eq!(
        reply.ok().split(",\"bytes\"").next().unwrap(),
        format!(r#"{{"ok":true,"frames":{FPS},"delayCs":10,"width":{SIZE},"height":{SIZE}"#)
    );
    let gif = fs::read(&gif_path).unwrap();
    check_gif(&gif);

    let reply = rig.doc_tool("export_svg", "");
    let svg = json_svg(reply.ok());
    assert_eq!(svg, saved.source);
    let svg_path = grant.file("walk.svg");
    fs::write(&svg_path, &svg).unwrap();
    check_svg(&fs::read(&svg_path).unwrap(), &reopened);
    assert_eq!(rig.state(), saved_state(&saved), "exports are read-only");

    Outputs {
        project: fs::read(grant.file("walk.vexel")).unwrap(),
        gif,
        svg,
    }
}

fn saved_state(saved: &State) -> State {
    State {
        source: saved.source.clone(),
        history: saved.history,
        revision: saved.revision,
        dirty: false,
    }
}

/// Scope 2: frame count, constant delay, loop block, canvas, blink pixels.
fn check_gif(bytes: &[u8]) {
    let gif = gif_decoder::decode(bytes).unwrap();
    assert_eq!(gif.frames.len(), FPS as usize, "fps * duration frames");
    assert_eq!((u32::from(gif.width), u32::from(gif.height)), (SIZE, SIZE));
    assert_eq!(gif.loop_count, Some(0), "infinite loop block");
    assert!(
        gif.frames.iter().all(|f| f.delay_cs == 10),
        "constant 10 cs delay"
    );
    let first = gif.rgba(0);
    let blink = gif.rgba(FPS as usize / 2);
    assert_ne!(
        rgba_at(&first, EYE),
        rgba_at(&blink, EYE),
        "eye pixel differs at blink time"
    );
    assert_eq!(rgba_at(&first, BODY), rgba_at(&blink, BODY), "body static");
    assert_ne!(first, blink);
}

/// Scope 3: the exported SVG decodes and renders like the saved document
/// (compose decodes through `vexel-svg-decode`; the render is exact, which is
/// inside the RENDER-01 tolerance of max channel delta 1).
fn check_svg(svg: &[u8], reopened: &AppDocument) {
    let from_export = compose_svg(svg, SIZE, SIZE).unwrap();
    let from_saved = compose_svg(&reopened.source_bytes().unwrap(), SIZE, SIZE).unwrap();
    assert_eq!(from_export.rgba.len(), from_saved.rgba.len());
    let worst = from_export
        .rgba
        .iter()
        .zip(&from_saved.rgba)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(worst <= 1, "render delta {worst}");
    assert!(from_export.rgba.iter().any(|b| *b != 255));
}

#[test]
fn walkthrough_steps_a_to_h() {
    walk();
}

#[test]
fn walkthrough_outputs_are_byte_identical_across_runs() {
    let first = walk();
    let second = walk();
    assert_eq!(first.project, second.project);
    assert_eq!(first.gif, second.gif);
    assert_eq!(first.svg, second.svg);
    if let Some(out) = std::env::var_os("WALKTHROUGH_OUT") {
        let out = PathBuf::from(out);
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("walk.vexel"), &first.project).unwrap();
        fs::write(out.join("walk.gif"), &first.gif).unwrap();
        fs::write(out.join("walk.svg"), &first.svg).unwrap();
    }
}

#[test]
fn step_i_open_and_edit_an_svg_and_disclose_unsupported_content() {
    let grant = Dir::new();
    let svg = fs::read(fixture("inkscape-subset.svg")).unwrap();
    fs::write(grant.file("in.svg"), &svg).unwrap();
    let mut rig = Rig::new(policy_for(&[&grant.0]));
    let reply = rig.tool(
        "document_open",
        &format!("{{{}}}", path_arg(&grant.text("in.svg"))),
    );
    assert_eq!(
        reply.ok(),
        r#"{"ok":true,"operation":"document_open","documentId":"preview-document","dirty":false}"#
    );
    let opened = rig.state();
    assert_eq!(opened.source, svg, "imported source is preserved");
    assert_eq!(rig.app.size(), (200, 120));
    assert!(compose_svg(&opened.source, 200, 120).is_ok());

    let reply = rig.doc_tool(
        "attribute_set",
        r##""elementId":"card","name":"fill","value":"#aa5500""##,
    );
    named(&reply, "attribute_set", Some("card"), true);
    let edited = rig.state();
    only_tag_changed(&opened.source, &edited.source, "card");
    let text = String::from_utf8(edited.source.clone()).unwrap();
    assert!(text.contains(r#"<inkscape:editor-state zoom="2.5" a='single'/>"#));
    assert!(text.contains(r#"transform="translate(5 5)""#), "group kept");
    rig.doc_tool("undo", "").ok();
    assert_eq!(rig.state().source, opened.source);
    // Export what was imported and drawn on top of it.
    let drawn = ellipse_on(&mut rig);
    let exported = json_svg(rig.doc_tool("export_svg", "").ok());
    assert_eq!(exported, rig.state().source);
    assert!(String::from_utf8_lossy(&exported).contains(&drawn));

    // Unsupported content is disclosed, never silently dropped or drawn.
    let text_svg = fs::read(format!(
        "{}/../../fixtures/v1/compose/unsupported-text.svg",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let error = compose_svg(&text_svg, 60, 40).unwrap_err().to_string();
    assert!(error.contains("unsupported in V1: text"), "{error}");
}

fn ellipse_on(rig: &mut Rig) -> String {
    ellipse(rig, 150, 90, 10, 10, "#112233")
}

#[test]
fn negative_paths_return_typed_errors_and_change_nothing() {
    let grant = Dir::new();
    let outside = Dir::new();
    let mut rig = Rig::new(policy_for(&[&grant.0]));
    let id = ellipse(&mut rig, 100, 100, 20, 20, "#445566");
    rig.app
        .save_as(grant.file("seed.vexel").to_str().unwrap().as_bytes())
        .unwrap();
    fs::write(outside.file("far.vexel"), b"x").unwrap();
    symlink(outside.file("far.vexel"), grant.file("link.vexel")).unwrap();
    let escape = format!(
        "{}/../{}/far.vexel",
        grant.0.display(),
        outside.0.file_name().unwrap().to_str().unwrap()
    );
    let long = "a".repeat(5000);
    let oversize =
        format!(r#""parentId":"root","template":"path","attributes":{{"d":"M 0 0 L {long}"}}"#);

    let cases: Vec<(&str, String, &str)> = vec![
        // ungranted path
        (
            "document_open",
            format!("{{{}}}", path_arg(&outside.text("far.vexel"))),
            "path not granted",
        ),
        (
            "save_as",
            format!(
                r#"{{"documentId":"preview-document",{}}}"#,
                path_arg(&outside.text("new.vexel"))
            ),
            "path not granted",
        ),
        // `..` escape out of the grant
        (
            "document_open",
            format!("{{{}}}", path_arg(&escape)),
            "path not granted",
        ),
        // symlink out of the grant
        (
            "document_open",
            format!("{{{}}}", path_arg(&grant.text("link.vexel"))),
            "path not granted",
        ),
        // unknown tool
        ("no_such_tool", "{}".to_owned(), "unknown"),
        // wrong documentId
        (
            "element_delete",
            format!(r#"{{"documentId":"nope","elementId":"{id}"}}"#),
            "unknown preview document",
        ),
        // out-of-order redo
        (
            "redo",
            r#"{"documentId":"preview-document"}"#.to_owned(),
            "nothing to redo",
        ),
        // oversize argument
        (
            "element_add",
            format!(r#"{{"documentId":"preview-document",{oversize}}}"#),
            "invalid preview tool arguments",
        ),
    ];
    for (tool, args, want) in &cases {
        let before = rig.state();
        let reply = rig.tool(tool, args);
        assert!(reply.is_error, "{tool}: {}", reply.body);
        assert!(
            reply.error().contains(want) || reply.body.contains(want),
            "{tool}: {} (wanted {want})",
            reply.body
        );
        assert_eq!(rig.state(), before, "{tool} must not mutate the document");
    }
    assert_eq!(fs::read(outside.file("far.vexel")).unwrap(), b"x");
    assert!(!outside.file("new.vexel").exists());

    // Malformed JSON-RPC gets a protocol error and touches nothing.
    let before = rig.state();
    for line in [
        "{not json",
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/call"}"#,
    ] {
        let out = rig.line(line);
        assert!(
            out.contains("\"error\"") || out.contains("isError"),
            "{out}"
        );
    }
    assert_eq!(rig.state(), before);
}
