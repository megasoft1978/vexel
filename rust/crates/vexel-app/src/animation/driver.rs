//! Window-free playback driver: one clock, one decoded scene per document
//! revision, a bounded queue of rendered frames.
//!
//! The shell calls [`Driver::tick`] from its event loop; keyboard, MCP and the
//! timeline strip all steer the same [`Playback`] through the driver. Playback
//! is not document state: nothing here touches the session's revision or
//! dirty flag.
//!
//! Frames are rendered synchronously, at most one per `tick`, by
//! [`AnimatedScene::render_sized`], so they use the compose view box and
//! aspect mapping and reach the screen through `compose::blit_document`.

use super::frame::{AnimatedScene, AnimationError, decode};
use super::playback::{FrameQueue, LoopClock, Playback};
use crate::compose::Frame;
use crate::session::PlaybackHost;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Seconds per presented frame (30 fps).
pub const FRAME_STEP: f64 = 1.0 / 30.0;
/// Rendered frames held before the oldest is dropped.
pub const QUEUE_CAPACITY: usize = 2;

/// One rendered frame, tagged with the document revision it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimFrame {
    pub revision: u64,
    pub time: f64,
    /// Straight-alpha RGBA at the driver's size, like a static compose frame.
    pub frame: Frame,
}

#[derive(Debug, PartialEq)]
pub enum TickOutcome {
    /// The document has no animation (or it could not be decoded).
    NoAnimation,
    /// Nothing new since the last presented frame.
    Unchanged,
    /// The newest frame; older queued frames were dropped.
    Frame(AnimFrame),
    /// The cancel flag was raised; nothing was presented.
    Cancelled,
    /// The renderer rejected the frame.
    Failed(AnimationError),
}

pub struct Driver<C: LoopClock> {
    playback: Playback<C>,
    scene: Option<AnimatedScene>,
    revision: Option<u64>,
    size: Option<(u32, u32)>,
    last_step: Option<i64>,
    queue: FrameQueue<AnimFrame>,
    cancel: Arc<AtomicBool>,
}

/// Frames of one loop (the last may be partial).
pub fn frame_count(loop_seconds: f64) -> i64 {
    ((loop_seconds / FRAME_STEP) - 1e-9).ceil().max(1.0) as i64
}

/// `seconds` snapped to the frame grid and kept inside `[0, loop_seconds]`.
pub fn quantize(seconds: f64, loop_seconds: f64) -> f64 {
    let snapped = (seconds / FRAME_STEP).round() * FRAME_STEP;
    snapped.clamp(0.0, loop_seconds.max(0.0))
}

fn step_of(time: f64) -> i64 {
    (time / FRAME_STEP).floor() as i64
}

