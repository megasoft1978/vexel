//! Custom native timeline painting using the editor's established tokens and
//! canvas. The active playhead is the single strong accent in the panel.

use super::interaction::{DragPreview, InteractionState};
use super::layout::{Hit, TRACK_HEIGHT, TimelineLayout};
use super::model::TimelineModel;
use crate::layout::paint::{centred, mono, text};
use vexel_ui::{FontWeight, IconKind, Rect, WidgetCanvas};

const PAD: u32 = 8;

fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Paints the panel at `panel`, clipping all generated geometry to that rect.
pub fn paint(
    canvas: &mut WidgetCanvas<'_>,
    panel: Rect,
    model: &TimelineModel,
    state: &InteractionState,
) {
    if panel.width == 0 || panel.height == 0 {
        return;
    }
    let layout = TimelineLayout::new(panel, model, state.viewport(model));
    let t = canvas.tokens;
    canvas.paint_rect(panel, t.panel);
    canvas.paint_rect(rect(panel.x, panel.y, panel.width, 1), t.separator);
    paint_transport(canvas, &layout, model, state);
    paint_ruler(canvas, &layout, model);
    paint_tracks(canvas, &layout, model, state);
    paint_playhead(canvas, &layout, model.current_tick);
    paint_bezier_editor(canvas, &layout, model, state);
    if state.focus && panel.width >= 3 && panel.height >= 2 {
        let x = panel.x.saturating_add(1);
        let y = panel.y.saturating_add(panel.height.saturating_sub(2));
        canvas.paint_rect(rect(x, y, panel.width.saturating_sub(2), 1), t.accent);
    }
}

