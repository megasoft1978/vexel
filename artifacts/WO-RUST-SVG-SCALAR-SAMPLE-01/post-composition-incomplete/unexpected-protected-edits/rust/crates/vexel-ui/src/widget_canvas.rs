//! Bounded borrowed widget canvas and custom raster painter.
use crate::{ControlState, IconCache, IconKind, Rect, Theme, Tokens, theme_tokens, type_scale};

pub const MAX_CANVAS_AXIS: u32 = 16_384;
pub const MAX_TEXT_BYTES: usize = 1_048_576;
const FONT_PT: u32 = 13;
#[path = "widget_geometry.rs"]
mod geometry;
use geometry::{GeometryPreflight, focus_bounds, preflight, scaled_rect};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    Auto,
    Label,
    IconButton,
    ButtonPrimary,
    ButtonSecondary,
    ButtonGhost,
    Segmented,
    Toggle,
    Checkbox,
    TabBar,
    Tooltip,
    HudPill,
    Divider,
    PanelHeader,
    SectionHeader,
    TreeRow,
    EmptyState,
    InlineError,
    Toast,
    NumberField,
    ScrubLabel,
    Stepper,
    ColorField,
    Slider,
    Dropdown,
    TextField,
    Count,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    System,
    Monospaced,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Regular,
    Semibold,
}

/// Native text drawing hook. The closure receives borrowed UTF-8 and caller-owned pixels.
pub type TextDraw<'a> = dyn FnMut(&str, Rect, Theme, u32, u32, FontFamily, FontWeight, &mut [u8], u32, u32, usize) -> bool
    + 'a;
