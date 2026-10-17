//! WO-RUST-UI-TOKENS-01 acceptance 8: token swatch sheet goldens.
//!
//! The sheet (6 surfaces, 3 text levels on each, accent family, semantics,
//! focus ring) is drawn with `WidgetCanvas` and the software `scalar_text_draw`
//! path only (no CoreText, no icons), so the policy tolerance is 0: the PNG
//! encoder is deterministic, hence byte equality of the encoded sheet equals
//! pixel equality. Variants: Dark, Light, Dark high contrast, each at 1x and 2x.
//!
//! Goldens are first-party candidates until a human has viewed the review
//! PNGs. `VEXEL_BLESS_GOLDENS=1` rewrites them (fixtures plus
//! `artifacts/WO-RUST-UI-TOKENS-01/review/`); normal runs never write.
use vexel_png::{Compression, encode_rgba8};
use vexel_ui::{FontFamily, FontWeight, Rect, Theme, WidgetCanvas};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/ui/tokens");
const REVIEW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../artifacts/WO-RUST-UI-TOKENS-01/review"
);
const WIDTH: u32 = 320;
const ROW: u32 = 36;
const HEIGHT: u32 = ROW * 6 + 64;

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn render(theme: Theme, high_contrast: bool, scale: u32) -> Vec<u8> {
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let len = rgba.len();
    let mut c = WidgetCanvas::new(
        &mut rgba,
        len,
        w,
        h,
        (w * 4) as usize,
        scale,
        theme,
        high_contrast,
        None,
    )
    .expect("canvas");
    let t = c.tokens;
    for (i, surface) in t.surface.iter().enumerate() {
        let y = i as u32 * ROW;
        c.paint_rect(rect(0, y, WIDTH, ROW), *surface);
        let samples = [
            ("HIGH", t.text_primary),
            ("MEDIUM", t.text_secondary),
            ("LOW", t.text_low),
            ("OFF", t.disabled_text),
        ];
        for (k, (label, color)) in samples.into_iter().enumerate() {
            let text = format!("L{i} {label}");
            assert!(c.paint_text(
                &text,
                rect(8 + k as u32 * 78, y, 74, ROW),
                color,
                FontFamily::System,
                FontWeight::Regular,
                13
            ));
        }
    }
    let y = ROW * 6;
    c.paint_rect(rect(0, y, WIDTH, 64), t.surface[1]);
    let chips = [
        t.accent,
        t.accent_hover,
        t.accent_subtle,
        t.success,
        t.warning,
        t.error,
    ];
    for (i, color) in chips.into_iter().enumerate() {
        c.paint_round(
            rect(8 + i as u32 * 50, y + 8, 44, 24),
            t.radius_control,
            color,
            0,
            0,
        );
    }
    c.paint_round(
        rect(8, y + 38, 44, 20),
        t.radius_control,
        t.surface[3],
        t.control_edge,
        1,
    );
    c.paint_round(
        rect(60, y + 38, 44, 20),
        t.radius_control,
        t.surface[3],
        t.focus_ring,
        2,
    );
    c.paint_rect(rect(112, y + 47, 200, 1), t.separator);
    rgba
}

fn variants() -> [(&'static str, Theme, bool); 3] {
    [
        ("dark", Theme::Dark, false),
        ("light", Theme::Light, false),
        ("dark-high-contrast", Theme::Dark, true),
    ]
}

#[test]
fn token_swatch_sheets_match_first_party_goldens() {
    let bless = std::env::var_os("VEXEL_BLESS_GOLDENS").is_some_and(|v| v == "1");
    for (name, theme, high_contrast) in variants() {
        for scale in [1, 2] {
            let rgba = render(theme, high_contrast, scale);
            let png =
                encode_rgba8(WIDTH * scale, HEIGHT * scale, &rgba, Compression::Fixed).unwrap();
            let file = format!("swatch-{name}@{scale}x.png");
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
                "{file}: swatch differs from the golden (tolerance 0)"
            );
        }
    }
}

#[test]
fn swatch_render_is_repeatable_and_not_blank() {
    let a = render(Theme::Dark, false, 1);
    assert_eq!(a, render(Theme::Dark, false, 1));
    assert!(a.chunks_exact(4).any(|p| p[..3] != a[..3]));
    assert_ne!(a, render(Theme::Light, false, 1));
}
