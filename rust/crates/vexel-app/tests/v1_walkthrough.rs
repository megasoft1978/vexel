//! V1 walkthrough (docs/product/vexel-v1-vertical-slice.md, steps 1, 2, 4, 5)
//! over the live JSON-RPC MCP surface.
//!
//! Calls go through `vexel_mcp_stdio::serve` to the app's `LiveBackend`; the
//! test thread plays the UI loop through `McpHost`. Covered: create, draw,
//! revise, undo/redo, save_as, save, reopen, export_svg.
//!
//! Not covered (blocked, see the S6 report): the native timeline steps and the
//! native GIF export. `native_timeline_*` is routed by `McpHost` but the
//! live-preview `tools/call` gate in `vexel-protocol` rejects it as
//! `unknown_tool`, and `NativeOp` / `animation::native_export` are not
//! reachable from an integration test without crate-visibility changes.

mod common;

use common::{Dir, Rig, State, element_id, policy_for};
use vexel_app::compose::compose_svg;
use vexel_app::session::AppDocument;
use vexel_json::JsonDocument;

const SIZE: u32 = 128;
const OK_SAVE_AS: &str = r#"{"ok":true,"operation":"save_as","dirty":false}"#;
const OK_SAVE: &str = r#"{"ok":true,"operation":"save","dirty":false}"#;
const OK_OPEN: &str =
    r#"{"ok":true,"operation":"document_open","documentId":"preview-document","dirty":false}"#;

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

/// One `element_add`: exactly one new element and one undo entry.
fn add(rig: &mut Rig, template: &str, attributes: &str) -> String {
    let before = rig.state();
    let reply = rig.doc_tool(
        "element_add",
        &format!(r#""parentId":"root","template":"{template}","attributes":{{{attributes}}}"#),
    );
    let id = element_id(reply.ok());
    let after = rig.state();
    assert!(after.dirty, "{template} marks dirty");
    assert_eq!(after.revision, before.revision + 1, "{template} revision");
    assert_eq!(
        after.history,
        before.history + 1,
        "{template} one undo entry"
    );
    assert!(String::from_utf8_lossy(&after.source).contains(&format!(r#"id="{id}""#)));
    id
}

fn ellipse(rig: &mut Rig, cx: u32, cy: u32, r: u32, fill: &str) -> String {
    add(
        rig,
        "ellipse",
        &format!(r#""cx":"{cx}","cy":"{cy}","rx":"{r}","ry":"{r}","fill":"{fill}""#),
    )
}

/// One `attribute_set`: only the named element's tag changes, one undo entry.
fn attribute(rig: &mut Rig, id: &str, name: &str, value: &str) -> State {
    let before = rig.state();
    let reply = rig.doc_tool(
        "attribute_set",
        &format!(r#""elementId":"{id}","name":"{name}","value":"{value}""#),
    );
    assert!(
        reply.ok().contains(r#""operation":"attribute_set""#),
        "{}",
        reply.body
    );
    let after = rig.state();
    only_tag_changed(&before.source, &after.source, id);
    assert_eq!(after.history, before.history + 1, "{name} one undo entry");
    after
}

fn json_svg(body: &str) -> Vec<u8> {
    let doc = JsonDocument::parse(body.as_bytes()).unwrap();
    doc.decoded_string(doc.object_get(doc.root(), b"svg").unwrap())
        .unwrap()
}

#[test]
fn v1_walkthrough_create_revise_save_reopen_export_svg() {
    let grant = Dir::new();
    let project = grant.text("character.vexel");

    // 1. Create a blank project and draw a text-free character.
    let app = AppDocument::new(SIZE, SIZE).unwrap();
    let mut rig = Rig::with_document(app, policy_for(&[&grant.0]));
    assert_eq!(rig.state().history, 0);
    let info = rig.doc_tool("document_info", "");
    assert!(
        info.ok()
            .contains(&format!(r#""width":{SIZE},"height":{SIZE}"#))
    );

    let body = add(
        &mut rig,
        "path",
        r##""d":"M 40 70 L 88 70 L 84 120 L 44 120 Z","fill":"#3a7bd5","stroke":"#202020","stroke-width":"3""##,
    );
    let head = ellipse(&mut rig, 64, 44, 26, "#ffe0b0");
    let eye_left = ellipse(&mut rig, 54, 40, 5, "#000000");
    let eye_right = ellipse(&mut rig, 74, 40, 5, "#000000");
    for (i, id) in [&body, &head, &eye_left, &eye_right].iter().enumerate() {
        assert_eq!(**id, format!("vexel-object-{}", i + 1));
    }

    // 2. Revise the head through MCP, then undo and redo that one edit.
    let before_revise = rig.state();
    let revised = attribute(&mut rig, &head, "fill", "#ffd0a0");
    assert!(rig.doc_tool("undo", "").ok().contains("Undo complete"));
    assert_eq!(
        rig.state().source,
        before_revise.source,
        "undo restores bytes"
    );
    assert!(rig.doc_tool("redo", "").ok().contains("Redo complete"));
    assert_eq!(rig.state().source, revised.source, "redo reapplies bytes");

    // 3. Save as the project, make one more drawing edit, then save again.
    let reply = rig.doc_tool("save_as", &format!(r#""path":"{project}""#));
    assert_eq!(reply.ok(), OK_SAVE_AS);
    assert!(!rig.state().dirty);
    let body_revised = attribute(&mut rig, &body, "fill", "#2a9d5c");
    let reply = rig.doc_tool("save", "");
    assert_eq!(reply.ok(), OK_SAVE);
    let saved = rig.state();
    assert!(!saved.dirty);
    assert_eq!(saved.source, body_revised.source);

    // 4. Reopen in a fresh host; the drawing must be intact and editable.
    let mut fresh = Rig::new(policy_for(&[&grant.0]));
    let reply = fresh.tool("document_open", &format!(r#"{{"path":"{project}"}}"#));
    assert_eq!(reply.ok(), OK_OPEN);
    let reopened = fresh.state();
    assert_eq!(reopened.source, saved.source, "reopened equals saved");
    assert_eq!(reopened.history, 0);
    assert!(!reopened.dirty);
    let text = String::from_utf8(reopened.source.clone()).unwrap();
    for (id, fill) in [
        (&body, "#2a9d5c"),
        (&head, "#ffd0a0"),
        (&eye_left, "#000000"),
        (&eye_right, "#000000"),
    ] {
        let (start, end) = tag(&reopened.source, id);
        assert!(text[start..end].contains(fill), "{id} keeps {fill}");
    }
    let edited = attribute(&mut fresh, &eye_left, "fill", "#103060");
    assert_ne!(edited.source, reopened.source);
    assert!(fresh.doc_tool("undo", "").ok().contains("Undo complete"));
    assert_eq!(fresh.state().source, reopened.source, "undo after reopen");

    // 5. Export SVG from the reopened document; it renders.
    let svg = json_svg(fresh.doc_tool("export_svg", "").ok());
    assert_eq!(svg, reopened.source, "export_svg equals saved bytes");
    let composed = compose_svg(&svg, SIZE, SIZE).expect("exported SVG renders");
    assert!(composed.rgba.iter().any(|b| *b != 255));
}
