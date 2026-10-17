//! Timeline painting. Surfaces step lighter as they rise (transport row,
//! group rows), keys are filled diamonds, the playhead is one accent line
//! with a handle over the ruler. Every cell stays inside its band.

use super::Timeline;
use super::layout::{Hit, ROW_H, TimelineLayout};
use super::model::{Row, TimelineModel};
use crate::animation::strip;
use crate::layout::paint::{centred, mono, text};
use vexel_ui::motion::lerp_color;
use vexel_ui::{FontWeight, IconKind, Rect, Theme, Tokens, WidgetCanvas};

const WHITE: u32 = 0xFFFF_FFFF;

fn rc(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn icon(c: &mut WidgetCanvas<'_>, cell: Rect, kind: IconKind, color: u32) {
    let x = cell.x + cell.width.saturating_sub(16) / 2;
    let y = cell.y + cell.height.saturating_sub(16) / 2;
    c.paint_icon_color(kind, rc(x, y, 16, 16), color);
}

/// Paints the whole footer into `rect`.
pub fn paint(c: &mut WidgetCanvas<'_>, rect: Rect, tl: &Timeline) {
    let t = c.tokens;
    let layout = TimelineLayout::new(rect);
    let model = &tl.model;
    c.paint_rect(rect, t.panel);
    c.paint_rect(rc(rect.x, rect.y, rect.width, 1), t.separator);
    paint_transport(c, &layout, tl, &t);
    if let Some(message) = model.empty_state() {
        paint_empty(c, &layout, &message, &t);
        return;
    }
    paint_ruler(c, &layout, model, &t);
    paint_tracks(c, &layout, tl, &t);
    paint_playhead(c, &layout, model, &t);
}

fn paint_transport(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, tl: &Timeline, t: &Tokens) {
    let model = &tl.model;
    let live = model.empty_state().is_none();
    c.paint_rect(l.transport, t.panel_raised);
    if l.transport.height >= 32 && l.ruler.height > 0 {
        c.paint_rect(
            rc(
                l.rect.x,
                l.transport.y + l.transport.height - 1,
                l.rect.width,
                1,
            ),
            t.separator,
        );
    }
    let glyph_rest = if live {
        t.text_secondary
    } else {
        t.disabled_text
    };
    for (hit, cell) in l.buttons() {
        let hovered = live && tl.hover == Some(hit);
        let pressed = live && tl.pressed == Some(hit) && hovered;
        match hit {
            Hit::Play => {
                let fill = if !live {
                    t.surface[3]
                } else if pressed {
                    lerp_color(t.accent, t.window, 0.2)
                } else if hovered {
                    lerp_color(t.accent, t.text_primary, 0.12)
                } else {
                    t.accent
                };
                c.paint_round(cell, 14, fill, 0, 0);
                let glyph = if !live {
                    t.disabled_text
                } else if c.theme == Theme::Light {
                    WHITE
                } else {
                    t.window
                };
                let kind = if model.playing {
                    IconKind::PAUSE
                } else {
                    IconKind::PLAY
                };
                icon(c, cell, kind, glyph);
            }
            Hit::Ease | Hit::AddKey => {
                c.paint_round(cell, 6, 0, t.separator, 1);
                let label = if hit == Hit::Ease { "Ease" } else { "+ Key" };
                centred(
                    c,
                    label,
                    rc(cell.x, cell.y + 6, cell.width, 16),
                    t.disabled_text,
                    12,
                    FontWeight::Regular,
                );
            }
            Hit::Loop => {
                let on = model.looping && live;
                if on {
                    c.paint_round(cell, 6, t.selection_fill, 0, 0);
                } else if pressed {
                    c.paint_round(cell, 6, t.pressed_fill, 0, 0);
                } else if hovered {
                    c.paint_round(cell, 6, t.panel_hover, 0, 0);
                }
                let color = if on {
                    t.accent
                } else if hovered {
                    t.text_primary
                } else if live {
                    t.text_tertiary
                } else {
                    t.disabled_text
                };
                icon(c, cell, IconKind::LOOP, color);
            }
            _ => {
                if pressed {
                    c.paint_round(cell, 6, t.pressed_fill, 0, 0);
                } else if hovered {
                    c.paint_round(cell, 6, t.panel_hover, 0, 0);
                }
                let kind = match hit {
                    Hit::SkipStart => IconKind::SKIP_START,
                    Hit::StepBack => IconKind::STEP_BACK,
                    Hit::StepForward => IconKind::STEP_FORWARD,
                    _ => IconKind::SKIP_END,
                };
                let color = if hovered { t.text_primary } else { glyph_rest };
                icon(c, cell, kind, color);
            }
        }
    }
    if let Some(cell) = l.readout {
        let (now, total) = (
            strip::format_time(model.time),
            strip::format_time(model.loop_seconds),
        );
        let y = cell.y + 6;
        if live {
            mono(c, &now, rc(cell.x, y, 58, 16), t.text_primary, 12);
            mono(
                c,
                &format!("/ {total}"),
                rc(cell.x + 62, y, cell.width.saturating_sub(62), 16),
                t.text_tertiary,
                12,
            );
        } else {
            mono(
                c,
                "0:00.00 / 0:00.00",
                rc(cell.x, y, cell.width, 16),
                t.disabled_text,
                12,
            );
        }
    }
    if let Some(cell) = l.speed {
        centred(
            c,
            "1x",
            rc(cell.x, cell.y + 6, cell.width, 16),
            t.text_tertiary,
            12,
            FontWeight::Regular,
        );
    }
}

fn paint_empty(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, message: &str, t: &Tokens) {
    let top = l.ruler.y.min(l.tracks.y);
    let bottom = l.rect.y + l.rect.height;
    let area = rc(l.rect.x, top, l.rect.width, bottom - top);
    if area.height < 20 {
        return;
    }
    let tall = area.height >= 56;
    let y = area.y + (area.height - if tall { 40 } else { 20 }) / 2;
    centred(
        c,
        message,
        rc(area.x, y, area.width, 20),
        t.text_secondary,
        13,
        FontWeight::Semibold,
    );
    if tall {
        let hint = "Open an SVG with SMIL animation to see its tracks.";
        centred(
            c,
            hint,
            rc(area.x, y + 22, area.width, 18),
            t.text_tertiary,
            12,
            FontWeight::Regular,
        );
    }
}

fn paint_ruler(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, model: &TimelineModel, t: &Tokens) {
    if l.ruler.height == 0 {
        return;
    }
    let r = l.ruler;
    c.paint_rect(rc(r.x, r.y + r.height - 1, r.width, 1), t.separator);
    if l.name_w >= 64 {
        text(
            c,
            "Tracks",
            rc(r.x + 12, r.y + 4, l.name_w.saturating_sub(16), 16),
            t.text_tertiary,
            12,
            FontWeight::Regular,
        );
    }
    let base = r.y + r.height - 1;
    for tick in l.ticks(model.loop_seconds) {
        let x = tick.x.round() as u32;
        let h = if tick.major { 10 } else { 5 };
        let color = if tick.major {
            t.text_tertiary
        } else {
            t.control_edge
        };
        c.paint_rect(rc(x, base - h, 1, h), color);
        if let Some((label, cell)) = tick.label {
            // The playhead handle (12 px wide) hides the label it sits on.
            let head = l.playhead_x(model.time, model.loop_seconds).round() as u32;
            let covered = head + 8 > cell.x && head < cell.x + cell.width + 8;
            if !covered {
                mono(c, &label, cell, t.text_secondary, 12);
            }
        }
    }
    // Loop end cap.
    let end = l.lane_x1.round() as u32;
    c.paint_rect(rc(end, r.y + 6, 1, r.height - 7), t.text_tertiary);
}

fn paint_tracks(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, tl: &Timeline, t: &Tokens) {
    let model = &tl.model;
    let rows = tl.rows();
    let visible = l.visible_rows();
    let divider_x = l.rect.x + l.name_w;
    for slot in 0..visible.min(rows.len().saturating_sub(tl.scroll)) {
        let row = rows[tl.scroll + slot];
        let cell = l.row_rect(slot);
        let group = matches!(row, Row::Element(_));
        if group {
            c.paint_rect(cell, t.panel_raised);
        }
        c.paint_rect(rc(cell.x, cell.y + ROW_H - 1, cell.width, 1), t.separator);
        let name_y = cell.y + 5;
        match row {
            Row::Element(e) => {
                let kind = if tl.collapsed.get(e).copied().unwrap_or(false) {
                    IconKind::CHEVRON_RIGHT
                } else {
                    IconKind::CHEVRON_DOWN
                };
                icon(c, rc(cell.x + 6, cell.y, 20, ROW_H), kind, t.text_secondary);
                let w = l.name_w.saturating_sub(34);
                text(
                    c,
                    &model.elements[e].name,
                    rc(cell.x + 28, name_y, w, 18),
                    t.text_primary,
                    13,
                    FontWeight::Semibold,
                );
            }
            Row::Prop(e, p) => {
                let w = l.name_w.saturating_sub(44);
                text(
                    c,
                    &model.elements[e].props[p].name,
                    rc(cell.x + 36, name_y, w, 18),
                    t.text_secondary,
                    13,
                    FontWeight::Regular,
                );
            }
        }
        let keys = model.row_keys(row);
        let mid = cell.y + (ROW_H - 1) / 2;
        if let (Some(first), Some(last)) = (keys.first(), keys.last()) {
            let x0 = l.time_to_x(*first, model.loop_seconds).round() as u32;
            let x1 = l.time_to_x(*last, model.loop_seconds).round() as u32;
            if x1 > x0 {
                let fill = if group {
                    t.surface[4]
                } else {
                    t.selection_fill
                };
                c.paint_round(rc(x0, mid - 3, x1 - x0, 6), 3, fill, 0, 0);
            }
        }
        for (i, time) in keys.iter().enumerate() {
            let x = l.time_to_x(*time, model.loop_seconds).round() as u32;
            let key = |sel: &super::model::KeyRef| match row {
                Row::Element(e) => sel.element == e && sel.prop.is_none() && sel.index == i,
                Row::Prop(e, p) => sel.element == e && sel.prop == Some(p) && sel.index == i,
            };
            let selected = tl.selection.as_ref().is_some_and(key);
            let hovered = matches!(tl.hover, Some(Hit::Key(k)) if key(&k));
            let spot = rc(x.saturating_sub(8), mid.saturating_sub(8), 16, 16);
            let color = if selected {
                c.paint_round(
                    rc(x.saturating_sub(11), mid.saturating_sub(11), 22, 22),
                    6,
                    t.selection_fill,
                    t.accent,
                    1,
                );
                t.accent
            } else if hovered {
                t.text_primary
            } else if group {
                t.text_tertiary
            } else {
                t.text_secondary
            };
            c.paint_icon_color(IconKind::KEYFRAME_FILLED, spot, color);
        }
    }
    // Name column divider over ruler and rows.
    let top = if l.ruler.height > 0 {
        l.ruler.y
    } else {
        l.tracks.y
    };
    let bottom = l.tracks.y + l.tracks.height;
    c.paint_rect(rc(divider_x, top, 1, bottom - top), t.separator);
}

fn paint_playhead(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, model: &TimelineModel, t: &Tokens) {
    if l.ruler.height == 0 {
        return;
    }
    let x = l.playhead_x(model.time, model.loop_seconds).round() as u32;
    let bottom = l.tracks.y + l.tracks.height;
    c.paint_rect(
        rc(x - 1, l.ruler.y + 10, 2, bottom - l.ruler.y - 10),
        t.accent,
    );
    c.paint_round(rc(x - 6, l.ruler.y + 2, 12, 12), 4, t.accent, 0, 0);
}