pub struct WidgetCanvas<'a> {
    pub rgba: &'a mut [u8],
    pub capacity: usize,
    pub stride: usize,
    pub width: u32,
    pub height: u32,
    pub backing_scale: u32,
    pub ui_scale: f64,
    pub text_scale_percent: u8,
    pub theme: Theme,
    pub increased_contrast: bool,
    pub tokens: Tokens,
    draw_text: Option<&'a mut TextDraw<'a>>,
    icons: IconCache,
    geometry: Option<GeometryPreflight>,
}
impl<'a> WidgetCanvas<'a> {
    pub fn new(
        rgba: &'a mut [u8],
        capacity: usize,
        width: u32,
        height: u32,
        stride: usize,
        backing_scale: u32,
        theme: Theme,
        increased_contrast: bool,
        draw_text: Option<&'a mut TextDraw<'a>>,
    ) -> Option<Self> {
        Self::new_scaled(
            rgba,
            capacity,
            width,
            height,
            stride,
            backing_scale,
            1.0,
            theme,
            increased_contrast,
            draw_text,
        )
    }
    pub fn new_scaled(
        rgba: &'a mut [u8],
        capacity: usize,
        width: u32,
        height: u32,
        stride: usize,
        backing_scale: u32,
        ui_scale: f64,
        theme: Theme,
        increased_contrast: bool,
        draw_text: Option<&'a mut TextDraw<'a>>,
    ) -> Option<Self> {
        let row = (width as usize).checked_mul(4)?;
        let needed = (height as usize).checked_mul(stride)?;
        if width == 0
            || height == 0
            || width > MAX_CANVAS_AXIS
            || height > MAX_CANVAS_AXIS
            || backing_scale == 0
            || backing_scale > 3
            || !ui_scale.is_finite()
            || !(1.0..=2.0).contains(&ui_scale)
            || stride < row
            || capacity < needed
            || rgba.len() < capacity
        {
            return None;
        }
        Some(Self {
            rgba,
            capacity,
            stride,
            width,
            height,
            backing_scale,
            ui_scale,
            text_scale_percent: 100,
            theme,
            increased_contrast,
            tokens: theme_tokens(theme, increased_contrast),
            draw_text,
            icons: IconCache::default(),
            geometry: None,
        })
    }
    pub fn set_text_scale(&mut self, scale: u8) -> bool {
        if !(100..=200).contains(&scale) || scale % 25 != 0 {
            return false;
        }
        self.text_scale_percent = scale;
        true
    }
    pub fn paint_text(
        &mut self,
        text: &str,
        bounds: Rect,
        color: u32,
        family: FontFamily,
        weight: FontWeight,
        font_points: u32,
    ) -> bool {
        if text.len() > MAX_TEXT_BYTES {
            return false;
        }
        self.text(text, bounds, color, family, weight, font_points)
    }
    pub fn paint_icon(&mut self, icon: IconKind, bounds: Rect, state: ControlState) -> bool {
        self.icon(icon, bounds, state, None)
    }
    pub fn paint_icon_color(&mut self, icon: IconKind, bounds: Rect, color: u32) -> bool {
        self.icon(icon, bounds, ControlState::Rest, Some(color))
    }
    pub fn paint_state_foreground(&self, state: ControlState) -> u32 {
        self.state_fg(state)
    }
    pub fn paint_tree_row(&mut self, spec: &WidgetDrawSpec<'_>) -> bool {
        let mut s = *spec;
        s.kind = WidgetKind::TreeRow;
        if !preflight(self, &s) {
            return false;
        }
        draw_tree_row(self, &s)
    }
    pub fn paint_empty_state(&mut self, spec: &WidgetDrawSpec<'_>) -> bool {
        let mut s = *spec;
        s.kind = WidgetKind::EmptyState;
        if !preflight(self, &s) {
            return false;
        }
        draw_empty(self, &s)
    }
    fn valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && self.width <= MAX_CANVAS_AXIS
            && self.height <= MAX_CANVAS_AXIS
            && self.backing_scale > 0
            && self.backing_scale <= 3
            && self.ui_scale.is_finite()
            && (1.0..=2.0).contains(&self.ui_scale)
            && (100..=200).contains(&self.text_scale_percent)
            && self.text_scale_percent % 25 == 0
            && self.stride >= self.width as usize * 4
            && (self.height as usize)
                .checked_mul(self.stride)
                .is_some_and(|n| self.capacity >= n && self.rgba.len() >= n)
    }
    fn scale(&self) -> f64 {
        self.backing_scale as f64 * self.ui_scale
    }
    fn pixel_rect(&self, r: Rect) -> Option<Rect> {
        if let Some(g) = &self.geometry {
            if !g.check(r, self.scale(), true) {
                return None;
            }
        }
        scaled_rect(r, self.scale())
    }
    fn pixel(&mut self, x: u32, y: u32, color: u32, cov: u8) {
        if self.geometry.is_some() {
            return;
        }
        if x >= self.width || y >= self.height {
            return;
        }
        let at = y as usize * self.stride + x as usize * 4;
        blend(&mut self.rgba[at..at + 4], color, cov)
    }
    pub fn paint_rect(&mut self, r: Rect, color: u32) {
        if !self.valid() {
            return;
        }
        if let Some(g) = &self.geometry {
            g.check(r, self.scale(), r.width > 0 && r.height > 0);
            return;
        }
        let Some(p) = self.pixel_rect(r) else { return };
        for y in p.y..p.y.saturating_add(p.height).min(self.height) {
            for x in p.x..p.x.saturating_add(p.width).min(self.width) {
                self.pixel(x, y, color, 255)
            }
        }
    }
    pub fn paint_round(&mut self, r: Rect, radius: u32, fill: u32, edge: u32, edge_points: u32) {
        if !self.valid() {
            return;
        }
        if let Some(g) = &self.geometry {
            g.check(r, self.scale(), r.width > 0 && r.height > 0);
            return;
        }
        let Some(p) = self.pixel_rect(r) else { return };
        let right = p.x.saturating_add(p.width).min(self.width);
        let bottom = p.y.saturating_add(p.height).min(self.height);
        let s = self.scale();
        let rad = (radius as f64 * s)
            .min(p.width as f64 / 2.0)
            .min(p.height as f64 / 2.0);
        let inset = edge_points as f64 * s;
        for y in p.y..bottom {
            for x in p.x..right {
                let (mut fc, mut ec) = (0u32, 0u32);
                for sy in 0..4 {
                    for sx in 0..4 {
                        let xx = x as f64 + (sx as f64 + 0.5) / 4.0;
                        let yy = y as f64 + (sy as f64 + 0.5) / 4.0;
                        if rounded(
                            xx,
                            yy,
                            p.x as f64,
                            p.y as f64,
                            (p.x + p.width) as f64,
                            (p.y + p.height) as f64,
                            rad,
                        ) {
                            if edge_points > 0
                                && inside_inner(xx, yy, p, inset, (rad - inset).max(0.0))
                            {
                                if fill != 0 {
                                    fc += 1
                                }
                            } else if edge_points > 0 {
                                ec += 1
                            } else if fill != 0 {
                                fc += 1
                            }
                        }
                    }
                }
                if fc > 0 {
                    self.pixel(x, y, fill, ((fc * 255 + 8) / 16) as u8)
                }
                if ec > 0 && edge != 0 {
                    self.pixel(x, y, edge, ((ec * 255 + 8) / 16) as u8)
                }
            }
        }
    }
    fn text(
        &mut self,
        text: &str,
        bounds: Rect,
        color: u32,
        family: FontFamily,
        weight: FontWeight,
        points: u32,
    ) -> bool {
        if !self.valid() {
            return false;
        }
        if let Some(g) = &self.geometry {
            let ok = g.check(bounds, self.scale(), !text.is_empty());
            let size =
                (points as f64 * self.scale() * self.text_scale_percent as f64 / 100.0).ceil();
            if !text.is_empty() && !(1.0..=256.0).contains(&size) {
                g.reject();
                return false;
            }
            return ok;
        }
        if text.is_empty() {
            return true;
        }
        let Some(bounds) = self.pixel_rect(bounds) else {
            return false;
        };
        let size =
            ((points as f64 * self.scale() * self.text_scale_percent as f64 / 100.0).ceil()) as u32;
        if !(1..=256).contains(&size) {
            return false;
        }
        if let Some(cb) = self.draw_text.as_deref_mut() {
            cb(
                text,
                bounds,
                self.theme,
                size,
                color,
                family,
                weight,
                self.rgba,
                self.width,
                self.height,
                self.stride,
            )
        } else {
            crate::scalar_text_draw(
                text,
                bounds,
                size,
                color,
                self.rgba,
                self.width,
                self.height,
                self.stride,
            )
        }
    }
    fn icon(
        &mut self,
        icon: IconKind,
        bounds: Rect,
        state: ControlState,
        color: Option<u32>,
    ) -> bool {
        if !self.valid() {
            return false;
        }
        if let Some(g) = &self.geometry {
            return g.check(bounds, self.scale(), true);
        }
        let Some(p) = self.pixel_rect(bounds) else {
            return false;
        };
        let scale = self.scale();
        let (icons, rgba) = (&mut self.icons, &mut *self.rgba);
        let result = if let Some(c) = color {
            icons.paint(
                icon,
                self.theme,
                scale,
                p,
                c,
                rgba,
                self.width,
                self.height,
                self.stride,
            )
        } else {
            icons.paint_state(
                icon,
                self.theme,
                self.increased_contrast,
                scale,
                p,
                state,
                &self.tokens,
                rgba,
                self.width,
                self.height,
                self.stride,
            )
        };
        result.is_ok()
    }
    fn state_fg(&self, s: ControlState) -> u32 {
        match s {
            ControlState::Disabled => self.tokens.disabled_text,
            ControlState::Invalid => self.tokens.error,
            _ => self.tokens.text_primary,
        }
    }
    fn state_surface(&self, s: ControlState, normal: u32) -> u32 {
        match s {
            ControlState::Hover => self.tokens.panel_hover,
            ControlState::Pressed => self.tokens.pressed_fill,
            ControlState::Selected => self.tokens.selection_fill,
            ControlState::Disabled => self.tokens.panel_raised,
            _ => normal,
        }
    }
}
fn rounded(x: f64, y: f64, l: f64, t: f64, r: f64, b: f64, rad: f64) -> bool {
    if x < l || x >= r || y < t || y >= b {
        return false;
    }
    let cx = if x < l + rad {
        l + rad
    } else if x >= r - rad {
        r - rad
    } else {
        x
    };
    let cy = if y < t + rad {
        t + rad
    } else if y >= b - rad {
        b - rad
    } else {
        y
    };
    (x - cx).powi(2) + (y - cy).powi(2) <= rad * rad
}
fn inside_inner(x: f64, y: f64, p: Rect, i: f64, r: f64) -> bool {
    let l = p.x as f64 + i;
    let t = p.y as f64 + i;
    let right = (p.x + p.width) as f64 - i;
    let bottom = (p.y + p.height) as f64 - i;
    right > l && bottom > t && rounded(x, y, l, t, right, bottom, r)
}
fn blend(dst: &mut [u8], color: u32, cov: u8) {
    let a = (((color & 255) * cov as u32) + 127) / 255;
    let inv = 255 - a;
    let da = dst[3] as u32;
    let oa = a + (da * inv + 127) / 255;
    if oa == 0 {
        return;
    }
    for (i, sh) in [24, 16, 8].into_iter().enumerate() {
        let src = (color >> sh) & 255;
        let n = src * a * 255 + dst[i] as u32 * da * inv;
        dst[i] = ((n + oa * 127) / (oa * 255)) as u8
    }
    dst[3] = oa as u8
}

