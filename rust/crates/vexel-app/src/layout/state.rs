//! In-memory UI state that shapes the layout: panel sizes, collapsed
//! inspector sections, keyboard focus and the MCP popover. Persistence is a
//! follow-up; the settings key would be `ui.chrome` (inspector width, footer
//! height, collapsed bit mask).

use super::{
    FOOTER_DEFAULT, FOOTER_MIN, INSPECTOR_DEFAULT, INSPECTOR_MAX, INSPECTOR_MIN, Layout, SECTIONS,
    Splitter,
};
use crate::model::Mode;
use vexel_ui::Rect;

/// A keyboard focus stop in reading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// The Draw | Animate segmented control (Left/Right change the segment).
    Segmented,
    /// An inspector section header (Space/Return toggle it).
    Section(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Drag {
    splitter: Splitter,
    start: (i32, i32),
    start_size: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiState {
    pub inspector_w: u32,
    pub footer_h: u32,
    /// Selection, Fill, Stroke open; Transform and Animation collapsed.
    pub collapsed: [bool; SECTIONS],
    pub inspector_visible: bool,
    pub mcp_open: bool,
    pub focus: Option<Focus>,
    drag: Option<Drag>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            inspector_w: INSPECTOR_DEFAULT,
            footer_h: FOOTER_DEFAULT,
            collapsed: [false, false, false, true, true],
            inspector_visible: true,
            mcp_open: false,
            focus: None,
            drag: None,
        }
    }
}

impl UiState {
    pub fn set_inspector_width(&mut self, width: u32) {
        self.inspector_w = width.clamp(INSPECTOR_MIN, INSPECTOR_MAX);
    }

    pub fn set_footer_height(&mut self, height: u32, content_height: u32) {
        self.footer_h = height.clamp(FOOTER_MIN, (content_height / 2).max(FOOTER_MIN));
    }

    pub fn toggle_section(&mut self, index: usize) -> bool {
        match self.collapsed.get_mut(index) {
            Some(c) => {
                *c = !*c;
                true
            }
            None => false,
        }
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Starts a splitter drag when `point` is on one in `layout`.
    pub fn begin_drag(&mut self, layout: &Layout, point: (i32, i32)) -> bool {
        let hit = |r: Option<Rect>| {
            r.is_some_and(|r| {
                point.0 >= r.x as i32
                    && point.1 >= r.y as i32
                    && point.0 < (r.x + r.width) as i32
                    && point.1 < (r.y + r.height) as i32
            })
        };
        let (splitter, size) = if hit(layout.inspector_splitter) {
            (Splitter::Inspector, layout.inspector.map_or(0, |r| r.width))
        } else if hit(layout.footer_splitter) {
            (Splitter::Footer, layout.footer.map_or(0, |r| r.height))
        } else {
            return false;
        };
        self.drag = Some(Drag {
            splitter,
            start: point,
            start_size: size,
        });
        true
    }

    /// Applies a drag to `point`; sizes clamp, so any delta is safe.
    pub fn drag_to(&mut self, point: (i32, i32), window: (u32, u32), mode: Mode) -> bool {
        let Some(drag) = self.drag else {
            return false;
        };
        let before = (self.inspector_w, self.footer_h);
        match drag.splitter {
            Splitter::Inspector => {
                let delta = i64::from(drag.start.0) - i64::from(point.0);
                let size = (i64::from(drag.start_size) + delta).clamp(0, 4096) as u32;
                self.set_inspector_width(size);
            }
            Splitter::Footer if mode == Mode::Animate => {
                let delta = i64::from(drag.start.1) - i64::from(point.1);
                let size = (i64::from(drag.start_size) + delta).clamp(0, 4096) as u32;
                let content = window.1.saturating_sub(super::TOOLBAR_H + super::STATUS_H);
                self.set_footer_height(size, content);
            }
            Splitter::Footer => {}
        }
        before != (self.inspector_w, self.footer_h)
    }

    pub fn end_drag(&mut self) -> bool {
        self.drag.take().is_some()
    }

    /// Next focus stop in reading order; wraps through "no focus".
    pub fn tab(&mut self, forward: bool, has_inspector: bool) {
        let mut order = vec![Focus::Segmented];
        if has_inspector {
            order.extend((0..SECTIONS).map(Focus::Section));
        }
        let at = self.focus.and_then(|f| order.iter().position(|o| *o == f));
        let next = match (at, forward) {
            (None, true) => Some(0),
            (None, false) => Some(order.len() - 1),
            (Some(i), true) => (i + 1 < order.len()).then_some(i + 1),
            (Some(i), false) => i.checked_sub(1),
        };
        self.focus = next.map(|i| order[i]);
    }
}
