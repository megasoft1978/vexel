use crate::chrome::{ChromeText, Inspector};
use crate::history::History;
use crate::layout::{Button, Focus, Splitter, UiState, contains};
use crate::timeline::Timeline;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Draw,
    Animate,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Pen,
    Rectangle,
    Ellipse,
    Line,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    DrawMode,
    AnimateMode,
    Tool(Tool),
    Button(Button),
    SectionHeader(usize),
    Splitter(Splitter),
}

pub use crate::layout::Layout;

pub fn checked_pixel_geometry(points: (u32, u32), scale: u32) -> Option<(u32, u32)> {
    if !(1..=3).contains(&scale) {
        return None;
    }
    let width = points.0.checked_mul(scale)?;
    let height = points.1.checked_mul(scale)?;
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return None;
    }
    let bytes = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    (bytes <= 64 * 1024 * 1024).then_some((width, height))
}

/// State of the MCP link, shown in the status bar by dot plus word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpState {
    /// This shell has no MCP link (the only state reachable today).
    Unavailable,
    Ready,
    Active,
}
impl McpState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unavailable => "MCP off",
            Self::Ready => "MCP on",
            Self::Active => "MCP busy",
        }
    }
    /// Popover line; the first state keeps the old activity-band wording.
    pub fn detail(self) -> &'static str {
        match self {
            Self::Unavailable => "Unavailable in this shell",
            Self::Ready => "Connected, idle",
            Self::Active => "Handling a request",
        }
    }
}

/// Pointer phase for splitter drags (kept free of platform types).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragPhase {
    Down,
    Move,
    Up,
}

