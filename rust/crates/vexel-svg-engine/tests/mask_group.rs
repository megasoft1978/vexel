//! WO-RUST-SVG-MASK-01: a referenced `<mask>` on a group decodes through the engine
//! and the published group carries the mask; source bytes stay untouched.
mod common;
use common::*;

const HEAD: &str = "<svg width='16' height='16'>";

fn masked_svg() -> String {
    format!(
        "{HEAD}<defs><mask id='m' x='0' y='0' width='16' height='16' maskUnits='userSpaceOnUse'>\
<rect width='4' height='4' fill='white'/></mask></defs>\
<g mask='url(#m)'><rect width='8' height='8'/><rect x='8' width='4' height='4'/></g></svg>"
    )
}

#[test]
fn referenced_mask_on_group_publishes_group_with_mask() {
    let svg = masked_svg();
    let graph = decode_ok(&svg);
    let scene = graph.root();
    assert_eq!(graph.source(), svg.as_bytes(), "source bytes untouched");
    assert_eq!(scene.shapes.len(), 2);
    assert_eq!(scene.masks.len(), 1);
    assert_eq!(scene.groups.len(), 1);
    let group = &scene.groups[0];
    assert_eq!(group.shapes.count, 2);
    let mask = group.mask.expect("group carries the mask");
    assert_eq!(scene.mask(mask).map(|m| m.luminance), Some(true));
}

#[test]
fn unreferenced_mask_still_preserved_and_invalid_reference_still_fails() {
    let unreferenced = format!(
        "{HEAD}<defs><mask id='m'><rect width='4' height='4' fill='white'/></mask></defs>\
<rect width='8' height='8'/></svg>"
    );
    let graph = decode_ok(&unreferenced);
    assert!(graph.root().masks.is_empty() && graph.root().groups.is_empty());
    let missing = format!("{HEAD}<g mask='url(#nope)'><rect width='8' height='8'/></g></svg>");
    assert_eq!(
        decode_err(&missing).kind,
        vexel_scene::ErrorKind::Unsupported
    );
}
