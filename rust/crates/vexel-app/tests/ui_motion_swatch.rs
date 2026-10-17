//! WO-RUST-UI-MOTION-01 acceptance 8: control-state swatch sheet goldens.
//!
//! Rows: button, icon button, segmented control, text field. Columns: rest,
//! hover, pressed, focus, disabled, selected. Colours come from `Tokens`
//! through `widget_paint`; text uses the software scalar glyph path (no
//! CoreText) and icons are software masks, so the policy tolerance is 0 and the
//! deterministic PNG bytes are compared. Variants: Dark and Light at 1x and 2x.
//!
//! Goldens are first-party candidates until a human has viewed the review
//! PNGs. `VEXEL_BLESS_GOLDENS=1` writes the fixtures plus review copies under
//! `artifacts/WO-RUST-UI-MOTION-01/review/`; normal runs never write.
use vexel_png::{Compression, encode_rgba8};
use vexel_ui::motion::{ControlColors, ControlVisual};
use vexel_ui::{
    ControlState, IconKind, Rect, Theme, WidgetCanvas, WidgetDrawSpec, WidgetKind, theme_tokens,
    widget_paint,
};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/ui/motion");
const REVIEW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../artifacts/WO-RUST-UI-MOTION-01/review"
);
const CELL_W: u32 = 104;
const CELL_H: u32 = 52;
const WIDTH: u32 = CELL_W * 6;
const HEIGHT: u32 = CELL_H * 4;
const STATES: [ControlState; 6] = [
    ControlState::Rest,
    ControlState::Hover,
    ControlState::Pressed,
    ControlState::Focus,
    ControlState::Disabled,
    ControlState::Selected,
];

fn render(theme: Theme, scale: u32) -> Vec<u8> {
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
        false,
        None,
    )
    .expect("canvas");
    let window = c.tokens.window;
    c.paint_rect(
        Rect {
            x: 0,
            y: 0,
            width: WIDTH,
            height: HEIGHT,
        },
        window,
    );
    let labels = ["One", "Two", "Three"];
    for (row, kind) in [
        WidgetKind::ButtonSecondary,
        WidgetKind::IconButton,
        WidgetKind::Segmented,
        WidgetKind::TextField,
    ]
    .into_iter()
    .enumerate()
    {
        for (col, state) in STATES.into_iter().enumerate() {
            let bounds = Rect {
                x: col as u32 * CELL_W + 10,
                y: row as u32 * CELL_H + 10,
                width: CELL_W - 20,
                height: 30,
            };
            let mut s = WidgetDrawSpec::new(kind, bounds);
            s.state = state;
            match kind {
                WidgetKind::ButtonSecondary => s.label = Some("Button"),
                WidgetKind::IconButton => {
                    s.icon = IconKind::LAYERS;
                    s.bounds.width = 30;
                }
                WidgetKind::Segmented => {
                    s.segment_count = 3;
                    s.segment_index = 1;
                    s.segment_labels = &labels;
                }
                _ => s.value = Some("Text"),
            }
            assert!(widget_paint(&mut c, &s), "{kind:?} {state:?}");
        }
    }
    rgba
}

#[test]
fn state_swatch_sheets_match_first_party_goldens() {
    let bless = std::env::var_os("VEXEL_BLESS_GOLDENS").is_some_and(|v| v == "1");
    for (name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for scale in [1, 2] {
            let rgba = render(theme, scale);
            let png =
                encode_rgba8(WIDTH * scale, HEIGHT * scale, &rgba, Compression::Fixed).unwrap();
            let file = format!("states-{name}@{scale}x.png");
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

/// The interpolated visuals end exactly on the token fills the swatch shows.
#[test]
fn control_visual_end_states_equal_token_fills() {
    for theme in [Theme::Dark, Theme::Light] {
        let t = theme_tokens(theme, false);
        let colors = ControlColors {
            rest_fill: t.panel_raised,
            hover_fill: t.panel_hover,
            pressed_fill: t.pressed_fill,
            rest_glyph: t.disabled_text,
            hover_glyph: t.text_primary,
            disabled_glyph: t.disabled_text,
            ring: t.focus_ring,
        };
        assert_eq!(
            ControlVisual::resolve(&colors, 0.0, 0.0, 0.0, false).fill,
            t.panel_raised
        );
        assert_eq!(
            ControlVisual::resolve(&colors, 1.0, 0.0, 0.0, false).fill,
            t.panel_hover
        );
        assert_eq!(
            ControlVisual::resolve(&colors, 1.0, 1.0, 0.0, false).fill,
            t.pressed_fill
        );
        assert_eq!(
            ControlVisual::resolve(&colors, 0.0, 0.0, 1.0, false).ring,
            t.focus_ring
        );
        assert_eq!(
            ControlVisual::resolve(&colors, 1.0, 0.0, 0.0, true).glyph,
            t.disabled_text
        );
    }
}
