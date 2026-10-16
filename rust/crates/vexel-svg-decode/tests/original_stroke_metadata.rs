#[path = "support/original_stroke_inputs.rs"]
mod inputs;
mod support;
use inputs::*;
use support::decode;
use vexel_scene::*;
#[test]
fn original_default_authored_strokes_transform_join_and_error_predicates() {
    let g = decode(DEFAULTS).unwrap();
    let s = g.root().shapes[0].stroke;
    assert_eq!(s.rgba[3], 0);
    assert_eq!(s.width, 1.);
    assert_eq!(s.cap, Linecap::Butt);
    assert_eq!(s.join, Linejoin::Miter);
    assert_eq!(s.miter_limit, 4.);
    assert_eq!(s.dash_values.count, 0);
    let g = decode(SOURCE).unwrap();
    let r = g.root();
    assert_eq!(r.shapes.len(), 1);
    let s = r.shapes[0];
    assert_eq!(s.edges.count, 1);
    assert_eq!(s.stroke.rgba, [255, 0, 0, 255]);
    assert_eq!(s.stroke.width, 2.);
    assert_eq!(s.stroke.dash_values.count, 3);
    assert_eq!(s.stroke.cap, Linecap::Round);
    assert_eq!(s.stroke.dash_offset, -1.);
    let g = decode(TRANSFORMED).unwrap();
    assert_eq!(g.root().shapes[0].stroke.width, 1.);
    assert!(!g.root().shapes[0].stroke_closed);
    let g = decode(JOINS).unwrap();
    assert_eq!(g.root().shapes[0].stroke.join, Linejoin::Round);
    assert_eq!(g.root().shapes[0].stroke.miter_limit, 1.);
    assert!(!g.root().shapes[0].stroke_closed);
    let e = decode(INVALID).unwrap_err();
    assert_eq!(
        (e.kind, e.offset),
        (ErrorKind::Number, INVALID.find("1,,2").unwrap())
    );
    // Original limit_work decodes successfully; its renderer work cap is later.
    assert!(decode(LIMIT_WORK).is_ok());
}
#[test]
fn original_open_stroke_path_and_implicit_fill_closure_metadata() {
    let g = decode(STROKED_PATH).unwrap();
    let r = g.root();
    assert_eq!(r.path_subpaths.len(), 1);
    assert_eq!(r.shapes[0].subpaths.count, 1);
    assert!(!r.path_subpaths[0].closed);
    assert_eq!(r.shapes[0].edges.count, 2);
    assert_eq!(r.shapes[0].stroke.rgba[2], 255);
}
