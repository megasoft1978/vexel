//! Hover, press and focus micro-transitions (design-direction section 4.5/4.6).
//!
//! Everything here is a pure function of `(start, now, duration, easing)`: no
//! timers, no heap, no clock reads. Time is an injected `u64` nanosecond
//! parameter ([`Clock`] is the optional source). The registry holds a fixed
//! array of [`MAX_CONTROLS`] controls; excess controls snap to their target
//! state. Pointer-driven drags are never animated: callers use
//! [`Transition::snap`] / [`ThumbSlide::snap_to`] for those.
use crate::Motion;
use std::time::Duration;

/// Fixed control bound of the registry.
pub const MAX_CONTROLS: usize = 64;
/// Redraw cadence while any transition is active.
pub const FRAME_NS: u64 = 16_000_000;
const MS: u64 = 1_000_000;

/// Injected monotonic nanosecond source.
pub trait Clock {
    fn now_ns(&self) -> u64;
}

/// The three curves of section 4.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    EaseOut,
    Standard,
    EaseIn,
}

impl Easing {
    pub fn points(self) -> [f32; 4] {
        let m = Motion::STANDARD;
        match self {
            Easing::EaseOut => m.ease_out,
            Easing::Standard => m.standard,
            Easing::EaseIn => m.ease_in,
        }
    }
    /// Progress for linear time `t` in 0..=1.
    pub fn eval(self, t: f32) -> f32 {
        bezier(self.points(), t as f64) as f32
    }
}

/// CSS-style cubic-bezier `y(x)`: Newton iterations with a bisection fallback.
pub fn bezier(p: [f32; 4], x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let (x1, y1, x2, y2) = (p[0] as f64, p[1] as f64, p[2] as f64, p[3] as f64);
    let axis = |a: f64, b: f64, s: f64| {
        ((1.0 - 3.0 * b + 3.0 * a) * s + (3.0 * b - 6.0 * a)) * s * s + 3.0 * a * s
    };
    let slope = |a: f64, b: f64, s: f64| {
        3.0 * (1.0 - 3.0 * b + 3.0 * a) * s * s + 2.0 * (3.0 * b - 6.0 * a) * s + 3.0 * a
    };
    let mut s = x;
    let mut solved = false;
    for _ in 0..8 {
        let err = axis(x1, x2, s) - x;
        if err.abs() < 1e-7 {
            solved = true;
            break;
        }
        let d = slope(x1, x2, s);
        if d.abs() < 1e-6 {
            break;
        }
        s -= err / d;
    }
    if !solved || !(0.0..=1.0).contains(&s) {
        let (mut lo, mut hi) = (0.0, 1.0);
        s = x;
        for _ in 0..48 {
            if axis(x1, x2, s) < x {
                lo = s;
            } else {
                hi = s;
            }
            s = (lo + hi) * 0.5;
        }
    }
    axis(y1, y2, s)
}

/// Scalar transition from a value to a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    from: f32,
    to: f32,
    start_ns: u64,
    duration_ns: u64,
    easing: Easing,
}

impl Transition {
    pub const fn at(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            start_ns: 0,
            duration_ns: 0,
            easing: Easing::EaseOut,
        }
    }
    /// No animation: the value is `to` immediately.
    pub fn snap(&mut self, to: f32) {
        *self = Self::at(to);
    }
    /// Continue from the current value toward `to`. A zero duration snaps.
    pub fn retarget(&mut self, now: u64, to: f32, duration_ns: u64, easing: Easing) {
        let from = self.value(now);
        if duration_ns == 0 {
            return self.snap(to);
        }
        *self = Self {
            from,
            to,
            start_ns: now,
            duration_ns,
            easing,
        };
    }
    pub fn target(&self) -> f32 {
        self.to
    }
    pub fn value(&self, now: u64) -> f32 {
        if self.duration_ns == 0 || now >= self.start_ns.saturating_add(self.duration_ns) {
            return self.to;
        }
        let t = now.saturating_sub(self.start_ns) as f32 / self.duration_ns as f32;
        self.from + (self.to - self.from) * self.easing.eval(t)
    }
    pub fn remaining_ns(&self, now: u64) -> u64 {
        if self.from == self.to {
            return 0;
        }
        self.start_ns
            .saturating_add(self.duration_ns)
            .saturating_sub(now)
    }
}

/// Packed `0xRRGGBBAA` channel interpolation.
pub fn lerp_color(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let mut out = 0u32;
    for shift in [0u32, 8, 16, 24] {
        let (x, y) = (((a >> shift) & 255) as f32, ((b >> shift) & 255) as f32);
        out |= ((x + (y - x) * t).round() as u32 & 255) << shift;
    }
    out
}