fn paint_transport(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    m: &TimelineModel,
    s: &InteractionState,
) {
    let t = c.tokens;
    c.paint_rect(l.transport, t.panel_raised);
    if l.transport.height > 0 {
        c.paint_rect(
            rect(
                l.transport.x,
                l.transport.y + l.transport.height - 1,
                l.transport.width,
                1,
            ),
            t.separator,
        );
    }
    let y = l
        .transport
        .y
        .saturating_add((l.transport.height.saturating_sub(24)) / 2);
    let controls = l.control_rects();
    for (hit, cell) in controls {
        let enabled = match hit {
            Hit::AddDrawingTrack => m.can_add_drawing_track && has_target(m, s),
            Hit::ImportDrawing => m.can_import_asset && has_target(m, s),
            _ => true,
        };
        let active = matches!(hit, Hit::Loop) && m.loop_enabled
            || matches!(hit, Hit::OnionBefore) && (m.onion_before > 0 || m.onion_after > 0);
        let hovered = enabled && s.hover == Some(hit);
        let pressed = hovered && s.pressed() == Some(hit);
        if active {
            c.paint_round(cell, 6, t.selection_fill, 0, 0);
        } else if pressed {
            c.paint_round(cell, 6, t.pressed_fill, 0, 0);
        } else if hovered {
            c.paint_round(cell, 6, t.panel_hover, 0, 0);
        }
        let color = if !enabled {
            t.disabled_text
        } else if active {
            t.accent
        } else if hovered {
            t.text_primary
        } else {
            t.text_secondary
        };
        let icon = match hit {
            Hit::Play => {
                if m.playing {
                    IconKind::PAUSE
                } else {
                    IconKind::PLAY
                }
            }
            Hit::Loop => IconKind::LOOP,
            Hit::OnionBefore => IconKind::LAYERS,
            Hit::AddDrawingTrack => IconKind::PLUS,
            _ => IconKind::IMAGE,
        };
        c.paint_icon_color(icon, rect(cell.x + 6, cell.y + 6, 16, 16), color);
    }
    if m.selected_item()
        .is_some_and(|(_, _, track, _)| matches!(track.kind, super::NativeTrackKind::Property))
    {
        let labels = [
            ("linear", "Lin"),
            ("ease_in", "In"),
            ("ease_out", "Out"),
            ("ease_in_out", "I/O"),
            ("step", "Stp"),
        ];
        let start = l.transport.x.saturating_add(super::layout::EASE_START);
        if l.transport.width >= super::layout::CUSTOM_MIN_WIDTH {
            let cell = rect(start + 5 * 28, y, 26, 24);
            let custom = m
                .selected_item()
                .and_then(|(_, _, _, item)| item.ease.as_deref())
                .is_some_and(|e| super::bezier::parse(e).is_some());
            c.paint_round(cell, 4, t.panel_hover, t.separator, 1);
            if s.hover == Some(Hit::EaseCustom) || s.bezier.open {
                c.paint_round(cell, 4, t.pressed_fill, t.accent, 1);
            }
            centred(c, "Bez", cell, t.text_secondary, 13, FontWeight::Semibold);
            if custom {
                c.paint_rect(rect(cell.x + 4, cell.y + 21, 18, 1), t.accent);
            }
        }
        if l.transport.width >= 380 {
            for (i, (ease, glyph)) in labels.iter().enumerate() {
                let cell = rect(start + i as u32 * 28, y, 26, 24);
                c.paint_round(cell, 4, t.panel_hover, t.separator, 1);
                if s.hover == Some(Hit::Ease(ease)) {
                    c.paint_round(cell, 4, t.pressed_fill, t.accent, 1);
                }
                centred(c, glyph, cell, t.text_secondary, 13, FontWeight::Semibold);
                if let Some((_, _, _, item)) = m.selected_item() {
                    if item.ease.as_deref() == Some(*ease) {
                        c.paint_rect(rect(cell.x + 4, cell.y + 21, 18, 1), t.accent);
                    }
                }
            }
        }
    }
    if let Some((_, _, _, item)) = m.selected_item() {
        if let Some(ease) = item.ease.as_deref() {
            let x = l
                .transport
                .x
                .saturating_add(super::layout::EASE_START + 6 * 28 + 2);
            let w = 76;
            if x + w <= l.transport.x + l.transport.width.saturating_sub(116 + PAD) {
                mono(
                    c,
                    super::bezier::label(ease),
                    rect(x, y, w, 24.min(l.transport.height)),
                    t.text_secondary,
                    11,
                );
            }
        }
    }
    let tick_label = format!(
        "{} / {}  {}%",
        m.current_tick,
        m.duration_ticks.saturating_sub(1),
        (m.onion_opacity * 100.0).round() as u32,
    );
    let readout_w = 116.min(l.transport.width.saturating_sub(PAD));
    let readout_x = l
        .transport
        .x
        .saturating_add(l.transport.width.saturating_sub(readout_w + PAD));
    let controls_right = controls
        .last()
        .map_or(l.transport.x, |(_, r)| r.x.saturating_add(r.width));
    if readout_x >= controls_right {
        mono(
            c,
            &tick_label,
            rect(readout_x, y, readout_w, 24.min(l.transport.height)),
            t.text_primary,
            12,
        );
    }
}

fn paint_ruler(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, m: &TimelineModel) {
    if l.ruler.height == 0 {
        return;
    }
    let t = c.tokens;
    c.paint_rect(l.ruler, t.surface[1]);
    let v = l.viewport;
    let span = v.last_tick.saturating_sub(v.first_tick).max(1);
    let step = ruler_step(span, (l.lane_right - l.lane_left).max(1.0));
    let mut tick = v.first_tick;
    let mut count = 0usize;
    while tick <= v.last_tick && count < 512 {
        let x = l.lane_x(tick).round().max(0.0) as u32;
        if x >= l.panel.x && x < l.panel.x.saturating_add(l.panel.width) {
            c.paint_rect(
                rect(
                    x,
                    l.ruler.y + 14.min(l.ruler.height),
                    1,
                    5.min(l.ruler.height.saturating_sub(14)),
                ),
                t.separator,
            );
            let next = tick.saturating_add(step);
            let right = l.lane_x(next.min(v.last_tick)).round().max(0.0) as u32;
            let width = right
                .saturating_sub(x)
                .max(24)
                .min(l.panel.x.saturating_add(l.panel.width).saturating_sub(x));
            mono(
                c,
                &tick.to_string(),
                rect(
                    x.saturating_add(3),
                    l.ruler.y,
                    width.saturating_sub(3),
                    16.min(l.ruler.height),
                ),
                t.text_tertiary,
                10,
            );
        }
        let next = tick.saturating_add(step);
        if next <= tick {
            break;
        }
        tick = next;
        count += 1;
    }
    let label = format!("{} t/s", m.ticks_per_second);
    let w = 52.min(l.name_width.saturating_sub(PAD));
    if w > 0 {
        text(
            c,
            &label,
            rect(l.panel.x + PAD, l.ruler.y, w, l.ruler.height),
            t.text_tertiary,
            10,
            FontWeight::Regular,
        );
    }
}

