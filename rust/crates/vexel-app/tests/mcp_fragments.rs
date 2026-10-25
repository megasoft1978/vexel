//! Which SVG constructs the MCP live-preview path accepts (`element_add_fragment`,
//! `use_add`) and renders; the table in docs/mcp.md mirrors these tests.
mod common;

use common::{Rig, element_id, no_grants};
use vexel_app::preview::SCENE_LIMIT;
use std::sync::atomic::AtomicBool;
use vexel_app::animation::frame;

fn rig() -> Rig {
    Rig::new(no_grants())
}

fn json_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn fragment(rig: &mut Rig, parent: &str, xml: &str) -> common::Reply {
    rig.doc_tool(
        "element_add_fragment",
        &format!(r#""parentId":"{parent}","xml":"{}""#, json_str(xml)),
    )
}

/// The 512x512 RGBA render at time 0, or the decode/render error text.
fn render(rig: &Rig) -> Result<Vec<u8>, String> {
    let source = rig.app.source_bytes().unwrap();
    let scene = frame::decode(&source).map_err(|e| format!("{e:?}"))?;
    let image = scene
        .render_sized(0.0, (512, 512), &AtomicBool::new(false))
        .map_err(|e| format!("{e:?}"))?;
    Ok(image.rgba().to_vec())
}

fn at(image: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * 512 + x) * 4;
    image[i..i + 4].try_into().unwrap()
}

