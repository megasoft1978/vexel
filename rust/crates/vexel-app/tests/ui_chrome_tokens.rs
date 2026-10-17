//! WO-RUST-UI-CHROME-01 acceptance 7: chrome fills are `Tokens` fields and the
//! text drawn on toolbar, rail, inspector and status surfaces passes the
//! TOKENS contrast gates (text 4.5:1, glyphs and UI components 3:1).
use vexel_ui::{Theme, theme_tokens};

const SOURCES: [&str; 3] = [
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/render.rs"),
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/layout/paint.rs"),
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/inspector.rs"),
];

fn production(source: &str) -> &str {
    source.split("#[cfg(test)]").next().unwrap_or(source)
}

#[test]
fn chrome_sources_contain_no_literal_colours() {
    for path in SOURCES {
        let text = std::fs::read_to_string(path).unwrap();
        for (n, line) in production(&text).lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let mut rest = code;
            while let Some(at) = rest.find("0x") {
                let digits: String = rest[at + 2..]
                    .chars()
                    .take_while(|c| c.is_ascii_hexdigit() || *c == '_')
                    .collect();
                assert!(
                    digits.len() < 6,
                    "{path}:{}: literal colour 0x{digits}",
                    n + 1
                );
                rest = &rest[at + 2..];
            }
            for banned in ["rgb(", "rgba(", "Color::", "from_rgb"] {
                let bare = code
                    .match_indices(banned)
                    .any(|(i, _)| !code[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_'));
                assert!(!bare, "{path}:{}: {banned}", n + 1);
            }
        }
    }
}

fn lin(c: u32, shift: u32) -> f64 {
    let v = ((c >> shift) & 255) as f64 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn lum(c: u32) -> f64 {
    0.2126 * lin(c, 24) + 0.7152 * lin(c, 16) + 0.0722 * lin(c, 8)
}

fn contrast(a: u32, b: u32) -> f64 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[test]
fn text_and_glyphs_on_chrome_surfaces_pass_the_gates() {
    for theme in [Theme::Dark, Theme::Light] {
        let t = theme_tokens(theme, false);
        let thumb = if theme == Theme::Light {
            t.window
        } else {
            t.panel_hover
        };
        // (name, foreground, background, minimum)
        let pairs = [
            ("toolbar title", t.text_primary, t.panel_raised, 4.5),
            ("toolbar secondary", t.text_secondary, t.panel_raised, 4.5),
            ("segmented selected", t.text_primary, thumb, 4.5),
            ("segmented idle", t.text_secondary, t.panel, 4.5),
            ("toolbar glyph", t.text_primary, t.surface[3], 3.0),
            (
                "disabled glyph is exempt but visible",
                t.disabled_text,
                t.surface[3],
                1.5,
            ),
            ("rail glyph", t.text_secondary, t.panel, 3.0),
            ("rail selected glyph", t.accent, t.selection_fill, 3.0),
            ("section header", t.text_primary, t.panel_raised, 4.5),
            ("chevron", t.text_secondary, t.panel_raised, 3.0),
            ("inspector body", t.text_secondary, t.panel, 4.5),
            ("inspector hint", t.text_tertiary, t.panel, 4.5),
            ("field value", t.text_primary, t.surface[3], 4.5),
            ("field caption", t.text_secondary, t.surface[3], 4.5),
            ("status hint", t.text_tertiary, t.window, 4.5),
            ("status report", t.text_secondary, t.window, 4.5),
            ("status error", t.error, t.window, 4.5),
            ("popover text", t.text_primary, t.pressed_fill, 4.5),
        ];
        for (name, fg, bg, min) in pairs {
            let ratio = contrast(fg, bg);
            assert!(ratio >= min, "{theme:?} {name}: {ratio:.2} < {min}");
        }
    }
}
