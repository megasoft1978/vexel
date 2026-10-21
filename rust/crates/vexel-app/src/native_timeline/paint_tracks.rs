//! Ruler, track rows, keys and playhead of the native timeline.

use super::interaction::{DragPreview, InteractionState};
use super::layout::{Hit, TRACK_HEIGHT, TimelineLayout};
use super::model::TimelineModel;
use super::naming;
use super::paint::{diamond, fit, hover_row, rect};
use crate::layout::paint::{centred, text, text_width};
use vexel_ui::{FontWeight, IconKind, WidgetCanvas};

pub(super) fn paint_ruler(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, m: &TimelineModel) {
    if l.ruler.height == 0 {
        return;
    }
    let t = c.tokens;
    c.paint_rect(l.ruler, t.surface[2]);
    c.paint_rect(
        rect(l.ruler.x, l.ruler.y + l.ruler.height - 1, l.ruler.width, 1),
        t.separator,
    );
    let v = l.viewport;
    let span = v.last_tick.saturating_sub(v.first_tick).max(1);
    let lane_w = (l.lane_right - l.lane_left).max(1.0);
    let step = ruler_step(span, lane_w);
    let per_tick = lane_w / f64::from(span);
    let right = l.panel.x.saturating_add(l.panel.width);
    let base = l.ruler.y + l.ruler.height - 1;
    if per_tick >= 5.0 {
        for tick in v.first_tick..=v.last_tick.min(v.first_tick + 4096) {
            let x = l.lane_x(tick).round().max(0.0) as u32;
            if x < right && tick % step != 0 {
                c.paint_rect(rect(x, base - 4, 1, 4), t.separator);
            }
        }
    }
    let mut tick = v.first_tick - v.first_tick % step;
    if tick < v.first_tick {
        tick += step;
    }
    let mut count = 0usize;
    while tick <= v.last_tick && count < 512 {
        let x = l.lane_x(tick).round().max(0.0) as u32;
        if x >= l.panel.x && x < right {
            c.paint_rect(rect(x, base - 7, 1, 7), t.control_edge);
            let width = (step as f64 * per_tick).round().max(28.0) as u32;
            text(
                c,
                &tick.to_string(),
                rect(
                    x + 4,
                    l.ruler.y + 2,
                    width.min(right.saturating_sub(x + 4)),
                    l.ruler.height.saturating_sub(10),
                ),
                t.text_tertiary,
                11,
                FontWeight::Regular,
            );
        }
        let next = tick.saturating_add(step);
        if next <= tick {
            break;
        }
        tick = next;
        count += 1;
    }
    // Header cell of the name column, in the inspector's section-header type.
    let name_w = l.name_width.saturating_sub(24);
    text(
        c,
        "Tracks",
        rect(l.panel.x + 12, l.ruler.y, name_w.min(80), l.ruler.height),
        t.text_primary,
        13,
        FontWeight::Semibold,
    );
    let count = m.tracks.len().to_string();
    let cw = text_width(&count, 12);
    text(
        c,
        &count,
        rect(
            l.panel.x + l.name_width.saturating_sub(cw + 8),
            l.ruler.y,
            cw + 4,
            l.ruler.height,
        ),
        t.text_tertiary,
        12,
        FontWeight::Regular,
    );
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

pub(super) fn paint_tracks(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    m: &TimelineModel,
    s: &InteractionState,
) {
    let t = c.tokens;
    let visible_rows = (l.rows.height / TRACK_HEIGHT) as usize;
    let hovered_row = hover_row(s.hover);
    for (index, track) in m.tracks.iter().enumerate().take(visible_rows) {
        let row = l.row_rect(index);
        if row.height == 0 {
            continue;
        }
        let selected = track.selected || s.selected_track == Some(index);
        let hovered = hovered_row == Some(index);
        let fill = if selected {
            t.selection_fill
        } else if hovered {
            t.surface[3]
        } else if index % 2 == 1 {
            t.surface[2]
        } else {
            t.panel
        };
        c.paint_rect(row, fill);
        if selected {
            c.paint_rect(
                rect(row.x, row.y + 4, 3, row.height.saturating_sub(8)),
                t.accent,
            );
        }
        paint_row_chrome(c, l, s, index, track, selected, hovered);
        paint_items(c, l, m, s, index, track);
    }
    // Divider between the name column and the lanes.
    let rows_h = visible_rows as u32 * TRACK_HEIGHT;
    if rows_h > 0 && m.tracks.len() > 0 {
        let h = rows_h.min((m.tracks.len() as u32) * TRACK_HEIGHT);
        c.paint_rect(rect(l.panel.x + l.name_width, l.rows.y, 1, h), t.separator);
    }
    paint_notices(c, l, m);
    if let Some(preview) = s.drag_preview {
        paint_preview(c, l, m, preview);
    }
}

fn paint_row_chrome(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    s: &InteractionState,
    index: usize,
    track: &super::model::Track,
    selected: bool,
    hovered: bool,
) {
    let t = c.tokens;
    let row = l.row_rect(index);
    let y = row.y + row.height.saturating_sub(16) / 2;
    let eye_hot = s.hover == Some(Hit::Visibility(index));
    let eye = rect(row.x + 12, y, 16, 16);
    c.paint_icon_color(
        if track.visible {
            IconKind::EYE
        } else {
            IconKind::EYE_OFF
        },
        eye,
        if eye_hot {
            t.text_primary
        } else if track.visible {
            t.text_secondary
        } else {
            t.text_tertiary
        },
    );
    let subtitle = naming::track_subtitle(track);
    let sub_w = subtitle.as_ref().map_or(0, |s| text_width(s, 11) + 4);
    let title_x = row.x + 36;
    let title_w = l
        .name_width
        .saturating_sub(36 + 10 + if sub_w > 0 { sub_w + 8 } else { 0 });
    let colour = if !track.visible {
        t.text_tertiary
    } else if selected || hovered {
        t.text_primary
    } else {
        t.text_secondary
    };
    text(
        c,
        &fit(&naming::track_title(track, index), 12, title_w),
        rect(title_x, row.y, title_w, row.height),
        colour,
        12,
        if selected {
            FontWeight::Semibold
        } else {
            FontWeight::Regular
        },
    );
    if let Some(sub) = subtitle {
        text(
            c,
            &sub,
            rect(
                row.x + l.name_width.saturating_sub(sub_w + 8),
                row.y,
                sub_w,
                row.height,
            ),
            t.text_tertiary,
            11,
            FontWeight::Regular,
        );
    }
    if row.width >= 48 {
        for (hit, dx, icon) in [
            (Hit::DuplicateTrack(index), 46, IconKind::DUPLICATE),
            (Hit::DeleteTrack(index), 22, IconKind::TRASH),
        ] {
            let glyph = rect(row.x + row.width - dx, y, 16, 16);
            let hot = s.hover == Some(hit);
            if hot {
                c.paint_round(
                    rect(glyph.x - 3, glyph.y - 3, 22, 22),
                    5,
                    t.surface[4],
                    t.control_edge,
                    1,
                );
            }
            c.paint_icon_color(
                icon,
                glyph,
                if hot { t.text_primary } else { t.text_tertiary },
            );
        }
    }
}

fn paint_items(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    m: &TimelineModel,
    s: &InteractionState,
    index: usize,
    track: &super::model::Track,
) {
    let t = c.tokens;
    let row = l.row_rect(index);
    let mid = row.y + row.height / 2;
    let property = matches!(track.kind, super::NativeTrackKind::Property);
    if property {
        for pair in track.items.windows(2) {
            let (a, b) = (
                l.lane_x(pair[0].tick).round().max(0.0) as u32,
                l.lane_x(pair[1].tick).round().max(0.0) as u32,
            );
            if b > a {
                c.paint_rect(rect(a, mid - 1, b - a, 2), t.control_edge);
            }
        }
    }
    for (item_index, item) in track.items.iter().enumerate() {
        let chosen = item.selected || s.selected_item == Some((index, item_index));
        if property {
            let cx = l.lane_x(item.tick).round().max(0.0) as u32;
            if cx < l.lane_left as u32 || cx > l.lane_right as u32 + 1 {
                continue;
            }
            let hot = s.hover
                == Some(Hit::Item {
                    track: index,
                    item: item_index,
                });
            if chosen {
                diamond(c, cx, mid, 7, t.text_primary);
                diamond(c, cx, mid, 5, t.accent);
            } else if hot {
                diamond(c, cx, mid, 6, t.accent_hover);
            } else {
                diamond(c, cx, mid, 5, t.accent);
            }
            continue;
        }
        let Some(mut r) = l.item_rect(m, index, item_index) else {
            continue;
        };
        r.y = row.y + 4;
        r.height = row.height.saturating_sub(8);
        let lane_right = l.lane_right.round().max(0.0) as u32;
        r.width = r.width.min(lane_right.saturating_sub(r.x));
        if r.width == 0 || r.height == 0 {
            continue;
        }
        c.paint_round(
            r,
            5,
            if chosen {
                t.selection_fill
            } else {
                t.surface[3]
            },
            if chosen { t.accent } else { t.control_edge },
            1,
        );
        if r.width > 30 {
            text(
                c,
                &fit(&item.label, 11, r.width.saturating_sub(12)),
                rect(r.x + 6, r.y, r.width.saturating_sub(12), r.height),
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
    }
}

fn paint_notices(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, m: &TimelineModel) {
    let t = c.tokens;
    if let Some(status) = &m.status {
        if l.rows.height == 0 || l.panel.width < 24 {
            centred(c, status, l.panel, t.warning, 12, FontWeight::Regular);
        } else {
            let y = l.rows.y.saturating_add(l.rows.height.saturating_sub(24));
            let w = l.panel.width.saturating_sub(16);
            c.paint_round(
                rect(l.panel.x + 8, y, w, 22.min(l.rows.height)),
                6,
                t.surface[3],
                t.warning,
                1,
            );
            text(
                c,
                &fit(status, 12, w.saturating_sub(16)),
                rect(l.panel.x + 16, y, w.saturating_sub(16), 22),
                t.text_primary,
                12,
                FontWeight::Regular,
            );
        }
    } else if m.tracks.is_empty() && l.rows.height >= 40 && l.panel.width >= 16 {
        let (title, hint) = if m.can_add_drawing_track {
            (
                "No tracks yet",
                "Select an element, then use + to add a drawing track.",
            )
        } else {
            ("No tracks yet", "Add drawing track unavailable")
        };
        let w = l.panel.width.saturating_sub(32);
        text(
            c,
            title,
            rect(l.panel.x + 16, l.rows.y + 8, w, 22),
            t.text_secondary,
            13,
            FontWeight::Semibold,
        );
        text(
            c,
            &fit(hint, 12, w),
            rect(l.panel.x + 16, l.rows.y + 30, w, 20),
            t.text_tertiary,
            12,
            FontWeight::Regular,
        );
    } else if !m.can_edit_selected_item && m.selected_item().is_some() && l.panel.width >= 24 {
        let y = l.rows.y.saturating_add(l.rows.height.saturating_sub(20));
        text(
            c,
            "Editing unavailable",
            rect(l.panel.x + 12, y, l.panel.width.saturating_sub(24), 18),
            t.text_tertiary,
            12,
            FontWeight::Regular,
        );
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

pub(super) fn paint_playhead(c: &mut WidgetCanvas<'_>, l: &TimelineLayout, tick: u32) {
    if l.ruler.height == 0 {
        return;
    }
    let x = l.lane_x(tick).round().max(0.0) as u32;
    let right = l.panel.x.saturating_add(l.panel.width);
    if x < l.panel.x || x >= right {
        return;
    }
    let t = c.tokens;
    let label = tick.to_string();
    let w = (text_width(&label, 11) + 14).max(26);
    let chip_x = x
        .saturating_sub(w / 2)
        .max(l.panel.x)
        .min(right.saturating_sub(w));
    let chip = rect(
        chip_x,
        l.ruler.y + 3,
        w,
        l.ruler.height.saturating_sub(8).max(14),
    );
    c.paint_rect(
        rect(
            x.saturating_sub(1),
            chip.y + chip.height,
            2,
            (l.ruler.y + l.ruler.height + l.rows.height).saturating_sub(chip.y + chip.height),
        ),
        t.accent,
    );
    c.paint_round(chip, 5, t.accent, 0, 0);
    centred(c, &label, chip, t.surface[0], 11, FontWeight::Semibold);
}
