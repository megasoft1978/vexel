//! Chrome painting: toolbar, tool rail, status bar and the MCP popover. All
//! colours come from `Tokens`; geometry comes from [`Layout`].

use super::{Button, Layout, Splitter};
use crate::model::{McpState, Mode, Model, Target, Tool};
use crate::motion::MotionFrame;
use vexel_ui::{
    ControlState, FontFamily, FontWeight, IconKind, Rect, Theme, WidgetCanvas, WidgetDrawSpec,
    WidgetKind, icon_centered, widget_paint,
};

/// Tooltip data (name, shortcut) for a hit target; the shell paints it on hover.
pub fn tooltip(target: Target) -> Option<(&'static str, &'static str)> {
    Some(match target {
        Target::Tool(Tool::Select) => ("Select", "V"),
        Target::Tool(Tool::Pen) => ("Pen", "P"),
        Target::Tool(Tool::Rectangle) => ("Rectangle", "R"),
        Target::Tool(Tool::Ellipse) => ("Ellipse", "E"),
        Target::Tool(Tool::Line) => ("Line", "L"),
        Target::Button(Button::Undo) => ("Undo", "Cmd+Z"),
        Target::Button(Button::Redo) => ("Redo", "Shift+Cmd+Z"),
        Target::Button(Button::Save) => ("Save", "Cmd+S"),
        Target::Button(Button::Export) => ("Export", "Shift+Cmd+E"),
        Target::Button(Button::ToggleInspector) => ("Toggle inspector", ""),
        Target::Button(Button::Mcp) => ("MCP activity", ""),
        Target::SectionHeader(i) => return super::ExtControl::from_index(i).map(|c| c.tip()),
        Target::NativeTimeline(hit) => match hit {
            crate::native_timeline::layout::Hit::Play => ("Play / Pause", "Space"),
            crate::native_timeline::layout::Hit::Loop => ("Loop mode", "once | loop | ping-pong"),
            crate::native_timeline::layout::Hit::OnionBefore => ("Onion skin", ""),
            crate::native_timeline::layout::Hit::AddDrawingTrack => ("Add drawing track", ""),
            crate::native_timeline::layout::Hit::ImportDrawing => ("Import SVG as drawing", ""),
            crate::native_timeline::layout::Hit::EaseCustom => ("Custom bezier easing", ""),
            _ => return None,
        },
        _ => return None,
    })
}

const TIP_H: u32 = 24;
const TIP_PAD: u32 = 8;

/// Tooltip label size from the (estimated) text widths.
pub(crate) fn tip_size(name: &str, shortcut: &str) -> (u32, u32) {
    let shortcut_w = if shortcut.is_empty() {
        0
    } else {
        text_width(shortcut, 12) + 10
    };
    (2 * TIP_PAD + text_width(name, 12) + shortcut_w, TIP_H)
}

/// Paints a rounded level-3 tooltip label (name plus shortcut) filling `r`.
pub fn paint_tip(c: &mut WidgetCanvas<'_>, r: Rect, name: &str, shortcut: &str) {
    let t = c.tokens;
    c.paint_round(r, 6, t.surface[3], t.control_edge, 1);
    let label = Rect {
        x: r.x + TIP_PAD,
        width: r.width - 2 * TIP_PAD,
        ..r
    };
    text(c, name, label, t.text_primary, 12, FontWeight::Regular);
    if !shortcut.is_empty() {
        let x = r.x + TIP_PAD + text_width(name, 12) + 10;
        let rect = Rect {
            x,
            width: (r.x + r.width).saturating_sub(x + TIP_PAD / 2),
            ..label
        };
        text(c, shortcut, rect, t.text_secondary, 12, FontWeight::Regular);
    }
}

/// Rough text width in points, for centring labels (CoreText gives no measure).
pub(crate) fn text_width(text: &str, size: u32) -> u32 {
    (text.chars().count() as u32 * size * 56 + 50) / 100
}

pub(crate) fn text(
    c: &mut WidgetCanvas<'_>,
    s: &str,
    bounds: Rect,
    color: u32,
    size: u32,
    weight: FontWeight,
) {
    let _ = c.paint_text(s, bounds, color, FontFamily::System, weight, size);
}

pub(crate) fn mono(c: &mut WidgetCanvas<'_>, s: &str, bounds: Rect, color: u32, size: u32) {
    let _ = c.paint_text(
        s,
        bounds,
        color,
        FontFamily::Monospaced,
        FontWeight::Regular,
        size,
    );
}

