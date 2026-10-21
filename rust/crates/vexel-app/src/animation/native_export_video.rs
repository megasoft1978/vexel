//! Synchronous compressed-video export of a native timeline: one frame per
//! loop tick rendered and appended one at a time, so memory stays at one frame.
//! Mirrors the GIF export's tick selection (ping-pong included).

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
    /// Quality 0..1 used to derive the bit rate when `bit_rate` is `None`;
    /// both `None` picks the encoder's size-based default.
    pub quality: Option<f64>,
    /// Target frames per second; `None` (or above the tick rate) shows one
    /// frame per tick. Frames are every `round(ticks_per_second / fps)`-th tick.
    pub fps: Option<u32>,
    /// How many times the loop plays back to back (at least 1).
    pub loops: u32,
}

/// Bits per pixel per frame at quality 0 and 1.
const BPP_LOW: f64 = 0.02;
const BPP_HIGH: f64 = 0.3;

/// Bit rate for `quality` (clamped to 0..1) at the output size and frame rate.
fn quality_bit_rate(quality: f64, width: u32, height: u32, frames_per_second: f64) -> u32 {
    let bpp = BPP_LOW + (BPP_HIGH - BPP_LOW) * quality.clamp(0.0, 1.0);
    let rate = f64::from(width) * f64::from(height) * frames_per_second * bpp;
    rate.clamp(250_000.0, 200_000_000.0) as u32
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
        let bit_rate = req.bit_rate.or_else(|| {
            req.quality
                .map(|q| quality_bit_rate(q, width, height, f64::from(tps) / stride as f64))
        });
        let mut writer = VideoWriter::create(VideoSpec {
            path: req.path.clone(),
            width,
            height,
            codec: req.codec,
            timescale: tps as i32,
            background: req.background,
            alpha: req.alpha,
            bit_rate,
        })?;
        let ticks_per_frame = u32::try_from(stride).map_err(|_| NativeExportError::Timeline)?;
        for (i, &tick) in ticks.iter().enumerate() {
            let frame = render_native_frame(
                self.svg_source(),
                &view,
                &assets,
                tick,
                width,
                height,
                geometry,
            )
            .map_err(NativeExportError::Frame)?;
            writer.append(&frame.rgba, i as i64 * i64::from(ticks_per_frame))?;
        }
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
