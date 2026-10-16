//! Deterministic theme and design-system values ported from ui_core.c.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tokens {
    pub window: u32,
    pub panel: u32,
    pub panel_raised: u32,
    pub panel_hover: u32,
    pub pressed_fill: u32,
    pub canvas_well: u32,
    pub text_primary: u32,
    pub text_secondary: u32,
    pub text_tertiary: u32,
    pub disabled_text: u32,
    pub separator: u32,
    pub control_edge: u32,
    pub accent: u32,
    pub selection_fill: u32,
    pub selection_edge: u32,
    pub focus_ring: u32,
    pub checker_a: u32,
    pub checker_b: u32,
    pub guide: u32,
    pub success: u32,
    pub warning: u32,
    pub error: u32,
    pub popover_shadow: u32,
    pub panel_alt: u32,
    pub canvas: u32,
    pub text: u32,
    pub muted: u32,
    pub border: u32,
    pub spacing: u32,
    pub radius: u32,
    pub font_px: u32,
    pub spacing_4: u32,
    pub spacing_8: u32,
    pub spacing_12: u32,
    pub spacing_16: u32,
    pub spacing_24: u32,
    pub spacing_32: u32,
    pub radius_small: u32,
    pub radius_control: u32,
    pub radius_large: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeScale {
    pub document_title_pt: u16,
    pub panel_heading_pt: u16,
    pub section_heading_pt: u16,
    pub body_pt: u16,
    pub secondary_pt: u16,
    pub metadata_pt: u16,
    pub geometry_pt: u16,
    pub empty_heading_pt: u16,
    pub geometry_monospaced: bool,
    pub title_semibold: bool,
    pub section_semibold: bool,
    pub empty_heading_semibold: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub toolbar_height: u16,
    pub context_height: u16,
    pub tool_rail_width: u16,
    pub structure_width: u16,
    pub inspector_width: u16,
    pub status_height: u16,
    pub tree_row_height: u16,
    pub section_header_height: u16,
    pub field_height: u16,
    pub icon_button_target: u16,
    pub icon_button_glyph: u16,
    pub control_radius: u16,
}

const fn color(r: u32, g: u32, b: u32) -> u32 {
    (r << 24) | (g << 16) | (b << 8) | 255
}

pub fn theme_tokens(theme: Theme, high_contrast: bool) -> Tokens {
    let light = theme == Theme::Light;
    let (secondary, tertiary, disabled, separator, edge) = if light {
        (
            if high_contrast {
                color(58, 70, 86)
            } else {
                color(82, 97, 113)
            },
            if high_contrast {
                color(70, 82, 99)
            } else {
                color(116, 128, 146)
            },
            if high_contrast {
                color(80, 91, 108)
            } else {
                color(137, 147, 162)
            },
            if high_contrast {
                color(151, 160, 172)
            } else {
                color(201, 208, 218)
            },
            if high_contrast {
                color(71, 82, 99)
            } else {
                color(124, 135, 151)
            },
        )
    } else {
        (
            if high_contrast {
                color(222, 228, 236)
            } else {
                color(185, 194, 207)
            },
            if high_contrast {
                color(207, 215, 225)
            } else {
                color(144, 153, 166)
            },
            if high_contrast {
                color(191, 200, 211)
            } else {
                color(124, 134, 148)
            },
            if high_contrast {
                color(89, 98, 111)
            } else {
                color(59, 65, 75)
            },
            if high_contrast {
                color(166, 177, 191)
            } else {
                color(116, 128, 145)
            },
        )
    };
    let (
        window,
        panel,
        raised,
        canvas,
        hover,
        pressed,
        primary,
        accent,
        selection,
        focus,
        checker_a,
        checker_b,
        guide,
        muted_success,
        muted_warning,
        muted_error,
        shadow,
    ) = if light {
        (
            color(242, 244, 247),
            color(255, 255, 255),
            color(237, 240, 244),
            color(231, 235, 240),
            color(245, 247, 250),
            color(220, 234, 255),
            color(31, 39, 50),
            color(24, 95, 197),
            color(220, 234, 255),
            if high_contrast {
                color(0, 73, 156)
            } else {
                color(0, 95, 204)
            },
            color(224, 228, 234),
            color(243, 245, 247),
            color(208, 0, 168),
            if high_contrast {
                color(13, 101, 65)
            } else {
                color(20, 122, 83)
            },
            if high_contrast {
                color(107, 67, 0)
            } else {
                color(138, 90, 0)
            },
            if high_contrast {
                color(153, 28, 18)
            } else {
                color(180, 35, 24)
            },
            0x00000024,
        )
    } else {
        (
            color(23, 26, 32),
            color(32, 36, 43),
            color(41, 46, 55),
            color(17, 21, 27),
            color(48, 54, 64),
            color(38, 60, 88),
            color(241, 244, 248),
            color(131, 180, 255),
            color(38, 60, 88),
            if high_contrast {
                color(190, 218, 255)
            } else {
                color(156, 197, 255)
            },
            color(41, 46, 54),
            color(32, 36, 43),
            color(255, 90, 200),
            color(102, 214, 164),
            color(244, 193, 93),
            color(255, 142, 134),
            0x0000006b,
        )
    };
    Tokens {
        window,
        panel,
        panel_raised: raised,
        panel_hover: hover,
        pressed_fill: pressed,
        canvas_well: canvas,
        text_primary: primary,
        text_secondary: secondary,
        text_tertiary: tertiary,
        disabled_text: disabled,
        separator,
        control_edge: edge,
        accent,
        selection_fill: selection,
        selection_edge: accent,
        focus_ring: focus,
        checker_a,
        checker_b,
        guide,
        success: muted_success,
        warning: muted_warning,
        error: muted_error,
        popover_shadow: shadow,
        panel_alt: raised,
        canvas,
        text: primary,
        muted: secondary,
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
        radius_large: 10,
    }
}