/// Colours one control resolves between; built from `Tokens` by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlColors {
    pub rest_fill: u32,
    pub hover_fill: u32,
    pub pressed_fill: u32,
    pub rest_glyph: u32,
    pub hover_glyph: u32,
    pub disabled_glyph: u32,
    pub ring: u32,
}

/// Interpolated, renderer-facing result for one control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlVisual {
    pub fill: u32,
    pub glyph: u32,
    /// Focus ring colour with its alpha scaled by `focus`.
    pub ring: u32,
    pub hover: f32,
    pub press: f32,
    pub focus: f32,
    pub disabled: bool,
}

impl ControlVisual {
    pub const EMPTY: ControlVisual = ControlVisual {
        fill: 0,
        glyph: 0,
        ring: 0,
        hover: 0.0,
        press: 0.0,
        focus: 0.0,
        disabled: false,
    };
    pub fn resolve(c: &ControlColors, hover: f32, press: f32, focus: f32, disabled: bool) -> Self {
        if disabled {
            return Self {
                fill: c.rest_fill,
                glyph: c.disabled_glyph,
                ring: 0,
                hover: 0.0,
                press: 0.0,
                focus: 0.0,
                disabled,
            };
        }
        let fill = lerp_color(
            lerp_color(c.rest_fill, c.hover_fill, hover),
            c.pressed_fill,
            press,
        );
        let a = ((c.ring & 255) as f32 * focus.clamp(0.0, 1.0)).round() as u32;
        Self {
            fill,
            glyph: lerp_color(c.rest_glyph, c.hover_glyph, hover.max(press)),
            ring: (c.ring & !255) | a,
            hover,
            press,
            focus,
            disabled,
        }
    }
    /// Rest, hover, pressed, focus: nothing left to draw but the rest look.
    pub fn is_rest(&self) -> bool {
        self.hover == 0.0 && self.press == 0.0 && self.focus == 0.0
    }
}

#[derive(Debug, Clone, Copy)]
struct Slot {
    id: u32,
    hover: Transition,
    press: Transition,
    focus: Transition,
}

impl Slot {
    fn remaining(&self, now: u64) -> u64 {
        self.hover
            .remaining_ns(now)
            .max(self.press.remaining_ns(now))
            .max(self.focus.remaining_ns(now))
    }
    fn idle(&self, now: u64) -> bool {
        self.remaining(now) == 0
            && self.hover.target() == 0.0
            && self.press.target() == 0.0
            && self.focus.target() == 0.0
    }
}

/// Bounded motion registry keyed by control id (never 0).
#[derive(Debug, Clone)]
pub struct MotionRegistry {
    slots: [Option<Slot>; MAX_CONTROLS],
    hovered: Option<u32>,
    pressed: Option<u32>,
    focused: Option<u32>,
    reduced: bool,
}

