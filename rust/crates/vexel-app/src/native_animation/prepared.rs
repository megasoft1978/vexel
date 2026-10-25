//! Everything a playback tick needs from one document revision, built once.
//!
//! The source copy, root viewport, decoded timing and the id table of the SVG
//! are fixed for a revision, so ticks reuse them instead of re-deriving them.
//! Any edit changes the state revision and the host drops the whole value.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use vexel_scene::SceneGraph;
use vexel_timeline::{DetachedTimeline, LoopMode};

use super::ahead::Ahead;

use crate::animation::native_drawing::{DrawingAsset, SvgGeometry};
use crate::animation::native_frame::{
    NativeFrameError, native_snapshot, render_native_frame_cached, render_native_frame_predecoded,
};
use crate::animation::native_properties::{NativePropertyError, SourceIndex};
use crate::compose::{DiffCache, Frame, SvgViewport, decode_owned, svg_viewport};
use crate::project_session::ProjectTimelineCandidate;
use crate::session::AppDocument;

pub struct Prepared {
    pub revision: u64,
    pub duration_ticks: u32,
    pub ticks_per_second: u32,
    pub loop_mode: LoopMode,
    source: Arc<Vec<u8>>,
    candidate: ProjectTimelineCandidate,
    /// Decoded structure of the candidate's timeline, so ticks skip the JSON decode.
    timeline: DetachedTimeline,
    /// Root viewport and id table: parsed on first render, so the worker pays
    /// for them and the UI thread never does.
    derived: OnceLock<Result<Derived, NativeFrameError>>,
    /// Previous frame for differential repainting; one render runs at a time.
    diff: Mutex<DiffCache>,
    /// Frames are for playback preview: they may resample cached layers.
    preview: AtomicBool,
    /// Decoded upcoming ticks while playing.
    ahead: Ahead,
}

struct Derived {
    viewport: SvgViewport,
    index: SourceIndex,
}

impl Prepared {
    /// `None` when the source or timeline cannot be read.
    pub fn build(document: &AppDocument, candidate: &ProjectTimelineCandidate) -> Option<Self> {
        let _prep = crate::profile::phase("native.prep");
        let (duration_ticks, ticks_per_second, loop_mode, timeline) = {
            let view = candidate.view().ok()?;
            (
                view.duration_ticks(),
                view.ticks_per_second(),
                view.loop_mode(),
                view.detach(),
            )
        };
        let source = Arc::new(document.source_bytes().ok()?);
        Some(Self {
            revision: document.state_revision(),
            duration_ticks,
            ticks_per_second,
            loop_mode,
            source,
            candidate: candidate.clone(),
            timeline,
            derived: OnceLock::new(),
            diff: Mutex::new(DiffCache::display()),
            preview: AtomicBool::new(false),
            ahead: Ahead::default(),
        })
    }

    /// Playback frames may resample cached layers; stills (paused, scrubbed,
    /// exported) stay exact. Takes effect with the next render.
    pub fn set_preview(&self, on: bool) {
        if self.preview.swap(on, Ordering::Relaxed) && !on {
            self.ahead.invalidate();
        }
    }

    pub fn set_ahead_enabled(&self, on: bool) {
        self.ahead.set_enabled(on);
    }

    /// Drops decoded-ahead ticks and cancels running decodes (new generation).
    pub fn cancel_ahead(&self) {
        self.ahead.invalidate();
    }

    pub(super) fn ahead(&self) -> &Ahead {
        &self.ahead
    }

    pub(super) fn repeats(&self) -> bool {
        self.loop_mode == LoopMode::Repeat
    }

    /// Patches and decodes `tick`; `None` when cancelled or invalid (the
    /// serial path then reports the error).
    pub(super) fn decode_tick(&self, tick: u32, cancel: &dyn Fn() -> bool) -> Option<SceneGraph> {
        let derived = self.derived().ok()?;
        let timeline = self.timeline.attach(self.candidate.timeline_bytes())?;
        let snapshot = native_snapshot(&self.source, Some(&derived.index), &timeline, tick).ok()?;
        decode_owned(Arc::new(snapshot), cancel).ok()
    }

    fn derived(&self) -> Result<&Derived, NativeFrameError> {
        self.derived
            .get_or_init(|| {
                let viewport = svg_viewport(self.source.clone(), &|| false)
                    .map_err(NativeFrameError::Compose)?;
                let index = SourceIndex::build(&self.source)
                    .map_err(|e: NativePropertyError| NativeFrameError::Property(e))?;
                Ok(Derived { viewport, index })
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Renders one integer tick at `size`.
    pub fn render(
        self: &Arc<Self>,
        tick: u32,
        size: (u32, u32),
    ) -> Result<Frame, NativeFrameError> {
        let derived = self.derived()?;
        let timeline = self
            .timeline
            .attach(self.candidate.timeline_bytes())
            .ok_or(NativeFrameError::InvalidTick)?;
        let assets: Vec<_> = self
            .candidate
            .assets()
            .iter()
            .map(|asset| DrawingAsset {
                name: &asset.name,
                media_type: &asset.media_type,
                source: &asset.source,
            })
            .collect();
        let viewport = derived.viewport;
        let mut cache = self
            .diff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let preview = self.preview.load(Ordering::Relaxed);
        cache.set_preview(preview);
        let graph = if preview {
            let graph = self.ahead.take(tick);
            self.ahead.plan(self, tick);
            graph
        } else {
            None
        };
        let geometry = SvgGeometry {
            width: f64::from(viewport.output_width),
            height: f64::from(viewport.output_height),
            view_box: [
                viewport.view_box.x,
                viewport.view_box.y,
                viewport.view_box.width,
                viewport.view_box.height,
            ],
        };
        if let Some(graph) = graph {
            return render_native_frame_predecoded(
                &graph, &timeline, &assets, tick, size.0, size.1, geometry, &mut cache,
            );
        }
        render_native_frame_cached(
            &self.source,
            Some(&derived.index),
            &timeline,
            &assets,
            tick,
            size.0,
            size.1,
            geometry,
            &mut cache,
        )
    }
}
