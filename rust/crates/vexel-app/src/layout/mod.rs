//! Pure window layout (design direction section 5): 44 px toolbar, 44 px tool
//! rail, fixed resizable inspector, footer slot for the timeline and a 24 px
//! status bar. No painting and no platform types live here.
//!
//! Minimum window: 720 x 480 points. Below 900 points wide the inspector is
//! absent. Splitters are 4 px strips; they are exempt from the 28 px target
//! minimum on their thin axis (their long axis is the whole panel edge).

mod inspector_ext;
mod menu;
pub mod paint;
mod state;
#[cfg(test)]
mod tests;
mod tooltip;

pub use inspector_ext::{
    EXT_COUNT, EXT_END, EXT_FIRST, ExtControl, ExtRow, STOP_SWATCHES, segments, stepper_buttons,
    stop_swatch,
};
pub use menu::draw_export_menu;
pub use paint::{draw_chrome, draw_chrome_with, draw_popover, status_hint, tooltip};
pub use state::{Focus, UiState};
pub use tooltip::{draw_tooltip, tooltip_rect};

use crate::model::{Mode, Target, Tool};
use vexel_ui::Rect;

pub const TOOLBAR_H: u32 = 44;
pub const RAIL_W: u32 = 44;
pub const STATUS_H: u32 = 24;
pub const SPLITTER: u32 = 4;
pub const STATUS_ROW_H: u32 = 16;
pub const MIN_W: u32 = 720;
pub const MIN_H: u32 = 480;
pub const MAX_AXIS: u32 = 4096;
pub const INSPECTOR_MIN: u32 = 240;
pub const INSPECTOR_MAX: u32 = 420;
pub const INSPECTOR_DEFAULT: u32 = 296;
/// Inspector is dropped below this window width.
pub const INSPECTOR_MIN_WINDOW: u32 = 900;
pub const FOOTER_MIN: u32 = 120;
pub const FOOTER_DEFAULT: u32 = 200;
pub const STAGE_MIN_W: u32 = 260;
pub const STAGE_MIN_H: u32 = 140;
pub const SECTIONS: usize = 5;
pub const SECTION_HEADER_H: u32 = 32;
/// Selection body height while it holds the empty-state card.
pub const EMPTY_CARD_BODY_H: u32 = 112;
/// Body heights of Selection, Fill, Stroke, Transform, Animation.
pub const SECTION_BODY_H: [u32; SECTIONS] = [68, 192, 166, 172, 52];
pub const SECTION_NAMES: [&str; SECTIONS] =
    ["Selection", "Fill", "Stroke", "Transform", "Animation"];
const EMPTY: Rect = Rect {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
};
const SEGMENT_W: u32 = 88;
const POPOVER: (u32, u32) = (288, 156);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Undo,
    Redo,
    Save,
    Export,
    ToggleInspector,
    /// The MCP indicator in the status bar (opens the popover).
    Mcp,
    /// Connect inside the MCP popover.
    Connect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Splitter {
    Inspector,
    Footer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub top: Rect,
    /// Segmented-control container; thumb target is the selected segment.
    pub segmented: Rect,
    pub draw_mode: Rect,
    pub animate_mode: Rect,
    pub doc_label: Rect,
    pub history_group: Rect,
    pub file_group: Rect,
    /// Undo, Redo, Save, Export, panel toggle.
    pub buttons: [Rect; 5],
    pub rail: Rect,
    pub tools: [Rect; 5],
    pub tool_divider: Rect,
    pub stage: Rect,
    pub inspector: Option<Rect>,
    pub inspector_splitter: Option<Rect>,
    pub footer: Option<Rect>,
    pub footer_splitter: Option<Rect>,
    /// Empty (zero size) when the section does not fit the inspector.
    pub section_headers: [Rect; SECTIONS],
    /// Empty when collapsed; clipped to the panel when it does not fit.
    pub section_bodies: [Rect; SECTIONS],
    pub status: Rect,
    pub status_hint: Rect,
    pub status_report: Rect,
    /// Zoom, size, save state; empty on narrow windows.
    pub status_readouts: [Rect; 3],
    pub mcp: Rect,
    pub mcp_popover: Rect,
    pub mcp_connect: Rect,
}

