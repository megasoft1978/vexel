//! Loop adapter for the UI micro-transitions: feeds pointer and keyboard
//! focus from the event loop into [`MotionRegistry`] and hands the renderer a
//! [`MotionFrame`] of interpolated [`ControlVisual`]s.
//!
//! Redraw cost model: events only set targets (O(1), no allocation). While
//! any transition is active [`MotionAdapter::redraw_after`] returns
//! `Some(<=16 ms)` and the loop marks the frame dirty; with nothing animating
//! it is `None`, so the loop redraws only for ordinary reasons. The transition
//! never gates the hit-test result: actions come from `Layout::hit_test`, not
//! from visual progress.
pub mod paint;

use crate::layout;
use crate::model::{Layout, Mode, Model, Target, Tool};
use std::time::{Duration, Instant};
use vexel_platform_apple::PointerKind;
use vexel_ui::motion::{Clock, ControlColors, ControlVisual, MotionRegistry};
use vexel_ui::{Theme, Tokens, theme_tokens};

pub const ID_DRAW: u32 = 1;
pub const ID_ANIMATE: u32 = 2;
const ID_TOOL0: u32 = 10;
/// Keyboard focus order: mode tabs, then the tool rail.
pub const FOCUS_ORDER: [u32; 7] = [ID_DRAW, ID_ANIMATE, 10, 11, 12, 13, 14];
const ID_BUTTON0: u32 = 20;
const ID_SECTION0: u32 = 40;
const ID_SPLITTER0: u32 = 60;
/// Section headers the registry can track.
pub const MAX_HEADERS: usize = 8;
/// The single registry id map: every id a [`Target`] maps to. Tab order is
/// [`FOCUS_ORDER`] (modes, tools); the rest react to hover and press only.
pub const REGISTERED: [u32; 24] = [
    ID_DRAW, ID_ANIMATE, 10, 11, 12, 13, 14, 20, 21, 22, 23, 24, 25, 26, 40, 41, 42, 43, 44, 45,
    46, 47, 60, 61,
];
/// Hover delay before a tooltip shows.
pub const TOOLTIP_DELAY_NS: u64 = 500_000_000;

pub fn control_id(target: Target) -> u32 {
    match target {
        Target::DrawMode => ID_DRAW,
        Target::AnimateMode => ID_ANIMATE,
        Target::Tool(tool) => ID_TOOL0 + tool as u32,
        Target::Button(button) => ID_BUTTON0 + button as u32,
        Target::SectionHeader(index) => ID_SECTION0 + index.min(MAX_HEADERS - 1) as u32,
        Target::Splitter(splitter) => ID_SPLITTER0 + splitter as u32,
    }
}
pub fn tool_id(tool: Tool) -> u32 {
    control_id(Target::Tool(tool))
}

/// Monotonic nanoseconds since construction.
pub struct SystemClock(Instant);
impl SystemClock {
    pub fn new() -> Self {
        Self(Instant::now())
    }
}
impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}
impl Clock for SystemClock {
    fn now_ns(&self) -> u64 {
        self.0.elapsed().as_nanos() as u64
    }
}

/// Colours for a control: segments sit on `panel_raised` (or the selection
/// fill), rail buttons carry a dimmed glyph until hovered.
pub fn colors_for(t: &Tokens, id: u32, selected: bool) -> ControlColors {
    let tab = id == ID_DRAW || id == ID_ANIMATE;
    let (rest_fill, hover_fill, pressed_fill) = if id >= ID_SPLITTER0 {
        (t.window, t.control_edge, t.accent)
    } else if id >= ID_SECTION0 {
        (t.panel, t.panel_hover, t.pressed_fill)
    } else if id == ID_BUTTON0 + 5 {
        (t.window, t.panel_hover, t.pressed_fill)
    } else if id >= ID_BUTTON0 {
        (t.surface[3], t.surface[4], t.surface[5])
    } else if selected {
        (
            t.selection_fill,
            vexel_ui::motion::lerp_color(t.selection_fill, t.text_primary, 0.08),
            vexel_ui::motion::lerp_color(t.selection_fill, t.text_primary, 0.16),
        )
    } else {
        (t.panel_raised, t.panel_hover, t.pressed_fill)
    };
    ControlColors {
        rest_fill,
        hover_fill,
        pressed_fill,
        rest_glyph: if tab { t.text_primary } else { t.disabled_text },
        hover_glyph: t.text_primary,
        disabled_glyph: t.disabled_text,
        ring: t.focus_ring,
    }
}

