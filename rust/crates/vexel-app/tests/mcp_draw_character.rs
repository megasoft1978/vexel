//! Draw a character over the MCP stdio path only: element adds, fill and
//! translate edits, one SMIL opacity animation, then GIF and SVG export. No
//! window, mouse or keyboard; every call is a synthetic JSON-RPC line through
//! `vexel_mcp_stdio::serve` with the app's `LiveBackend`.

mod common;
#[path = "../../vexel-gif/tests/support/mod.rs"]
mod gif_decoder;

use common::{Dir, Rig, element_id, policy_for};
use std::fs;
use vexel_json::JsonDocument;

const FPS: u32 = 10;
const SIZE: u32 = 512;
/// Centre of the head after its translate(10 5); blink time is half a second.
const HEAD: (usize, usize) = (266, 205);

fn add(rig: &mut Rig, template: &str, attributes: &str) -> String {
    let reply = rig.doc_tool(
        "element_add",
        &format!(r#""parentId":"root","template":"{template}","attributes":{{{attributes}}}"#),
    );
    element_id(reply.ok())
}

fn attribute(rig: &mut Rig, id: &str, name: &str, value: &str) {
    let reply = rig.doc_tool(
        "attribute_set",
        &format!(r#""elementId":"{id}","name":"{name}","value":"{value}""#),
    );
    assert!(
        reply.ok().contains(&format!(r#""elementId":"{id}""#)),
        "{}",
        reply.body
    );
}

/// The start tag carrying `id="<id>"`, lower-cased.
fn tag(source: &[u8], id: &str) -> String {
    let text = std::str::from_utf8(source).unwrap();
    let at = text.find(&format!(r#"id="{id}""#)).expect(id);
    let start = text[..at].rfind('<').unwrap();
    let end = at + text[at..].find('>').unwrap() + 1;
    text[start..end].to_ascii_lowercase()
}

/// The unsigned integer that follows `"key":` in a JSON reply body.
fn number_field(body: &str, key: &str) -> u64 {
    let at = body.find(&format!(r#""{key}":"#)).expect(body) + key.len() + 3;
    body[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap()
}

fn rgba_at(frame: &[u8], (x, y): (usize, usize)) -> [u8; 3] {
    let at = (y * SIZE as usize + x) * 4;
    [frame[at], frame[at + 1], frame[at + 2]]
}

#[test]
fn draw_character_over_stdio_mcp() {
    let grant = Dir::new();
    let mut rig = Rig::new(policy_for(&[&grant.0]));

    let tools = rig.list_tools();
    for name in [
        "element_add",
        "attribute_set",
        "document_info",
        "animation_add",
        "animation_control",
        "export_animation",
        "export_svg",
    ] {
        assert!(tools.iter().any(|n| n == name), "tools/list lacks {name}");
    }

    // Draw: rect body, ellipse head, path arm.
    let body = add(
        &mut rig,
        "rect",
        r##""x":"200","y":"260","width":"112","height":"140","fill":"#3a7bd5""##,
    );
    let head = add(
        &mut rig,
        "ellipse",
        r##""cx":"256","cy":"200","rx":"80","ry":"70","fill":"#ffe0b0""##,
    );
    let arm = add(
        &mut rig,
        "path",
        r##""d":"M 312 300 L 380 330 L 400 390","fill":"none","stroke":"#202020","stroke-width":"6""##,
    );
    assert_eq!(
        [body.as_str(), head.as_str(), arm.as_str()],
        ["vexel-object-1", "vexel-object-2", "vexel-object-3"]
    );
    let info = rig.doc_tool("document_info", "");
    assert_eq!(number_field(info.ok(), "elementCount"), 3);

    // Revise: body fill, head fill, one translate on the head.
    attribute(&mut rig, &body, "fill", "#2a9d5c");
    attribute(&mut rig, &head, "fill", "#ffd0a0");
    attribute(&mut rig, &head, "transform", "translate(10 5)");
    let source = rig.state().source;
    assert!(tag(&source, &body).contains(r##"fill="#2a9d5c""##));
    assert!(tag(&source, &head).contains(r##"fill="#ffd0a0""##));
    // The translate is baked into the geometry: cx 256 -> 266, cy 200 -> 205.
    assert!(
        tag(&source, &head).contains(r#"cx="2.66e2""#),
        "{}",
        tag(&source, &head)
    );
    assert!(tag(&source, &head).contains(r#"cy="2.05e2""#));
    assert!(tag(&source, &arm).contains(r##"stroke="#202020""##));
    let info = rig.doc_tool("document_info", "");
    assert_eq!(
        number_field(info.ok(), "elementCount"),
        3,
        "edits add nothing"
    );

    // Animate the head: opacity dips to 0.2 at the half-second mark.
    let reply = rig.doc_tool(
        "animation_add",
        &format!(
            r#""targetId":"{head}","xml":"<animate attributeName=\"opacity\" values=\"1;0.2;1\" dur=\"1s\"/>""#
        ),
    );
    assert!(reply.ok().contains("Animation added"), "{}", reply.body);

    // Export the animated GIF from the granted directory.
    let gif_path = grant.text("character.gif");
    let reply = rig.doc_tool(
        "export_animation",
        &format!(r#""path":"{gif_path}","format":"gif","fps":{FPS},"duration":1"#),
    );
    let bytes = number_field(reply.ok(), "bytes");
    assert!(bytes > 0, "export reports bytes: {}", reply.body);
    let gif = fs::read(&gif_path).unwrap();
    assert_eq!(gif.len() as u64, bytes, "reported bytes match the file");
    let decoded = gif_decoder::decode(&gif).unwrap();
    assert_eq!(decoded.frames.len(), FPS as usize);
    assert_eq!(
        (u32::from(decoded.width), u32::from(decoded.height)),
        (SIZE, SIZE)
    );
    let first = decoded.rgba(0);
    let blink = decoded.rgba(FPS as usize / 2);
    assert_ne!(
        rgba_at(&first, HEAD),
        rgba_at(&blink, HEAD),
        "head fades at blink time"
    );

    // Export the SVG: non-empty bytes that equal the saved source.
    let reply = rig.doc_tool("export_svg", "");
    let doc = JsonDocument::parse(reply.ok().as_bytes()).unwrap();
    let svg = doc
        .decoded_string(doc.object_get(doc.root(), b"svg").unwrap())
        .unwrap();
    assert!(!svg.is_empty(), "export_svg returned bytes");
    assert_eq!(svg, rig.state().source);
    let svg_text = String::from_utf8(svg).unwrap();
    assert!(svg_text.contains(&format!(r#"id="{head}""#)));
    assert!(svg_text.contains("<animate"), "animation survives export");
}

#[test]
fn malformed_path_data_is_a_typed_error() {
    let grant = Dir::new();
    let mut rig = Rig::new(policy_for(&[&grant.0]));
    add(
        &mut rig,
        "rect",
        r##""x":"0","y":"0","width":"10","height":"10","fill":"#000000""##,
    );
    let before = rig.state();

    for d in ["M 0 0 L", "M 0 0 Q 1 1 2 2", "", "L 1 1"] {
        let reply = rig.doc_tool(
            "element_add",
            &format!(r#""parentId":"root","template":"path","attributes":{{"d":"{d}"}}"#),
        );
        assert!(reply.is_error, "malformed d {d:?} accepted: {}", reply.body);
        assert_eq!(reply.error(), "invalid preview tool arguments", "d {d:?}");
        assert_eq!(rig.state(), before, "rejected d {d:?} must not mutate");
    }
}
