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

use crate::model::{Layout, Mode, Target, Tool};
use std::time::{Duration, Instant};
use vexel_platform_apple::PointerKind;
use vexel_ui::motion::{Clock, ControlColors, ControlVisual, MotionRegistry};
use vexel_ui::{Theme, Tokens, theme_tokens};

pub const ID_DRAW: u32 = 1;
pub const ID_ANIMATE: u32 = 2;
const ID_TOOL0: u32 = 10;
/// Keyboard focus order: mode tabs, then the tool rail.
pub const FOCUS_ORDER: [u32; 7] = [ID_DRAW, ID_ANIMATE, 10, 11, 12, 13, 14];

pub fn control_id(target: Target) -> u32 {
    match target {
        Target::DrawMode => ID_DRAW,
        Target::AnimateMode => ID_ANIMATE,
        Target::Tool(tool) => ID_TOOL0 + tool as u32,
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
    let (rest_fill, hover_fill, pressed_fill) = if selected {
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
    items: [ControlVisual; 7],
}
impl MotionFrame {
    /// Visual of a known control (rest look for an unknown id).
    pub fn get(&self, id: u32) -> ControlVisual {
        FOCUS_ORDER
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
        let mut items = [ControlVisual::EMPTY; 7];
        for (slot, id) in items.iter_mut().zip(FOCUS_ORDER) {
            let selected = (id == ID_DRAW && mode == Mode::Draw)
                || (id == ID_ANIMATE && mode == Mode::Animate);
            *slot = registry.visual(id, now, &colors_for(&tokens, id, selected), false);
        }
        Self { items }
    }
}

pub struct MotionAdapter<C: Clock = SystemClock> {
    registry: MotionRegistry,
    clock: C,
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
        let now = self.clock.now_ns();
        let before = (
            self.registry.hovered(),
            self.registry.pressed(),
            self.registry.focused(),
        );
        let hit = (logical.0 >= 0 && logical.1 >= 0)
            .then(|| layout.hit_test((logical.0 as u32, logical.1 as u32)))
            .flatten();
        let over = hit.map(control_id);
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
        MotionFrame::build(&self.registry, self.clock.now_ns(), dark, mode)
    }
}