/// One frame of per-control visuals for the renderer (fixed size, no heap).
#[derive(Clone, Copy, Debug)]
pub struct MotionFrame {
    items: [ControlVisual; REGISTERED.len()],
    /// Tooltip to paint this frame (after the hover delay), if any.
    pub tooltip: Option<Target>,
}
impl MotionFrame {
    /// Visual of a known control (rest look for an unknown id).
    pub fn get(&self, id: u32) -> ControlVisual {
        REGISTERED
            .iter()
            .position(|f| *f == id)
            .map_or(ControlVisual::EMPTY, |i| self.items[i])
    }
    /// Every control at rest: the static look.
    pub fn rest(dark: bool, mode: Mode) -> Self {
        Self::build(&MotionRegistry::new(true), 0, dark, mode)
    }
    fn build(registry: &MotionRegistry, now: u64, dark: bool, mode: Mode) -> Self {
        let tokens = theme_tokens(if dark { Theme::Dark } else { Theme::Light }, false);
        let mut items = [ControlVisual::EMPTY; REGISTERED.len()];
        for (slot, id) in items.iter_mut().zip(REGISTERED) {
            let selected = (id == ID_DRAW && mode == Mode::Draw)
                || (id == ID_ANIMATE && mode == Mode::Animate);
            *slot = registry.visual(id, now, &colors_for(&tokens, id, selected), false);
        }
        Self {
            items,
            tooltip: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Tip {
    target: Option<Target>,
    since: u64,
    dismissed: bool,
    shown: bool,
}

pub struct MotionAdapter<C: Clock = SystemClock> {
    registry: MotionRegistry,
    clock: C,
    tip: Tip,
}

impl MotionAdapter<SystemClock> {
    /// `reduced_motion`: the platform layer has no accessor for
    /// `accessibilityDisplayShouldReduceMotion` yet, so `main` passes `false`.
    pub fn new(reduced_motion: bool) -> Self {
        Self::with_clock(SystemClock::new(), reduced_motion)
    }
}

impl<C: Clock> MotionAdapter<C> {
    pub fn with_clock(clock: C, reduced_motion: bool) -> Self {
        Self {
            registry: MotionRegistry::new(reduced_motion),
            clock,
            tip: Tip::default(),
        }
    }
    pub fn registry(&self) -> &MotionRegistry {
        &self.registry
    }
    /// Feed one pointer event. Returns the hit target (the action input,
    /// independent of transition progress) and whether the control state changed.
    pub fn pointer(
        &mut self,
        layout: &Layout,
        kind: PointerKind,
        logical: (i32, i32),
        primary: bool,
    ) -> (Option<Target>, bool) {
        self.pointer_inner(layout, kind, logical, primary, false, &|_| true)
    }
    /// [`MotionAdapter::pointer`] aware of the open MCP popover (points under
    /// it hit the popover, not the chrome beneath) and of disabled controls
    /// (they never hover or press).
    pub fn pointer_in(
        &mut self,
        layout: &Layout,
        kind: PointerKind,
        logical: (i32, i32),
        primary: bool,
        model: &Model,
    ) -> (Option<Target>, bool) {
        let enabled = |t: Target| model.target_enabled(layout, t);
        self.pointer_inner(layout, kind, logical, primary, model.ui.mcp_open, &enabled)
    }
    fn pointer_inner(
        &mut self,
        layout: &Layout,
        kind: PointerKind,
        logical: (i32, i32),
        primary: bool,
        popover_open: bool,
        enabled: &dyn Fn(Target) -> bool,
    ) -> (Option<Target>, bool) {
        let now = self.clock.now_ns();
        let before = (
            self.registry.hovered(),
            self.registry.pressed(),
            self.registry.focused(),
        );
        let hit = (logical.0 >= 0 && logical.1 >= 0)
            .then(|| layout.hit_test_with((logical.0 as u32, logical.1 as u32), popover_open))
            .flatten();
        let over = hit.filter(|t| enabled(*t)).map(control_id);
        if kind == PointerKind::Move || self.tip.target != hit {
            let tip = hit.filter(|t| !popover_open && layout::tooltip(*t).is_some());
            if tip != self.tip.target {
                self.tip = Tip {
                    target: tip,
                    since: now,
                    dismissed: false,
                    shown: self.tip.shown,
                };
            }
        }
        if kind == PointerKind::Down {
            self.tip.dismissed = true;
        }
        match kind {
            PointerKind::Move => self.registry.pointer_move(over, now),
            PointerKind::Down if primary => {
                self.registry.focus(None, now);
                self.registry.pointer_down(over, now);
            }
            PointerKind::Up if primary => self.registry.pointer_up(now),
            _ => {}
        }
        let after = (
            self.registry.hovered(),
            self.registry.pressed(),
            self.registry.focused(),
        );
        (hit, before != after)
    }
    /// Tab / Shift-Tab through [`FOCUS_ORDER`]. Always consumed.
    pub fn tab(&mut self, backward: bool) -> bool {
        let n = FOCUS_ORDER.len();
        let at = self
            .registry
            .focused()
            .and_then(|id| FOCUS_ORDER.iter().position(|f| *f == id));
        let next = match (at, backward) {
            (None, false) => 0,
            (None, true) => n - 1,
            (Some(i), false) => (i + 1) % n,
            (Some(i), true) => (i + n - 1) % n,
        };
        self.registry
            .focus(Some(FOCUS_ORDER[next]), self.clock.now_ns());
        true
    }
    /// `Some(delay)` while a transition needs another frame; `None` when idle.
    pub fn redraw_after(&self) -> Option<Duration> {
        self.registry.next_deadline(self.clock.now_ns())
    }
    pub fn frame(&self, dark: bool, mode: Mode) -> MotionFrame {
        let mut frame = MotionFrame::build(&self.registry, self.clock.now_ns(), dark, mode);
        frame.tooltip = self.tooltip();
        frame
    }
    /// The tooltip target once the hover delay has passed (also with reduced
    /// motion; there is no fade). Gone on press and when the pointer leaves.
    pub fn tooltip(&self) -> Option<Target> {
        let t = self.tip;
        let due = self.clock.now_ns().saturating_sub(t.since) >= TOOLTIP_DELAY_NS;
        t.target.filter(|_| due && !t.dismissed)
    }
    /// True when the tooltip appeared or vanished since the last call (the
    /// loop redraws then).
    pub fn poll_tooltip(&mut self) -> bool {
        let visible = self.tooltip().is_some();
        std::mem::replace(&mut self.tip.shown, visible) != visible
    }
}
