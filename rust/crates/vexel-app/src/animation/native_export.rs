//! Synchronous GIF export of a native timeline: one frame per loop tick,
//! rendered through `render_native_frame` and encoded by `vexel-gif`.

use super::native_drawing::{DrawingAsset, SvgGeometry};
use super::native_frame::{NativeFrameError, render_native_frame, render_native_frame_cached};
use crate::compose::{ComposeError, DiffCache, svg_viewport};
use crate::project_session::ProjectTimelineCandidate;
use crate::session::AppDocument;
use std::sync::Arc;
use vexel_gif::{Frame, GifError, MAX_FPS, MAX_FRAMES, MAX_TOTAL_PIXELS};
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

#[derive(Debug, Clone, PartialEq)]
pub enum NativeExportError {
    /// The timeline is unreadable or has no ticks.
    Timeline,
    Viewport(ComposeError),
    Frame(NativeFrameError),
    Encode(GifError),
    /// `time` is past the end of the timeline (it is in seconds, not ticks).
    TimeOutOfRange {
        seconds: f64,
    },
}

/// A finished GIF and what it contains.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeGif {
    pub bytes: Vec<u8>,
    pub frames: usize,
    pub delay_cs: u16,
    pub width: u32,
    pub height: u32,
}

/// Immutable inputs of one native export.
#[derive(Debug, PartialEq)]
pub struct NativeExportSource {
    source: Arc<Vec<u8>>,
    candidate: ProjectTimelineCandidate,
}

impl NativeExportSource {
    pub fn new(source: Vec<u8>, candidate: ProjectTimelineCandidate) -> Self {
        Self {
            source: Arc::new(source),
            candidate,
        }
    }

    /// The document's native timeline, or `None` when it has no native
    /// tracks (SMIL fallback) or cannot be read.
    pub fn from_document(doc: &AppDocument) -> Option<Self> {
        let settings = EmptyTimelineSettings {
            ticks_per_second: 24,
            duration_ticks: 24,
            fps_numerator: 24,
            fps_denominator: 1,
            loop_mode: LoopMode::Repeat,
            onion_before: 0,
            onion_after: 0,
            onion_opacity: 0.25,
        };
        let candidate = doc
            .native_timeline_candidate(settings, EditLimits::default())
            .ok()?;
        let source = doc.source_bytes().ok()?;
        let this = Self::new(source, candidate);
        (this.candidate.view().ok()?.track_count() > 0).then_some(this)
    }

    /// Raw SVG source bytes.
    pub(super) fn svg_source(&self) -> &Arc<Vec<u8>> {
        &self.source
    }

    /// The candidate timeline.
    pub(super) fn timeline(&self) -> &ProjectTimelineCandidate {
        &self.candidate
    }

