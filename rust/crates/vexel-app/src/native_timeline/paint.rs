//! Custom native timeline painting. It reuses the inspector's tokens, 12/13 pt
//! type, rounded level-3 fields, bordered buttons and segmented rows (see
//! `ui_kit`), so the footer and the inspector read as one design. The
//! playhead is the single strong accent line of the panel.

use super::interaction::InteractionState;
use super::layout::{Hit, TimelineLayout};
use super::model::TimelineModel;
use crate::layout::paint::text_width;
use vexel_ui::{Rect, WidgetCanvas};

pub(super) fn rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// `s` shortened with `..` so its estimated width fits `width` points.
pub(super) fn fit(s: &str, size: u32, width: u32) -> String {
    if text_width(s, size) <= width {
        return s.to_owned();
    }
    let mut out = String::new();
    for ch in s.chars() {
        out.push(ch);
        if text_width(&out, size) + text_width("..", size) > width {
            out.pop();
            break;
        }
    }
    out.push_str("..");
    out
}

/// A filled diamond of half-diagonal `r` centred on (`cx`, `cy`).
pub(super) fn diamond(c: &mut WidgetCanvas<'_>, cx: u32, cy: u32, r: u32, color: u32) {
    for dy in 0..=2 * r {
        let half = r - dy.abs_diff(r);
        let (Some(x), Some(y)) = (cx.checked_sub(half), (cy + dy).checked_sub(r)) else {
            continue;
        };
        c.paint_rect(rect(x, y, 2 * half + 1, 1), color);
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
    super::paint_transport::paint_transport(canvas, &layout, model, state);
    super::paint_transport::paint_keybar(canvas, &layout, model, state);
    super::paint_tracks::paint_ruler(canvas, &layout, model);
    super::paint_tracks::paint_tracks(canvas, &layout, model, state);
    super::paint_tracks::paint_playhead(canvas, &layout, model.current_tick);
    super::paint_bezier::paint_bezier_editor(canvas, &layout, model, state);
    if state.focus && panel.width >= 3 && panel.height >= 2 {
        let x = panel.x.saturating_add(1);
        let y = panel.y.saturating_add(panel.height.saturating_sub(2));
        canvas.paint_rect(rect(x, y, panel.width.saturating_sub(2), 1), t.accent);
    }
}

pub(super) fn has_target(model: &TimelineModel, state: &InteractionState) -> bool {
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

/// Row index a hover target belongs to.
pub(super) fn hover_row(hover: Option<Hit>) -> Option<usize> {
    match hover? {
        Hit::Track(i)
        | Hit::Visibility(i)
        | Hit::Lane(i)
        | Hit::DuplicateTrack(i)
        | Hit::DeleteTrack(i)
        | Hit::Item { track: i, .. } => Some(i),
        _ => None,
    }
}