fn ruler_step(span: u32, lane_width: f64) -> u32 {
    let desired = (f64::from(span) * 64.0 / lane_width.max(1.0)).ceil() as u32;
    let desired = desired.max(1);
    let magnitude = 10u32.saturating_pow(desired.ilog10());
    [1u32, 2, 5, 10]
        .into_iter()
        .map(|n| n.saturating_mul(magnitude))
        .find(|step| *step >= desired)
        .unwrap_or(u32::MAX)
}

fn paint_tracks(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    m: &TimelineModel,
    s: &InteractionState,
) {
    let t = c.tokens;
    let visible_rows = l.rows.height.div_ceil(TRACK_HEIGHT) as usize;
    for (index, track) in m.tracks.iter().enumerate().take(visible_rows) {
        let row = l.row_rect(index);
        if row.height == 0 {
            continue;
        }
        let selected = track.selected || s.selected_track == Some(index);
        c.paint_rect(row, if selected { t.panel_hover } else { t.panel });
        c.paint_rect(
            rect(row.x, row.y + row.height - 1, row.width, 1),
            t.separator,
        );
        if selected {
            c.paint_rect(
                rect(
                    row.x,
                    row.y + 4,
                    2.min(row.width),
                    row.height.saturating_sub(8),
                ),
                t.accent,
            );
        }
        if row.width >= 16 && row.height >= 16 {
            let visibility_x = row
                .x
                .saturating_add(l.name_width.saturating_sub(22))
                .min(row.x.saturating_add(row.width).saturating_sub(16));
            let visibility = rect(visibility_x, row.y + 4, 16, 16);
            c.paint_icon_color(
                if track.visible {
                    IconKind::EYE
                } else {
                    IconKind::EYE_OFF
                },
                visibility,
                if track.visible {
                    t.text_secondary
                } else {
                    t.text_tertiary
                },
            );
        }
        if row.width >= 48 {
            let duplicate = rect(row.x + row.width - 44, row.y + 6, 16, 16);
            let delete = rect(row.x + row.width - 22, row.y + 6, 16, 16);
            c.paint_icon_color(IconKind::DUPLICATE, duplicate, t.text_tertiary);
            c.paint_icon_color(IconKind::TRASH, delete, t.text_tertiary);
            if s.hover == Some(Hit::DuplicateTrack(index)) {
                c.paint_round(
                    rect(duplicate.x - 4, duplicate.y - 4, 24, 24),
                    4,
                    t.panel_hover,
                    0,
                    0,
                );
            }
            if s.hover == Some(Hit::DeleteTrack(index)) {
                c.paint_round(
                    rect(delete.x - 4, delete.y - 4, 24, 24),
                    4,
                    t.panel_hover,
                    0,
                    0,
                );
            }
        }
        let kind = if matches!(track.kind, super::NativeTrackKind::Drawing) {
            "Drawing"
        } else {
            "Property"
        };
        let label = format!("{}  -  {}", track.name, kind);
        let label_x = row
            .x
            .saturating_add(10)
            .min(row.x.saturating_add(row.width));
        let label_w = l
            .name_width
            .saturating_sub(34)
            .min(row.x.saturating_add(row.width).saturating_sub(label_x));
        text(
            c,
            &label,
            rect(label_x, row.y, label_w, row.height),
            if selected {
                t.text_primary
            } else {
                t.text_secondary
            },
            12,
            if selected {
                FontWeight::Semibold
            } else {
                FontWeight::Regular
            },
        );
        for (item_index, item) in track.items.iter().enumerate() {
            let Some(mut item_rect) = l.item_rect(m, index, item_index) else {
                continue;
            };
            item_rect.y = row
                .y
                .saturating_add((row.height.saturating_sub(item_rect.height)) / 2);
            item_rect.height = item_rect.height.min(row.height.saturating_sub(8));
            item_rect.x = item_rect.x.max(l.lane_left.round().max(0.0) as u32);
            let lane_right = l.lane_right.round().max(0.0) as u32;
            item_rect.width = item_rect.width.min(lane_right.saturating_sub(item_rect.x));
            let chosen = item.selected || s.selected_item == Some((index, item_index));
            if item_rect.width == 0 || item_rect.height == 0 {
                continue;
            }
            if matches!(track.kind, super::NativeTrackKind::Drawing) {
                c.paint_round(
                    item_rect,
                    4,
                    if chosen {
                        t.selection_fill
                    } else {
                        t.surface[2]
                    },
                    if chosen { t.accent } else { t.separator },
                    1,
                );
                if item_rect.width > 30 {
                    text(
                        c,
                        &item.label,
                        rect(
                            item_rect.x + 6,
                            item_rect.y,
                            item_rect.width.saturating_sub(12),
                            item_rect.height,
                        ),
                        if chosen {
                            t.text_primary
                        } else {
                            t.text_secondary
                        },
                        11,
                        if chosen {
                            FontWeight::Semibold
                        } else {
                            FontWeight::Regular
                        },
                    );
                }
            } else {
                c.paint_round(
                    rect(
                        item_rect.x,
                        item_rect.y + item_rect.height.saturating_sub(8) / 2,
                        8,
                        8,
                    ),
                    4,
                    if chosen { t.accent } else { t.text_secondary },
                    if chosen { t.text_primary } else { 0 },
                    if chosen { 1 } else { 0 },
                );
            }
        }
    }
    if let Some(status) = &m.status {
        if l.rows.height == 0 || l.panel.width < 24 {
            centred(c, status, l.panel, t.warning, 12, FontWeight::Regular);
        } else {
            let y = l.rows.y.saturating_add(l.rows.height.saturating_sub(20));
            let w = l.panel.width.saturating_sub(16);
            c.paint_round(
                rect(l.panel.x + 4, y, w, 18.min(l.rows.height)),
                4,
                t.surface[3],
                t.warning,
                1,
            );
            text(
                c,
                status,
                rect(l.panel.x + 10, y, w.saturating_sub(12), 18),
                t.text_primary,
                11,
                FontWeight::Regular,
            );
        }
    } else if !m.can_add_drawing_track
        && m.tracks.is_empty()
        && l.rows.height >= 20
        && l.panel.width >= 16
    {
        let y = l.rows.y.saturating_add(l.rows.height.saturating_sub(18));
        text(
            c,
            "Add drawing track unavailable",
            rect(l.panel.x + 8, y, l.panel.width.saturating_sub(16), 18),
            t.text_tertiary,
            11,
            FontWeight::Regular,
        );
    } else if !m.can_edit_selected_item && m.selected_item().is_some() && l.panel.width >= 24 {
        let y = l.rows.y.saturating_add(l.rows.height.saturating_sub(18));
        text(
            c,
            "Editing unavailable",
            rect(l.panel.x + 8, y, l.panel.width.saturating_sub(16), 18),
            t.text_tertiary,
            11,
            FontWeight::Regular,
        );
    }
    if let Some(preview) = s.drag_preview {
        paint_preview(c, l, m, preview);
    }
}