const RED: [u8; 4] = [255, 0, 0, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const BOX: &str = r##"<rect id="r" x="100" y="100" width="100" height="100" fill="#ff0000" FX/>"##;

/// Adds every part under root; the last reply (or first error) comes back.
fn add_all(rig: &mut Rig, parts: &[&str]) -> common::Reply {
    let mut last = None;
    for part in parts {
        let reply = fragment(rig, "root", part);
        if reply.is_error {
            return reply;
        }
        last = Some(reply);
    }
    last.unwrap()
}

fn boxed(fx: &str) -> String {
    BOX.replace("FX", fx)
}

#[test]
fn filters_blur_dropshadow_colormatrix_work() {
    let defs = |body: &str| format!(r##"<defs><filter id="f1" x="-50%" y="-50%" width="200%" height="200%">{body}</filter></defs>"##);
    let mut rig = rig();
    add_all(&mut rig, &[&defs(r##"<feDropShadow dx="30" dy="30" stdDeviation="0" flood-color="#0000ff"/>"##), &boxed(r##"filter="url(#f1)""##)]).ok();
    let img = render(&rig).unwrap();
    assert_eq!(at(&img, 125, 125), RED);
    assert_eq!(at(&img, 215, 215), BLUE, "shadow offset 30,30");

    let mut rig = self::rig();
    add_all(&mut rig, &[&defs(r#"<feGaussianBlur stdDeviation="4"/>"#), &boxed(r##"filter="url(#f1)""##)]).ok();
    let img = render(&rig).unwrap();
    assert_ne!(at(&img, 100, 125), RED, "blurred edge is soft");

    let mut rig = self::rig();
    add_all(&mut rig, &[&defs(r#"<feColorMatrix type="hueRotate" values="90"/>"#), &boxed(r##"filter="url(#f1)""##)]).ok();
    let img = render(&rig).unwrap();
    assert_ne!(at(&img, 125, 125), RED, "hue rotated");
}

#[test]
fn turbulence_and_blend_modes_are_refused_and_rolled_back() {
    for (parts, name) in [
        (vec![r##"<defs><filter id="f1"><feTurbulence baseFrequency="0.05"/></filter></defs>"##.to_owned(), boxed(r##"filter="url(#f1)""##)], "feTurbulence"),
        (vec![boxed(r#"mix-blend-mode="multiply""#)], "mix-blend-mode attribute"),
        (vec![boxed(r#"style="mix-blend-mode:multiply""#)], "mix-blend-mode style"),
    ] {
        let mut rig = rig();
        let mut refused = None;
        for part in &parts {
            let before = rig.state().source;
            let reply = fragment(&mut rig, "root", part);
            if reply.is_error {
                assert!(reply.error().contains("does not support"), "{name}: {}", reply.body);
                assert_eq!(rig.state().source, before, "{name}: edit undone");
                refused = Some(());
            }
        }
        assert!(refused.is_some(), "{name} must be refused");
        assert!(render(&rig).is_ok(), "{name}: document still renders");
    }
}

#[test]
fn symbol_use_mask_clip_text_stroke_and_image_render() {
    let mut rig = rig();
    add_all(&mut rig, &[
        r##"<defs><symbol id="sym"><rect x="0" y="0" width="50" height="50" fill="#ff0000"/></symbol></defs>"##,
        r##"<use id="u" href="#sym" x="100" y="100" width="50" height="50"/>"##,
    ]).ok();
    let img = render(&rig).unwrap();
    assert_eq!((at(&img, 125, 125), at(&img, 175, 125)), (RED, CLEAR), "symbol + use x/y");

    let mut rig = self::rig();
    add_all(&mut rig, &[
        r##"<defs><rect id="shape" x="0" y="0" width="50" height="50" fill="#ff0000"/></defs>"##,
        r##"<use id="u" href="#shape" x="10" y="10" transform="translate(100 100)"/>"##,
    ]).ok();
    let img = render(&rig).unwrap();
    assert_eq!(at(&img, 125, 125), RED, "use transform + x/y");

    for (defs, attr) in [
        (r##"<defs><mask id="m"><rect x="0" y="0" width="150" height="300" fill="#ffffff"/></mask></defs>"##, r##"mask="url(#m)""##),
        (r##"<defs><clipPath id="c"><path d="M100 100 L150 100 L150 200 L100 200 Z"/></clipPath></defs>"##, r##"clip-path="url(#c)""##),
    ] {
        let mut rig = self::rig();
        add_all(&mut rig, &[defs, &boxed(attr)]).ok();
        let img = render(&rig).unwrap();
        assert_eq!((at(&img, 125, 125), at(&img, 175, 125)), (RED, CLEAR), "{attr}");
    }

    let mut rig = self::rig();
    fragment(&mut rig, "root", r##"<text id="t" x="100" y="200" font-size="120" fill="none" stroke="#ff0000" stroke-width="6">M</text>"##).ok();
    let img = render(&rig).unwrap();
    assert!(img.chunks(4).any(|p| p == RED), "stroked text paints");

    let mut rig = self::rig();
    fragment(&mut rig, "root", r##"<image id="i" x="100" y="100" width="100" height="100" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg=="/>"##).ok();
    let img = render(&rig).unwrap();
    assert_eq!(at(&img, 125, 125), RED, "image data URI");
}

#[test]
fn fragment_rejects_duplicate_ids_bad_xml_and_unknown_parents() {
    let mut rig = rig();
    fragment(&mut rig, "root", r##"<g id="a"/>"##).ok();
    let state = rig.state();
    for (parent, xml) in [
        ("root", r##"<g id="a"/>"##),
        ("root", r##"<g id="b"><g id="b"/></g>"##),
        ("root", "<g><g></g>"),
        ("root", "<g/><g/>"),
        ("nope", r##"<g id="c"/>"##),
    ] {
        assert!(fragment(&mut rig, parent, xml).is_error, "{xml}");
        assert_eq!(rig.state(), state, "{xml}");
    }
}

fn star(rig: &mut Rig) {
    add_all(rig, &[r##"<defs><g id="star"><rect id="star-body" x="0" y="0" width="30" height="30" fill="#ff0000"/></g></defs>"##]).ok();
}

#[test]
fn use_add_counts_one_element_undoes_and_serializes() {
    let mut rig = rig();
    star(&mut rig);
    let before = rig.state().source;
    let count = |rig: &mut Rig| {
        let info = rig.doc_tool("document_info", "");
        let body = info.ok();
        let rest = body.split("\"elementCount\":").nth(1).unwrap();
        rest[..rest.find(',').unwrap()].parse::<usize>().unwrap()
    };
    let base = count(&mut rig);
    let id = element_id(rig.doc_tool("use_add", r##""refId":"star","x":200,"y":100,"fill":"#00ff00","opacity":0.5"##).ok());
    assert_eq!(count(&mut rig), base + 1, "the instance is one element, not star + star-body");
    let img = render(&rig).unwrap();
    assert_ne!(at(&img, 210, 110), CLEAR, "instance renders");
    assert_eq!(at(&img, 150, 110), CLEAR);
    let svg = String::from_utf8(rig.state().source).unwrap();
    assert!(svg.contains(&format!(r##"<use id="{id}" href="#star" x="200" y="100" fill="#00ff00" opacity="0.5"/>"##)), "{svg}");
    // Transform object, explicit id, nested parent.
    let named = element_id(rig.doc_tool("use_add", r##""refId":"star","id":"star-2","transform":{"rotate":45,"pivotX":15,"pivotY":15}"##).ok());
    assert_eq!(named, "star-2");
    assert!(String::from_utf8(rig.state().source).unwrap().contains("id=\"star-2\" href=\"#star\" transform=\"matrix("));
    rig.doc_tool("undo", "").ok();
    rig.doc_tool("undo", "").ok();
    assert_eq!(rig.state().source, before, "one undo per instance");
}

#[test]
fn use_add_rejects_bad_requests_without_changes() {
    let mut rig = rig();
    star(&mut rig);
    rig.doc_tool("use_add", r##""refId":"star","id":"dup""##).ok();
    let state = rig.state();
    for (extra, why) in [
        (r##""refId":"missing""##, "unknown ref"),
        (r##""refId":"star","id":"dup""##, "duplicate id"),
        (r##""refId":"star","parentId":"star""##, "parent is the target"),
        (r##""refId":"star","parentId":"nope""##, "unknown parent"),
        (r##""refId":"star","opacity":2"##, "opacity range"),
        (r##""refId":"star","fill":"red""##, "paint form"),
        (r##""refId":"star","x":"1""##, "x type"),
        (r##""refId":"star","transform":{"spin":1}"##, "transform member"),
    ] {
        let reply = rig.doc_tool("use_add", extra);
        assert!(reply.is_error, "{why}: {}", reply.body);
        assert_eq!(rig.state(), state, "{why}");
    }
}

#[test]
fn use_is_hit_testable_movable_and_animatable() {
    let mut rig = rig();
    star(&mut rig);
    let id = element_id(rig.doc_tool("use_add", r##""refId":"star","x":200,"y":100"##).ok());
    assert_eq!(rig.app.document().hit_test(210.0, 110.0).as_deref(), Some(id.as_str()));
    assert_eq!(rig.app.document().hit_test(10.0, 10.0), None);
    rig.app.edit(|doc| doc.translate(&id, 50.0, 0.0)).unwrap();
    assert_eq!(rig.app.document().hit_test(260.0, 110.0).as_deref(), Some(id.as_str()));
    for (track, property, value, end) in [("t", "transform", r##"{"pivot":[215,115]}"##, r##"{"translate":[100,0]}"##), ("o", "opacity", "1", "0")] {
        let reply = rig.doc_tool(
            "native_timeline_add_property_track",
            &format!(r#""trackId":"{track}","targetId":"{id}","property":"{property}","valueType":"{}","interpolation":"linear","value":{value}"#, if property == "opacity" { "scalar" } else { "trs" }),
        );
        reply.ok();
        rig.doc_tool("native_timeline_set_property_key", &format!(r#""trackId":"{track}","tick":24,"value":{end}"#)).ok();
    }
    let at0 = render(&rig).unwrap();
    rig.doc_tool("animation_control", r#""action":"seek","timeMs":1000"#).ok();
    assert!(render(&rig).is_ok());
    let _ = at0;
}

#[test]
fn scene_limit_counts_use_instances_as_one_element_each() {
    let mut rig = rig();
    star(&mut rig);
    // Fill the scene with instances, up to 1000 per fragment; the wrapper
    // `<g>` of each fragment gets an id and counts as one element too.
    let count = |rig: &mut Rig| -> usize {
        let body = rig.doc_tool("document_info", "").ok().to_owned();
        body.split("\"elementCount\":").nth(1).unwrap().split(',').next().unwrap().parse().unwrap()
    };
    let mut n = 0;
    loop {
        let room = SCENE_LIMIT - count(&mut rig);
        if room < 2 {
            break;
        }
        let take = (room - 1).min(1000);
        let uses: String = (n..n + take).map(|i| format!(r##"<use id="i{i}" href="#star"/>"##)).collect();
        let reply = fragment(&mut rig, "root", &format!("<g>{uses}</g>"));
        assert!(!reply.is_error, "{n}+{take}: {}", reply.body);
        n += take;
    }
    assert_eq!(count(&mut rig), SCENE_LIMIT);
    assert_eq!(rig.doc_tool("use_add", r##""refId":"star""##).error(), "preview scene limit reached");
    assert_eq!(fragment(&mut rig, "root", r##"<g id="z"/>"##).error(), "preview scene limit reached");
    assert!(render(&rig).is_ok(), "SCENE_LIMIT instances render");
}

#[test]
fn unsafe_fragments_and_use_cycles_are_refused_end_to_end() {
    let mut rig = rig();
    star(&mut rig);
    fragment(&mut rig, "root", r##"<g id="host"><symbol id="sy"/></g>"##).ok();
    let state = rig.state();
    for (parent, xml, want) in [
        ("root", r##"<script id="s">x</script>"##, "not allowed"),
        ("root", r##"<foreignObject id="f"/>"##, "not allowed"),
        ("root", r##"<g id="a" onclick="x()"/>"##, "on*"),
        ("root", r##"<image id="i" href="http://example.com/a.png"/>"##, "local #id"),
        ("root", r##"<rect id="r" fill="url(http://x/y#p)"/>"##, "local #id"),
        ("root", r##"<use id="self" href="#self"/>"##, "cycle"),
        ("sy", r##"<use id="us" href="#host"/>"##, "cycle"),
    ] {
        let reply = fragment(&mut rig, parent, xml);
        assert!(reply.error().contains(want), "{xml}: {}", reply.body);
        assert_eq!(rig.state(), state, "{xml}");
    }
    assert!(rig.doc_tool("use_add", r##""refId":"host","parentId":"sy""##).is_error);
    assert_eq!(rig.state(), state);
}

#[test]
fn decoder_survives_a_cyclic_use_source() {
    // Cyclic sources cannot be built through the tools; the decoder and
    // renderer must still end (typed error or empty render), not recurse.
    let cyclic = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><g id="g"><rect width="8" height="8"/><use href="#g"/></g><use id="a" href="#b"/><use id="b" href="#a"/></svg>"##;
    let outcome = frame::decode(cyclic)
        .and_then(|scene| scene.render_sized(0.0, (64, 64), &AtomicBool::new(false)).map_err(Into::into));
    assert!(outcome.is_err(), "a cyclic <use> source is a typed decode error");
}
