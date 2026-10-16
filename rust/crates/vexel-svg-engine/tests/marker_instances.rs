//! WO-RUST-CORPUS-UNSUPPORTED-01: marker instancing through the real Decoder.
//! The marker content is a 1x1 square with `markerUnits='userSpaceOnUse'`, so
//! each instance's bounding box is its placement point plus (0..1, 0..1) before
//! the marker transform (scale 1, `refX=refY=0`).
mod common;
use common::*;

const HEAD: &str = "<svg width='32' height='32'>";
const DEFS: &str = "<defs><marker id='m' markerUnits='userSpaceOnUse' markerWidth='1' markerHeight='1'>\
<rect width='1' height='1'/></marker></defs>";

/// Bounding boxes `[x0, y0, x1, y1]` of every shape after the first (the host).
fn instances(svg: &str) -> Vec<[f64; 4]> {
    let graph = decode_ok(svg);
    let scene = graph.root();
    scene
        .shapes
        .iter()
        .skip(1)
        .map(|shape| {
            let edges = shape.edges.slice(scene.edges).expect("shape edges");
            let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
            for p in edges.iter().flat_map(|e| [e.a, e.b]) {
                b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
            }
            b
        })
        .collect()
}

fn near(a: [f64; 4], b: [f64; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
}

#[test]
fn start_mid_and_end_each_publish_one_instance_at_their_vertex() {
    let svg = format!(
        "{HEAD}{DEFS}<polyline points='2,2 10,2 10,12 20,12' fill='none' stroke='red' \
marker-start='url(#m)' marker-mid='url(#m)' marker-end='url(#m)'/></svg>"
    );
    let boxes = instances(&svg);
    let expected = [
        [2., 2., 3., 3.],
        [10., 2., 11., 3.],
        [10., 12., 11., 13.],
        [20., 12., 21., 13.],
    ];
    assert_eq!(boxes.len(), expected.len(), "{boxes:?}");
    for (got, want) in boxes.iter().zip(expected) {
        assert!(near(*got, want), "{got:?} vs {want:?}");
    }
}

#[test]
fn only_the_referenced_positions_are_instanced() {
    let svg = format!(
        "{HEAD}{DEFS}<path d='M2 2 L10 2 L10 12' fill='none' stroke='red' marker-end='url(#m)'/></svg>"
    );
    let boxes = instances(&svg);
    assert_eq!(boxes.len(), 1);
    assert!(near(boxes[0], [10., 12., 11., 13.]));
}

#[test]
fn marker_shorthand_property_applies_to_all_positions() {
    let svg = format!(
        "{HEAD}{DEFS}<path d='M2 2 L10 2 L10 12' fill='none' stroke='red' style='marker:url(#m)'/></svg>"
    );
    assert_eq!(instances(&svg).len(), 3);
}

#[test]
fn subpaths_get_their_own_start_and_end() {
    let svg = format!(
        "{HEAD}{DEFS}<path d='M2 2 L10 2 M20 20 L28 20' fill='none' stroke='red' \
marker-start='url(#m)' marker-end='url(#m)'/></svg>"
    );
    let boxes = instances(&svg);
    assert_eq!(boxes.len(), 4, "{boxes:?}");
    assert!(boxes.iter().any(|b| near(*b, [20., 20., 21., 21.])));
    assert!(boxes.iter().any(|b| near(*b, [28., 20., 29., 21.])));
}

#[test]
fn a_fixed_orient_angle_is_not_replaced_by_the_mid_vertex_bisector() {
    // Without orient="auto" the mid marker keeps angle 0 (legacy C behaviour).
    let fixed = format!(
        "{HEAD}{DEFS}<path d='M2 2 L10 10 L18 2' fill='none' stroke='red' marker-mid='url(#m)'/></svg>"
    );
    let boxes = instances(&fixed);
    assert_eq!(boxes.len(), 1);
    assert!(near(boxes[0], [10., 10., 11., 11.]), "{:?}", boxes[0]);
}

#[test]
fn unreferenced_position_without_any_marker_property_publishes_no_instances() {
    let svg = format!("{HEAD}{DEFS}<path d='M2 2 L10 2' fill='none' stroke='red'/></svg>");
    assert!(instances(&svg).is_empty());
}