pub(crate) fn centred(
    c: &mut WidgetCanvas<'_>,
    s: &str,
    bounds: Rect,
    color: u32,
    size: u32,
    w: FontWeight,
) {
    let tw = text_width(s, size).min(bounds.width);
    let x = bounds.x + (bounds.width - tw) / 2;
    text(
        c,
        s,
        Rect {
            x,
            width: bounds.width - (x - bounds.x),
            ..bounds
        },
        color,
        size,
        w,
    );
}

fn glyph(c: &mut WidgetCanvas<'_>, rect: Rect, kind: IconKind, color: u32) {
    let g = Rect {
        x: rect.x + (rect.width.saturating_sub(16)) / 2,
        y: rect.y + (rect.height.saturating_sub(16)) / 2,
        width: 16,
        height: 16,
    };
    c.paint_icon_color(kind, g, color);
}

fn focus_ring(c: &mut WidgetCanvas<'_>, r: Rect, radius: u32) {
    let ring = c.tokens.focus_ring;
    let outer = Rect {
        x: r.x.saturating_sub(2),
        y: r.y.saturating_sub(2),
        width: r.width + 4,
        height: r.height + 4,
    };
    c.paint_round(outer, radius + 2, 0, ring, 2);
}

pub(crate) fn paint_focus(c: &mut WidgetCanvas<'_>, r: Rect, radius: u32) {
    focus_ring(c, r, radius);
}

/// Toolbar, tool rail and status bar. Panels (inspector, footer) and the
/// stage are drawn by the caller; the popover by [`draw_popover`].
pub fn draw_chrome(c: &mut WidgetCanvas<'_>, l: &Layout, model: &Model) {
    draw_chrome_with(c, l, model, None);
}

/// Paints the interpolated hover/press fill of an animated control, if it differs from rest.
pub(crate) fn motion_fill(
    c: &mut WidgetCanvas<'_>,
    motion: Option<&MotionFrame>,
    id: u32,
    selected: bool,
    rect: Rect,
    radius: u32,
) {
    let Some(frame) = motion else { return };
    let visual = frame.get(id);
    let rest = crate::motion::colors_for(&c.tokens, id, selected).rest_fill;
    if visual.fill != rest {
        c.paint_round(rect, radius, visual.fill, 0, 0);
    }
}

