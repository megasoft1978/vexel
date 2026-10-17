//! WO-RUST-UI-TOKENS-01 acceptance tests: ramp, contrast, doc parity, accent
//! adaptation, type/spacing, determinism. Writes the contrast report artifact.
//! Golden swatch comparisons live in `vexel-app/tests/ui_tokens_swatch.rs`.

use crate::theme::{ACCENT_CHROMA_CAP, rgb_lch};
use crate::tokens::contrast;
use crate::*;

const VARIANTS: [(&str, Theme, bool); 4] = [
    ("dark", Theme::Dark, false),
    ("light", Theme::Light, false),
    ("dark-high-contrast", Theme::Dark, true),
    ("light-high-contrast", Theme::Light, true),
];

fn hex(c: u32) -> String {
    format!("#{:06X}", c >> 8)
}

fn is_pure(c: u32) -> bool {
    matches!(c >> 8, 0x000000 | 0xffffff)
}

/// AC1: strictly monotonic elevation ramp, at least 3 L per step, no pure black/white.
#[test]
fn surface_ramp_is_monotonic_and_never_pure() {
    for (name, theme, hc) in VARIANTS {
        let t = theme_tokens(theme, hc);
        let ls: Vec<f64> = t.surface.iter().map(|c| rgb_lch(*c).0).collect();
        for pair in ls.windows(2) {
            let step = if theme == Theme::Dark {
                pair[1] - pair[0]
            } else {
                pair[0] - pair[1]
            };
            assert!(step >= 3.0, "{name}: step {step:.2} between {ls:?}");
        }
        for c in t
            .surface
            .iter()
            .chain([&t.text_primary, &t.text_secondary, &t.text_low])
        {
            assert!(!is_pure(*c), "{name}: pure black or white {c:08x}");
        }
        assert_eq!(t.window, t.surface[0]);
        assert_eq!(t.panel, t.surface[1]);
        assert_eq!(t.panel_raised, t.surface[2]);
        assert_eq!(
            (t.text_low, t.accent_subtle),
            (t.text_tertiary, t.selection_fill)
        );
    }
}

struct Row {
    variant: &'static str,
    pair: String,
    fg: u32,
    bg: u32,
    ratio: f64,
    threshold: Option<f64>,
}

fn rows() -> Vec<Row> {
    let mut out = Vec::new();
    for (variant, theme, hc) in VARIANTS {
        let t = theme_tokens(theme, hc);
        let mut add = |pair: String, fg: u32, bg: u32, threshold: Option<f64>| {
            out.push(Row {
                variant,
                pair,
                fg,
                bg,
                ratio: contrast(fg, bg),
                threshold,
            });
        };
        let text_min = if hc { 7.0 } else { 4.5 };
        for (n, c) in [
            ("text.high", t.text_primary),
            ("text.medium", t.text_secondary),
            ("text.low", t.text_low),
        ] {
            for l in [1, 2] {
                add(format!("{n} on level {l}"), c, t.surface[l], Some(text_min));
            }
            for l in 1..=3 {
                add(
                    format!("{n} at 18px+ on level {l}"),
                    c,
                    t.surface[l],
                    Some(3.0),
                );
            }
        }
        for l in 1..=3 {
            add(
                format!("focus ring on level {l}"),
                t.focus_ring,
                t.surface[l],
                Some(3.0),
            );
            add(
                format!("accent glyph on level {l}"),
                t.accent,
                t.surface[l],
                Some(3.0),
            );
            add(
                format!("accent.hover glyph on level {l}"),
                t.accent_hover,
                t.surface[l],
                Some(3.0),
            );
            let edge_min = if hc { Some(4.5) } else { None };
            add(
                format!("control edge on level {l}"),
                t.control_edge,
                t.surface[l],
                edge_min,
            );
        }
        add(
            "accent on accent.subtle (selected text)".into(),
            t.accent,
            t.accent_subtle,
            Some(3.0),
        );
        add(
            "text.high on accent.subtle".into(),
            t.text_primary,
            t.accent_subtle,
            Some(4.5),
        );
        for (n, c) in [
            ("accent", t.accent),
            ("success", t.success),
            ("warning", t.warning),
            ("error", t.error),
        ] {
            add(format!("{n} on level 1"), c, t.surface[1], Some(4.5));
        }
        add(
            "separator on level 1".into(),
            t.separator,
            t.surface[1],
            None,
        );
        add(
            "text.disabled on level 1".into(),
            t.disabled_text,
            t.surface[1],
            None,
        );
    }
    out
}

