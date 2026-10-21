//! Clock-driven sampling and bounded frame publication for native timelines.

use super::native_drawing::{DrawingAsset, SvgGeometry};
use super::native_frame::render_native_frame;
use super::playback::{FrameQueue, LoopClock, Playback};
use crate::native_animation::{NativeFrameResult, NativeFrameTag, NativeRenderRequest};
use vexel_timeline::LoopMode;

pub const NATIVE_QUEUE_CAPACITY: usize = 2;

/// Playback wrapper that samples native timelines on their integer tick grid.
pub struct NativeDriver<C: LoopClock> {
    playback: Playback<C>,
    source_revision: Option<u64>,
    pub(super) duration_ticks: u32,
    pub(super) ticks_per_second: u32,
    pub(super) loop_mode: LoopMode,
    last_tag: Option<NativeFrameTag>,
    queue: FrameQueue<NativeFrameResult>,
}

impl<C: LoopClock> NativeDriver<C> {
    pub fn new(clock: C) -> Self {
        Self {
            playback: Playback::new(clock, 0.0),
            source_revision: None,
            duration_ticks: 0,
            ticks_per_second: 0,
            loop_mode: LoopMode::Once,
            last_tag: None,
            queue: FrameQueue::new(NATIVE_QUEUE_CAPACITY),
        }
    }

    pub fn playback(&self) -> &Playback<C> {
        &self.playback
    }

    pub fn is_playing(&self) -> bool {
        self.playback.is_playing()
    }

    pub fn time(&self) -> f64 {
        self.playback.time()
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn play(&mut self) -> bool {
        self.playback.play()
    }

    pub fn pause(&mut self) {
        self.playback.pause();
    }

    pub fn seek(&mut self, seconds: f64) -> bool {
        let changed = self.playback.seek(seconds);
        if changed {
            self.last_tag = None;
        }
        changed
    }

    pub fn invalidate(&mut self) {
        self.last_tag = None;
        while self.queue.pop().is_some() {}
    }

    /// Synchronize timing from a request, render its current integer sample,
    /// and return only a result matching the requested revision and geometry.
    pub fn tick(&mut self, mut request: NativeRenderRequest) -> Option<NativeFrameResult> {
        let timeline = request.timeline.view().ok()?;
        let duration_ticks = timeline.duration_ticks();
        let ticks_per_second = timeline.ticks_per_second();
        let loop_mode = timeline.loop_mode();
        self.sync_timing(
            request.state_revision,
            duration_ticks,
            ticks_per_second,
            loop_mode,
        );
        if duration_ticks == 0 || ticks_per_second == 0 {
            return None;
        }
        self.playback.settle();
        let tick = self.sample_tick(self.playback.time());
        request.tick = tick;
        let tag = NativeFrameTag {
            state_revision: request.state_revision,
            tick,
            width: request.output_width,
            height: request.output_height,
        };
        if self.last_tag != Some(tag) {
            let result = render_request(&request);
            self.queue.push(NativeFrameResult { tag, result });
            self.last_tag = Some(tag);
        }
        newest_matching(&mut self.queue, tag)
    }

    /// Adopts the timing of `state_revision`'s timeline, resetting queued frames
    /// when the revision or the timing changed. Playback is clocked from it, so
    /// play and seek work before the first rendered frame.
    pub fn sync_timing(
        &mut self,
        state_revision: u64,
        duration_ticks: u32,
        ticks_per_second: u32,
        loop_mode: LoopMode,
    ) {
        let revision_changed = self.source_revision != Some(state_revision);
        let timing_changed = duration_ticks != self.duration_ticks
            || ticks_per_second != self.ticks_per_second
            || loop_mode != self.loop_mode;
        if revision_changed || timing_changed {
            self.invalidate();
            self.source_revision = Some(state_revision);
            self.duration_ticks = duration_ticks;
            self.ticks_per_second = ticks_per_second;
            self.loop_mode = loop_mode;
            let seconds = duration_ticks as f64 / f64::from(ticks_per_second.max(1));
            let clock_duration = match loop_mode {
                LoopMode::PingPong if duration_ticks > 1 => {
                    f64::from(duration_ticks.saturating_sub(1) * 2) / f64::from(ticks_per_second)
                }
                _ => seconds,
            };
            self.playback.set_looping(loop_mode != LoopMode::Once);
            self.playback.set_duration(clock_duration);
            self.last_tag = None;
        }
    }

    /// Integer tick sampled at `seconds` under the synced timing.
    pub fn sample_tick(&self, seconds: f64) -> u32 {
        let duration = self.duration_ticks;
        let tps = f64::from(self.ticks_per_second);
        let raw = match self.loop_mode {
            LoopMode::PingPong => {
                if duration <= 1 {
                    0
                } else {
                    let cycle = duration.saturating_sub(1) * 2;
                    let phase = ((seconds.max(0.0) * tps).floor() as u32) % cycle;
                    if phase < duration {
                        phase
                    } else {
                        cycle - phase
                    }
                }
            }
            _ => (seconds.max(0.0) * tps).floor() as u32,
        };
        raw.min(duration.saturating_sub(1))
    }
}

fn newest_matching(
    queue: &mut FrameQueue<NativeFrameResult>,
    expected: NativeFrameTag,
) -> Option<NativeFrameResult> {
    let mut result = None;
    while let Some(candidate) = queue.pop() {
        if candidate.tag == expected {
            result = Some(candidate);
        }
    }
    result
}

fn render_request(
    request: &NativeRenderRequest,
) -> Result<crate::compose::Frame, super::native_frame::NativeFrameError> {
    let timeline = request
        .timeline
        .view()
        .map_err(|_| super::native_frame::NativeFrameError::InvalidTick)?;
    let assets: Vec<_> = request
        .timeline
        .assets()
        .iter()
        .map(|asset| DrawingAsset {
            name: &asset.name,
            media_type: &asset.media_type,
            source: &asset.source,
        })
        .collect();
    let viewport = request.viewport;
    render_native_frame(
        &request.source_svg,
        &timeline,
        &assets,
        request.tick,
        request.output_width,
        request.output_height,
        SvgGeometry {
            width: f64::from(viewport.output_width),
            height: f64::from(viewport.output_height),
            view_box: [
                viewport.view_box.x,
                viewport.view_box.y,
                viewport.view_box.width,
                viewport.view_box.height,
            ],
        },
    )
}

#[cfg(test)]
#[path = "native_driver_tests.rs"]
mod tests;
