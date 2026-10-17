//! WO-RUST-UI-CANVAS-01 acceptance 8: stage goldens (artboard, shadow,
//! checkerboard under transparency, hover, selection and handles) at 1x and
//! 2x in dark and light.
//!
//! Everything is software-rasterised (`vexel-render` composition plus the
//! canvas painter; no CoreText glyphs, no icons), so the policy tolerance is 0:
//! the PNG encoder is deterministic, hence byte equality of the encoded image
//! equals pixel equality. The zoom cluster draws text and is covered by the
//! geometry assertions in `ui_canvas_view.rs` instead.
//!
//! Goldens are first-party candidates until a human has viewed the review
//! PNGs. `VEXEL_BLESS_GOLDENS=1` rewrites them (fixtures plus
//! `artifacts/WO-RUST-UI-CANVAS-01/review/`); normal runs never write.
#[path = "ui_canvas_support.rs"]
mod support;

use support::{COMPOSE_V1, Scene, composed, render};
use vexel_app::canvas::{View, ViewMap, artboard_pixels};
use vexel_png::{Compression, encode_rgba8};
use vexel_ui::Rect;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/ui/canvas");
const REVIEW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../artifacts/WO-RUST-UI-CANVAS-01/review"
);
const STAGE: (u32, u32) = (320, 220);

fn stage_image(dark: bool, scale: u32) -> Vec<u8> {
    let natural = (160.0, 120.0);
    let view = View::fit((f64::from(STAGE.0), f64::from(STAGE.1)), natural);
    let size = artboard_pixels(natural, view.zoom, scale);
    let doc = composed(COMPOSE_V1, size);
    let map = ViewMap::new(view, natural, size, doc.map).expect("frame matches the artboard");
    let bounds = |index: usize| {
        let shape = doc.shapes.iter().find(|s| s.scene_index == index).unwrap();
        map.shape_bounds(shape).unwrap()
    };
    let shot = render(&Scene {
        window: STAGE,
        stage: Rect {
            x: 0,
            y: 0,
            width: STAGE.0,
            height: STAGE.1,
        },
        scale,
        dark,
        view,
        artboard: natural,
        frame: Some(&doc.frame),
        selection: Some(bounds(0)),
        hover: Some(bounds(1)),
    });
    shot.px
}

#[test]
fn stage_goldens_match_first_party_images() {
    let bless = std::env::var_os("VEXEL_BLESS_GOLDENS").is_some_and(|v| v == "1");
    for (name, dark) in [("dark", true), ("light", false)] {
        for scale in [1u32, 2] {
            let rgba = stage_image(dark, scale);
            assert_eq!(rgba, stage_image(dark, scale), "rendering is repeatable");
            let png =
                encode_rgba8(STAGE.0 * scale, STAGE.1 * scale, &rgba, Compression::Fixed).unwrap();
            let file = format!("stage-{name}@{scale}x.png");
            if bless {
                std::fs::create_dir_all(FIXTURES).unwrap();
                std::fs::create_dir_all(REVIEW).unwrap();
                std::fs::write(format!("{FIXTURES}/{file}"), &png).unwrap();
                std::fs::write(format!("{REVIEW}/{file}"), &png).unwrap();
                continue;
            }
            let golden = std::fs::read(format!("{FIXTURES}/{file}"))
                .unwrap_or_else(|e| panic!("{file}: {e}"));
            assert!(
                golden == png,
                "{file}: stage differs from the golden (tolerance 0)"
            );
        }
    }
}