fn report_json(rows: &[Row]) -> String {
    let mut s = String::from(
        "{\n  \"wcag\": \"2.x relative luminance\",\n  \"exempt\": [\"separator (decorative)\", \"control edge (decorative outside high contrast; every control also has a fill)\", \"text.disabled\"],\n  \"pairs\": [\n",
    );
    for (i, r) in rows.iter().enumerate() {
        let (threshold, status) = match r.threshold {
            Some(t) => (
                format!("{t:.1}"),
                if r.ratio >= t { "pass" } else { "fail" },
            ),
            None => ("null".to_string(), "exempt"),
        };
        s += &format!(
            "    {{\"variant\":\"{}\",\"pair\":\"{}\",\"fg\":\"{}\",\"bg\":\"{}\",\"ratio\":{:.2},\"threshold\":{},\"status\":\"{}\"}}{}\n",
            r.variant,
            r.pair,
            hex(r.fg),
            hex(r.bg),
            r.ratio,
            threshold,
            status,
            if i + 1 == rows.len() { "" } else { "," }
        );
    }
    s + "  ]\n}\n"
}

/// AC2: every pair clears its gate; the report artifact is written from the same data.
#[test]
fn contrast_gates_hold_and_report_is_written() {
    let rows = rows();
    let failures: Vec<String> = rows
        .iter()
        .filter(|r| r.threshold.is_some_and(|t| r.ratio < t))
        .map(|r| {
            format!(
                "{} {}: {:.2} < {:?}",
                r.variant, r.pair, r.ratio, r.threshold
            )
        })
        .collect();
    assert!(
        failures.is_empty(),
        "contrast failures:\n{}",
        failures.join("\n")
    );
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../artifacts/WO-RUST-UI-TOKENS-01");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("contrast-report.json"), report_json(&rows)).unwrap();
}

fn parse_hex(s: &str) -> u32 {
    (u32::from_str_radix(&s[1..], 16).unwrap() << 8) | 255
}

fn within_one(a: u32, b: u32) -> bool {
    [24, 16, 8]
        .iter()
        .all(|s| ((a >> s & 255) as i32 - (b >> s & 255) as i32).abs() <= 1)
}