#[derive(Debug, Clone, Copy)]
pub struct WidgetDrawSpec<'a> {
    pub kind: WidgetKind,
    pub state: ControlState,
    pub bounds: Rect,
    pub icon: IconKind,
    pub label: Option<&'a str>,
    pub value: Option<&'a str>,
    pub secondary: Option<&'a str>,
    pub unit: Option<&'a str>,
    pub numeric_value: f64,
    pub rgba: u32,
    pub font_size_pt: u8,
    pub font_family: FontFamily,
    pub font_weight: FontWeight,
    pub segment_index: u8,
    pub segment_count: u8,
    pub segment_labels: &'a [&'a str],
    pub indent: u8,
    pub selection_start: u16,
    pub selection_length: u16,
    pub cursor_byte_offset: u16,
    pub tone: crate::Tone,
    pub text_color: u32,
    pub checked: bool,
    pub expanded: bool,
    pub invalid: bool,
    pub placeholder: bool,
    pub focus_visible: bool,
    pub has_children: bool,
    pub visible: bool,
    pub locked: bool,
    pub show_row_actions: bool,
    pub has_transition: bool,
    pub transition_from: ControlState,
    pub transition_to: ControlState,
    pub transition_progress_permille: u16,
    pub transition_from_color: u32,
    pub transition_to_color: u32,
}
impl<'a> WidgetDrawSpec<'a> {
    pub fn new(kind: WidgetKind, bounds: Rect) -> Self {
        Self {
            kind,
            state: ControlState::Rest,
            bounds,
            icon: IconKind::None,
            label: None,
            value: None,
            secondary: None,
            unit: None,
            numeric_value: 0.0,
            rgba: 0,
            font_size_pt: 0,
            font_family: FontFamily::System,
            font_weight: FontWeight::Regular,
            segment_index: 0,
            segment_count: 0,
            segment_labels: &[],
            indent: 0,
            selection_start: 0,
            selection_length: 0,
            cursor_byte_offset: 0,
            tone: crate::Tone::Neutral,
            text_color: 0,
            checked: false,
            expanded: false,
            invalid: false,
            placeholder: false,
            focus_visible: false,
            has_children: false,
            visible: true,
            locked: false,
            show_row_actions: false,
            has_transition: false,
            transition_from: ControlState::Rest,
            transition_to: ControlState::Rest,
            transition_progress_permille: 0,
            transition_from_color: 0,
            transition_to_color: 0,
        }
    }
}

fn text(s: Option<&str>) -> &str {
    s.unwrap_or("")
}
fn inset(r: Rect, l: u32, t: u32, right: u32, b: u32) -> Rect {
    if l.saturating_add(right) >= r.width || t.saturating_add(b) >= r.height {
        return Rect {
            x: r.x,
            y: r.y,
            width: 0,
            height: 0,
        };
    }
    Rect {
        x: r.x + l,
        y: r.y + t,
        width: r.width - l - right,
        height: r.height - t - b,
    }
}
fn blend_color(a: u32, b: u32, p: u16) -> u32 {
    let t = p as f64 / 1000.0;
    let mut out = 0;
    for sh in [0, 8, 16, 24] {
        let x = (a >> sh) & 255;
        let y = (b >> sh) & 255;
        out |= ((x as f64 * (1.0 - t) + y as f64 * t).round() as u32) << sh
    }
    out
}

/// Paint one widget. All borrowed text and the target buffer remain caller-owned.
pub fn widget_paint(c: &mut WidgetCanvas<'_>, spec: &WidgetDrawSpec<'_>) -> bool {
    if !valid_spec(c, spec) {
        return false;
    }
    let mut adjusted = *spec;
    if adjusted.kind == WidgetKind::Toast && adjusted.bounds.width > 360 {
        adjusted.bounds.width = 360
    }
    if !preflight(c, &adjusted) {
        return false;
    }
    let result = paint_component(c, &adjusted);
    if result && (adjusted.focus_visible || adjusted.state == ControlState::Focus) {
        if let Some(r) = focus_bounds(&adjusted) {
            c.paint_round(r, 9, 0, c.tokens.focus_ring, 2)
        }
    }
    result
}
fn valid_spec(c: &WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    c.valid()
        && s.kind != WidgetKind::Auto
        && s.kind != WidgetKind::Count
        && s.bounds.width > 0
        && s.bounds.height > 0
        && s.bounds.width <= MAX_CANVAS_AXIS
        && s.bounds.height <= MAX_CANVAS_AXIS
        && s.segment_count <= 8
        && s.segment_labels.len() <= 8
        && ![s.label, s.value, s.secondary, s.unit]
            .iter()
            .flatten()
            .any(|v| v.len() > MAX_TEXT_BYTES)
        && !s.segment_labels.iter().any(|v| v.len() > MAX_TEXT_BYTES)
        && !s.unit.is_some_and(|v| v.len() > 8)
}
fn paint_component(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    let fg = c.state_fg(s.state);
    let label = text(s.label);
    match s.kind {
        WidgetKind::Label => c.text(
            label,
            s.bounds,
            if s.text_color == 0 { fg } else { s.text_color },
            s.font_family,
            s.font_weight,
            if s.font_size_pt == 0 {
                13
            } else {
                s.font_size_pt as u32
            },
        ),
        WidgetKind::IconButton
        | WidgetKind::ButtonGhost
        | WidgetKind::ButtonPrimary
        | WidgetKind::ButtonSecondary => draw_button(c, s, fg),
        WidgetKind::Toggle => draw_toggle(c, s),
        WidgetKind::Checkbox => draw_checkbox(c, s),
        WidgetKind::Segmented => draw_segmented(c, s),
        WidgetKind::TabBar => draw_tab(c, s),
        WidgetKind::Tooltip | WidgetKind::HudPill => {
            let tip = s.kind == WidgetKind::Tooltip;
            if tip {
                shadow(c, s.bounds)
            }
            c.paint_round(
                s.bounds,
                6,
                if tip {
                    c.tokens.panel
                } else {
                    c.tokens.panel_raised
                },
                c.tokens.separator,
                1,
            );
            c.text(
                label,
                inset(s.bounds, 10, 0, 10, 0),
                if tip { c.tokens.text_primary } else { fg },
                FontFamily::System,
                FontWeight::Regular,
                12,
            )
        }
        WidgetKind::Divider => {
            let mut r = s.bounds;
            if r.height > r.width {
                r.width = 1;
                if r.height > 16 {
                    r.y += (r.height - 16) / 2;
                    r.height = 16
                }
            } else {
                r.height = 1;
                if r.width > 16 {
                    r.x += (r.width - 16) / 2;
                    r.width = 16
                }
            }
            c.paint_rect(r, c.tokens.separator);
            true
        }
        WidgetKind::PanelHeader => c.text(
            label,
            inset(s.bounds, 12, 0, 8, 0),
            c.tokens.text_primary,
            FontFamily::System,
            FontWeight::Semibold,
            13,
        ),
        WidgetKind::SectionHeader => {
            c.icon(
                if s.expanded {
                    IconKind::CHEVRON_DOWN
                } else {
                    IconKind::CHEVRON_RIGHT
                },
                Rect {
                    x: s.bounds.x + 8,
                    y: s.bounds.y + 8,
                    width: 16,
                    height: 16,
                },
                s.state,
                None,
            ) && c.text(
                label,
                inset(s.bounds, 30, 0, 8, 0),
                fg,
                FontFamily::System,
                FontWeight::Semibold,
                12,
            )
        }
        WidgetKind::TreeRow => draw_tree_row(c, s),
        WidgetKind::EmptyState => draw_empty(c, s),
        WidgetKind::InlineError | WidgetKind::Toast => draw_message(c, s),
        WidgetKind::NumberField
        | WidgetKind::ScrubLabel
        | WidgetKind::Stepper
        | WidgetKind::ColorField
        | WidgetKind::Slider
        | WidgetKind::Dropdown
        | WidgetKind::TextField => draw_input(c, s),
        WidgetKind::Auto | WidgetKind::Count => false,
    }
}
pub const fn widget_paint_scratch_limit() -> usize {
    0
}

