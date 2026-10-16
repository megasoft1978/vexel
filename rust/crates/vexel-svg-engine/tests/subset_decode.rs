//! ADR-0029 V1 subset: real Decoder through `vexel-svg-engine` (AC2, AC3).
mod common;
use common::*;
use std::sync::Arc;
use vexel_scene::*;

const SUBSET: &str = "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='50' viewBox='0 0 200 100'>\
<path d='M10 10 L30 10 L30 30 Z' fill='#ff0000'/>\
<rect x='40' y='10' width='20' height='10' rx='2' fill='rgb(0,128,0)' stroke='#000000' stroke-width='2'/>\
<circle cx='80' cy='20' r='8' fill='blue' stroke='#ffff00' stroke-width='3' stroke-linecap='round' stroke-linejoin='bevel'/>\
<ellipse cx='110' cy='20' rx='10' ry='5' fill='none' stroke='#00f' style='stroke-width:2;stroke-opacity:0.5'/>\
<line x1='130' y1='10' x2='150' y2='30' stroke='#0000ff' stroke-width='4' stroke-dasharray='4 2' stroke-miterlimit='6'/>\
<polyline points='160,10 170,20 160,30' fill='none' stroke='red'/>\
<polygon points='10,60 30,60 20,80' style='fill:#123456;fill-opacity:0.5'/>\
<g transform='translate(10 20) scale(2)'><g transform='rotate(90)'>\
<rect x='1' y='2' width='3' height='4' fill='#abcdef'/></g></g></svg>";