/// AC3: values printed in design-direction section 4 versus generated values.
#[test]
fn generated_values_match_the_design_doc_table() {
    let d = theme_tokens(Theme::Dark, false);
    let l = theme_tokens(Theme::Light, false);
    let doc: [(&str, u32, &str); 18] = [
        ("dark level 0", d.surface[0], "#171A1D"),
        ("dark level 1", d.surface[1], "#1E2125"),
        ("dark level 2", d.surface[2], "#25282C"),
        ("dark level 3", d.surface[3], "#2E3135"),
        ("dark level 4", d.surface[4], "#36393D"),
        ("dark level 5", d.surface[5], "#3F4247"),
        ("dark separator", d.separator, "#36393D"),
        ("dark control edge", d.control_edge, "#4D5055"),
        ("dark text.medium", d.text_secondary, "#A8ABB0"),
        ("dark text.low", d.text_low, "#8E9196"),
        ("dark text.disabled", d.disabled_text, "#5B5F63"),
        ("dark accent", d.accent, "#5EA3FA"),
        ("dark success", d.success, "#6FCA9E"),
        ("dark warning", d.warning, "#F1BE69"),
        ("dark error", d.error, "#FF7F7A"),
        ("light text.high", l.text_primary, "#1D2024"),
        ("light text.medium", l.text_secondary, "#575A5E"),
        (
            "dark focus ring is lighter than accent",
            d.focus_ring,
            "#7DBEFF",
        ),
    ];
    for (name, actual, expected) in doc {
        assert!(
            within_one(actual, parse_hex(expected)),
            "{name}: generated {} vs doc {expected}",
            hex(actual)
        );
    }
    // Stale doc row: the doc light accent #1074C6 (L48) is only 4.27:1 on the light panel
    // (level 1 is L95 here), below the 4.5:1 gate, so the token moved to L45.
    let stale = parse_hex("#1074C6");
    assert!(contrast(stale, l.surface[1]) < 4.5);
    assert!(!within_one(l.accent, stale));
    // Stale doc row: dark text.high #E9EBEF (L93) is 13.54:1 on level 1; the pre-existing
    // 14:1 assertion in tests.rs needs L95.
    let stale = parse_hex("#E9EBEF");
    assert!(contrast(stale, d.surface[1]) < 14.0);
    assert!(contrast(d.text_primary, d.surface[1]) >= 14.0);
}

const SAMPLES: [u32; 12] = [
    0xff3b30ff, 0xff9500ff, 0xffcc00ff, 0x34c759ff, 0x00c7beff, 0x007affff, 0xaf52deff, 0x8e8e93ff,
    0xff0000ff, 0xffff00ff, 0xf8f8f8ff, 0x080808ff,
];

/// AC4: chroma cap, contrast on levels 1 to 3, default set for zero or alpha-0 accents.
#[test]
fn accent_adaptation_caps_chroma_and_keeps_contrast() {
    for (name, theme, hc) in VARIANTS {
        for source in SAMPLES {
            let t = theme_tokens_accented(theme, hc, source);
            let (_, chroma, _) = rgb_lch(t.accent);
            assert!(
                chroma <= ACCENT_CHROMA_CAP,
                "{name} {source:08x}: chroma {chroma:.1}"
            );
            let need = if hc { [4.5; 3] } else { [4.5, 3.0, 3.0] };
            for i in 0..3 {
                let c = contrast(t.accent, t.surface[i + 1]);
                assert!(
                    c >= need[i],
                    "{name} {source:08x}: {c:.2} on level {}",
                    i + 1
                );
            }
            assert_eq!((t.accent, t.accent), (t.selection_edge, t.focus_ring));
            assert_eq!(t.accent_subtle, t.selection_fill);
            assert!(
                contrast(t.accent, t.accent_subtle) >= 3.0,
                "{name} {source:08x} on subtle"
            );
        }
        let base = theme_tokens(theme, hc);
        assert_eq!(theme_tokens_accented(theme, hc, 0), base);
        assert_eq!(theme_tokens_accented(theme, hc, 0x12345600), base);
    }
}

