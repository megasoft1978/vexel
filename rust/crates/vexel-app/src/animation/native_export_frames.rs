//! Frame-by-frame render of a native timeline loop for animated-PNG and
//! PNG-sequence export. Frames reach the caller one at a time, so a sequence
//! never holds more than one frame; tick selection and geometry are shared
//! with the GIF and video exports.

use super::native_export::{NativeExportError, NativeExportSource, loop_ticks, svg_geometry};
use super::native_frame::render_native_frame_cached;
use crate::compose::svg_viewport;
use std::sync::Arc;

/// Most frames one export may contain; longer loops keep every `stride`-th tick.
pub const MAX_EXPORT_FRAMES: usize = 1440;
/// Longest edge accepted, in pixels.
pub const MAX_EXPORT_SIDE: u32 = 4096;

/// How to sample and size the frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeFramesRequest {
    /// Output size, `None` for the document's (long edge capped to
    /// [`MAX_EXPORT_SIDE`]).
    pub size: Option<(u32, u32)>,
    /// Opaque sRGB colour transparent areas are composited onto; `None`
    /// keeps straight RGBA.
    pub background: Option<[u8; 3]>,
    /// Target frames per second; `None` shows one frame per tick.
    pub fps: Option<u32>,
}

/// What a render produced.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeFrames {
    pub frames: usize,
    pub width: u32,
    pub height: u32,
    /// Ticks between frames after any stride, at the timeline's tick rate.
    pub ticks_per_frame: u32,
    pub ticks_per_second: u32,
}

#[derive(Debug, PartialEq)]
pub enum NativeFramesError<E> {
    Export(NativeExportError),
    /// Requested size is zero or past [`MAX_EXPORT_SIDE`].
    BadSize,
    /// The frame sink refused a frame.
    Sink(E),
}

impl<E> From<NativeExportError> for NativeFramesError<E> {
    fn from(e: NativeExportError) -> Self {
        Self::Export(e)
    }
}

/// Frame count and size an export would produce, without rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramesPlan {
    pub frames: usize,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub ticks_per_second: u32,
}

impl NativeExportSource {
    /// Plans one pass over the loop (ping-pong included), without rendering.
    pub fn plan_frames(
        &self,
        req: &NativeFramesRequest,
    ) -> Result<FramesPlan, NativeFramesError<()>> {
        let view = self
            .timeline()
            .view()
            .map_err(|_| NativeExportError::Timeline)?;
        let steps = loop_ticks(view.duration_ticks(), view.loop_mode()).len() as u64;
        let tps = view.ticks_per_second();
        if steps == 0 || tps == 0 {
            return Err(NativeExportError::Timeline.into());
        }
        let fps_stride = req.fps.map_or(1, |fps| {
            ((f64::from(tps) / f64::from(fps.max(1))).round() as u64).max(1)
        });
        let stride = fps_stride.max(steps.div_ceil(MAX_EXPORT_FRAMES as u64));
        let viewport = svg_viewport(Arc::clone(self.svg_source()), &|| false)
            .map_err(NativeExportError::Viewport)?;
        let (width, height) = req.size.unwrap_or_else(|| {
            let (w, h) = (viewport.output_width.max(1), viewport.output_height.max(1));
            let long = w.max(h);
            if long <= MAX_EXPORT_SIDE {
                (w, h)
            } else {
                let scale = |v: u32| {
                    (u64::from(v) * u64::from(MAX_EXPORT_SIDE) / u64::from(long)).max(1) as u32
                };
                (scale(w), scale(h))
            }
        });
        if width == 0 || height == 0 || width > MAX_EXPORT_SIDE || height > MAX_EXPORT_SIDE {
            return Err(NativeFramesError::BadSize);
        }
        Ok(FramesPlan {
            frames: steps.div_ceil(stride) as usize,
            width,
            height,
            stride: u32::try_from(stride).map_err(|_| NativeExportError::Timeline)?,
            ticks_per_second: tps,
        })
    }

    /// Renders the planned frames in order and hands each one (straight
    /// RGBA, or opaque over `background`) to `sink` with its index.
    pub fn render_frames<E>(
        &self,
        req: &NativeFramesRequest,
        mut sink: impl FnMut(usize, &[u8]) -> Result<(), E>,
    ) -> Result<NativeFrames, NativeFramesError<E>> {
        let plan = self.plan_frames(req).map_err(|e| match e {
            NativeFramesError::Export(e) => NativeFramesError::Export(e),
            _ => NativeFramesError::BadSize,
        })?;
        let view = self
            .timeline()
            .view()
            .map_err(|_| NativeExportError::Timeline)?;
        let steps = loop_ticks(view.duration_ticks(), view.loop_mode());
        let viewport = svg_viewport(Arc::clone(self.svg_source()), &|| false)
            .map_err(NativeExportError::Viewport)?;
        let geometry = svg_geometry(&viewport, plan.width, plan.height);
        let assets = self.drawing_assets();
        let mut cache = crate::compose::DiffCache::default();
        for index in 0..plan.frames {
            let tick = steps[index * plan.stride as usize];
            let frame = render_native_frame_cached(
                self.svg_source(),
                None,
                &view,
                &assets,
                tick,
                plan.width,
                plan.height,
                geometry,
                &mut cache,
            )
            .map_err(NativeExportError::Frame)?;
            let result = match req.background {
                Some(colour) => sink(index, &over_colour(&frame.rgba, colour)),
                None => sink(index, &frame.rgba),
            };
            result.map_err(NativeFramesError::Sink)?;
        }
        Ok(NativeFrames {
            frames: plan.frames,
            width: plan.width,
            height: plan.height,
            ticks_per_frame: plan.stride,
            ticks_per_second: plan.ticks_per_second,
        })
    }
}

/// Straight RGBA composited over an opaque colour.
fn over_colour(rgba: &[u8], colour: [u8; 3]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    for px in rgba.chunks_exact(4) {
        let a = u32::from(px[3]);
        for (&c, &b) in px[..3].iter().zip(&colour) {
            out.push(((u32::from(c) * a + u32::from(b) * (255 - a) + 127) / 255) as u8);
        }
        out.push(255);
    }
    out
}