/// [`draw_chrome`] with the animated visuals of the UI micro-transitions.
pub fn draw_chrome_with(
    c: &mut WidgetCanvas<'_>,
    l: &Layout,
    model: &Model,
    motion: Option<&MotionFrame>,
) {
    let t = c.tokens;
    let state = &model.chrome.state;
    // Toolbar: level 2 over the level 0 window, no border needed.
    c.paint_rect(l.top, t.panel_raised);
    let (name, name_color) = match (state.open, state.name.as_deref()) {
        (true, name) => (name.unwrap_or("Untitled"), t.text_primary),
        (false, _) => ("No document open", t.text_secondary),
    };
    text(c, name, l.doc_label, name_color, 15, FontWeight::Semibold);
    if state.open && state.dirty {
        // Unsaved badge, placed after the (estimated) name width.
        let x = l.doc_label.x + text_width(name, 15).min(l.doc_label.width) + 10;
        let dot = Rect {
            x,
            y: l.doc_label.y + 12,
            width: 8,
            height: 8,
        };
        c.paint_round(dot, 4, t.accent, 0, 0);
    }

    // Segmented Draw | Animate with the thumb at the selected segment.
    c.paint_round(l.segmented, 8, t.panel, 0, 0);
    let thumb = if c.theme == Theme::Light {
        t.window
    } else {
        t.panel_hover
    };
    c.paint_round(l.thumb_target(model.mode), 6, thumb, 0, 0);
    for (mode, rect, label) in [
        (Mode::Draw, l.draw_mode, "Draw"),
        (Mode::Animate, l.animate_mode, "Animate"),
    ] {
        let on = model.mode == mode;
        if !on {
            let id = if mode == Mode::Draw {
                crate::motion::ID_DRAW
            } else {
                crate::motion::ID_ANIMATE
            };
            motion_fill(c, motion, id, false, rect, 6);
        }
        let (color, weight) = if on {
            (t.text_primary, FontWeight::Semibold)
        } else {
            (t.text_secondary, FontWeight::Regular)
        };
        centred(c, label, rect, color, 12, weight);
    }
    if model.ui.focus == Some(super::Focus::Segmented) {
        paint_focus(c, l.segmented, 8);
    }

    // Grouped icon buttons share one rounded container each.
    c.paint_round(l.history_group, 8, t.surface[3], 0, 0);
    c.paint_round(l.file_group, 8, t.surface[3], 0, 0);
    let items = [
        (IconKind::UNDO, Button::Undo),
        (IconKind::REDO, Button::Redo),
        (IconKind::SAVE, Button::Save),
        (IconKind::EXPORT, Button::Export),
    ];
    for (i, (kind, button)) in items.into_iter().enumerate() {
        let enabled = model.target_enabled(l, Target::Button(button));
        let hovered = enabled && toolbar_hover(c, motion, button, l.buttons[i]);
        let s = match (enabled, hovered) {
            (false, _) => ControlState::Disabled,
            (true, true) => ControlState::Hover,
            _ => ControlState::Rest,
        };
        icon_centered(c, l.buttons[i], kind, s, &t);
        button_ring(c, motion, model, button, l.buttons[i]);
    }
    let toggle = if model.ui.inspector_visible {
        ControlState::Selected
    } else {
        ControlState::Rest
    };
    if !model.target_enabled(l, Target::Button(Button::ToggleInspector)) {
        icon_centered(
            c,
            l.buttons[4],
            IconKind::SIDEBAR_RIGHT,
            ControlState::Disabled,
            &t,
        );
    } else {
        let hovered = toolbar_hover(c, motion, Button::ToggleInspector, l.buttons[4]);
        let state = if hovered && toggle == ControlState::Rest {
            ControlState::Hover
        } else {
            toggle
        };
        icon_centered(c, l.buttons[4], IconKind::SIDEBAR_RIGHT, state, &t);
    }
    button_ring(c, motion, model, Button::ToggleInspector, l.buttons[4]);

    // Tool rail: level 1, selected tool is accent.subtle with an accent glyph.
    c.paint_rect(l.rail, t.panel);
    c.paint_rect(l.tool_divider, t.separator);
    let kinds = [
        (Tool::Select, IconKind::SELECT),
        (Tool::Pen, IconKind::PEN),
        (Tool::Rectangle, IconKind::RECTANGLE),
        (Tool::Ellipse, IconKind::ELLIPSE),
        (Tool::Line, IconKind::LINE),
    ];
    for (i, (tool, kind)) in kinds.into_iter().enumerate() {
        let rect = l.tools[i];
        if tool == model.tool {
            c.paint_round(rect, 6, t.selection_fill, 0, 0);
            icon_centered(c, rect, kind, ControlState::Selected, &t);
        } else {
            motion_fill(c, motion, crate::motion::tool_id(tool), false, rect, 6);
            glyph(c, rect, kind, t.text_secondary);
        }
    }

    // Splitters: a hover/drag strip over the window fill.
    for (rect, splitter) in [
        (l.inspector_splitter, Splitter::Inspector),
        (l.footer_splitter, Splitter::Footer),
    ] {
        if let Some(rect) = rect {
            let target = Target::Splitter(splitter);
            let id = crate::motion::control_id(target);
            motion_fill(c, motion, id, false, rect, 0);
        }
    }

    draw_status(c, l, model, motion);
}

/// Keyboard focus ring of a toolbar button: the animated ring when a motion
/// frame is given, the static one otherwise.
fn button_ring(
    c: &mut WidgetCanvas<'_>,
    motion: Option<&MotionFrame>,
    model: &Model,
    button: Button,
    rect: Rect,
) {
    match motion {
        Some(m) => crate::motion::paint::ring(
            c,
            rect,
            &m.get(crate::motion::control_id(Target::Button(button))),
        ),
        None if model.focus_button == Some(button) => focus_ring(c, rect, 6),
        None => {}
    }
}

/// Hover/press fill of a toolbar button over its group container; true when
/// the control is hovered or pressed.
fn toolbar_hover(
    c: &mut WidgetCanvas<'_>,
    motion: Option<&MotionFrame>,
    button: Button,
    rect: Rect,
) -> bool {
    let id = crate::motion::control_id(Target::Button(button));
    motion_fill(c, motion, id, false, rect, 6);
    motion.is_some_and(|m| m.get(id).hover > 0.5)
}