fn r(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Equal-width segment rects inside `container` (2 px padding), for N segments.
pub fn segment_rects(container: Rect, count: u32) -> Vec<Rect> {
    (0..count)
        .map(|i| {
            r(
                container.x + 2 + i * SEGMENT_W,
                container.y + 2,
                SEGMENT_W,
                container.height - 4,
            )
        })
        .collect()
}

impl Layout {
    /// Default panel sizes and section state.
    pub fn new(width: u32, height: u32, mode: Mode) -> Option<Self> {
        Self::with_state(width, height, mode, &UiState::default())
    }

    pub fn with_state(width: u32, height: u32, mode: Mode, ui: &UiState) -> Option<Self> {
        if !(MIN_W..=MAX_AXIS).contains(&width) || !(MIN_H..=MAX_AXIS).contains(&height) {
            return None;
        }
        let content_y = TOOLBAR_H;
        let content_h = height - TOOLBAR_H - STATUS_H;
        let inspector_on = width >= INSPECTOR_MIN_WINDOW && ui.inspector_visible;
        let max_inspector = (width - RAIL_W - SPLITTER - STAGE_MIN_W).min(INSPECTOR_MAX);
        let insp_w = ui.inspector_w.clamp(INSPECTOR_MIN, max_inspector);
        let (inspector, inspector_splitter, column_right) = if inspector_on {
            let x = width - insp_w;
            (
                Some(r(x, content_y, insp_w, content_h)),
                Some(r(x - SPLITTER, content_y, SPLITTER, content_h)),
                x - SPLITTER,
            )
        } else {
            (None, None, width)
        };
        let column_w = column_right - RAIL_W;
        let (stage_h, footer, footer_splitter) = if mode == Mode::Animate {
            let max_f = (content_h / 2).min(content_h - SPLITTER - STAGE_MIN_H);
            let f = ui.footer_h.clamp(FOOTER_MIN, max_f.max(FOOTER_MIN));
            let stage_h = content_h - f - SPLITTER;
            (
                stage_h,
                Some(r(RAIL_W, content_y + stage_h + SPLITTER, column_w, f)),
                Some(r(RAIL_W, content_y + stage_h, column_w, SPLITTER)),
            )
        } else {
            (content_h, None, None)
        };
        let stage = r(RAIL_W, content_y, column_w, stage_h);

        // Toolbar.
        let cx = width / 2;
        let segmented = r(cx - SEGMENT_W - 2, 6, SEGMENT_W * 2 + 4, 32);
        let seg = segment_rects(segmented, 2);
        let toggle = r(width - 16 - 32, 8, 32, 28);
        let file_group = r(toggle.x - 12 - 68, 6, 68, 32);
        let history_group = r(file_group.x - 12 - 68, 6, 68, 32);
        let buttons = [
            r(history_group.x + 2, 8, 32, 28),
            r(history_group.x + 34, 8, 32, 28),
            r(file_group.x + 2, 8, 32, 28),
            r(file_group.x + 34, 8, 32, 28),
            toggle,
        ];
        let doc_x = 16;

        // Rail: select, divider, then the shape tools.
        let tools = std::array::from_fn(|i| {
            let y = if i == 0 {
                content_y + 8
            } else {
                content_y + 8 + 32 + 13 + (i as u32 - 1) * 36
            };
            r(6, y, 32, 32)
        });
        let tool_divider = r(12, content_y + 8 + 32 + 6, RAIL_W - 24, 1);

        // Inspector sections.
        let mut headers = [EMPTY; SECTIONS];
        let mut bodies = [EMPTY; SECTIONS];
        if let Some(panel) = inspector {
            let bottom = panel.y + panel.height;
            let mut y = panel.y;
            for i in 0..SECTIONS {
                if ui.empty_selection && (1..=3).contains(&i) {
                    continue;
                }
                if y + SECTION_HEADER_H + 4 > bottom {
                    break;
                }
                headers[i] = r(panel.x, y, panel.width, SECTION_HEADER_H);
                y += SECTION_HEADER_H;
                if !ui.collapsed[i] {
                    let full = if ui.empty_selection && i == 0 {
                        EMPTY_CARD_BODY_H
                    } else {
                        SECTION_BODY_H[i]
                    };
                    let h = full.min(bottom - y);
                    bodies[i] = r(panel.x, y, panel.width, h);
                    y += full;
                }
            }
        }

        // Status bar.
        let status_y = height - STATUS_H;
        let wide = width >= 1000;
        let right = width - 12;
        // Hit rect reaches 4 px above the bar to meet the 28 px target minimum.
        let mcp = r(right - 80, status_y - 4, 80, STATUS_H + 4);
        let save = r(mcp.x - 12 - 72, status_y, 72, STATUS_H);
        let (zoom, size) = if wide {
            let size = r(save.x - 12 - 84, status_y, 84, STATUS_H);
            (r(size.x - 12 - 40, status_y, 40, STATUS_H), size)
        } else {
            (EMPTY, EMPTY)
        };
        let readouts_x = if wide { zoom.x } else { save.x };
        let report_w = 280u32.min(2 * readouts_x.saturating_sub(12 + cx).max(1));
        let report = r(cx - report_w / 2, status_y, report_w, STATUS_H);
        let hint = r(12, status_y, report.x.saturating_sub(12 + 12), STATUS_H);
        let pop = r(
            width - 12 - POPOVER.0,
            status_y - 8 - POPOVER.1,
            POPOVER.0,
            POPOVER.1,
        );
        Some(Self {
            width,
            height,
            top: r(0, 0, width, TOOLBAR_H),
            segmented,
            draw_mode: seg[0],
            animate_mode: seg[1],
            doc_label: r(doc_x, 6, segmented.x.saturating_sub(doc_x + 16), 32),
            history_group,
            file_group,
            buttons,
            rail: r(0, content_y, RAIL_W, content_h),
            tools,
            tool_divider,
            stage,
            inspector,
            inspector_splitter,
            footer,
            footer_splitter,
            section_headers: headers,
            section_bodies: bodies,
            status: r(0, status_y, width, STATUS_H),
            status_hint: hint,
            status_report: report,
            status_readouts: [zoom, size, save],
            mcp,
            mcp_popover: pop,
            mcp_connect: r(pop.x + 16, pop.y + pop.height - 16 - 28, 96, 28),
        })
    }

    /// A status-bar text row: every readout shares this one vertical band so
    /// all baselines agree.
    pub fn status_row(&self, r: Rect) -> Rect {
        Rect {
            y: self.status.y + (STATUS_H - STATUS_ROW_H) / 2,
            height: STATUS_ROW_H,
            ..r
        }
    }

    /// Text rects of the zoom, size, save and MCP readouts (empty ones on
    /// narrow windows keep zero width).
    pub fn status_texts(&self) -> [Rect; 4] {
        let [zoom, size, save] = self.status_readouts;
        let mcp = Rect {
            x: self.mcp.x + 14,
            width: self.mcp.width - 14,
            ..self.status
        };
        let mut rows = [zoom, size, save, mcp].map(|r| self.status_row(r));
        // The monospaced face sits about a point high in the same band.
        for r in &mut rows[..3] {
            r.y += 1;
        }
        rows
    }

    /// Screen rect of a hit target (the tooltip anchor).
    pub fn target_rect(&self, target: Target) -> Option<Rect> {
        Some(match target {
            Target::DrawMode => self.draw_mode,
            Target::NativeTimeline(_) => return None,
            Target::AnimateMode => self.animate_mode,
            Target::Tool(tool) => self.tools[tool as usize],
            Target::Button(Button::Mcp) => self.mcp,
            Target::Button(Button::Connect) => self.mcp_connect,
            Target::Button(b) => self.buttons[b as usize],
            Target::MenuItem(f) => self.export_items()[f as usize],
            Target::SectionHeader(i) if i < SECTIONS => *self.section_headers.get(i)?,
            Target::SectionHeader(i) => self.inspector_control_rect(i)?,
            Target::Splitter(Splitter::Inspector) => self.inspector_splitter?,
            Target::Splitter(Splitter::Footer) => self.footer_splitter?,
        })
    }

    /// Rect of the segment for `mode` (the sliding thumb's target).
    pub fn thumb_target(&self, mode: Mode) -> Rect {
        match mode {
            Mode::Draw => self.draw_mode,
            Mode::Animate => self.animate_mode,
        }
    }

    /// Hit-test in priority order. The MCP popover is only live when open.
    pub fn hit_test(&self, point: (u32, u32)) -> Option<Target> {
        self.hit_test_with(point, false)
    }

    pub fn hit_test_with(&self, point: (u32, u32), popover_open: bool) -> Option<Target> {
        self.hit_test_menus(point, popover_open, false)
    }

    /// [`Layout::hit_test`] with the MCP and/or Export popover live: points
    /// under a popover hit it (or one of its rows), never the chrome beneath.
    pub fn hit_test_menus(
        &self,
        point: (u32, u32),
        popover_open: bool,
        export_open: bool,
    ) -> Option<Target> {
        if export_open {
            for (format, rect) in crate::model::ExportFormat::ALL
                .iter()
                .zip(self.export_items())
            {
                if contains(rect, point) {
                    return Some(Target::MenuItem(*format));
                }
            }
            if contains(self.export_popover(), point) {
                return None;
            }
        }
        if popover_open {
            if contains(self.mcp_connect, point) {
                return Some(Target::Button(Button::Connect));
            }
            if contains(self.mcp_popover, point) {
                return None;
            }
        }
        for index in 5..EXT_END {
            let target = Target::SectionHeader(index);
            if self.target_rect(target).is_some_and(|r| contains(r, point)) {
                return Some(target);
            }
        }
        if contains(self.draw_mode, point) {
            return Some(Target::DrawMode);
        }
        if contains(self.animate_mode, point) {
            return Some(Target::AnimateMode);
        }
        const TOOLBAR: [Button; 5] = [
            Button::Undo,
            Button::Redo,
            Button::Save,
            Button::Export,
            Button::ToggleInspector,
        ];
        for (b, rect) in TOOLBAR.iter().zip(self.buttons) {
            if contains(rect, point) {
                return Some(Target::Button(*b));
            }
        }
        const ORDER: [Tool; 5] = [
            Tool::Select,
            Tool::Pen,
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::Line,
        ];
        for (tool, rect) in ORDER.iter().zip(self.tools) {
            if contains(rect, point) {
                return Some(Target::Tool(*tool));
            }
        }
        if contains(self.mcp, point) {
            return Some(Target::Button(Button::Mcp));
        }
        for (i, rect) in self.section_headers.iter().enumerate() {
            if contains(*rect, point) {
                return Some(Target::SectionHeader(i));
            }
        }
        if self.inspector_splitter.is_some_and(|s| contains(s, point)) {
            return Some(Target::Splitter(Splitter::Inspector));
        }
        if self.footer_splitter.is_some_and(|s| contains(s, point)) {
            return Some(Target::Splitter(Splitter::Footer));
        }
        None
    }

    fn inspector_control_rect(&self, index: usize) -> Option<Rect> {
        if (5..11).contains(&index) {
            return self.inspector_color_rect(false, (index - 5) as u8);
        }
        if (11..17).contains(&index) {
            return self.inspector_color_rect(true, (index - 11) as u8);
        }
        if (17..21).contains(&index) {
            return self.inspector_width_rect((index - 17) as u8);
        }
        self.ext_control_rect_at(index)
    }

    fn inspector_color_rect(&self, stroke: bool, index: u8) -> Option<Rect> {
        let section = if stroke { 2 } else { 1 };
        let body = *self.section_bodies.get(section)?;
        if body.height != SECTION_BODY_H[section] || index >= 6 {
            return None;
        }
        let row_y = body.y + if stroke { 78 } else { 42 };
        let size = 20;
        let gap = 8;
        let x = body.x + 12 + u32::from(index) * (size + gap);
        Some(Rect {
            x,
            y: row_y,
            width: size,
            height: size,
        })
    }

    fn inspector_width_rect(&self, index: u8) -> Option<Rect> {
        let body = *self.section_bodies.get(2)?;
        if body.height != SECTION_BODY_H[2] || index >= 4 {
            return None;
        }
        Some(Rect {
            x: body.x + 64 + u32::from(index) * 38,
            y: body.y + 106,
            width: 32,
            height: 22,
        })
    }
}

pub(crate) fn contains(r: Rect, p: (u32, u32)) -> bool {
    p.0 >= r.x
        && p.1 >= r.y
        && p.0 < r.x.saturating_add(r.width)
        && p.1 < r.y.saturating_add(r.height)
}
