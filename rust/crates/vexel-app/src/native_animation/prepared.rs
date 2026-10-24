//! Everything a playback tick needs from one document revision, built once.
//!
//! The source copy, root viewport, decoded timing and the id table of the SVG
//! are fixed for a revision, so ticks reuse them instead of re-deriving them.
//! Any edit changes the state revision and the host drops the whole value.

use std::sync::Arc;

use vexel_timeline::LoopMode;

use crate::animation::native_drawing::{DrawingAsset, SvgGeometry};
use crate::animation::native_frame::{NativeFrameError, render_native_frame_indexed};
use crate::animation::native_properties::SourceIndex;
use crate::compose::{Frame, SvgViewport, svg_viewport};
use crate::project_session::ProjectTimelineCandidate;
use crate::session::AppDocument;

pub struct Prepared {
    pub revision: u64,
    pub duration_ticks: u32,
    pub ticks_per_second: u32,
    pub loop_mode: LoopMode,
    source: Arc<Vec<u8>>,
    candidate: ProjectTimelineCandidate,
    viewport: SvgViewport,
    index: SourceIndex,
}

impl Prepared {
    /// `None` when the source or timeline cannot be read.
    pub fn build(document: &AppDocument, candidate: &ProjectTimelineCandidate) -> Option<Self> {
        let _prep = crate::profile::phase("native.prep");
        let (duration_ticks, ticks_per_second, loop_mode) = {
            let view = candidate.view().ok()?;
            (view.duration_ticks(), view.ticks_per_second(), view.loop_mode())
        };
        let source = Arc::new(document.source_bytes().ok()?);
        let viewport = svg_viewport(source.clone(), &|| false).ok()?;
        let index = SourceIndex::build(&source).ok()?;
        Some(Self {
            revision: document.state_revision(),
            duration_ticks,
            ticks_per_second,
            loop_mode,
            source,
            candidate: candidate.clone(),
            viewport,
            index,
        })
    }

    /// Renders one integer tick at `size`.
    pub fn render(&self, tick: u32, size: (u32, u32)) -> Result<Frame, NativeFrameError> {
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
        let viewport = self.viewport;
        render_native_frame_indexed(
            &self.source,
            &self.index,
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
        )
    }
}
