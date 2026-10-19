//! Gradient paint goldens: handwritten SVG fixtures in `rust/fixtures/gradients`
//! composed through `compose_svg` (the production decode, render and frame path)
//! and compared byte-for-byte with PNG goldens (tolerance 0, the PNG encoder is
//! deterministic).
//!
//! Goldens are first-party: `VEXEL_BLESS_GOLDENS=1` rewrites them, normal runs
//! never write. Each blessed image must be viewed by a human before it is kept.
use vexel_app::compose::compose_svg;
use vexel_png::{Compression, encode_rgba8};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/gradients");

/// (fixture id, width, height)
const CASES: [(&str, u32, u32); 5] = [
    ("linear-pad", 64, 64),
    ("radial", 128, 128),
    ("linear-reflect", 64, 64),
    ("linear-repeat", 64, 64),
    ("stroke-rect", 64, 64),
];

fn png(id: &str, width: u32, height: u32) -> Vec<u8> {
    let source =
        std::fs::read(format!("{FIXTURES}/{id}.svg")).unwrap_or_else(|e| panic!("{id}: {e}"));
    let frame = compose_svg(&source, width, height).unwrap_or_else(|e| panic!("{id}: {e}"));
    assert!(
        frame.disclosures.is_empty(),
        "{id}: {:?}",
        frame.disclosures
    );
    encode_rgba8(width, height, &frame.rgba, Compression::Fixed).unwrap()
}

#[test]
fn gradient_fixtures_match_their_png_goldens() {
    let bless = std::env::var_os("VEXEL_BLESS_GOLDENS").is_some_and(|v| v == "1");
    for (id, width, height) in CASES {
        let rendered = png(id, width, height);
        assert_eq!(
            rendered,
            png(id, width, height),
            "{id}: rendering is repeatable"
        );
        let file = format!("{FIXTURES}/{id}.png");
        if bless {
            std::fs::write(&file, &rendered).unwrap();
            continue;
        }
        let golden = std::fs::read(&file).unwrap_or_else(|e| panic!("{id}.png: {e}"));
        assert!(
            golden == rendered,
            "{id}.png differs from the golden (tolerance 0)"
        );
    }
}
