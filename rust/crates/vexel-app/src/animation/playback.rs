//! Loop playback state on an injected clock, plus a bounded frame queue.
//!
//! No wall clock is read here: callers supply a [`LoopClock`]. Production
//! code uses [`MonotonicClock`]; tests use [`ManualClock`].

// `Playback`, `FrameQueue` and `ManualClock` are exercised by the headless
// tests; the preview window still drives playback from `PreviewState`, so they
// are not yet referenced from the non-test build.
#![cfg_attr(not(test), allow(dead_code))]

use std::cell::Cell;
use std::collections::VecDeque;
use std::time::Instant;

/// Monotonic seconds source.
pub trait LoopClock {
    fn now_seconds(&self) -> f64;
}

/// Production clock: seconds since construction on `Instant`.
pub struct MonotonicClock(Instant);

impl MonotonicClock {
    pub fn new() -> Self {
        Self(Instant::now())
    }
}

impl LoopClock for MonotonicClock {
    fn now_seconds(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

/// Deterministic clock advanced by hand.
#[derive(Default)]
pub struct ManualClock(Cell<f64>);

impl ManualClock {
    pub fn advance(&self, seconds: f64) {
        self.0.set(self.0.get() + seconds);
    }
}

impl LoopClock for ManualClock {
    fn now_seconds(&self) -> f64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Paused { at: f64 },
    Playing { clock_start: f64, time_start: f64 },
}

/// Play/pause/scrub over a loop of `duration` seconds. While playing, time
/// wraps into `[0, duration)`; while paused it holds the scrubbed value in
/// `[0, duration]` (so the exact end frame can be inspected).
pub struct Playback<C: LoopClock> {
    clock: C,
    duration: f64,
    mode: Mode,
}

impl<C: LoopClock> Playback<C> {
    pub fn new(clock: C, duration: f64) -> Self {
        let duration = if duration.is_finite() && duration > 0.0 {
            duration
        } else {
            0.0
        };
        Self {
            clock,
            duration,
            mode: Mode::Paused { at: 0.0 },
        }
    }

    pub fn clock(&self) -> &C {
        &self.clock
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.mode, Mode::Playing { .. })
    }

    /// Current loop time in seconds.
    pub fn time(&self) -> f64 {
        match self.mode {
            Mode::Paused { at } => at,
            Mode::Playing {
                clock_start,
                time_start,
            } => {
                let elapsed = (self.clock.now_seconds() - clock_start).max(0.0);
                (time_start + elapsed).rem_euclid(self.duration)
            }
        }
    }

    /// Starts (or continues) playing. Returns false when there is no loop.
    pub fn play(&mut self) -> bool {
        if self.duration <= 0.0 {
            return false;
        }
        if !self.is_playing() {
            let time_start = self.time().rem_euclid(self.duration);
            self.mode = Mode::Playing {
                clock_start: self.clock.now_seconds(),
                time_start,
            };
        }
        true
    }

    pub fn pause(&mut self) {
        self.mode = Mode::Paused { at: self.time() };
    }

    /// Scrubs to `seconds` (clamped to `[0, duration]`), keeping play state.
    /// Returns false for non-finite input, leaving state unchanged.
    pub fn seek(&mut self, seconds: f64) -> bool {
        if !seconds.is_finite() {
            return false;
        }
        let at = seconds.clamp(0.0, self.duration);
        self.mode = if self.is_playing() {
            Mode::Playing {
                clock_start: self.clock.now_seconds(),
                time_start: at.rem_euclid(self.duration),
            }
        } else {
            Mode::Paused { at }
        };
        true
    }
}

/// Fixed-capacity queue of rendered frames; pushing onto a full queue drops
/// the oldest so a slow presenter never grows memory.
pub struct FrameQueue<T> {
    items: VecDeque<T>,
    capacity: usize,
}

impl<T> FrameQueue<T> {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Returns the frame dropped to make room, if any.
    pub fn push(&mut self, item: T) -> Option<T> {
        let dropped = (self.items.len() == self.capacity)
            .then(|| self.items.pop_front())
            .flatten();
        self.items.push_back(item);
        dropped
    }

    pub fn pop(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}