impl<C: LoopClock> Driver<C> {
    pub fn new(clock: C) -> Self {
        Self {
            playback: Playback::new(clock, 0.0),
            scene: None,
            revision: None,
            size: None,
            last_step: None,
            queue: FrameQueue::new(QUEUE_CAPACITY),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn playback(&self) -> &Playback<C> {
        &self.playback
    }

    /// Raise this flag (from any thread) to cancel: the next render stops and
    /// queued frames are dropped.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_playing(&self) -> bool {
        self.playback.is_playing()
    }

    pub fn time(&self) -> f64 {
        self.playback.time()
    }

    pub fn loop_seconds(&self) -> f64 {
        self.playback.duration()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn is_looping(&self) -> bool {
        self.playback.is_looping()
    }

    /// Whether the timeline strip is shown: the document has a loop.
    pub fn strip_visible(&self) -> bool {
        self.loop_seconds() > 0.0
    }

    /// True when the scene for `revision` is already decoded.
    pub fn knows(&self, revision: u64) -> bool {
        self.revision == Some(revision)
    }

    /// Output size of rendered frames (the stage); `None` uses the document's.
    /// A change drops queued frames and forces a re-render.
    pub fn set_size(&mut self, size: (u32, u32)) {
        if self.size != Some(size) {
            self.size = Some(size);
            self.invalidate();
        }
    }

    /// Forgets what was last presented (the screen shows something else now).
    pub fn invalidate(&mut self) {
        self.last_step = None;
        while self.queue.pop().is_some() {}
    }

    /// Decodes `bytes` when `revision` differs from the cached scene: queued
    /// frames are dropped and the playback time is clamped into the new loop
    /// (paused at 0 when the loop vanished). True when the scene changed.
    pub fn sync(&mut self, revision: u64, bytes: &[u8]) -> bool {
        if self.knows(revision) {
            return false;
        }
        self.invalidate();
        self.scene = decode(bytes).ok();
        self.revision = Some(revision);
        let loop_seconds = self.scene.as_ref().map_or(0.0, AnimatedScene::loop_seconds);
        self.playback.set_duration(loop_seconds);
        true
    }

    /// Starts playing; false (and paused) when there is no animation.
    pub fn play(&mut self) -> bool {
        self.playback.play()
    }

    pub fn pause(&mut self) {
        self.playback.pause();
    }

    pub fn toggle(&mut self) -> bool {
        if self.is_playing() {
            self.pause();
            true
        } else {
            self.play()
        }
    }

    /// Scrubs to `seconds`, clamped to the loop; false for non-finite input.
    pub fn seek(&mut self, seconds: f64) -> bool {
        self.playback.seek(seconds)
    }

    /// Steps one frame on the 1/30 s grid. Playing or paused, it pauses first
    /// and stays paused. Wraps when looping, clamps to the loop otherwise.
    pub fn step(&mut self, forward: bool) -> bool {
        let loop_seconds = self.loop_seconds();
        if loop_seconds <= 0.0 {
            return false;
        }
        self.pause();
        let frames = frame_count(loop_seconds);
        let current = (self.time() / FRAME_STEP).round() as i64;
        let next = current + if forward { 1 } else { -1 };
        let next = if self.is_looping() {
            next.rem_euclid(frames)
        } else {
            next.clamp(0, frames)
        };
        self.seek(next as f64 * FRAME_STEP)
    }

    pub fn toggle_looping(&mut self) {
        let looping = !self.playback.is_looping();
        self.playback.set_looping(looping);
    }

    /// Renders at most one frame at the current playback time and returns the
    /// newest queued frame. `bytes` is only read when `revision` is new.
    pub fn tick(&mut self, revision: u64, bytes: &[u8]) -> TickOutcome {
        self.sync(revision, bytes);
        if self.scene.is_none() || self.loop_seconds() <= 0.0 {
            return TickOutcome::NoAnimation;
        }
        if self.cancel.swap(false, Ordering::SeqCst) {
            self.invalidate();
            return TickOutcome::Cancelled;
        }
        self.playback.settle();
        let time = self.time();
        if self.last_step != Some(step_of(time)) {
            let result = self.render(time);
            if self.cancelled() {
                return TickOutcome::Cancelled;
            }
            match result {
                Ok(()) => {}
                Err(AnimationError::Cancelled) => {
                    self.invalidate();
                    return TickOutcome::Cancelled;
                }
                Err(error) => {
                    self.last_step = Some(step_of(time));
                    return TickOutcome::Failed(error);
                }
            }
        }
        self.newest()
            .map_or(TickOutcome::Unchanged, TickOutcome::Frame)
    }

    fn cancelled(&mut self) -> bool {
        if self.cancel.swap(false, Ordering::SeqCst) {
            self.invalidate();
            return true;
        }
        false
    }

    /// Renders `time` into the queue (the oldest frame is dropped when full).
    fn render(&mut self, time: f64) -> Result<(), AnimationError> {
        let (Some(scene), Some(revision)) = (&self.scene, self.revision) else {
            return Err(AnimationError::NotDrawable);
        };
        let size = self.size.unwrap_or_else(|| scene.size());
        let tile = scene.render_sized(time, size, &self.cancel)?;
        let frame = Frame {
            width: size.0,
            height: size.1,
            rgba: tile.rgba().to_vec(),
            disclosures: Vec::new(),
        };
        self.queue.push(AnimFrame {
            revision,
            time,
            frame,
        });
        self.last_step = Some(step_of(time));
        Ok(())
    }

    /// Newest queued frame of the current revision; older ones are dropped.
    fn newest(&mut self) -> Option<AnimFrame> {
        let mut newest = None;
        while let Some(frame) = self.queue.pop() {
            if self.revision == Some(frame.revision) {
                newest = Some(frame);
            }
        }
        newest
    }
}

/// The production consumer of `AppDocument::request_frame`: decodes the
/// revision if new and queues a frame at `seconds`, without moving playback.
impl<C: LoopClock> PlaybackHost for Driver<C> {
    fn request_frame(&mut self, source: &[u8], revision: u64, seconds: f64) {
        self.sync(revision, source);
        if self.scene.is_some() && self.loop_seconds() > 0.0 && seconds.is_finite() {
            let _ = self.render(seconds.clamp(0.0, self.loop_seconds()));
        }
    }
}