    /// Embedded assets in the form the frame renderer takes.
    pub(super) fn drawing_assets(&self) -> Vec<DrawingAsset<'_>> {
        self.candidate
            .assets()
            .iter()
            .map(|asset| DrawingAsset {
                name: &asset.name,
                media_type: &asset.media_type,
                source: &asset.source,
            })
            .collect()
    }

    /// The tick shown at each step of one loop (ping-pong included).
    pub fn loop_tick_list(&self) -> Option<Vec<u32>> {
        let view = self.candidate.view().ok()?;
        Some(loop_ticks(view.duration_ticks(), view.loop_mode()))
    }

    /// Length of one loop in seconds, `None` when the timeline is unreadable.
    pub fn loop_seconds(&self) -> Option<f64> {
        let view = self.candidate.view().ok()?;
        let tps = view.ticks_per_second();
        (tps != 0).then(|| f64::from(view.duration_ticks()) / f64::from(tps))
    }

    /// Renders one still at `time` seconds (clamped to the last tick) as
    /// straight RGBA. `size` is the output size, `None` for the document's.
    pub fn render_still(
        &self,
        time: f64,
        size: impl FnOnce((u32, u32)) -> Result<(u32, u32), &'static str>,
    ) -> Result<(u32, u32, Vec<u8>), NativeExportError> {
        self.render_still_at(time, size)
            .map(|(w, h, _, rgba)| (w, h, rgba))
    }

    /// Like [`Self::render_still`], also returning the tick rendered.
    pub fn render_still_at(
        &self,
        time: f64,
        size: impl FnOnce((u32, u32)) -> Result<(u32, u32), &'static str>,
    ) -> Result<(u32, u32, u32, Vec<u8>), NativeExportError> {
        let view = self
            .candidate
            .view()
            .map_err(|_| NativeExportError::Timeline)?;
        let (tps, duration) = (view.ticks_per_second(), view.duration_ticks());
        if tps == 0 || duration == 0 {
            return Err(NativeExportError::Timeline);
        }
        let scaled = time.max(0.0) * f64::from(tps);
        // Up to one tick past the end still means the last tick; beyond that
        // the caller most likely passed ticks instead of seconds.
        if scaled > f64::from(duration) {
            return Err(NativeExportError::TimeOutOfRange {
                seconds: f64::from(duration) / f64::from(tps),
            });
        }
        let tick = scaled.round().min(f64::from(duration - 1)) as u32;
        let viewport = svg_viewport(Arc::clone(&self.source), &|| false)
            .map_err(NativeExportError::Viewport)?;
        let (width, height) = size((viewport.output_width, viewport.output_height))
            .map_err(|_| NativeExportError::Timeline)?;
        let geometry = svg_geometry(&viewport, width, height);
        let assets = self.drawing_assets();
        let frame =
            render_native_frame(&self.source, &view, &assets, tick, width, height, geometry)
                .map_err(NativeExportError::Frame)?;
        Ok((width, height, tick, frame.rgba))
    }

    /// Renders the loop ticks (ping-pong included) and encodes the GIF. Each
    /// frame lasts one tick, `round(100 / ticks_per_second)` centiseconds.
    /// Loops longer than [`MAX_FRAMES`] ticks keep every `stride`-th tick,
    /// with the smallest `stride` that fits, and scale the delay by `stride`
    /// so the total duration stays about the same.
    pub fn export_gif(&self) -> Result<NativeGif, NativeExportError> {
        let view = self
            .candidate
            .view()
            .map_err(|_| NativeExportError::Timeline)?;
        let loop_steps = loop_ticks(view.duration_ticks(), view.loop_mode());
        let tps = view.ticks_per_second();
        if loop_steps.is_empty() || tps == 0 {
            return Err(NativeExportError::Timeline);
        }
        let stride = loop_steps.len().div_ceil(MAX_FRAMES);
        let ticks: Vec<u32> = loop_steps.into_iter().step_by(stride).collect();
        let viewport = svg_viewport(Arc::clone(&self.source), &|| false)
            .map_err(NativeExportError::Viewport)?;
        let (width, height) = (viewport.output_width, viewport.output_height);
        if (u64::from(width) * u64::from(height)).saturating_mul(ticks.len() as u64)
            > MAX_TOTAL_PIXELS
        {
            return Err(NativeExportError::Encode(GifError::TooManyPixels));
        }
        let geometry = svg_geometry(&viewport, width, height);
        let assets = self.drawing_assets();
        let mut rendered = Vec::new();
        let mut cache = DiffCache::default();
        for &tick in &ticks {
            let frame = render_native_frame_cached(
                &self.source,
                None,
                &view,
                &assets,
                tick,
                width,
                height,
                geometry,
                &mut cache,
            )
            .map_err(NativeExportError::Frame)?;
            rendered.push(over_white(&frame.rgba));
        }
        let tick_cs = ((100 + tps / 2) / tps).clamp(1, u32::from(u16::MAX));
        let delay_cs = u16::try_from(u64::from(tick_cs) * stride as u64)
            .map_err(|_| NativeExportError::Encode(GifError::TooManyFrames))?;
        let frames: Vec<Frame<'_>> = rendered
            .iter()
            .map(|rgba| Frame {
                width,
                height,
                rgba,
                delay_cs: Some(delay_cs),
            })
            .collect();
        let bytes =
            vexel_gif::encode(&frames, tps.clamp(1, MAX_FPS)).map_err(NativeExportError::Encode)?;
        Ok(NativeGif {
            bytes,
            frames: frames.len(),
            delay_cs,
            width,
            height,
        })
    }
}

/// Frame geometry for rendering the document viewport at `width` x `height`.
pub(super) fn svg_geometry(
    viewport: &crate::compose::SvgViewport,
    width: u32,
    height: u32,
) -> SvgGeometry {
    SvgGeometry {
        width: f64::from(width),
        height: f64::from(height),
        view_box: [
            viewport.view_box.x,
            viewport.view_box.y,
            viewport.view_box.width,
            viewport.view_box.height,
        ],
    }
}

/// The tick played at each step of one loop; ping-pong reflects without
/// repeating the end ticks, as `NativeDriver` does.
pub fn loop_ticks(duration: u32, mode: LoopMode) -> Vec<u32> {
    let mut ticks: Vec<u32> = (0..duration).collect();
    if mode == LoopMode::PingPong && duration > 2 {
        ticks.extend((1..duration - 1).rev());
    }
    ticks
}

fn over_white(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    for px in rgba.chunks_exact(4) {
        let a = u32::from(px[3]);
        for &c in &px[..3] {
            out.push(((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8);
        }
        out.push(255);
    }
    out
}
