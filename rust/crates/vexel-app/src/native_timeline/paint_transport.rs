//! Transport bar (buttons and readouts) and the key bar (easing picker) of the
//! native timeline. The Native / SVG SMIL selector is painted by
//! `presentation::paint_selector` with the same segmented component.

use super::interaction::InteractionState;
use super::layout::{EASES, Hit, Readout, TimelineLayout};
use super::model::TimelineModel;
use super::naming;
use super::paint::{fit, has_target, rect};
use super::paint_bezier::paint_curve;
use crate::layout::paint::{mono, text, text_width};
use crate::ui_kit::{field, icon_button, segmented_cells};
use vexel_ui::{FontWeight, IconKind, Rect, WidgetCanvas};

fn readout(c: &mut WidgetCanvas<'_>, r: Rect, caption: &str, value: &str) {
    let t = c.tokens;
    field(c, r);
    text(
        c,
        caption,
        rect(r.x + 8, r.y, 40, r.height),
        t.text_secondary,
        12,
        FontWeight::Regular,
    );
    let x = r.x + 8 + text_width("Zoom", 12).max(14) + 10;
    mono(
        c,
        value,
        rect(x, r.y, (r.x + r.width).saturating_sub(x + 6), r.height),
        t.text_primary,
        12,
    );
}

pub(super) fn paint_transport(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    m: &TimelineModel,
    s: &InteractionState,
) {
    let t = c.tokens;
    if l.transport.height == 0 {
        return;
    }
    c.paint_rect(l.transport, t.panel);
    c.paint_rect(
        rect(
            l.transport.x,
            l.transport.y + l.transport.height - 1,
            l.transport.width,
            1,
        ),
        t.separator,
    );
    let controls = l.control_rects();
    for (hit, cell) in controls {
        let enabled = match hit {
            Hit::AddDrawingTrack => m.can_add_drawing_track && has_target(m, s),
            Hit::ImportDrawing => m.can_import_asset && has_target(m, s),
            _ => true,
        };
        let active = matches!(hit, Hit::Play) && m.playing
            || matches!(hit, Hit::Loop) && m.loop_enabled
            || matches!(hit, Hit::OnionBefore) && (m.onion_before > 0 || m.onion_after > 0);
        let hovered = enabled && s.hover == Some(hit);
        let pressed = hovered && s.pressed() == Some(hit);
        let icon = match hit {
            Hit::Play if m.playing => IconKind::PAUSE,
            Hit::Play => IconKind::PLAY,
            Hit::Loop => IconKind::LOOP,
            Hit::OnionBefore => IconKind::LAYERS,
            Hit::AddDrawingTrack => IconKind::PLUS,
            _ => IconKind::IMAGE,
        };
        icon_button(c, cell, icon, enabled, active, hovered, pressed);
    }
    let controls_right = controls
        .last()
        .map_or(l.transport.x, |(_, r)| r.x.saturating_add(r.width));
    let span = l
        .viewport
        .last_tick
        .saturating_sub(l.viewport.first_tick)
        .max(1);
    let full = m.duration_ticks.saturating_sub(1).max(1);
    let zoom = (u64::from(full) * 100 / u64::from(span)) as u32;
    for (which, caption, value) in [
        (
            Readout::Tick,
            "Tick",
            format!(
                "{} / {}",
                m.current_tick,
                m.duration_ticks.saturating_sub(1)
            ),
        ),
        (Readout::Rate, "Rate", format!("{}/s", m.ticks_per_second)),
        (Readout::Zoom, "Zoom", format!("{zoom}%")),
    ] {
        let r = l.readout(which);
        if r.x >= controls_right + 16 {
            readout(c, r, caption, &value);
        }
    }
}

/// Which easing cell (0..5, 5 = bezier) shows the key's curve.
fn active_cell(ease: &str) -> Option<usize> {
    if super::bezier::parse(ease).is_some() {
        return Some(5);
    }
    EASES.iter().position(|(id, _)| *id == ease)
}

pub(super) fn paint_keybar(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    m: &TimelineModel,
    s: &InteractionState,
) {
    if l.keybar.height == 0 {
        return;
    }
    let t = c.tokens;
    c.paint_rect(l.keybar, t.panel);
    c.paint_rect(rect(l.keybar.x, l.keybar.y, l.keybar.width, 1), t.separator);
    let key = m.selected_item();
    let Some((ti, _, tr, item)) =
        key.filter(|(_, _, tr, _)| matches!(tr.kind, super::NativeTrackKind::Property))
    else {
        return;
    };
    text(
        c,
        "Easing",
        rect(l.keybar.x + 12, l.keybar.y, 52, l.keybar.height),
        t.text_secondary,
        12,
        FontWeight::Regular,
    );
    let ease = item.ease.as_deref();
    let mut active = ease.and_then(active_cell);
    if s.bezier.open {
        active = Some(5);
    }
    let cells = l.ease_cells();
    let rects: Vec<Rect> = cells.iter().map(|(_, r)| *r).collect();
    let mut labels: Vec<&str> = EASES.iter().map(|(_, label)| *label).collect();
    labels.push("Bezier");
    let hover = cells.iter().position(|(hit, _)| s.hover == Some(*hit));
    segmented_cells(c, &rects, &labels, active, hover, true);
    let last = rects.last().copied().unwrap_or_else(|| rect(0, 0, 0, 0));
    let preview = rect(last.x + last.width + 12, last.y, 52, last.height);
    if preview.x + preview.width + 8 > l.keybar.x + l.keybar.width {
        return;
    }
    field(c, preview);
    paint_curve(c, preview, ease.unwrap_or("linear"), 6);
    let x = preview.x + preview.width + 12;
    let w = (l.keybar.x + l.keybar.width).saturating_sub(x + 8);
    let info = format!(
        "{} - tick {} - {}",
        naming::ease_name(ease.unwrap_or("linear")),
        item.tick,
        naming::track_title(tr, ti)
    );
    text(
        c,
        &fit(&info, 12, w),
        rect(x, l.keybar.y, w, l.keybar.height),
        t.text_secondary,
        12,
        FontWeight::Regular,
    );
}
