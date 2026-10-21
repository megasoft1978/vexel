//! Geometry of the custom cubic-bezier editor popover. Layout, left to right:
//! a curve preview and a 2 x 2 grid of X1 Y1 / X2 Y2 steppers, with Cancel and
//! Apply in the title row, all in the inspector's field and button metrics.

use super::layout::{BEZIER_EDITOR_H, BEZIER_EDITOR_W, Hit, TimelineLayout, inside, rect};
use vexel_ui::Rect;

const PAD: u32 = 12;
const CELL_W: u32 = 112;
const CELL_H: u32 = 28;
const CELL_GAP: u32 = 6;

impl TimelineLayout {
    /// The popover, opening upward from the Bezier cell of the key bar.
    /// `None` while the key bar is hidden or the panel is too small.
    pub fn bezier_editor(&self) -> Option<Rect> {
        let cell = super::layout::ease_cell_in(self.panel, 5)?;
        if self.keybar.height == 0
            || self.panel.width < BEZIER_EDITOR_W + 16
            || self.keybar.y < self.panel.y + BEZIER_EDITOR_H + 8
        {
            return None;
        }
        let max_x = self.panel.x + self.panel.width - BEZIER_EDITOR_W - 8;
        Some(rect(
            (cell.x + cell.width)
                .saturating_sub(BEZIER_EDITOR_W)
                .max(self.panel.x + 8)
                .min(max_x),
            self.keybar.y - BEZIER_EDITOR_H - 4,
            BEZIER_EDITOR_W,
            BEZIER_EDITOR_H,
        ))
    }

    pub fn bezier_preview(&self) -> Option<Rect> {
        let e = self.bezier_editor()?;
        Some(rect(e.x + PAD, e.y + 36, 100, 2 * CELL_H + CELL_GAP + 22))
    }

    /// Stepper field `field` (0..4 = x1, y1, x2, y2) of the 2 x 2 grid.
    pub fn bezier_cell_rect(&self, field: u8) -> Option<Rect> {
        let e = self.bezier_editor()?;
        let f = u32::from(field.min(3));
        let (col, row) = (f % 2, f / 2);
        Some(rect(
            e.x + PAD + 100 + 12 + col * (CELL_W + CELL_GAP),
            e.y + 36 + row * (CELL_H + CELL_GAP),
            CELL_W,
            CELL_H,
        ))
    }

    /// Stepper row `field` (0..4): minus / plus buttons.
    pub fn bezier_step_rect(&self, field: u8, up: bool) -> Option<Rect> {
        let c = self.bezier_cell_rect(field)?;
        let dx = if up { 25 } else { 50 };
        Some(rect(c.x + c.width - dx, c.y + 3, 22, CELL_H - 6))
    }

    pub fn bezier_value_rect(&self, field: u8) -> Option<Rect> {
        let c = self.bezier_cell_rect(field)?;
        Some(rect(
            c.x + 32,
            c.y,
            c.width.saturating_sub(32 + 54),
            c.height,
        ))
    }

    pub fn bezier_apply_rect(&self) -> Option<Rect> {
        let e = self.bezier_editor()?;
        Some(rect(e.x + e.width - PAD - 56, e.y + 8, 56, 24))
    }

    pub fn bezier_cancel_rect(&self) -> Option<Rect> {
        let apply = self.bezier_apply_rect()?;
        Some(rect(apply.x - 6 - 64, apply.y, 64, apply.height))
    }

    /// Hit inside the open editor; `None` outside it.
    pub fn bezier_hit(&self, x: i32, y: i32) -> Option<Hit> {
        let e = self.bezier_editor()?;
        if !inside(e, x, y) {
            return None;
        }
        for field in 0..4u8 {
            for up in [false, true] {
                if self
                    .bezier_step_rect(field, up)
                    .is_some_and(|r| inside(r, x, y))
                {
                    return Some(Hit::BezierStep { field, up });
                }
            }
        }
        if self.bezier_apply_rect().is_some_and(|r| inside(r, x, y)) {
            return Some(Hit::BezierApply);
        }
        if self.bezier_cancel_rect().is_some_and(|r| inside(r, x, y)) {
            return Some(Hit::BezierCancel);
        }
        Some(Hit::BezierPanel)
    }
}
