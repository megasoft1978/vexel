//! Tooltip placement and painting: a rounded level-3 label (name plus
//! shortcut) beside its target, kept inside the window.

use super::paint::{text, text_width, tooltip};
use super::{Layout, RAIL_W, TOOLBAR_H};
use crate::model::Target;
use vexel_ui::{FontWeight, Rect, WidgetCanvas};

const HEIGHT: u32 = 24;
const PAD: u32 = 8;
const GAP: u32 = 6;
const EDGE: u32 = 4;

/// Label size from the (estimated) text widths.
fn size(name: &str, shortcut: &str) -> (u32, u32) {
    let shortcut_w = if shortcut.is_empty() {
        0
    } else {
        text_width(shortcut, 12) + 10
    };
    (2 * PAD + text_width(name, 12) + shortcut_w, HEIGHT)
}

/// The tooltip rect for `target`, always inside the window; `None` when the
/// target has no tooltip or no rect. Rail tools get it to their right,
/// toolbar buttons below, status controls above.
pub fn tooltip_rect(l: &Layout, target: Target) -> Option<Rect> {
    let (name, shortcut) = tooltip(target)?;
    let a = l.target_rect(target)?;
    let (w, h) = size(name, shortcut);
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
    let t = c.tokens;
    c.paint_round(r, 6, t.surface[3], t.control_edge, 1);
    let label = Rect {
        x: r.x + PAD,
        width: r.width - 2 * PAD,
        ..r
    };
    text(c, name, label, t.text_primary, 12, FontWeight::Regular);
    if !shortcut.is_empty() {
        let x = r.x + PAD + text_width(name, 12) + 10;
        let rect = Rect {
            x,
            width: (r.x + r.width).saturating_sub(x + PAD / 2),
            ..label
        };
        text(c, shortcut, rect, t.text_secondary, 12, FontWeight::Regular);
    }
}