fn draw_button(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>, mut fg: u32) -> bool {
    let icon_only = s.kind == WidgetKind::IconButton;
    let primary = s.kind == WidgetKind::ButtonPrimary;
    let mut surface = c.state_surface(
        s.state,
        if primary {
            c.tokens.accent
        } else if s.kind == WidgetKind::ButtonSecondary {
            c.tokens.panel_raised
        } else {
            0
        },
    );
    if s.has_transition && (icon_only || s.kind == WidgetKind::ButtonGhost) {
        surface = blend_color(
            s.transition_from_color,
            s.transition_to_color,
            s.transition_progress_permille,
        )
    }
    if primary && s.state == ControlState::Pressed {
        surface = if c.theme == Theme::Light {
            0x124b97ff
        } else {
            c.tokens.selection_fill
        }
    }
    if primary && s.state != ControlState::Disabled {
        fg = if c.theme == Theme::Light {
            0xffffffff
        } else if s.state == ControlState::Pressed {
            c.tokens.text_primary
        } else {
            c.tokens.window
        }
    }
    if surface != 0 {
        c.paint_round(s.bounds, 6, surface, 0, 0)
    }
    if icon_only {
        let size = (if s.icon == IconKind::CLOSE { 14 } else { 22 })
            .min(s.bounds.width)
            .min(s.bounds.height);
        return c.icon(
            s.icon,
            Rect {
                x: s.bounds.x + (s.bounds.width - size) / 2,
                y: s.bounds.y + (s.bounds.height - size) / 2,
                width: size,
                height: size,
            },
            s.state,
            if s.icon == IconKind::CLOSE && s.state == ControlState::Rest {
                Some(c.tokens.text_secondary)
            } else {
                None
            },
        );
    }
    if s.icon != IconKind::None
        && !c.icon(
            s.icon,
            Rect {
                x: s.bounds.x + 8,
                y: s.bounds.y + s.bounds.height.saturating_sub(16) / 2,
                width: 16,
                height: 16,
            },
            s.state,
            None,
        )
    {
        return false;
    }
    c.text(
        text(s.label),
        inset(
            s.bounds,
            if s.icon == IconKind::None { 8 } else { 28 },
            0,
            8,
            0,
        ),
        fg,
        FontFamily::System,
        FontWeight::Regular,
        FONT_PT,
    )
}
fn draw_toggle(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    let x = s.bounds.x + 8;
    let y = s.bounds.y + s.bounds.height.saturating_sub(16) / 2;
    let track = if s.state == ControlState::Disabled {
        c.tokens.panel_raised
    } else if s.checked {
        c.tokens.accent
    } else {
        c.tokens.control_edge
    };
    c.paint_round(
        Rect {
            x,
            y,
            width: 28,
            height: 16,
        },
        8,
        track,
        if s.checked { 0 } else { c.tokens.control_edge },
        if s.checked { 0 } else { 1 },
    );
    let knob = if s.state == ControlState::Disabled {
        c.tokens.disabled_text
    } else if c.theme == Theme::Light {
        0xffffffff
    } else {
        c.tokens.text_primary
    };
    c.paint_round(
        Rect {
            x: x + if s.checked { 14 } else { 2 },
            y: y + 2,
            width: 12,
            height: 12,
        },
        6,
        knob,
        0,
        0,
    );
    c.text(
        text(s.label),
        Rect {
            x: x + 36,
            y: s.bounds.y,
            width: s.bounds.width.saturating_sub(44),
            height: s.bounds.height,
        },
        c.state_fg(s.state),
        FontFamily::System,
        FontWeight::Regular,
        13,
    )
}
fn draw_checkbox(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    let b = Rect {
        x: s.bounds.x + 6,
        y: s.bounds.y + s.bounds.height.saturating_sub(16) / 2,
        width: 16,
        height: 16,
    };
    let active = s.checked || s.state == ControlState::Selected || s.state == ControlState::Pressed;
    let fill = if s.state == ControlState::Disabled {
        c.tokens.panel_raised
    } else if active {
        c.tokens.accent
    } else {
        c.tokens.panel
    };
    c.paint_round(
        b,
        3,
        fill,
        if active { 0 } else { c.tokens.control_edge },
        if active { 0 } else { 1 },
    );
    if active {
        let Some(p) = c.pixel_rect(inset(b, 2, 2, 2, 2)) else {
            return false;
        };
        let sx = p.x + p.width / 4;
        let mx = p.x + p.width * 2 / 5;
        let ex = p.x + p.width * 3 / 4;
        let my = p.y + p.height * 3 / 5;
        let sy = p.y + p.height / 2;
        let ey = p.y + p.height / 4;
        let ink = if c.theme == Theme::Light {
            0xffffffff
        } else {
            c.tokens.window
        };
        line(c, sx as i64, sy as i64, mx as i64, my as i64, 3, ink);
        line(c, mx as i64, my as i64, ex as i64, ey as i64, 3, ink)
    }
    c.text(
        text(s.label),
        Rect {
            x: b.x + 24,
            y: s.bounds.y,
            width: s.bounds.width.saturating_sub(36),
            height: s.bounds.height,
        },
        c.state_fg(s.state),
        FontFamily::System,
        FontWeight::Regular,
        13,
    )
}
fn draw_segmented(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    let n = (if s.segment_count < 2 {
        3
    } else {
        s.segment_count as u32
    })
    .min(8);
    c.paint_round(s.bounds, 6, c.tokens.panel_raised, c.tokens.separator, 1);
    let sw = s.bounds.width / n;
    let selected = (s.segment_index as u32).min(n - 1);
    c.paint_round(
        Rect {
            x: s.bounds.x + sw * selected + 1,
            y: s.bounds.y + 1,
            width: if sw > 2 { sw - 2 } else { sw },
            height: s.bounds.height.saturating_sub(2).max(1),
        },
        5,
        c.tokens.panel,
        0,
        0,
    );
    for i in 1..n {
        c.paint_rect(
            Rect {
                x: s.bounds.x + sw * i,
                y: s.bounds.y + 6,
                width: 1,
                height: s.bounds.height.saturating_sub(12).max(1),
            },
            c.tokens.separator,
        )
    }
    for (i, t) in s.segment_labels.iter().take(n as usize).enumerate() {
        let mut left = s.bounds.x + sw * i as u32 + 4;
        let right = s.bounds.x + sw * (i as u32 + 1);
        let label_width = (t.len() as u32).saturating_mul(7);
        let available = right.saturating_sub(left).saturating_sub(4);
        if label_width < available {
            left += (available - label_width) / 2
        }
        let bounds = Rect {
            x: left,
            y: s.bounds.y + 1,
            width: right.saturating_sub(left).saturating_sub(2),
            height: s.bounds.height.saturating_sub(2).max(1),
        };
        if !c.text(
            t,
            bounds,
            c.state_fg(s.state),
            FontFamily::System,
            if i as u32 == selected {
                FontWeight::Semibold
            } else {
                FontWeight::Regular
            },
            12,
        ) {
            return false;
        }
    }
    if s.segment_labels.is_empty() {
        c.text(
            text(s.label),
            inset(s.bounds, 8, 0, 8, 0),
            c.state_fg(s.state),
            FontFamily::System,
            FontWeight::Regular,
            13,
        )
    } else {
        true
    }
}
fn draw_tab(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    let sw = s.bounds.width / 2;
    let idx = if s.segment_index < 2 {
        s.segment_index as u32
    } else {
        0
    };
    let r = Rect {
        x: s.bounds.x + idx * sw,
        y: s.bounds.y,
        width: sw,
        height: s.bounds.height,
    };
    c.paint_rect(
        Rect {
            x: r.x + 8,
            y: r.y + r.height.saturating_sub(2),
            width: r.width.saturating_sub(16).max(1),
            height: 2,
        },
        c.tokens.accent,
    );
    c.text(
        if idx == 0 {
            text(s.label)
        } else {
            text(s.secondary)
        },
        r,
        c.state_fg(s.state),
        FontFamily::System,
        if idx == s.segment_index as u32 {
            FontWeight::Semibold
        } else {
            FontWeight::Regular
        },
        13,
    )
}
fn shadow(c: &mut WidgetCanvas<'_>, b: Rect) {
    let a = [45, 70, 90];
    for (i, w) in a.into_iter().enumerate() {
        let col =
            (c.tokens.popover_shadow & 0xffffff00) | ((c.tokens.popover_shadow & 255) * w / 255);
        c.paint_round(
            Rect {
                x: b.x,
                y: b.y + i as u32 + 1,
                width: b.width,
                height: b.height,
            },
            6 + i as u32,
            col,
            0,
            0,
        )
    }
}
fn draw_input(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    let edge = if s.invalid || s.state == ControlState::Invalid {
        c.tokens.error
    } else {
        c.tokens.control_edge
    };
    let mut b = s.bounds;
    if s.kind == WidgetKind::Dropdown && s.expanded {
        b.height = 30
    }
    c.paint_round(
        b,
        6,
        c.state_surface(s.state, c.tokens.panel_raised),
        edge,
        1,
    );
    let label = text(s.label);
    let value = text(s.value);
    let unit = text(s.unit);
    match s.kind {
        WidgetKind::NumberField | WidgetKind::ScrubLabel => {
            let uw = if !unit.is_empty() { 36 } else { 8 };
            let vs = if s.kind == WidgetKind::ScrubLabel {
                104
            } else if !label.is_empty() {
                44
            } else {
                8
            };
            let reserve = uw
                + if s.kind == WidgetKind::NumberField {
                    20
                } else {
                    0
                };
            if s.kind == WidgetKind::ScrubLabel && s.icon != IconKind::None {
                let _ = c.icon(
                    s.icon,
                    Rect {
                        x: b.x + 6,
                        y: b.y + 7,
                        width: 14,
                        height: 14,
                    },
                    s.state,
                    None,
                );
            }
            if !label.is_empty() {
                let lr = if s.kind == WidgetKind::ScrubLabel {
                    Rect {
                        x: b.x + 24,
                        y: b.y,
                        width: 76,
                        height: b.height,
                    }
                } else {
                    Rect {
                        x: b.x + 8,
                        y: b.y,
                        width: 30,
                        height: b.height,
                    }
                };
                if !c.text(
                    label,
                    lr,
                    c.tokens.text_secondary,
                    FontFamily::System,
                    FontWeight::Regular,
                    12,
                ) && s.kind == WidgetKind::NumberField
                {
                    return false;
                }
            }
            let vr = Rect {
                x: b.x + vs,
                y: b.y,
                width: b.width.saturating_sub(vs + 8 + reserve),
                height: b.height,
            };
            if !c.text(
                value,
                vr,
                c.state_fg(s.state),
                FontFamily::Monospaced,
                FontWeight::Regular,
                12,
            ) {
                return false;
            }
            let ux = b.x
                + b.width
                    .saturating_sub(if s.kind == WidgetKind::NumberField {
                        54
                    } else {
                        34
                    });
            if !unit.is_empty()
                && !c.text(
                    unit,
                    Rect {
                        x: ux,
                        y: b.y,
                        width: 28,
                        height: b.height,
                    },
                    c.tokens.text_secondary,
                    FontFamily::System,
                    FontWeight::Regular,
                    11,
                )
            {
                return false;
            }
            if s.kind == WidgetKind::NumberField {
                draw_chevron(
                    c,
                    Rect {
                        x: b.x + b.width.saturating_sub(16),
                        y: b.y + 4,
                        width: 10,
                        height: 8,
                    },
                    true,
                    c.state_fg(s.state),
                );
                draw_chevron(
                    c,
                    Rect {
                        x: b.x + b.width.saturating_sub(16),
                        y: b.y + 15,
                        width: 10,
                        height: 8,
                    },
                    false,
                    c.state_fg(s.state),
                )
            }
            true
        }
        WidgetKind::Stepper => {
            let w = b.width / 2;
            c.paint_round(
                Rect {
                    x: b.x,
                    y: b.y,
                    width: w,
                    height: b.height,
                },
                5,
                c.state_surface(s.state, c.tokens.panel_raised),
                c.tokens.control_edge,
                1,
            );
            c.paint_round(
                Rect {
                    x: b.x + w,
                    y: b.y,
                    width: b.width - w,
                    height: b.height,
                },
                5,
                c.state_surface(s.state, c.tokens.panel_raised),
                c.tokens.control_edge,
                1,
            );
            draw_chevron(
                c,
                Rect {
                    x: b.x + w.saturating_sub(16) / 2,
                    y: b.y + b.height.saturating_sub(16) / 2,
                    width: 16,
                    height: 16,
                },
                true,
                c.state_fg(s.state),
            );
            draw_chevron(
                c,
                Rect {
                    x: b.x + w + (b.width - w).saturating_sub(16) / 2,
                    y: b.y + b.height.saturating_sub(16) / 2,
                    width: 16,
                    height: 16,
                },
                false,
                c.state_fg(s.state),
            );
            true
        }
        WidgetKind::ColorField => {
            let named = !label.is_empty();
            if named
                && !c.text(
                    label,
                    Rect {
                        x: b.x + 8,
                        y: b.y,
                        width: 38,
                        height: b.height,
                    },
                    c.tokens.text_secondary,
                    FontFamily::System,
                    FontWeight::Regular,
                    12,
                )
            {
                return false;
            }
            let sx = b.x + if named { 50 } else { 4 };
            c.paint_round(
                Rect {
                    x: sx,
                    y: b.y + 4,
                    width: 22,
                    height: 22,
                },
                4,
                c.tokens.checker_a,
                0,
                0,
            );
            c.paint_rect(
                Rect {
                    x: sx + 3,
                    y: b.y + 7,
                    width: 8,
                    height: 19,
                },
                c.tokens.checker_b,
            );
            c.paint_round(
                Rect {
                    x: sx + 11,
                    y: b.y + 7,
                    width: 8,
                    height: 19,
                },
                2,
                s.rgba,
                c.tokens.separator,
                1,
            );
            let _ = c.icon(
                IconKind::EYEDROPPER,
                Rect {
                    x: b.x + b.width.saturating_sub(24),
                    y: b.y + 7,
                    width: 16,
                    height: 16,
                },
                s.state,
                None,
            );
            c.text(
                value,
                inset(b, if named { 78 } else { 34 }, 0, 28, 0),
                c.state_fg(s.state),
                FontFamily::System,
                FontWeight::Regular,
                12,
            )
        }
        WidgetKind::Slider => {
            let tw = b.width.saturating_sub(64).max(1);
            let track = Rect {
                x: b.x + 8,
                y: b.y + (b.height / 2).saturating_sub(1),
                width: tw,
                height: 3,
            };
            c.paint_round(track, 2, c.tokens.separator, 0, 0);
            let n = if s.numeric_value.is_finite() {
                s.numeric_value.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let fw = (tw as f64 * n).round() as u32;
            if fw > 0 {
                c.paint_round(Rect { width: fw, ..track }, 2, c.tokens.accent, 0, 0)
            }
            let tx = track.x + fw;
            c.paint_round(
                Rect {
                    x: tx.saturating_sub(6),
                    y: b.y + (b.height / 2).saturating_sub(6),
                    width: 12,
                    height: 12,
                },
                6,
                if s.state == ControlState::Disabled {
                    c.tokens.disabled_text
                } else {
                    c.tokens.panel
                },
                c.tokens.accent,
                1,
            );
            c.text(
                value,
                inset(b, if b.width > 56 { b.width - 48 } else { 8 }, 0, 8, 0),
                c.state_fg(s.state),
                FontFamily::System,
                FontWeight::Regular,
                11,
            )
        }
        WidgetKind::Dropdown => {
            let labelr = inset(b, 8, 0, if !value.is_empty() { b.width / 2 } else { 24 }, 0);
            if !c.text(
                label,
                labelr,
                c.tokens.text_secondary,
                FontFamily::System,
                FontWeight::Regular,
                12,
            ) || !c.text(
                value,
                inset(b, b.width / 2, 0, 25, 0),
                c.state_fg(s.state),
                FontFamily::System,
                FontWeight::Regular,
                12,
            ) {
                return false;
            }
            let _ = c.icon(
                IconKind::CHEVRON_DOWN,
                Rect {
                    x: b.x + b.width.saturating_sub(20),
                    y: b.y + b.height.saturating_sub(16) / 2,
                    width: 16,
                    height: 16,
                },
                s.state,
                None,
            );
            if s.expanded && s.bounds.height > 34 {
                let p = Rect {
                    x: s.bounds.x,
                    y: s.bounds.y + 32,
                    width: s.bounds.width,
                    height: s.bounds.height.saturating_sub(32),
                };
                shadow(c, p);
                c.paint_round(p, 6, c.tokens.panel, c.tokens.separator, 1);
                let items = ["Auto", "Pixels", "Percent"];
                let h = p.height / 3;
                for (i, t) in items.iter().enumerate() {
                    let item = Rect {
                        x: p.x + 2,
                        y: p.y + i as u32 * h + 1,
                        width: p.width.saturating_sub(4),
                        height: h,
                    };
                    if i == s.segment_index as usize {
                        c.paint_round(item, 4, c.tokens.selection_fill, 0, 0)
                    }
                    if !c.text(
                        t,
                        inset(item, 8, 0, 8, 0),
                        c.tokens.text_primary,
                        FontFamily::System,
                        if i == s.segment_index as usize {
                            FontWeight::Semibold
                        } else {
                            FontWeight::Regular
                        },
                        12,
                    ) {
                        return false;
                    }
                }
            }
            true
        }
        WidgetKind::TextField => {
            let named = !label.is_empty();
            let has_icon = s.icon != IconKind::None;
            if has_icon
                && !c.icon(
                    s.icon,
                    Rect {
                        x: b.x + 6,
                        y: b.y + 6,
                        width: 18,
                        height: 18,
                    },
                    s.state,
                    None,
                )
            {
                return false;
            }
            if named
                && !c.text(
                    label,
                    inset(b, 8, 0, b.width / 2, 0),
                    c.tokens.text_secondary,
                    FontFamily::System,
                    FontWeight::Regular,
                    12,
                )
            {
                return false;
            }
            let tr = if named {
                inset(b, b.width / 2, 0, 8, 0)
            } else if has_icon {
                inset(b, 30, 0, 8, 0)
            } else {
                inset(b, 8, 0, 8, 0)
            };
            if s.selection_length > 0 {
                c.paint_round(
                    Rect {
                        x: b.x + if has_icon { 30 } else { 8 } + s.selection_start as u32 * 7,
                        y: b.y + 5,
                        width: s.selection_length as u32 * 7,
                        height: b.height.saturating_sub(10).max(1),
                    },
                    2,
                    c.tokens.selection_fill,
                    0,
                    0,
                )
            }
            if !c.text(
                value,
                tr,
                if s.placeholder {
                    c.tokens.text_tertiary
                } else {
                    c.state_fg(s.state)
                },
                FontFamily::System,
                FontWeight::Regular,
                13,
            ) {
                return false;
            }
            if s.state == ControlState::Focus && (s.cursor_byte_offset as usize) <= value.len() {
                c.paint_rect(
                    Rect {
                        x: b.x + if has_icon { 30 } else { 8 } + s.cursor_byte_offset as u32 * 7,
                        y: b.y + 7,
                        width: 1,
                        height: b.height.saturating_sub(14).max(1),
                    },
                    c.tokens.text_primary,
                )
            }
            true
        }
        _ => false,
    }
}
fn line(c: &mut WidgetCanvas<'_>, x0: i64, y0: i64, x1: i64, y1: i64, thickness: u32, color: u32) {
    if c.geometry.is_some() {
        return;
    }
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs());
    let radius = (thickness / 2) as i64;
    for i in 0..=steps {
        let t = if steps == 0 {
            0.0
        } else {
            i as f64 / steps as f64
        };
        let x = (x0 as f64 + dx as f64 * t).round() as i64;
        let y = (y0 as f64 + dy as f64 * t).round() as i64;
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                if x + ox >= 0 && y + oy >= 0 && x + ox < c.width as i64 && y + oy < c.height as i64
                {
                    c.pixel((x + ox) as u32, (y + oy) as u32, color, 255)
                }
            }
        }
    }
}
fn draw_chevron(c: &mut WidgetCanvas<'_>, b: Rect, up: bool, color: u32) {
    let Some(p) = c.pixel_rect(b) else { return };
    let cx = (p.x + p.width / 2) as i64;
    let cy = (p.y + p.height / 2) as i64;
    let wing = (p.width / 3) as i64;
    let rise = (p.height / 4) as i64;
    let tip = if up { cy - rise } else { cy + rise };
    let side = if up { cy + rise } else { cy - rise };
    let thickness = (2.0 * c.scale()).ceil() as u32;
    line(c, cx - wing, side, cx, tip, thickness, color);
    line(c, cx, tip, cx + wing, side, thickness, color)
}
fn draw_message(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    let toast = s.kind == WidgetKind::Toast;
    if toast {
        shadow(c, s.bounds)
    }
    let surface = if s.state == ControlState::Disabled {
        c.tokens.panel_raised
    } else {
        c.tokens.panel
    };
    let tone = match s.tone {
        crate::Tone::Success => c.tokens.success,
        crate::Tone::Warning => c.tokens.warning,
        crate::Tone::Error => c.tokens.error,
        crate::Tone::Neutral => {
            if toast {
                c.tokens.accent
            } else {
                c.tokens.error
            }
        }
    };
    c.paint_round(s.bounds, 6, surface, tone, 1);
    if !c.icon(
        IconKind::CLOSE,
        Rect {
            x: s.bounds.x + 8,
            y: s.bounds.y + 8,
            width: 14,
            height: 14,
        },
        if toast {
            ControlState::Rest
        } else {
            ControlState::Invalid
        },
        None,
    ) {
        return false;
    }
    let label = Rect {
        x: s.bounds.x + 28,
        y: s.bounds.y + 1,
        width: s.bounds.width.saturating_sub(36),
        height: 17,
    };
    let detail = Rect {
        x: label.x,
        y: s.bounds.y + 18,
        width: label.width,
        height: 17,
    };
    c.text(
        text(s.label),
        label,
        tone,
        FontFamily::System,
        FontWeight::Semibold,
        11,
    ) && c.text(
        text(s.secondary),
        detail,
        c.tokens.text_secondary,
        FontFamily::System,
        FontWeight::Regular,
        10,
    )
}

