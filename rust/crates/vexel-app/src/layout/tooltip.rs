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
    let native_button = if let Target::NativeTimeline(hit) = target {
        let anchor = l.footer?;
        let size = crate::native_timeline::layout::CONTROL_SIZE;
        let gap = 4;
        let index = match hit {
            crate::native_timeline::layout::Hit::Play => 0,
            crate::native_timeline::layout::Hit::Loop => 1,
            crate::native_timeline::layout::Hit::OnionBefore => 2,
            crate::native_timeline::layout::Hit::AddDrawingTrack => 3,
            crate::native_timeline::layout::Hit::ImportDrawing => 4,
            _ => return None,
        };
        let transport = crate::native_timeline::layout::TRANSPORT_HEIGHT;
        let button = Rect {
            x: anchor.x
                + crate::native_timeline::layout::MODE_SELECTOR_WIDTH
                + 8
                + index * (size + gap),
            y: anchor.y + (transport.saturating_sub(size) / 2),
            width: size.min(anchor.width),
            height: size.min(anchor.height),
        };
        Some(button)
    } else {
        None
    };
    let a = match native_button {
        Some(button) => button,
        None => l.target_rect(target)?,
    };
    let (w, h) = tip_size(name, shortcut);
    let (x, y) = if matches!(target, Target::NativeTimeline(_)) {
        (a.x + a.width / 2 - w / 2, a.y.saturating_sub(h + GAP))
    } else if a.x < RAIL_W && a.y >= TOOLBAR_H {
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