/// AC5: type floor, spacing multiples of 2 and strictly increasing, radii strictly increasing.
#[test]
fn type_spacing_radii_and_motion_scales() {
    let ts = type_scale();
    let roles = [
        ts.title_pt,
        ts.heading_pt,
        ts.body_pt,
        ts.label_pt,
        ts.meta_pt,
        ts.numeric_pt,
        ts.document_title_pt,
        ts.panel_heading_pt,
        ts.section_heading_pt,
        ts.secondary_pt,
        ts.metadata_pt,
        ts.geometry_pt,
        ts.empty_heading_pt,
    ];
    assert!(roles.iter().all(|pt| *pt >= 12), "{roles:?}");
    assert_eq!(
        (ts.title_pt, ts.heading_pt, ts.body_pt, ts.label_pt),
        (15, 13, 13, 12)
    );
    assert!(ts.numeric_tabular && ts.geometry_monospaced);
    assert_eq!(
        (ts.weight_regular, ts.weight_medium, ts.weight_semibold),
        (400, 500, 600)
    );
    for (_, theme, hc) in VARIANTS {
        let t = theme_tokens(theme, hc);
        let spacing = [
            t.spacing_2,
            t.spacing_4,
            t.spacing_8,
            t.spacing_12,
            t.spacing_16,
            t.spacing_24,
            t.spacing_32,
        ];
        assert_eq!(spacing, [2, 4, 8, 12, 16, 24, 32]);
        assert!(spacing.iter().all(|v| v % 2 == 0) && spacing.windows(2).all(|w| w[0] < w[1]));
        let radii = [
            t.radius_small,
            t.radius_control,
            t.radius_card,
            t.radius_large,
        ];
        assert_eq!(radii, [4, 6, 8, 12]);
        let m = t.motion;
        assert_eq!(
            (m.instant_ms, m.fast_ms, m.base_ms, m.slow_ms),
            (80, 120, 180, 240)
        );
        assert_eq!(m.ease_out, [0.2, 0.0, 0.0, 1.0]);
        assert_eq!(m.standard, [0.3, 0.0, 0.2, 1.0]);
    }
    let dm = design_metrics();
    assert_eq!(
        (
            dm.toolbar_height,
            dm.tool_rail_width,
            dm.inspector_width,
            dm.status_height,
            dm.section_header_height,
            dm.field_height
        ),
        (44, 44, 296, 24, 28, 28)
    );
}

fn words(t: &Tokens) -> Vec<u32> {
    let mut w = vec![
        t.window,
        t.panel,
        t.panel_raised,
        t.panel_hover,
        t.pressed_fill,
        t.canvas_well,
        t.text_primary,
        t.text_secondary,
        t.text_tertiary,
        t.disabled_text,
        t.separator,
        t.control_edge,
        t.accent,
        t.selection_fill,
        t.selection_edge,
        t.focus_ring,
        t.checker_a,
        t.checker_b,
        t.guide,
        t.success,
        t.warning,
        t.error,
        t.popover_shadow,
        t.panel_alt,
        t.canvas,
        t.text,
        t.muted,
        t.border,
        t.spacing,
        t.radius,
        t.font_px,
        t.spacing_2,
        t.spacing_4,
        t.spacing_8,
        t.spacing_12,
        t.spacing_16,
        t.spacing_24,
        t.spacing_32,
        t.radius_small,
        t.radius_control,
        t.radius_card,
        t.radius_large,
        t.text_low,
        t.accent_hover,
        t.accent_subtle,
    ];
    w.extend(t.surface);
    let m = t.motion;
    w.extend(
        [
            m.instant_ms,
            m.fast_ms,
            m.base_ms,
            m.slow_ms,
            m.exit_percent,
        ]
        .map(u32::from),
    );
    w.extend(
        [m.ease_out, m.standard, m.ease_in]
            .iter()
            .flatten()
            .map(|f| f.to_bits()),
    );
    w
}

fn fnv(words: &[u32]) -> u64 {
    words
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
        })
}

/// Frozen hashes of every token word. Identical in the debug and release test profiles.
const FROZEN: [u64; 4] = [
    0x47be696848e16cf4,
    0x09dd0a986faea101,
    0xad98a539030ce031,
    0x6fcff6e5d9580b6f,
];

/// AC7: determinism and frozen hashes.
#[test]
fn tokens_are_deterministic_with_frozen_hashes() {
    let mut actual = Vec::new();
    for (name, theme, hc) in VARIANTS {
        let (a, b) = (theme_tokens(theme, hc), theme_tokens(theme, hc));
        assert_eq!(a, b, "{name}");
        actual.push(fnv(&words(&a)));
    }
    assert_eq!(
        actual, FROZEN,
        "actual hashes (update FROZEN only after reviewing a deliberate token change): {actual:016x?}"
    );
}