#[derive(Debug)]
pub struct Model {
    pub mode: Mode,
    pub tool: Tool,
    pub dark: bool,
    pub dirty: bool,
    /// Shell notice shown in the status bar; empty when there is none.
    pub status: &'static str,
    /// Document-derived text, refreshed by the shell from the open document.
    pub chrome: ChromeText,
    pub mcp: McpState,
    /// Panel sizes, collapsed sections, focus and popover (in memory only).
    pub ui: UiState,
    /// Animate footer: tracks, playback readout and interaction state.
    pub timeline: Timeline,
    /// Redo cursor (see `history`).
    pub history: History,
}
impl Model {
    pub fn new(dark: bool) -> Self {
        Self {
            mode: Mode::Draw,
            tool: Tool::Select,
            dark,
            dirty: true,
            status: "",
            chrome: ChromeText::default(),
            mcp: McpState::Unavailable,
            ui: UiState::default(),
            timeline: Timeline::default(),
            history: History::default(),
        }
    }
    /// Layout for the current mode, UI state and selection.
    pub fn layout(&self, width: u32, height: u32) -> Option<Layout> {
        let mut ui = self.ui;
        ui.empty_selection = matches!(self.chrome.inspector, Inspector::NoSelection);
        Layout::with_state(width, height, self.mode, &ui)
    }
    /// Whether a control reacts to hover and clicks. Undo keeps today's rule
    /// (document open and modified); Redo needs a redoable step.
    pub fn target_enabled(&self, layout: &Layout, target: Target) -> bool {
        let state = &self.chrome.state;
        match target {
            Target::Button(Button::Undo | Button::Save) => state.open && state.dirty,
            Target::Button(Button::Redo) => state.open && self.history.can_redo(),
            Target::Button(Button::Export) => state.open,
            Target::Button(Button::ToggleInspector) => layout.inspector.is_some(),
            Target::Button(Button::Connect) => false,
            _ => true,
        }
    }
    pub fn select_mode(&mut self, mode: Mode) -> bool {
        if self.mode == mode {
            return false;
        }
        self.mode = mode;
        self.dirty = true;
        true
    }
    pub fn key(&mut self, code: u16) -> bool {
        match code {
            2 => self.select_mode(Mode::Draw),
            0 => self.select_mode(Mode::Animate),
            _ => false,
        }
    }
    /// Keyboard focus: Tab / Shift-Tab move through the stops, Left/Right
    /// change the focused segmented control, Space/Return toggle a focused
    /// section header, Escape clears focus. True when the key was consumed.
    pub fn focus_key(&mut self, code: u16, shift: bool, has_inspector: bool) -> bool {
        let consumed = match (code, self.ui.focus) {
            (48, _) => {
                self.ui.tab(!shift, has_inspector);
                true
            }
            (123, Some(Focus::Segmented)) => {
                self.select_mode(Mode::Draw);
                true
            }
            (124, Some(Focus::Segmented)) => {
                self.select_mode(Mode::Animate);
                true
            }
            (49 | 36, Some(Focus::Section(i))) => self.ui.toggle_section(i),
            (53, Some(_)) => {
                self.ui.focus = None;
                true
            }
            _ => false,
        };
        self.dirty |= consumed;
        consumed
    }
    pub fn pointer(&mut self, layout: &Layout, point: (u32, u32)) -> bool {
        let open = self.ui.mcp_open;
        let inside_popover = open && contains(layout.mcp_popover, point);
        let target = layout.hit_test_with(point, open);
        let mut changed = match target {
            Some(Target::DrawMode) => self.select_mode(Mode::Draw),
            Some(Target::AnimateMode) => self.select_mode(Mode::Animate),
            Some(Target::SectionHeader(i)) => {
                self.ui.focus = Some(Focus::Section(i));
                self.ui.toggle_section(i)
            }
            Some(Target::Button(Button::ToggleInspector)) => {
                self.ui.inspector_visible = !self.ui.inspector_visible;
                true
            }
            Some(Target::Button(Button::Mcp)) => {
                self.ui.mcp_open = !open;
                true
            }
            _ => false,
        };
        let keeps_popover = matches!(target, Some(Target::Button(Button::Mcp | Button::Connect)));
        if open && !inside_popover && !keeps_popover {
            self.ui.mcp_open = false;
            changed = true;
        }
        self.dirty |= changed;
        changed
    }
    /// Splitter drag; `layout` is the current layout, `window` its size.
    pub fn splitter(
        &mut self,
        phase: DragPhase,
        point: (i32, i32),
        layout: &Layout,
        window: (u32, u32),
    ) -> bool {
        let changed = match phase {
            DragPhase::Down => self.ui.begin_drag(layout, point),
            DragPhase::Move => self.ui.drag_to(point, window, self.mode),
            DragPhase::Up => self.ui.end_drag(),
        };
        self.dirty |= changed;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_workbench_keeps_stage_dominant_and_footer_docked() {
        let draw = Layout::new(1280, 820, Mode::Draw).unwrap();
        assert!(draw.inspector.is_some());
        assert!(draw.footer.is_none());
        assert!(draw.stage.width > 700 && draw.stage.height > 500);
        let animate = Layout::new(1280, 820, Mode::Animate).unwrap();
        assert!(animate.footer.is_some());
        assert!(animate.stage.width > 700);
    }
    #[test]
    fn compact_bounds_hide_inspector_and_keep_stage_valid() {
        // Pin updated: the minimum window is now 720x480 (was 440x300).
        let l = Layout::new(720, 480, Mode::Draw).unwrap();
        assert!(l.inspector.is_none());
        assert!(l.stage.width >= 260);
        assert!(Layout::new(719, 480, Mode::Draw).is_none());
        assert!(Layout::new(720, 479, Mode::Draw).is_none());
    }
    #[test]
    fn inspector_appears_from_900_points_wide() {
        // Pin updated: height no longer hides the inspector; sections clip.
        assert!(
            Layout::new(899, 600, Mode::Draw)
                .unwrap()
                .inspector
                .is_none()
        );
        let layout = Layout::new(900, 600, Mode::Draw).unwrap();
        let inspector = layout.inspector.expect("inspector at 900 wide");
        assert!(inspector.y + inspector.height <= layout.status.y);
        assert!(layout.stage.x + layout.stage.width <= inspector.x);
        let animate = Layout::new(900, 600, Mode::Animate).unwrap();
        assert!(animate.inspector.is_some());
        assert!(animate.stage.y + animate.stage.height <= animate.footer.unwrap().y);
    }
    #[test]
    fn compact_painted_control_geometry_stays_inside_panels_in_both_modes() {
        for mode in [Mode::Draw, Mode::Animate] {
            let compact = Layout::new(720, 480, mode).unwrap();
            for tool in compact.tools {
                assert!(tool.y >= compact.rail.y);
                assert!(tool.y + tool.height <= compact.rail.y + compact.rail.height);
            }
            if let Some(footer) = compact.footer {
                assert!(footer.y + footer.height <= compact.status.y);
            }
        }
    }
    #[test]
    fn mode_selection_works_by_keyboard_pointer_and_is_idempotent() {
        let l = Layout::new(1280, 820, Mode::Draw).unwrap();
        let mut model = Model::new(true);
        assert!(model.key(0));
        assert_eq!(model.mode, Mode::Animate);
        model.dirty = false;
        assert!(!model.key(0));
        assert!(!model.dirty);
        assert!(model.pointer(&l, (l.draw_mode.x + 2, l.draw_mode.y + 2)));
        assert_eq!(model.mode, Mode::Draw);
    }
    #[test]
    fn rejects_unsupported_backing_scales_and_oversized_frames() {
        assert_eq!(checked_pixel_geometry((1280, 820), 2), Some((2560, 1640)));
        assert_eq!(checked_pixel_geometry((4097, 1), 1), None);
        assert_eq!(checked_pixel_geometry((1400, 820), 3), None);
        assert_eq!(checked_pixel_geometry((4096, 4096), 1), Some((4096, 4096)));
        assert_eq!(checked_pixel_geometry((100, 100), 2), Some((200, 200)));
        assert_eq!(checked_pixel_geometry((100, 100), 4), None);
    }
    #[test]
    fn pointer_hit_testing_is_deterministic_and_ignores_unavailable_actions() {
        let l = Layout::new(1280, 820, Mode::Draw).unwrap();
        assert_eq!(
            l.hit_test((l.tools[2].x + 3, l.tools[2].y + 3)),
            Some(Target::Tool(Tool::Rectangle))
        );
        let mut model = Model::new(true);
        assert!(!model.pointer(&l, (l.tools[2].x + 3, l.tools[2].y + 3)));
        assert_eq!(l.hit_test((0, 0)), None);
        assert_eq!(l.hit_test((l.stage.x + 20, l.stage.y + 20)), None);
        assert_eq!(
            l.hit_test((l.animate_mode.x, l.animate_mode.y)),
            Some(Target::AnimateMode)
        );
    }
    #[test]
    fn line_tool_is_the_fifth_rail_slot_and_fits_in_every_layout() {
        for mode in [Mode::Draw, Mode::Animate] {
            for (w, h) in [(720, 480), (900, 520), (1280, 820)] {
                let l = Layout::new(w, h, mode).unwrap();
                let line = l.tools[4];
                assert_eq!(
                    l.hit_test((line.x + 2, line.y + 2)),
                    Some(Target::Tool(Tool::Line))
                );
                for pair in l.tools.windows(2) {
                    assert!(
                        pair[0].y + pair[0].height <= pair[1].y,
                        "rail slots must not overlap"
                    );
                }
                assert!(line.y + line.height <= l.rail.y + l.rail.height);
            }
        }
    }
}
