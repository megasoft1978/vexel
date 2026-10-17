//! The Export popover: geometry under the Export button and its painting.
//! Drawn like the MCP popover (same shadow, fill and edge tokens); its three
//! rows are the only targets that react while it is open.

use super::paint::{motion_fill, text};
use super::{Layout, TOOLBAR_H, r};
use crate::model::{ExportFormat, Model, Target};
use crate::motion::{MotionFrame, control_id};
use vexel_ui::{FontWeight, Rect, WidgetCanvas};

const MENU_W: u32 = 196;
const ITEM_H: u32 = 32;
const PAD: u32 = 4;

impl Layout {
    /// The popover, right-aligned to the Export button, just under the toolbar.
    pub fn export_popover(&self) -> Rect {
        let button = self.buttons[3];
        let right = (button.x + button.width).min(self.width.saturating_sub(8));
        r(
            right.saturating_sub(MENU_W).max(8),
            TOOLBAR_H + 4,
            MENU_W,
            ExportFormat::ALL.len() as u32 * ITEM_H + 2 * PAD,
        )
    }

    /// Rows in [`ExportFormat::ALL`] order.
    pub fn export_items(&self) -> [Rect; 3] {
        let p = self.export_popover();
        std::array::from_fn(|i| {
            r(
                p.x + PAD,
                p.y + PAD + i as u32 * ITEM_H,
                p.width - 2 * PAD,
                ITEM_H,
            )
        })
    }
}

/// Paints the Export popover; nothing while it is closed.
pub fn draw_export_menu(
    c: &mut WidgetCanvas<'_>,
    l: &Layout,
    model: &Model,
    motion: Option<&MotionFrame>,
) {
    if !model.export_open {
        return;
    }
    let t = c.tokens;
    let p = l.export_popover();
    c.paint_round(Rect { y: p.y + 2, ..p }, 8, t.popover_shadow, 0, 0);
    c.paint_round(p, 8, t.pressed_fill, t.control_edge, 1);
    for (i, (format, rect)) in ExportFormat::ALL.iter().zip(l.export_items()).enumerate() {
        let target = Target::MenuItem(*format);
        let enabled = model.target_enabled(l, target);
        let id = control_id(target);
        if enabled {
            motion_fill(c, motion, id, false, rect, 6);
        }
        let label = Rect {
            x: rect.x + 12,
            width: rect.width - 24,
            ..rect
        };
        let color = if enabled {
            t.text_primary
        } else {
            t.disabled_text
        };
        text(c, format.label(), label, color, 13, FontWeight::Regular);
        let focused = model.menu_focus == Some(i);
        match motion {
            Some(m) => crate::motion::paint::ring(c, rect, &m.get(id)),
            None if focused => super::paint::paint_focus(c, rect, 6),
            None => {}
        }
    }
}