fn paint_preview(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    m: &TimelineModel,
    preview: DragPreview,
) {
    let Some(row) = m
        .tracks
        .get(preview.track_index)
        .map(|_| l.row_rect(preview.track_index))
    else {
        return;
    };
    let x = l.lane_x(preview.to_tick).round().max(0.0) as u32;
    if x < l.panel.x.saturating_add(l.panel.width) && row.height > 0 {
        c.paint_rect(
            rect(
                x.min(l.panel.x.saturating_add(l.panel.width).saturating_sub(1)),
                row.y + 3,
                1,
                row.height.saturating_sub(6),
            ),
            c.tokens.accent,
        );
    }
}

fn paint_playhead(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, tick: u32) {
    if l.rows.height == 0 {
        return;
    }
    let x = l.lane_x(tick).round().max(0.0) as u32;
    let right = l.panel.x.saturating_add(l.panel.width);
    if x < l.panel.x || x >= right {
        return;
    }
    c.paint_rect(
        rect(
            x,
            l.ruler.y,
            1,
            l.ruler.height.saturating_add(l.rows.height),
        ),
        c.tokens.accent,
    );
    let handle_x = x.saturating_sub(4).max(l.panel.x);
    let handle = rect(
        handle_x,
        l.ruler.y,
        9.min(right.saturating_sub(handle_x)),
        7.min(l.ruler.height),
    );
    c.paint_round(handle, 3, c.tokens.accent, 0, 0);
}