fn tag(graph: &SceneGraph, shape: &Shape) -> String {
    let span = shape.source.element;
    let bytes = &graph.source()[span.start..span.end];
    let name_end = bytes[1..]
        .iter()
        .position(|b| !b.is_ascii_alphanumeric())
        .unwrap();
    String::from_utf8(bytes[1..1 + name_end].to_vec()).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn subset_publishes_exact_shape_kinds_in_source_order() {
    let graph = decode_ok(SUBSET);
    let scene = graph.root();
    let kinds: Vec<String> = scene.shapes.iter().map(|s| tag(&graph, s)).collect();
    assert_eq!(
        kinds,
        [
            "path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "rect"
        ]
    );
    // Each shape's recorded source span is exactly its element text.
    for (shape, needle) in scene.shapes.iter().zip([
        "<path d=",
        "<rect x='40'",
        "<circle ",
        "<ellipse ",
        "<line ",
        "<polyline ",
        "<polygon ",
        "<rect x='1'",
    ]) {
        assert_eq!(shape.source.element.start, SUBSET.find(needle).unwrap());
    }
    let counts: Vec<usize> = scene.shapes.iter().map(|s| s.edges.count).collect();
    assert_eq!((counts[0], counts[1], counts[4]), (3, 12, 1));
    assert_eq!((counts[5], counts[6], counts[7]), (3, 3, 4));
    assert!(counts[2] >= 8 && counts[3] >= 8, "curved shapes flatten");
    let closed: Vec<bool> = scene.shapes.iter().map(|s| s.stroke_closed).collect();
    assert_eq!(closed, [true, true, true, true, false, false, true, true]);
}

#[test]
fn subset_publishes_root_viewport_from_viewbox() {
    let graph = decode_ok(SUBSET);
    let viewport = graph.root().viewport;
    assert_eq!((viewport.width, viewport.height), (100, 50));
    assert_eq!((viewport.view_x, viewport.view_y), (0.0, 0.0));
    assert_eq!((viewport.view_width, viewport.view_height), (200.0, 100.0));
    assert!(viewport.aspect.set && !viewport.aspect.none && !viewport.aspect.slice);
}

#[test]
fn subset_resolves_group_transforms_and_user_space_edges() {
    let graph = decode_ok(SUBSET);
    let scene = graph.root();
    for shape in &scene.shapes[..7] {
        let t = shape.source.transform;
        assert_eq!(
            (t.a, t.b, t.c, t.d, t.e, t.f),
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
        );
    }
    // translate(10 20) scale(2) rotate(90) = [0 2 -2 0 10 20] (a b c d e f).
    let t = scene.shapes[7].source.transform;
    assert!(close(t.a, 0.0) && close(t.b, 2.0) && close(t.c, -2.0));
    assert!(close(t.d, 0.0) && close(t.e, 10.0) && close(t.f, 20.0));
    // Frames: nested group chains to its parent group frame.
    let node = scene.shapes[7].source.transform_node.unwrap();
    let frame = graph.root().transform(node).unwrap();
    assert!(frame.parent.is_some());
    // Edges are published in transformed user space (viewBox is viewport-only).
    let e = scene.edges[0];
    assert_eq!((e.a.x, e.a.y, e.b.x, e.b.y), (10.0, 10.0, 30.0, 10.0));
    let first = scene.shapes[7].edges.first;
    let (a, b) = (scene.edges[first].a, scene.edges[first].b);
    assert_eq!((a.x, a.y, b.x, b.y), (6.0, 22.0, 6.0, 28.0));
}

#[test]
fn subset_publishes_solid_fill_and_stroke_channels() {
    let graph = decode_ok(SUBSET);
    let scene = graph.root();
    let fill = |i: usize| scene.edge_colors[scene.shapes[i].edges.first];
    assert_eq!(fill(0), [255, 0, 0, 255]);
    assert_eq!(fill(1), [0, 128, 0, 255]);
    assert_eq!(fill(2), [0, 0, 255, 255]);
    assert_eq!(fill(3), [0, 0, 0, 0], "fill none is transparent");
    assert_eq!(fill(5), [0, 0, 0, 0]);
    assert_eq!(fill(6), [18, 52, 86, 128], "style fill + fill-opacity");
    assert_eq!(fill(7), [171, 205, 239, 255]);
    // Stroke: color, width, cap, join, miter limit and dashes.
    let rect = scene.shapes[1].stroke;
    assert_eq!((rect.rgba, rect.width), ([0, 0, 0, 255], 2.0));
    let circle = scene.shapes[2].stroke;
    assert_eq!(circle.rgba, [255, 255, 0, 255]);
    assert_eq!(circle.width, 3.0);
    assert_eq!((circle.cap, circle.join), (Linecap::Round, Linejoin::Bevel));
    let ellipse = scene.shapes[3].stroke;
    assert_eq!((ellipse.rgba, ellipse.width), ([0, 0, 255, 128], 2.0));
    let line = scene.shapes[4].stroke;
    assert_eq!((line.rgba, line.width), ([0, 0, 255, 255], 4.0));
    assert_eq!(line.miter_limit, 6.0);
    let dashes = &scene.dash_values[line.dash_values.first..][..line.dash_values.count];
    assert_eq!(dashes, [4.0, 2.0]);
    assert_eq!(scene.shapes[5].stroke.rgba, [255, 0, 0, 255]);
    // Unstroked shapes keep a transparent stroke with the initial width.
    assert_eq!(scene.shapes[0].stroke.rgba[3], 0);
    assert_eq!(scene.shapes[6].stroke.rgba[3], 0);
    assert!(scene.shapes.iter().all(|s| s.fill_paint.is_none()));
}

const PRESERVED: &str = "<?xml version='1.0' encoding='UTF-8'?>\r\n<!-- caf\u{e9} \u{2603} -->\r\n\
<svg xmlns='http://www.w3.org/2000/svg' xmlns:x='urn:example:x' xmlns:inkscape='urn:inkscape' \
xmlns:sodipodi='urn:sodipodi' x:custom='1' width='20' height='20'>\n\
\t<metadata><rdf:RDF xmlns:rdf='urn:rdf'><unknown-meta k='v'/></rdf:RDF></metadata>\n\
  <sodipodi:namedview id='base' inkscape:zoom='2'/>\n\
<defs><foo:bar xmlns:foo='urn:foo' a='1'><baz/></foo:bar></defs>\n\
<title>t</title><desc>d</desc>\n\
<g inkscape:label='L' inkscape:groupmode='layer' x:data='z'>\
<rect x:extra='1' width='5' height='5' fill='red'/></g>\n</svg>\n";

#[test]
fn unknown_elements_namespace_attributes_and_editor_metadata_preserve_source() {
    let graph = decode_ok(PRESERVED);
    assert_eq!(graph.source(), PRESERVED.as_bytes());
    assert_eq!(graph.root().shapes.len(), 1);
    let shape = graph.root().shapes[0];
    let rect = "<rect x:extra='1' width='5' height='5' fill='red'/>";
    assert_eq!(shape.source.element.start, PRESERVED.find(rect).unwrap());
    assert_eq!(
        &graph.source()[shape.source.element.start..shape.source.element.end],
        rect.as_bytes()
    );
}

#[test]
fn preserved_element_offsets_match_text_and_published_source_equals_input() {
    let owner = Arc::new(PRESERVED.as_bytes().to_vec());
    let mut control = Control::default();
    let document = LexicalDocument::parse(owner.clone(), 1 << 20, &mut control).unwrap();
    // Lexical offsets of the preserved unknown/editor elements equal their text offsets.
    for name in [
        "rdf:RDF",
        "unknown-meta",
        "sodipodi:namedview",
        "foo:bar",
        "baz",
    ] {
        let want = PRESERVED.find(&format!("<{name}")).unwrap();
        assert!(
            document.tokens().iter().any(|t| t.source.offset == want),
            "{name} offset {want} present in lexical tokens"
        );
    }
    let DecodedScene::Drawable(graph) = try_decode(PRESERVED.as_bytes(), &mut control).unwrap()
    else {
        panic!("drawable")
    };
    assert_eq!(graph.source(), owner.as_slice());
    assert_eq!(
        graph.root().shapes[0].source.element.start,
        PRESERVED.find("<rect").unwrap()
    );
}
