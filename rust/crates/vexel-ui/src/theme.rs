//! Deterministic token generator: CIE LCH(ab) under D65 to sRGB.
//!
//! Every colour token is derived from a small table of `(L, C, h)` triples
//! (design-direction section 4). Only `std` float functions are used and no
//! state is kept, so two calls always return identical `u32`s.

use crate::tokens::{Motion, Theme, Tokens};

/// Highest chroma an accent or semantic colour may carry.
pub const ACCENT_CHROMA_CAP: f64 = 50.0;
/// Neutral hue (blue-grey) shared by surfaces and text.
const NEUTRAL_HUE: f64 = 265.0;
const WHITE: [f64; 3] = [0.95047, 1.0, 1.08883];
const DELTA: f64 = 6.0 / 29.0;

fn lab_inverse(t: f64) -> f64 {
    if t > DELTA {
        t * t * t
    } else {
        3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
    }
}

fn lab_forward(t: f64) -> f64 {
    if t > DELTA * DELTA * DELTA {
        t.cbrt()
    } else {
        t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
    }
}

fn encode(linear: f64) -> f64 {
    let v = linear.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn decode(channel: u32) -> f64 {
    let v = channel as f64 / 255.0;
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn pack(linear: [f64; 3]) -> u32 {
    let byte = |v: f64| (encode(v) * 255.0).round() as u32;
    (byte(linear[0]) << 24) | (byte(linear[1]) << 16) | (byte(linear[2]) << 8) | 255
}

fn lch_linear(l: f64, c: f64, h: f64) -> [f64; 3] {
    let (a, b) = (c * h.to_radians().cos(), c * h.to_radians().sin());
    let fy = (l + 16.0) / 116.0;
    let x = WHITE[0] * lab_inverse(fy + a / 500.0);
    let y = WHITE[1] * lab_inverse(fy);
    let z = WHITE[2] * lab_inverse(fy - b / 200.0);
    [
        3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z,
        -0.969_266_0 * x + 1.876_010_8 * y + 0.041_556_0 * z,
        0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z,
    ]
}

/// LCH to opaque `0xRRGGBBAA`, clamping out-of-gamut channels.
pub fn lch_rgb(l: f64, c: f64, h: f64) -> u32 {
    pack(lch_linear(l, c, h))
}

/// LCH to opaque `0xRRGGBBAA`, reducing chroma until the colour is in gamut.
pub fn lch_rgb_fit(l: f64, c: f64, h: f64) -> u32 {
    let inside = |c: f64| {
        lch_linear(l, c, h)
            .iter()
            .all(|v| (-0.0005..=1.0005).contains(v))
    };
    if inside(c) {
        return lch_rgb(l, c, h);
    }
    let (mut low, mut high) = (0.0, c);
    for _ in 0..24 {
        let mid = (low + high) / 2.0;
        if inside(mid) {
            low = mid;
        } else {
            high = mid;
        }
    }
    lch_rgb(l, low, h)
}

/// sRGB `0xRRGGBBAA` to LCH(ab) under D65.
pub fn rgb_lch(color: u32) -> (f64, f64, f64) {
    let (r, g, b) = (
        decode(color >> 24 & 255),
        decode(color >> 16 & 255),
        decode(color >> 8 & 255),
    );
    let x = (0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / WHITE[0];
    let y = (0.212_672_9 * r + 0.715_152_2 * g + 0.072_175_0 * b) / WHITE[1];
    let z = (0.019_333_9 * r + 0.119_192_0 * g + 0.950_304_1 * b) / WHITE[2];
    let (fx, fy, fz) = (lab_forward(x), lab_forward(y), lab_forward(z));
    let (l, a, b) = (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz));
    let h = b.atan2(a).to_degrees();
    (l, a.hypot(b), if h < 0.0 { h + 360.0 } else { h })
}

type Lch = (f64, f64, f64);

/// One theme's source triples.
struct Spec {
    surface_l: [f64; 6],
    surface_c: f64,
    high: Lch,
    medium: Lch,
    low: Lch,
    disabled: Lch,
    edge: Lch,
    accent: Lch,
    hover: Lch,
    success: Lch,
    warning: Lch,
    error: Lch,
    subtle_percent: u32,
}

fn neutral(l: f64, c: f64) -> Lch {
    (l, c, NEUTRAL_HUE)
}

fn spec(light: bool, hc: bool) -> Spec {
    match (light, hc) {
        // Dark rises with elevation. Text.high carries chroma 2, the rest 3. The doc's
        // text.high L93 is 13.54:1 on level 1; the existing 14:1 pin needs L95.
        (false, false) => Spec {
            surface_l: [9.0, 12.5, 16.0, 20.0, 24.0, 28.0],
            surface_c: 3.0,
            high: neutral(95.0, 2.0),
            medium: neutral(70.0, 3.0),
            low: neutral(60.0, 3.0),
            disabled: neutral(40.0, 3.0),
            edge: neutral(34.0, 3.0),
            accent: (66.0, 50.0, 275.0),
            hover: (72.0, 50.0, 275.0),
            success: (75.0, 40.0, 160.0),
            warning: (80.0, 50.0, 80.0),
            error: (68.0, 55.0, 28.0),
            subtle_percent: 16,
        },
        (false, true) => Spec {
            high: neutral(96.0, 2.0),
            medium: neutral(78.0, 3.0),
            low: neutral(74.0, 3.0),
            disabled: neutral(46.0, 3.0),
            edge: neutral(66.0, 3.0),
            subtle_percent: 20,
            ..spec(false, false)
        },
        // Light falls with elevation (edge plus shadow carry the raise).
        (true, false) => Spec {
            surface_l: [98.5, 95.0, 91.5, 88.0, 84.5, 81.0],
            surface_c: 2.0,
            high: neutral(12.0, 3.0),
            medium: neutral(38.0, 3.0),
            low: neutral(43.0, 3.0),
            disabled: neutral(65.0, 3.0),
            edge: neutral(76.0, 3.0),
            accent: (45.0, 50.0, 275.0),
            hover: (40.0, 50.0, 275.0),
            success: (45.0, 40.0, 160.0),
            warning: (45.0, 50.0, 80.0),
            error: (45.0, 55.0, 28.0),
            subtle_percent: 12,
        },
        (true, true) => Spec {
            high: neutral(8.0, 3.0),
            medium: neutral(26.0, 3.0),
            low: neutral(30.0, 3.0),
            disabled: neutral(58.0, 3.0),
            edge: neutral(40.0, 3.0),
            accent: (40.0, 50.0, 275.0),
            hover: (36.0, 50.0, 275.0),
            subtle_percent: 20,
            ..spec(true, false)
        },
    }
}

/// Integer blend of `front` over `back` (opaque), `percent` of front.
pub(crate) fn blend(back: u32, front: u32, percent: u32) -> u32 {
    let mut out = 255;
    for shift in [8, 16, 24] {
        let (b, f) = ((back >> shift) & 255, (front >> shift) & 255);
        out |= ((b * (100 - percent) + f * percent + 50) / 100) << shift;
    }
    out
}

fn rgb(t: Lch) -> u32 {
    lch_rgb(t.0, t.1, t.2)
}

/// The full token set for a theme (no system-accent adaptation).
pub(crate) fn build(theme: Theme, hc: bool) -> Tokens {
    let light = theme == Theme::Light;
    let s = spec(light, hc);
    let surface = s.surface_l.map(|l| lch_rgb(l, s.surface_c, NEUTRAL_HUE));
    let (accent, edge) = (rgb(s.accent), rgb(s.edge));
    let accent_subtle = blend(surface[1], accent, s.subtle_percent);
    let focus = if light {
        accent
    } else if hc {
        lch_rgb(86.0, 40.0, 275.0)
    } else {
        lch_rgb(76.0, 50.0, 275.0)
    };
    let (checker_a, checker_b, guide, shadow) = if light {
        (0xe0e4eaff, 0xf3f5f7ff, 0xd000a8ff, 0x0000_0024)
    } else {
        (0x292e36ff, 0x20242bff, 0xff5ac8ff, 0x0000_006b)
    };
    let text = rgb(s.high);
    let medium = rgb(s.medium);
    let low = rgb(s.low);
    let separator = if hc { edge } else { surface[4] };
    Tokens {
        window: surface[0],
        panel: surface[1],
        panel_raised: surface[2],
        panel_hover: surface[4],
        pressed_fill: surface[5],
        canvas_well: surface[0],
        text_primary: text,
        text_secondary: medium,
        text_tertiary: low,
        disabled_text: rgb(s.disabled),
        separator,
        control_edge: edge,
        accent,
        selection_fill: accent_subtle,
        selection_edge: accent,
        focus_ring: focus,
        checker_a,
        checker_b,
        guide,
        success: rgb(s.success),
        warning: rgb(s.warning),
        error: rgb(s.error),
        popover_shadow: shadow,
        panel_alt: surface[2],
        canvas: surface[0],
        text,
        muted: medium,
        border: separator,
        spacing: 8,
        radius: 6,
        font_px: 13,
        spacing_4: 4,
        spacing_8: 8,
        spacing_12: 12,
        spacing_16: 16,
        spacing_24: 24,
        spacing_32: 32,
        radius_small: 4,
        radius_control: 6,
        radius_large: 12,
        surface,
        text_low: low,
        accent_hover: rgb(s.hover),
        accent_subtle,
        spacing_2: 2,
        radius_card: 8,
        motion: Motion::STANDARD,
    }
}

/// Re-derive the accent family from a system accent: chroma capped, then the
/// lightness is searched until the accent clears the contrast gates on levels 1 to 3.
pub(crate) fn adapt_accent(tokens: &mut Tokens, theme: Theme, hc: bool, source: u32) {
    let light = theme == Theme::Light;
    let (l0, c0, h) = rgb_lch(source);
    // One point of headroom: 8-bit rounding can move chroma by under one.
    let chroma = c0.min(ACCENT_CHROMA_CAP - 1.0);
    let need = if hc { [4.5, 4.5, 4.5] } else { [4.5, 3.0, 3.0] };
    let passes =
        |c: u32| (0..3).all(|i| crate::tokens::contrast(c, tokens.surface[i + 1]) >= need[i]);
    let step = if light { -0.5 } else { 0.5 };
    let (mut l, mut best, mut best_score) = (l0, lch_rgb_fit(l0, chroma, h), 0.0_f64);
    loop {
        let candidate = lch_rgb_fit(l, chroma, h);
        let score = (0..3)
            .map(|i| crate::tokens::contrast(candidate, tokens.surface[i + 1]) / need[i])
            .fold(f64::MAX, f64::min);
        if score > best_score {
            (best, best_score) = (candidate, score);
        }
        if passes(candidate) || !(0.0..=100.0).contains(&(l + step)) {
            break;
        }
        l += step;
    }
    let accent = best;
    let (al, ac, ah) = rgb_lch(accent);
    let ac = ac.min(chroma);
    let hover_l = if light { al - 5.0 } else { al + 6.0 };
    tokens.accent = accent;
    tokens.selection_edge = accent;
    tokens.focus_ring = accent;
    tokens.accent_hover = lch_rgb_fit(hover_l.clamp(0.0, 100.0), ac, ah);
    let percent = if hc {
        20
    } else if light {
        12
    } else {
        16
    };
    tokens.accent_subtle = blend(tokens.surface[1], accent, percent);
    tokens.selection_fill = tokens.accent_subtle;
}