fn has_target(model: &TimelineModel, state: &InteractionState) -> bool {
    state
        .selected_track
        .and_then(|index| model.tracks.get(index))
        .or_else(|| model.selected_track().map(|(_, track)| track))
        .is_some_and(|track| !track.target.is_empty())
        || model
            .selected_target
            .as_ref()
            .is_some_and(|target| !target.is_empty())
}

/// Popover with a live curve preview, four steppers and Apply.
fn paint_bezier_editor(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    m: &TimelineModel,
    s: &InteractionState,
) {
    if !s.bezier.open || m.selected_item().is_none() {
        return;
    }
    let (Some(editor), Some(preview)) = (l.bezier_editor(), l.bezier_preview()) else {
        return;
    };
    let t = c.tokens;
    c.paint_round(editor, 6, t.panel_raised, t.separator, 1);
    c.paint_rect(preview, t.panel);
    // Unit square guides: baseline and top edge of 0..1.
    let unit_h = preview.height.saturating_sub(24);
    let base_y = preview.y + preview.height - 12;
    c.paint_rect(rect(preview.x, base_y, preview.width, 1), t.separator);
    c.paint_rect(
        rect(preview.x, base_y.saturating_sub(unit_h), preview.width, 1),
        t.separator,
    );
    let curve = super::bezier::sample(s.bezier.values, 40);
    for (x, y) in curve {
        let px = preview.x + (x * f64::from(preview.width - 2)).round() as u32;
        let py = (f64::from(base_y) - y * f64::from(unit_h)).round();
        let py = py.clamp(
            f64::from(preview.y),
            f64::from(preview.y + preview.height - 2),
        ) as u32;
        c.paint_rect(rect(px, py, 2, 2), t.accent);
    }
    let names = ["x1", "y1", "x2", "y2"];
    for field in 0..4u8 {
        let (Some(minus), Some(plus), Some(value)) = (
            l.bezier_step_rect(field, false),
            l.bezier_step_rect(field, true),
            l.bezier_value_rect(field),
        ) else {
            continue;
        };
        text(
            c,
            names[usize::from(field)],
            rect(minus.x - 20, minus.y, 20, minus.height),
            t.text_tertiary,
            11,
            FontWeight::Regular,
        );
        for (cell, up) in [(minus, false), (plus, true)] {
            let hit = Hit::BezierStep { field, up };
            let fill = if s.pressed() == Some(hit) && s.hover == Some(hit) {
                t.pressed_fill
            } else if s.hover == Some(hit) {
                t.panel_hover
            } else {
                t.panel
            };
            c.paint_round(cell, 4, fill, t.separator, 1);
            centred(
                c,
                if up { "+" } else { "-" },
                cell,
                t.text_secondary,
                13,
                FontWeight::Semibold,
            );
        }
        let v = s.bezier.values[usize::from(field)];
        mono(c, &format!("{v:.2}"), value, t.text_primary, 11);
    }
    if let Some(apply) = l.bezier_apply_rect() {
        let fill = if s.hover == Some(Hit::BezierApply) {
            t.pressed_fill
        } else {
            t.panel_hover
        };
        c.paint_round(apply, 4, fill, t.accent, 1);
        centred(c, "Apply", apply, t.text_primary, 12, FontWeight::Semibold);
    }
}
