use crate::chrome::{ChromeText, Inspector};
use crate::history::Availability;
use crate::layout::{Button, Focus, INSPECTOR_MIN_WINDOW, SECTIONS, Splitter, UiState, contains};
use crate::session::AppDocument;
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
    /// A row of the Export popover.
    MenuItem(ExportFormat),
    SectionHeader(usize),
    Splitter(Splitter),
}

/// Export file formats offered by the Export popover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Gif,
    Svg,
    Png,
}
impl ExportFormat {
    pub const ALL: [Self; 3] = [Self::Gif, Self::Svg, Self::Png];
    pub fn label(self) -> &'static str {
        match self {
            Self::Gif => "Export as GIF\u{2026}",
            Self::Svg => "Export as SVG\u{2026}",
            Self::Png => "Export as PNG\u{2026}",
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Gif => "gif",
            Self::Svg => "svg",
            Self::Png => "png",
        }
    }
}

/// What the shell must execute after a toolbar press or key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonAction {
    Undo,
    Redo,
    Save,
    Export(ExportFormat),
}

/// A keyboard stop: the segmented control, a section header or a toolbar button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stop {
    Ui(Focus),
    Button(Button),
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
    /// Ready once directory grants exist; Active while a request or an
    /// export it started is still in flight.
    pub fn for_session(granted: bool, busy: bool) -> Self {
        match (granted, busy) {
            (false, _) => Self::Unavailable,
            (true, false) => Self::Ready,
            (true, true) => Self::Active,
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
    /// Outcome of the most recent MCP request: `None` before any request.
    pub mcp_last: Option<&'static str>,
    /// Panel sizes, collapsed sections, focus and popover (in memory only).
    pub ui: UiState,
    /// Animate footer: tracks, playback readout and interaction state.
    pub timeline: Timeline,
    /// Undo/redo availability, refreshed from the document by the shell.
    pub history: Availability,
    /// The Export popover is open.
    pub export_open: bool,
    /// Toolbar button holding keyboard focus.
    pub focus_button: Option<Button>,
    /// Export popover row holding keyboard focus (index into `ExportFormat::ALL`).
    pub menu_focus: Option<usize>,
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
            mcp_last: None,
            ui: UiState::default(),
            timeline: Timeline::default(),
            history: Availability::default(),
            export_open: false,
            focus_button: None,
            menu_focus: None,
        }
    }
    /// Layout for the current mode, UI state and selection.
    pub fn layout(&self, width: u32, height: u32) -> Option<Layout> {
        let mut ui = self.ui;
        ui.empty_selection = matches!(self.chrome.inspector, Inspector::NoSelection);
        Layout::with_state(width, height, self.mode, &ui)
    }
    /// Whether a control reacts to hover and clicks. Undo and Redo follow the
    /// document's history cursor; Save needs a modified document; Export a
    /// document; its GIF row an animation; the inspector toggle a window wide
    /// enough to hold the inspector.
    pub fn target_enabled(&self, layout: &Layout, target: Target) -> bool {
        let state = &self.chrome.state;
        match target {
            Target::Button(Button::Undo) => state.open && self.history.undo,
            Target::Button(Button::Redo) => state.open && self.history.redo,
            Target::Button(Button::Save) => state.open && state.dirty,
            Target::Button(Button::Export) => state.open,
            Target::Button(Button::ToggleInspector) => layout.width >= INSPECTOR_MIN_WINDOW,
            Target::Button(Button::Connect) => false,
            Target::MenuItem(ExportFormat::Gif) => state.open && state.loop_seconds > 0.0,
            Target::MenuItem(_) => state.open,
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
    /// Keyboard focus without a layout: the inspector toggle is a stop exactly
    /// when the inspector is shown (see [`Model::nav`]).
    pub fn focus_key(&mut self, code: u16, shift: bool, has_inspector: bool) -> bool {
        self.nav(code, shift, has_inspector, has_inspector)
    }
    /// Keyboard focus: Tab / Shift-Tab move through the stops (segmented
    /// control, enabled toolbar buttons, section headers), Left/Right change
    /// the focused segmented control, Space/Return toggle a focused section
    /// header, Escape clears focus. True when the key was consumed.
    pub fn nav(&mut self, code: u16, shift: bool, has_inspector: bool, toggle: bool) -> bool {
        let consumed = match (code, self.ui.focus) {
            (48, _) => {
                self.tab(!shift, has_inspector, toggle);
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
            (53, None) => self.focus_button.take().is_some(),
            _ => false,
        };
        self.dirty |= consumed;
        consumed
    }
    fn button_enabled(&self, button: Button, toggle: bool) -> bool {
        let state = &self.chrome.state;
        match button {
            Button::Undo => state.open && self.history.undo,
            Button::Redo => state.open && self.history.redo,
            Button::Save => state.open && state.dirty,
            Button::Export => state.open,
            Button::ToggleInspector => toggle,
            Button::Mcp | Button::Connect => false,
        }
    }
    fn tab(&mut self, forward: bool, has_inspector: bool, toggle: bool) {
        let mut order = vec![Stop::Ui(Focus::Segmented)];
        order.extend(
            [
                Button::Undo,
                Button::Redo,
                Button::Save,
                Button::Export,
                Button::ToggleInspector,
            ]
            .into_iter()
            .filter(|b| self.button_enabled(*b, toggle))
            .map(Stop::Button),
        );
        if has_inspector {
            order.extend((0..SECTIONS).map(|i| Stop::Ui(Focus::Section(i))));
        }
        let current = self
            .focus_button
            .map(Stop::Button)
            .or(self.ui.focus.map(Stop::Ui));
        let at = current.and_then(|f| order.iter().position(|o| *o == f));
        let next = match (at, forward) {
            (None, true) => Some(0),
            (None, false) => Some(order.len() - 1),
            (Some(i), true) => (i + 1 < order.len()).then_some(i + 1),
            (Some(i), false) => i.checked_sub(1),
        };
        self.export_open = false;
        self.menu_focus = None;
        self.ui.focus = None;
        self.focus_button = None;
        match next.map(|i| order[i]) {
            Some(Stop::Ui(f)) => self.ui.focus = Some(f),
            Some(Stop::Button(b)) => self.focus_button = Some(b),
            None => {}
        }
    }
    /// Motion-registry id of the control holding button or menu focus.
    pub fn focus_id(&self) -> Option<u32> {
        use crate::motion::control_id;
        match (self.menu_focus, self.focus_button) {
            (Some(i), _) if self.export_open => ExportFormat::ALL
                .get(i)
                .map(|f| control_id(Target::MenuItem(*f))),
            (_, Some(b)) => Some(control_id(Target::Button(b))),
            _ => None,
        }
    }
    /// Refreshes undo/redo availability from the session; true on change.
    pub fn sync_history(&mut self, doc: &AppDocument) -> bool {
        let next = Availability::of(doc);
        let changed = next != self.history;
        self.history = next;
        self.dirty |= changed;
        changed
    }
    /// Activates a toolbar button exactly as a click does. Undo, Redo and Save
    /// come back as an action for the shell to run; Export toggles its
    /// popover, the toggle flips the inspector, MCP toggles its popover.
    /// Disabled buttons do nothing.
    pub fn button(&mut self, button: Button, layout: &Layout) -> Option<ButtonAction> {
        if button != Button::Mcp && !self.target_enabled(layout, Target::Button(button)) {
            return None;
        }
        let action = match button {
            Button::Undo => Some(ButtonAction::Undo),
            Button::Redo => Some(ButtonAction::Redo),
            Button::Save => Some(ButtonAction::Save),
            Button::Export => {
                self.export_open = !self.export_open;
                self.ui.mcp_open = false;
                self.menu_focus = None;
                None
            }
            Button::ToggleInspector => {
                self.ui.inspector_visible = !self.ui.inspector_visible;
                None
            }
            Button::Mcp => {
                self.ui.mcp_open = !self.ui.mcp_open;
                self.export_open = false;
                None
            }
            Button::Connect => return None,
        };
        self.dirty = true;
        action
    }
    /// An Export popover row; closes the popover when it fires.
    pub fn menu_item(&mut self, format: ExportFormat, layout: &Layout) -> Option<ButtonAction> {
        if !self.export_open || !self.target_enabled(layout, Target::MenuItem(format)) {
            return None;
        }
        self.export_open = false;
        self.menu_focus = None;
        self.dirty = true;
        Some(ButtonAction::Export(format))
    }
    /// Space/Return on the focused button or popover row.
    pub fn activate_focused(&mut self, layout: &Layout) -> Option<ButtonAction> {
        if self.export_open {
            let format = *ExportFormat::ALL.get(self.menu_focus?)?;
            let action = self.menu_item(format, layout);
            if action.is_some() {
                self.focus_button = Some(Button::Export);
            }
            return action;
        }
        let action = self.button(self.focus_button?, layout);
        if self.export_open {
            self.menu_focus = ExportFormat::ALL
                .iter()
                .position(|f| self.target_enabled(layout, Target::MenuItem(*f)));
        }
        action
    }
    /// Up/Down inside the open Export popover moves the row focus.
    pub fn menu_nav(&mut self, down: bool, layout: &Layout) -> bool {
        if !self.export_open {
            return false;
        }
        let rows: Vec<usize> = (0..ExportFormat::ALL.len())
            .filter(|i| self.target_enabled(layout, Target::MenuItem(ExportFormat::ALL[*i])))
            .collect();
        if rows.is_empty() {
            return true;
        }
        let at = self
            .menu_focus
            .and_then(|f| rows.iter().position(|r| *r == f));
        let next = match (at, down) {
            (None, true) => rows[0],
            (None, false) => rows[rows.len() - 1],
            (Some(i), true) => rows[(i + 1) % rows.len()],
            (Some(i), false) => rows[(i + rows.len() - 1) % rows.len()],
        };
        self.menu_focus = Some(next);
        self.dirty = true;
        true
    }
    /// Escape closes the Export popover and returns focus to its button.
    pub fn close_export(&mut self) -> bool {
        if !self.export_open {
            return false;
        }
        self.export_open = false;
        self.menu_focus = None;
        self.focus_button = Some(Button::Export);
        self.dirty = true;
        true
    }
    /// Whether `point` is inside an open popover (such clicks never reach the
    /// chrome or canvas beneath).
    pub fn popover_hit(&self, layout: &Layout, point: (u32, u32)) -> bool {
        (self.ui.mcp_open && contains(layout.mcp_popover, point))
            || (self.export_open && contains(layout.export_popover(), point))
    }
    pub fn pointer(&mut self, layout: &Layout, point: (u32, u32)) -> bool {
        self.pointer_action(layout, point).0
    }
    /// A primary press at `point`: (redraw needed, action for the shell).
    pub fn pointer_action(
        &mut self,
        layout: &Layout,
        point: (u32, u32),
    ) -> (bool, Option<ButtonAction>) {
        let (mcp, export) = (self.ui.mcp_open, self.export_open);
        let inside_popover = self.popover_hit(layout, point);
        let target = layout.hit_test_menus(point, mcp, export);
        let mut action = None;
        let mut changed = self.focus_button.take().is_some() | self.menu_focus.take().is_some();
        changed |= match target {
            Some(Target::DrawMode) => self.select_mode(Mode::Draw),
            Some(Target::AnimateMode) => self.select_mode(Mode::Animate),
            Some(Target::SectionHeader(i)) => {
                self.ui.focus = Some(Focus::Section(i));
                self.ui.toggle_section(i)
            }
            Some(Target::Button(button)) => {
                let before = (
                    self.export_open,
                    self.ui.mcp_open,
                    self.ui.inspector_visible,
                );
                action = self.button(button, layout);
                action.is_some()
                    || before
                        != (
                            self.export_open,
                            self.ui.mcp_open,
                            self.ui.inspector_visible,
                        )
            }
            Some(Target::MenuItem(format)) => {
                action = self.menu_item(format, layout);
                action.is_some()
            }
            _ => false,
        };
        let keeps_mcp = matches!(target, Some(Target::Button(Button::Mcp | Button::Connect)));
        if mcp && !inside_popover && !keeps_mcp {
            self.ui.mcp_open = false;
            changed = true;
        }
        if export && self.export_open && !inside_popover {
            self.export_open = false;
            changed = true;
        }
        self.dirty |= changed;
        (changed, action)
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
    fn mcp_state_transitions() {
        assert_eq!(McpState::for_session(false, false), McpState::Unavailable);
        assert_eq!(McpState::for_session(false, true), McpState::Unavailable);
        assert_eq!(McpState::for_session(true, false), McpState::Ready);
        assert_eq!(McpState::for_session(true, true), McpState::Active);
        let mut model = Model::new(true);
        assert_eq!(model.mcp, McpState::Unavailable);
        assert_eq!(model.mcp_last, None);
        model.mcp = McpState::for_session(true, false);
        assert_eq!(model.mcp.label(), "MCP on");
        model.mcp = McpState::for_session(true, true);
        assert_eq!(model.mcp.label(), "MCP busy");
        model.mcp_last = Some("Export needs an animation");
        assert_eq!(model.mcp_last, Some("Export needs an animation"));
    }
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
