//! WO-RUST-CORPUS-UNSUPPORTED-01: the decoder hands definitions their owner's
//! bounds as `[x0, y0, x1, y1]`. It used to pass `[x, y, width, height]`, which
//! the planner rejected for any owner with `x0 > width` or `y0 > height`.
mod common;
use common::*;

#[test]
fn clip_in_object_bounding_box_units_decodes_for_an_owner_away_from_the_origin() {
    let svg = "<svg width='64' height='64'><defs><clipPath id='c' clipPathUnits='objectBoundingBox'>\
<rect width='0.5' height='1'/></clipPath></defs>\
<rect x='40' y='30' width='8' height='8' clip-path='url(#c)'/></svg>";
    let graph = decode_ok(svg);
    assert_eq!(graph.root().shapes.len(), 1);
    assert!(graph.root().shapes[0].clip.is_some());
}

#[test]
fn marker_on_a_thin_shape_away_from_the_origin_decodes() {
    let svg = "<svg width='64' height='64'><defs><marker id='m' markerWidth='2' markerHeight='2'>\
<rect width='1' height='1'/></marker></defs>\
<path d='M30 40 L50 40' stroke='red' marker-end='url(#m)'/></svg>";
    assert_eq!(decode_ok(svg).root().shapes.len(), 2);
}
