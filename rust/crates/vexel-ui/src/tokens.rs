//! Theme and design-system values. Colours are generated in `theme.rs`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

/// Motion constants (design-direction section 4.5). Easings are cubic-bezier
/// control points `[x1, y1, x2, y2]`; durations are milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub instant_ms: u16,
    pub fast_ms: u16,
    pub base_ms: u16,
    pub slow_ms: u16,
    /// Exit duration as a percentage of the entry duration.
    pub exit_percent: u16,
    pub ease_out: [f32; 4],
    pub standard: [f32; 4],
    pub ease_in: [f32; 4],
}

// The control points are finite constants, so equality is reflexive.
impl Eq for Motion {}

impl Motion {
    pub const STANDARD: Motion = Motion {
        instant_ms: 80,
        fast_ms: 120,
        base_ms: 180,
        slow_ms: 240,
        exit_percent: 75,
        ease_out: [0.2, 0.0, 0.0, 1.0],
        standard: [0.3, 0.0, 0.2, 1.0],
        ease_in: [0.4, 0.0, 1.0, 1.0],
    };
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
    /// Elevation levels 0 to 5 (window, panel, raised, control, hover, pressed).
    pub surface: [u32; 6],
    /// Third text emphasis level (metadata, placeholders); same as `text_tertiary`.
    pub text_low: u32,
    pub accent_hover: u32,
    /// Accent over level 1 at 12 to 20 percent; same as `selection_fill`.
    pub accent_subtle: u32,
    pub spacing_2: u32,
    pub radius_card: u32,
    pub motion: Motion,
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
    pub title_pt: u16,
    pub heading_pt: u16,
    pub label_pt: u16,
    pub meta_pt: u16,
    pub numeric_pt: u16,
    /// Numeric role uses tabular (monospaced digit) figures.
    pub numeric_tabular: bool,
    pub weight_regular: u16,
    pub weight_medium: u16,
    pub weight_semibold: u16,
    /// Line height in percent of the font size.
    pub line_height_percent: u16,
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
    crate::theme::build(theme, high_contrast)
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
        surface: [0; 6],
        text_low: 0,
        accent_hover: 0,
        accent_subtle: 0,
        spacing_2: 0,
        radius_card: 0,
        motion: Motion::STANDARD,
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

/// WCAG 2.x contrast ratio between two `0xRRGGBBAA` colours (alpha ignored).
pub(crate) fn contrast(a: u32, b: u32) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

pub fn theme_tokens_accented(theme: Theme, high_contrast: bool, system_accent_srgb: u32) -> Tokens {
    let mut tokens = theme_tokens(theme, high_contrast);
    if system_accent_srgb == 0 || system_accent_srgb & 255 == 0 {
        return tokens;
    }
    crate::theme::adapt_accent(&mut tokens, theme, high_contrast, system_accent_srgb);
    tokens
}

pub const fn type_scale() -> TypeScale {
    TypeScale {
        document_title_pt: 15,
        panel_heading_pt: 13,
        section_heading_pt: 13,
        body_pt: 13,
        secondary_pt: 12,
        metadata_pt: 12,
        geometry_pt: 12,
        empty_heading_pt: 20,
        geometry_monospaced: true,
        title_semibold: true,
        section_semibold: true,
        empty_heading_semibold: true,
        title_pt: 15,
        heading_pt: 13,
        label_pt: 12,
        meta_pt: 12,
        numeric_pt: 12,
        numeric_tabular: true,
        weight_regular: 400,
        weight_medium: 500,
        weight_semibold: 600,
        line_height_percent: 135,
    }
}

pub const fn design_metrics() -> Metrics {
    Metrics {
        toolbar_height: 44,
        context_height: 32,
        tool_rail_width: 44,
        structure_width: 232,
        inspector_width: 296,
        status_height: 24,
        tree_row_height: 28,
        section_header_height: 28,
        field_height: 28,
        icon_button_target: 32,
        icon_button_glyph: 22,
        control_radius: 6,
    }
}
