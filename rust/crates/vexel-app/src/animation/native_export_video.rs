//! Synchronous compressed-video export of a native timeline: one frame per
//! loop tick rendered and appended one at a time, so memory stays at one frame.
//! Mirrors the GIF export's tick selection (ping-pong included).

use super::native_export::ordered::{OrderedError, render_ordered, worker_count};
use super::native_export::{NativeExportError, NativeExportSource, loop_ticks, svg_geometry};
use super::native_frame::render_native_frame;
use crate::compose::svg_viewport;
use std::path::PathBuf;
use std::sync::Arc;
use vexel_platform_apple::{MAX_VIDEO_DIMENSION, VideoCodec, VideoError, VideoSpec, VideoWriter};

/// Most frames one video export may contain; longer loops keep every
/// `stride`-th tick (the same rule the GIF export uses).
pub const MAX_VIDEO_FRAMES: usize = 1440;
/// Longest edge accepted for a video, in pixels.
pub const MAX_VIDEO_SIDE: u32 = MAX_VIDEO_DIMENSION;

/// What to write and how.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeVideoRequest {
    /// Output file; `.mp4` or `.mov`.
    pub path: PathBuf,
    pub codec: VideoCodec,
    /// Output size, `None` for the document's. Odd sizes are rounded up.
    pub size: Option<(u32, u32)>,
    /// Opaque sRGB colour transparent areas are composited onto.
    pub background: [u8; 3],
    /// Keep transparency (HEVC with alpha, `.mov`); `background` is ignored.
    pub alpha: bool,
    pub bit_rate: Option<u32>,
    /// Quality 0..1 (see [`quality_level`]) used when `bit_rate` is `None`;
    /// both `None` uses [`DEFAULT_QUALITY`].
    pub quality: Option<f64>,
    /// Target frames per second; `None` (or above the tick rate) shows one
    /// frame per tick. Frames are every `round(ticks_per_second / fps)`-th tick.
    pub fps: Option<u32>,
    /// How many times the loop plays back to back (at least 1).
    pub loops: u32,
}

/// Quality used when the caller gives neither `quality` nor `bit_rate`.
pub(crate) const DEFAULT_QUALITY: f64 = 0.5;

/// Encoder constant-quality level (`AVVideoQualityKey`, 0..1) for the
/// tool's `quality` argument (clamped to 0..1). Pure so it is testable
/// without encoding.
///
/// Linear from 0.6 (quality 0, smallest) to 1.0 (quality 1, near lossless):
/// VideoToolbox's H.264/HEVC size does not fall monotonically below about
/// 0.6. File size grows with pixel count and fps on its own at constant
/// quality.
pub(crate) fn quality_level(quality: f64) -> f64 {
    let q = quality.clamp(0.0, 1.0);
    0.6 + 0.4 * q
}

/// A finished video and what it contains.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeVideo {
    pub path: PathBuf,
    pub frames: usize,
    pub width: u32,
    pub height: u32,
    /// Ticks between frames after any stride, at the timeline's tick rate.
    pub ticks_per_frame: u32,
    pub ticks_per_second: u32,
}

#[derive(Debug, PartialEq)]
pub enum NativeVideoError {
    Export(NativeExportError),
    /// Requested size is zero or past [`MAX_VIDEO_SIDE`].
    BadSize {
        width: u32,
        height: u32,
    },
    Video(VideoError),
    /// `loops` is zero.
    BadLoops,
}

impl From<NativeExportError> for NativeVideoError {
    fn from(e: NativeExportError) -> Self {
        Self::Export(e)
    }
}

impl From<VideoError> for NativeVideoError {
    fn from(e: VideoError) -> Self {
        Self::Video(e)
    }
}

fn even(v: u32) -> u32 {
    v + (v & 1)
}

impl NativeExportSource {
    /// Renders the loop ticks and encodes them as H.264/HEVC. Frame `i` is
    /// shown at `i * stride` ticks, so durations are exact at any tick rate.
    pub fn export_video(&self, req: &NativeVideoRequest) -> Result<NativeVideo, NativeVideoError> {
        let view = self
            .timeline()
            .view()
            .map_err(|_| NativeExportError::Timeline)?;
        let loop_steps = loop_ticks(view.duration_ticks(), view.loop_mode());
        let tps = view.ticks_per_second();
        if loop_steps.is_empty() || tps == 0 || i32::try_from(tps).is_err() {
            return Err(NativeExportError::Timeline.into());
        }
        if req.loops == 0 {
            return Err(NativeVideoError::BadLoops);
        }
        // Frame `i` shows tick `loop_steps[(i * stride) % len]`; the loops are
        // never materialised.
        let total = loop_steps.len() as u64 * u64::from(req.loops);
        let fps_stride = req.fps.map_or(1, |fps| {
            ((f64::from(tps) / f64::from(fps.max(1))).round() as u64).max(1)
        });
        let stride = fps_stride.max(total.div_ceil(MAX_VIDEO_FRAMES as u64));
        let frame_count = total.div_ceil(stride) as usize;
        let ticks: Vec<u32> = (0..frame_count as u64)
            .map(|i| loop_steps[((i * stride) % loop_steps.len() as u64) as usize])
            .collect();
        let viewport = svg_viewport(Arc::clone(self.svg_source()), &|| false)
            .map_err(NativeExportError::Viewport)?;
        let (w, h) = req
            .size
            .unwrap_or((viewport.output_width, viewport.output_height));
        let (width, height) = (even(w), even(h));
        if w == 0 || h == 0 || width > MAX_VIDEO_SIDE || height > MAX_VIDEO_SIDE {
            return Err(NativeVideoError::BadSize {
                width: w,
                height: h,
            });
        }
        let geometry = svg_geometry(&viewport, width, height);
        let assets = self.drawing_assets();
        let quality = quality_level(req.quality.unwrap_or(DEFAULT_QUALITY));
        let mut writer = VideoWriter::create(VideoSpec {
            path: req.path.clone(),
            width,
            height,
            codec: req.codec,
            timescale: tps as i32,
            background: req.background,
            alpha: req.alpha,
            bit_rate: req.bit_rate,
            quality: Some(quality),
        })?;
        let ticks_per_frame = u32::try_from(stride).map_err(|_| NativeExportError::Timeline)?;
        render_ordered(
            ticks.len(),
            worker_count(ticks.len()),
            |i| {
                render_native_frame(
                    self.svg_source(),
                    &view,
                    &assets,
                    ticks[i],
                    width,
                    height,
                    geometry,
                )
            },
            |i, frame| writer.append(&frame.rgba, i as i64 * i64::from(ticks_per_frame)),
        )
        .map_err(|e| match e {
            OrderedError::Render(e) => NativeVideoError::Export(NativeExportError::Frame(e)),
            OrderedError::Consume(e) => NativeVideoError::Video(e),
        })?;
        writer.finish_at(ticks.len() as i64 * i64::from(ticks_per_frame))?;
        Ok(NativeVideo {
            path: req.path.clone(),
            frames: ticks.len(),
            width,
            height,
            ticks_per_frame,
            ticks_per_second: tps,
        })
    }
}