// C clamps guide count to six; the content origin uses the full u8 indent.
fn draw_tree_row(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    let mut bg = match s.state {
        ControlState::Selected => c.tokens.selection_fill,
        ControlState::Hover | ControlState::Pressed => c.tokens.panel_hover,
        _ => 0,
    };
    if s.has_transition {
        bg = blend_color(
            s.transition_from_color,
            s.transition_to_color,
            s.transition_progress_permille,
        )
    }
    if bg != 0 {
        c.paint_round(s.bounds, 4, bg, 0, 0)
    }
    let selected = if s.has_transition {
        if s.transition_from == ControlState::Selected {
            1.0 - s.transition_progress_permille as f64 / 1000.0
        } else if s.transition_to == ControlState::Selected {
            s.transition_progress_permille as f64 / 1000.0
        } else {
            0.0
        }
    } else if s.state == ControlState::Selected {
        1.0
    } else {
        0.0
    };
    if selected > 0.0 {
        let alpha = (selected * 255.0).round() as u32;
        c.paint_round(
            Rect {
                x: s.bounds.x,
                y: s.bounds.y + 4,
                width: 2,
                height: s.bounds.height.saturating_sub(8).max(1),
            },
            1,
            (c.tokens.accent & 0xffffff00) | alpha,
            0,
            0,
        )
    }
    let indent = (s.indent as u32).min(6);
    for d in 0..indent {
        c.paint_rect(
            Rect {
                x: s.bounds.x + 12 + d * 14,
                y: s.bounds.y,
                width: 1,
                height: s.bounds.height,
            },
            c.tokens.separator,
        )
    }
    let mut start = s.bounds.x + 4 + s.indent as u32 * 14;
    if s.has_children {
        let icon = if s.expanded {
            IconKind::CHEVRON_DOWN
        } else {
            IconKind::CHEVRON_RIGHT
        };
        if !c.icon(
            icon,
            Rect {
                x: start,
                y: s.bounds.y + 5,
                width: 16,
                height: 16,
            },
            s.state,
            if s.visible {
                None
            } else {
                Some(c.tokens.text_secondary)
            },
        ) {
            return false;
        }
    }
    start += 18;
    if s.icon != IconKind::None
        && !c.icon(
            s.icon,
            Rect {
                x: start,
                y: s.bounds.y + 5,
                width: 16,
                height: 16,
            },
            s.state,
            if s.visible {
                None
            } else {
                Some(c.tokens.text_secondary)
            },
        )
    {
        return false;
    }
    start += 20;
    let right = s.bounds.x + s.bounds.width;
    let text_right = right.saturating_sub(if s.show_row_actions { 40 } else { 8 });
    let label = text(s.label);
    let mut primary = label;
    let mut identifier = "";
    if let Some(i) = label.find(" #") {
        primary = &label[..i];
        identifier = &label[i + 1..]
    }
    let available = text_right.saturating_sub(start);
    let mut idw = if !identifier.is_empty() && primary.chars().count() as u32 * 7 + 32 < available {
        available
            .saturating_sub(primary.chars().count() as u32 * 7 + 8)
            .min(64)
    } else {
        0
    };
    if idw < 21 {
        idw = 0
    }
    let pw = if idw > 0 {
        available.saturating_sub(idw + 4)
    } else {
        available
    };
    let shown = middle_ellipsis(primary, (pw / 7) as usize);
    if !c.text(
        &shown,
        Rect {
            x: start,
            y: s.bounds.y,
            width: pw,
            height: s.bounds.height,
        },
        if s.visible {
            c.state_fg(s.state)
        } else {
            c.tokens.text_secondary
        },
        FontFamily::System,
        FontWeight::Regular,
        13,
    ) {
        return false;
    }
    if idw > 0
        && !c.text(
            &trailing_ellipsis(identifier, (idw / 7) as usize),
            Rect {
                x: text_right - idw,
                y: s.bounds.y,
                width: idw,
                height: s.bounds.height,
            },
            if s.visible {
                c.tokens.text_tertiary
            } else {
                c.tokens.text_secondary
            },
            FontFamily::System,
            FontWeight::Regular,
            12,
        )
    {
        return false;
    }
    if !s.show_row_actions {
        return true;
    }
    let x = s.bounds.x + s.bounds.width.saturating_sub(34);
    let st = if s.state == ControlState::Selected {
        ControlState::Selected
    } else {
        ControlState::Hover
    };
    c.icon(
        if s.visible {
            IconKind::EYE
        } else {
            IconKind::EYE_OFF
        },
        Rect {
            x,
            y: s.bounds.y + 4,
            width: 16,
            height: 16,
        },
        st,
        None,
    ) && c.icon(
        if s.locked {
            IconKind::LOCK
        } else {
            IconKind::UNLOCK
        },
        Rect {
            x: x + 16,
            y: s.bounds.y + 4,
            width: 16,
            height: 16,
        },
        st,
        None,
    )
}
fn middle_ellipsis(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max && s.len() < 64 {
        return s.to_owned();
    }
    if max == 0 {
        return String::new();
    }
    let mut left = max / 2;
    let mut right = max - left - 1;
    let mut a: usize = s.char_indices().nth(left).map_or(s.len(), |(i, _)| i);
    let mut start = s
        .char_indices()
        .nth(count.saturating_sub(right))
        .map_or(s.len(), |(i, _)| i);
    let mut bytes = s.len() - start;
    while a + 3 >= 64 && left > 0 {
        left -= 1;
        a = s.char_indices().nth(left).map_or(s.len(), |(i, _)| i)
    }
    while a + 3 + bytes >= 64 && right > 0 {
        right -= 1;
        start = s
            .char_indices()
            .nth(count.saturating_sub(right))
            .map_or(s.len(), |(i, _)| i);
        bytes = s.len() - start
    }
    format!("{}…{}", &s[..a], &s[start..])
}
fn trailing_ellipsis(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max && s.len() < 64 {
        return s.to_owned();
    }
    if max == 0 {
        return String::new();
    }
    let mut prefix = max - 1;
    let mut bytes = s.char_indices().nth(prefix).map_or(s.len(), |(i, _)| i);
    while bytes + 3 >= 64 && prefix > 0 {
        prefix -= 1;
        bytes = s.char_indices().nth(prefix).map_or(s.len(), |(i, _)| i)
    }
    format!("{}…", &s[..bytes])
}
fn draw_empty(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    let roomy = s.bounds.width >= 240 && s.bounds.height >= 112;
    let size = if roomy { 40 } else { 24 };
    if s.bounds.width < size {
        return false;
    }
    let bh = if roomy { 112 } else { 76 };
    let top = s.bounds.height.saturating_sub(bh) / 2;
    let it = top + if roomy { 8 } else { 3 };
    let ht = top + if roomy { 54 } else { 29 };
    let mt = top + if roomy { 86 } else { 53 };
    let icon = if s.icon == IconKind::None {
        IconKind::LAYERS
    } else {
        s.icon
    };
    if !c.icon(
        icon,
        Rect {
            x: s.bounds.x + (s.bounds.width - size) / 2,
            y: s.bounds.y + it,
            width: size,
            height: size,
        },
        ControlState::Rest,
        None,
    ) {
        return false;
    }
    let heading = Rect {
        x: s.bounds.x + 6,
        y: s.bounds.y + ht,
        width: s.bounds.width.saturating_sub(12),
        height: (if roomy {
            type_scale().empty_heading_pt as u32
        } else {
            16
        }) + 6,
    };
    if !c.text(
        text(s.label),
        heading,
        c.tokens.text_primary,
        FontFamily::System,
        FontWeight::Semibold,
        if roomy {
            type_scale().empty_heading_pt as u32
        } else {
            16
        },
    ) {
        return false;
    }
    c.text(
        text(s.secondary),
        Rect {
            x: s.bounds.x + 8,
            y: s.bounds.y + mt,
            width: s.bounds.width.saturating_sub(16),
            height: if roomy { 24 } else { 20 },
        },
        c.tokens.text_secondary,
        FontFamily::System,
        FontWeight::Regular,
        if roomy { 12 } else { 11 },
    )
}