/// The context hint shown at the left of the status bar.
pub fn status_hint(model: &Model) -> &'static str {
    if !model.chrome.state.open {
        return "Open an SVG to begin (Cmd+O)";
    }
    match (model.mode, model.tool) {
        (Mode::Animate, _) => "Space plays, drag the ruler to scrub",
        (Mode::Draw, Tool::Select) => "Click a shape to select it",
        (Mode::Draw, Tool::Pen) => "Click to add points, double-click or Return to finish",
        (Mode::Draw, Tool::Rectangle) => "Drag on the canvas to draw a rectangle",
        (Mode::Draw, Tool::Ellipse) => "Drag on the canvas to draw an ellipse",
        (Mode::Draw, Tool::Line) => "Drag on the canvas to draw a line",
    }
}

fn draw_status(c: &mut WidgetCanvas<'_>, l: &Layout, model: &Model, motion: Option<&MotionFrame>) {
    let t = c.tokens;
    let state = &model.chrome.state;
    c.paint_rect(l.status, t.window);
    text(
        c,
        status_hint(model),
        l.status_row(l.status_hint),
        t.text_tertiary,
        12,
        FontWeight::Regular,
    );
    let (report, color) = match (&state.error, model.status) {
        (Some(error), _) => (format!("Error: {error}"), t.error),
        (None, s) if !s.is_empty() => (s.to_owned(), t.text_secondary),
        _ => ("Ready".to_owned(), t.text_secondary),
    };
    centred(
        c,
        &report,
        l.status_row(l.status_report),
        color,
        12,
        FontWeight::Regular,
    );
    let [zoom, size, save, mcp_label] = l.status_texts();
    if state.open {
        let dims = format!("{} x {}", state.size.0, state.size.1);
        let saved = if state.dirty { "Unsaved" } else { "Saved" };
        if zoom.width > 0 {
            let percent = crate::canvas::percent_label(state.zoom_percent);
            mono(c, &percent, zoom, t.text_secondary, 12);
            mono(c, &dims, size, t.text_secondary, 12);
        }
        mono(c, saved, save, t.text_secondary, 12);
    }
    // MCP indicator: dot plus word, never colour alone.
    let id = crate::motion::control_id(Target::Button(Button::Mcp));
    let bar_hit = Rect {
        y: l.status.y,
        height: l.status.height,
        ..l.mcp
    };
    motion_fill(c, motion, id, false, bar_hit, 4);
    let dot_color = match model.mcp {
        McpState::Unavailable => t.text_tertiary,
        McpState::Ready => t.success,
        McpState::Active => t.accent,
    };
    let dot = Rect {
        x: l.mcp.x + 2,
        y: l.status.y + (l.status.height - 8) / 2,
        width: 8,
        height: 8,
    };
    c.paint_round(dot, 4, dot_color, 0, 0);
    text(
        c,
        model.mcp.label(),
        mcp_label,
        t.text_secondary,
        12,
        FontWeight::Regular,
    );
}

/// The MCP popover (activity text and Connect); only while open.
pub fn draw_popover(c: &mut WidgetCanvas<'_>, l: &Layout, model: &Model) {
    if !model.ui.mcp_open {
        return;
    }
    let t = c.tokens;
    let p = l.mcp_popover;
    let shadow = Rect { y: p.y + 2, ..p };
    c.paint_round(shadow, 8, t.popover_shadow, 0, 0);
    c.paint_round(p, 8, t.pressed_fill, t.control_edge, 1);
    let row = |y: u32, h: u32| Rect {
        x: p.x + 16,
        y: p.y + y,
        width: p.width - 32,
        height: h,
    };
    text(
        c,
        "MCP activity",
        row(10, 24),
        t.text_primary,
        13,
        FontWeight::Semibold,
    );
    text(
        c,
        model.mcp.detail(),
        row(38, 22),
        t.text_primary,
        12,
        FontWeight::Regular,
    );
    text(
        c,
        model.status,
        row(62, 22),
        t.text_primary,
        12,
        FontWeight::Regular,
    );
    text(
        c,
        &crate::chrome::mcp_last_line(model.mcp_last),
        row(86, 20),
        t.text_secondary,
        11,
        FontWeight::Regular,
    );
    let mut s = WidgetDrawSpec::new(WidgetKind::ButtonSecondary, l.mcp_connect);
    s.label = Some("Connect");
    s.state = ControlState::Disabled;
    let _ = widget_paint(c, &s);
}
