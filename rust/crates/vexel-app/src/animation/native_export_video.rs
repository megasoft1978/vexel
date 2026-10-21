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
    pub bit_rate: Option<u32>,
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
        let stride = loop_steps.len().div_ceil(MAX_VIDEO_FRAMES);
        let ticks: Vec<u32> = loop_steps.into_iter().step_by(stride).collect();
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
        let mut writer = VideoWriter::create(VideoSpec {
            path: req.path.clone(),
            width,
            height,
            codec: req.codec,
            timescale: tps as i32,
            background: req.background,
            bit_rate: req.bit_rate,
        })?;
        let ticks_per_frame = stride as u32;
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
