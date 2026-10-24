//! Everything a playback tick needs from one document revision, built once.
//!
//! The source copy, root viewport, decoded timing and the id table of the SVG
//! are fixed for a revision, so ticks reuse them instead of re-deriving them.
//! Any edit changes the state revision and the host drops the whole value.

use std::sync::{Arc, Mutex, OnceLock};

use vexel_timeline::LoopMode;

use crate::animation::native_drawing::{DrawingAsset, SvgGeometry};
use crate::animation::native_frame::{NativeFrameError, render_native_frame_cached};
use crate::animation::native_properties::{NativePropertyError, SourceIndex};
use crate::compose::{DiffCache, Frame, SvgViewport, svg_viewport};
use crate::project_session::ProjectTimelineCandidate;
use crate::session::AppDocument;

pub struct Prepared {
    pub revision: u64,
    pub duration_ticks: u32,
    pub ticks_per_second: u32,
    pub loop_mode: LoopMode,
    source: Arc<Vec<u8>>,
    candidate: ProjectTimelineCandidate,
    /// Root viewport and id table: parsed on first render, so the worker pays
    /// for them and the UI thread never does.
    derived: OnceLock<Result<Derived, NativeFrameError>>,
    /// Previous frame for differential repainting; one render runs at a time.
    diff: Mutex<DiffCache>,
}

struct Derived {
    viewport: SvgViewport,
    index: SourceIndex,
}

impl Prepared {
    /// `None` when the source or timeline cannot be read.
    pub fn build(document: &AppDocument, candidate: &ProjectTimelineCandidate) -> Option<Self> {
        let _prep = crate::profile::phase("native.prep");
        let (duration_ticks, ticks_per_second, loop_mode) = {
            let view = candidate.view().ok()?;
            (
                view.duration_ticks(),
                view.ticks_per_second(),
                view.loop_mode(),
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
            derived: OnceLock::new(),
            diff: Mutex::default(),
        })
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
    pub fn render(&self, tick: u32, size: (u32, u32)) -> Result<Frame, NativeFrameError> {
        let derived = self.derived()?;
        let timeline = self
            .candidate
            .view()
            .map_err(|_| NativeFrameError::InvalidTick)?;
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
        render_native_frame_cached(
            &self.source,
            Some(&derived.index),
            &timeline,
            &assets,
            tick,
            size.0,
            size.1,
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
            &mut cache,
        )
    }
}
