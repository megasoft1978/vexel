//! Tooltip placement and painting: a rounded level-3 label (name plus
//! shortcut) beside its target, kept inside the window.

use super::paint::{paint_tip, tip_size, tooltip};
use super::{Layout, RAIL_W, TOOLBAR_H};
use crate::model::Target;
use vexel_ui::{Rect, WidgetCanvas};

const GAP: u32 = 6;
const EDGE: u32 = 4;

/// The tooltip rect for `target`, always inside the window; `None` when the
/// target has no tooltip or no rect. Rail tools get it to their right,
/// toolbar buttons below, status controls above.
pub fn tooltip_rect(l: &Layout, target: Target) -> Option<Rect> {
    let (name, shortcut) = tooltip(target)?;
    let a = l.target_rect(target)?;
    let (w, h) = tip_size(name, shortcut);
    let (x, y) = if a.x < RAIL_W && a.y >= TOOLBAR_H {
        (a.x + a.width + GAP + 2, a.y + a.height / 2 - h / 2)
    } else if a.y < TOOLBAR_H {
        (a.x + a.width / 2 - w / 2, a.y + a.height + GAP)
    } else {
        (a.x + a.width / 2 - w / 2, a.y.saturating_sub(h + GAP))
    };
    let max_x = l.width.saturating_sub(w + EDGE).max(EDGE);
    let max_y = l.height.saturating_sub(h + EDGE).max(EDGE);
    Some(Rect {
        x: x.clamp(EDGE, max_x),
        y: y.clamp(EDGE, max_y),
        width: w,
        height: h,
    })
}

/// Paints the tooltip of `target` (nothing when it has none).
pub fn draw_tooltip(c: &mut WidgetCanvas<'_>, l: &Layout, target: Target) {
    let (Some(r), Some((name, shortcut))) = (tooltip_rect(l, target), tooltip(target)) else {
        return;
    };
    paint_tip(c, r, name, shortcut);
}