impl MotionRegistry {
    pub fn new(reduced_motion: bool) -> Self {
        Self {
            slots: [None; MAX_CONTROLS],
            hovered: None,
            pressed: None,
            focused: None,
            reduced: reduced_motion,
        }
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced = reduced;
    }
    pub fn hovered(&self) -> Option<u32> {
        self.hovered
    }
    pub fn pressed(&self) -> Option<u32> {
        self.pressed
    }
    pub fn focused(&self) -> Option<u32> {
        self.focused
    }
    fn dur(&self, ms: u64) -> u64 {
        if self.reduced { 0 } else { ms * MS }
    }
    /// Slot for `id`, reclaiming a fully idle slot if the table is full.
    /// `None` means the control is untracked and snaps to its target state.
    fn slot(&mut self, id: u32, now: u64) -> Option<&mut Slot> {
        let index = match self
            .slots
            .iter()
            .position(|s| s.is_some_and(|s| s.id == id))
        {
            Some(i) => i,
            None => {
                let free = self
                    .slots
                    .iter()
                    .position(|s| s.is_none_or(|s| s.idle(now)))?;
                let on = |v: Option<u32>| Transition::at(if v == Some(id) { 1.0 } else { 0.0 });
                self.slots[free] = Some(Slot {
                    id,
                    hover: on(self.hovered),
                    press: on(self.pressed),
                    focus: on(self.focused),
                });
                free
            }
        };
        self.slots[index].as_mut()
    }
    /// Pointer is over control `over` (or none). Hover in 120 ms ease-out, out 90 ms ease-in.
    pub fn pointer_move(&mut self, over: Option<u32>, now: u64) {
        if self.hovered == over {
            return;
        }
        let (d_in, d_out) = (self.dur(Motion::STANDARD.fast_ms as u64), self.dur(90));
        if let Some(old) = self.hovered.take() {
            if let Some(s) = self.slot(old, now) {
                s.hover.retarget(now, 0.0, d_out, Easing::EaseIn);
            }
        }
        if let Some(id) = over {
            if let Some(s) = self.slot(id, now) {
                s.hover.retarget(now, 1.0, d_in, Easing::EaseOut);
            }
        }
        self.hovered = over;
    }
    /// Pressed target is set in this call; the 80 ms ramp only affects colour.
    pub fn pointer_down(&mut self, over: Option<u32>, now: u64) {
        self.pointer_move(over, now);
        self.pointer_up(now);
        let d = self.dur(Motion::STANDARD.instant_ms as u64);
        if let Some(id) = over {
            if let Some(s) = self.slot(id, now) {
                s.press.retarget(now, 1.0, d, Easing::EaseOut);
            }
        }
        self.pressed = over;
    }
    /// Release in 120 ms.
    pub fn pointer_up(&mut self, now: u64) {
        let d = self.dur(Motion::STANDARD.fast_ms as u64);
        if let Some(id) = self.pressed.take() {
            if let Some(s) = self.slot(id, now) {
                s.press.retarget(now, 0.0, d, Easing::EaseOut);
            }
        }
    }
    /// Keyboard focus moves to `id` (or away): ring fades 120 ms.
    pub fn focus(&mut self, id: Option<u32>, now: u64) {
        if self.focused == id {
            return;
        }
        let d = self.dur(Motion::STANDARD.fast_ms as u64);
        if let Some(old) = self.focused.take() {
            if let Some(s) = self.slot(old, now) {
                s.focus.retarget(now, 0.0, d, Easing::EaseOut);
            }
        }
        if let Some(id) = id {
            if let Some(s) = self.slot(id, now) {
                s.focus.retarget(now, 1.0, d, Easing::EaseOut);
            }
        }
        self.focused = id;
    }
    /// Target (non-animated) state of a control.
    pub fn target_state(&self, id: u32, disabled: bool) -> crate::ControlState {
        use crate::ControlState as S;
        if disabled {
            S::Disabled
        } else if self.pressed == Some(id) {
            S::Pressed
        } else if self.hovered == Some(id) {
            S::Hover
        } else if self.focused == Some(id) {
            S::Focus
        } else {
            S::Rest
        }
    }
    pub fn visual(
        &self,
        id: u32,
        now: u64,
        colors: &ControlColors,
        disabled: bool,
    ) -> ControlVisual {
        let (h, p, f) = match self.slots.iter().flatten().find(|s| s.id == id) {
            Some(s) => (s.hover.value(now), s.press.value(now), s.focus.value(now)),
            None => {
                let on = |v: Option<u32>| if v == Some(id) { 1.0 } else { 0.0 };
                (on(self.hovered), on(self.pressed), on(self.focused))
            }
        };
        ControlVisual::resolve(colors, h, p, f, disabled)
    }
    /// One frame for every tracked control: no allocation, fixed array only.
    pub fn visuals_into(
        &self,
        now: u64,
        colors: &ControlColors,
        out: &mut [(u32, ControlVisual); MAX_CONTROLS],
    ) -> usize {
        let mut n = 0;
        for s in self.slots.iter().flatten() {
            out[n] = (
                s.id,
                ControlVisual::resolve(
                    colors,
                    s.hover.value(now),
                    s.press.value(now),
                    s.focus.value(now),
                    false,
                ),
            );
            n += 1;
        }
        n
    }
    /// Time to the next required redraw: `None` when nothing animates.
    pub fn next_deadline(&self, now: u64) -> Option<Duration> {
        let left = self
            .slots
            .iter()
            .flatten()
            .map(|s| s.remaining(now))
            .filter(|r| *r > 0)
            .min()?;
        Some(Duration::from_nanos(left.min(FRAME_NS)))
    }
}

/// Segmented-control thumb position (180 ms standard curve).
#[derive(Debug, Clone, Copy)]
pub struct ThumbSlide {
    x: Transition,
    reduced: bool,
}

impl ThumbSlide {
    pub fn new(x: f32, reduced_motion: bool) -> Self {
        Self {
            x: Transition::at(x),
            reduced: reduced_motion,
        }
    }
    pub fn slide_to(&mut self, now: u64, x: f32) {
        let d = if self.reduced {
            0
        } else {
            Motion::STANDARD.base_ms as u64 * MS
        };
        self.x.retarget(now, x, d, Easing::Standard);
    }
    /// Pointer-driven drags are never animated.
    pub fn snap_to(&mut self, x: f32) {
        self.x.snap(x);
    }
    pub fn x(&self, now: u64) -> f32 {
        self.x.value(now)
    }
    pub fn is_animating(&self, now: u64) -> bool {
        self.x.remaining_ns(now) > 0
    }
}
