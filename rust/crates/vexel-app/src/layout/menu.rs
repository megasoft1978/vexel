//! The Export popover: geometry under the Export button and its painting.
//! Drawn like the MCP popover (same shadow, fill and edge tokens); its seven
//! rows are the only targets that react while it is open.

use super::paint::{motion_fill, text};
use super::{Layout, SECTION_HEADER_H, r};
use crate::model::{ExportFormat, Model, Target};
use crate::motion::{MotionFrame, control_id};
use vexel_ui::{FontWeight, Rect, WidgetCanvas};

const MENU_W: u32 = 196;
const ITEM_H: u32 = 32;
const PAD: u32 = 4;

impl Layout {
    /// The popover, right-aligned to the Export button, 4 px below it. With the
    /// inspector open it starts below the inspector's first header row instead,
    /// so the menu never covers that header text.
    pub fn export_popover(&self) -> Rect {
        let button = self.buttons[3];
        let right = (button.x + button.width).min(self.width.saturating_sub(8));
        let mut top = button.y + button.height + 4;
        if let Some(panel) = self.inspector {
            top = top.max(panel.y + SECTION_HEADER_H + 4);
        }
        r(
            right.saturating_sub(MENU_W).max(8),
            top,
            MENU_W,
            ExportFormat::ALL.len() as u32 * ITEM_H + 2 * PAD,
        )
    }

    /// Rows in [`ExportFormat::ALL`] order.
    pub fn export_items(&self) -> [Rect; 7] {
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
    // Soft shadow from two offset layers (the canvas has no blur), then the
    // highest surface with a visible 1 px edge.
    let halo = Rect {
        x: p.x.saturating_sub(2),
        y: p.y + 2,
        width: p.width + 4,
        height: p.height + 4,
    };
    c.paint_round(halo, 10, t.popover_shadow, 0, 0);
    c.paint_round(Rect { y: p.y + 4, ..p }, 8, t.popover_shadow, 0, 0);
    c.paint_round(p, 8, t.surface[5], t.text_tertiary, 1);
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