pub(crate) fn legacy_tokens(theme: Theme) -> Tokens {
    let (window, panel, raised, canvas, text, muted, accent, border, error) =
        if theme == Theme::Light {
            (
                color(242, 244, 247),
                color(255, 255, 255),
                color(232, 236, 242),
                color(250, 251, 253),
                color(30, 36, 45),
                color(105, 115, 128),
                color(45, 111, 219),
                color(211, 217, 225),
                color(180, 35, 24),
            )
        } else {
            (
                color(25, 28, 34),
                color(35, 39, 47),
                color(44, 49, 59),
                color(21, 24, 30),
                color(233, 237, 244),
                color(148, 157, 171),
                color(105, 159, 255),
                color(55, 61, 72),
                color(255, 142, 134),
            )
        };
    Tokens {
        window,
        panel,
        panel_raised: raised,
        panel_alt: raised,
        canvas_well: canvas,
        canvas,
        text_primary: text,
        text,
        text_secondary: muted,
        muted,
        separator: border,
        border,
        control_edge: border,
        accent,
        selection_edge: accent,
        selection_fill: raised,
        focus_ring: accent,
        error,
        spacing: 8,
        radius: 6,
        font_px: 13,
        panel_hover: 0,
        pressed_fill: 0,
        text_tertiary: 0,
        disabled_text: 0,
        checker_a: 0,
        checker_b: 0,
        guide: 0,
        success: 0,
        warning: 0,
        popover_shadow: 0,
        spacing_4: 0,
        spacing_8: 0,
        spacing_12: 0,
        spacing_16: 0,
        spacing_24: 0,
        spacing_32: 0,
        radius_small: 0,
        radius_control: 0,
        radius_large: 0,
    }
}

fn linear(c: u32, shift: u32) -> f64 {
    let v = ((c >> shift) & 255) as f64 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn luminance(c: u32) -> f64 {
    0.2126 * linear(c, 24) + 0.7152 * linear(c, 16) + 0.0722 * linear(c, 8)
}
fn contrast(a: u32, b: u32) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}
fn accent_candidate(source: u32, lighten: bool, percent: u32) -> u32 {
    let mut out = 0;
    for shift in [8, 16, 24] {
        let channel = (source >> shift) & 255;
        let target = if lighten { 255 } else { 0 };
        let adjusted = (channel * (100 - percent) + target * percent + 50) / 100;
        out |= adjusted << shift;
    }
    out | 255
}

pub fn theme_tokens_accented(theme: Theme, high_contrast: bool, system_accent_srgb: u32) -> Tokens {
    let mut tokens = theme_tokens(theme, high_contrast);
    if system_accent_srgb == 0 || system_accent_srgb & 255 == 0 {
        return tokens;
    }
    let required = if high_contrast { 4.5 } else { 3.0 };
    let lighten = theme == Theme::Dark;
    let mut best = system_accent_srgb | 255;
    let mut best_score = 0.0;
    for amount in 0..=100 {
        let candidate = accent_candidate(system_accent_srgb, lighten, amount);
        let score = contrast(candidate, tokens.panel).min(contrast(candidate, tokens.panel_raised));
        if score > best_score {
            best = candidate;
            best_score = score;
        }
        if score >= required {
            best = candidate;
            break;
        }
    }
    tokens.accent = best;
    tokens.selection_edge = best;
    tokens.focus_ring = best;
    let percent = if high_contrast { 20 } else { 14 };
    let mut blended = 255;
    for shift in [8, 16, 24] {
        let back = (tokens.panel >> shift) & 255;
        let front = (best >> shift) & 255;
        blended |= ((back * (100 - percent) + front * percent + 50) / 100) << shift;
    }
    tokens.selection_fill = blended;
    tokens
}

pub const fn type_scale() -> TypeScale {
    TypeScale {
        document_title_pt: 14,
        panel_heading_pt: 13,
        section_heading_pt: 12,
        body_pt: 13,
        secondary_pt: 12,
        metadata_pt: 11,
        geometry_pt: 12,
        empty_heading_pt: 22,
        geometry_monospaced: true,
        title_semibold: true,
        section_semibold: true,
        empty_heading_semibold: true,
    }
}
pub const fn design_metrics() -> Metrics {
    Metrics {
        toolbar_height: 40,
        context_height: 32,
        tool_rail_width: 44,
        structure_width: 232,
        inspector_width: 320,
        status_height: 24,
        tree_row_height: 28,
        section_header_height: 24,
        field_height: 30,
        icon_button_target: 32,
        icon_button_glyph: 22,
        control_radius: 6,
    }
}
