//! Synchronous compressed-video export of a native timeline: one frame per
//! loop tick rendered and appended one at a time, so memory stays at one frame.
//! Mirrors the GIF export's tick selection (ping-pong included).

use super::native_export::ordered::{OrderedError, render_ordered, worker_count};
use super::native_export::{NativeExportError, NativeExportSource, loop_ticks, svg_geometry};
use super::native_frame::render_native_frame;
use crate::compose::svg_viewport;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use vexel_platform_apple::{MAX_VIDEO_DIMENSION, VideoCodec, VideoError, VideoSpec, VideoWriter};
use vexel_timeline::audio::AudioTrack;
use vexel_timeline::audio_codec::{self, Pcm};
use vexel_timeline::audio_mix::{MixError, frames_for_ticks, mixdown};

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

/// Average bit rate (bits/s) for `quality` (clamped to 0..1) at `pixels` per
/// frame and `fps`. Explicit rate control is deterministic, unlike the
/// encoder's constant-quality mode whose output size varies run to run.
///
/// Bits per pixel per frame is exponential in quality: x0.25 at 0, the
/// default (0.5) at 0.2363 (= 14.7 Mbps at 1080p30), x4 at 1.
pub(crate) fn quality_bit_rate(quality: f64, pixels: u64, fps: f64) -> u32 {
    const DEFAULT_BPP: f64 = 14_700_000.0 / (1920.0 * 1080.0 * 30.0);
    let bpp = DEFAULT_BPP * 16f64.powf(quality.clamp(0.0, 1.0) - 0.5);
    (bpp * pixels as f64 * fps)
        .round()
        .clamp(10_000.0, f64::from(u32::MAX)) as u32
}

/// Audio to mux into a video: the tracks, how to load a track's decoded
/// samples, and a scratch WAV path (removed before returning).
pub struct NativeAudio<'a> {
    pub tracks: &'a [AudioTrack],
    pub load: &'a mut dyn FnMut(&AudioTrack) -> Result<Pcm, MixError>,
    pub scratch_wav: PathBuf,
}

static SCRATCH_SEQ: AtomicU64 = AtomicU64::new(0);

/// A unique sibling of `base` (pid + counter) so a predictable name is never
/// reused or pre-created by another party.
fn unique_scratch_path(base: &Path) -> PathBuf {
    let seq = SCRATCH_SEQ.fetch_add(1, Ordering::Relaxed);
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("audio");
    base.with_file_name(format!("{stem}.{}.{seq}.wav", std::process::id()))
}

/// Creates a new 0600 file (fails if it exists, never follows a planted path).
fn write_scratch(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    let written = file.write_all(bytes);
    if written.is_err() {
        drop(file);
        let _ = std::fs::remove_file(path);
    }
    written
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
    /// The audio mixdown failed.
    Audio(MixError),
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
        self.export_video_with_audio(req, None)
    }

    /// [`NativeExportSource::export_video`] plus an AAC track mixed from
    /// `audio` for the video's whole length; `None` leaves the output as
    /// without audio support.
    pub fn export_video_with_audio(
        &self,
        req: &NativeVideoRequest,
        audio: Option<NativeAudio<'_>>,
    ) -> Result<NativeVideo, NativeVideoError> {
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
        let quality = quality_level(req.quality.unwrap_or(DEFAULT_QUALITY));
        let fps = f64::from(tps) / stride as f64;
        let level = req.quality.unwrap_or(DEFAULT_QUALITY);
        // Maximum quality stays on the encoder's constant-quality mode, which
        // is deterministic; explicit-rate encodes depend on how the frames
        // arrive (parallel vs serial rendering), so lower levels use a rate.
        let bit_rate = req.bit_rate.or_else(|| {
            (level < 1.0)
                .then(|| quality_bit_rate(level, u64::from(width) * u64::from(height), fps))
        });
        let video_ticks = ticks.len() as u64 * stride;
        let scratch = match audio {
            Some(audio) if !audio.tracks.is_empty() => {
                let frames = frames_for_ticks(video_ticks, tps);
                let mixed = mixdown(audio.tracks, tps, frames, audio.load)
                    .map_err(NativeVideoError::Audio)?;
                let wav = audio_codec::encode_wav16(48_000, 2, &mixed);
                let unique = unique_scratch_path(&audio.scratch_wav);
                write_scratch(&unique, &wav)
                    .map_err(|_| NativeVideoError::Audio(MixError::Alloc))?;
                Some(unique)
            }
            _ => None,
        };
        let created = VideoWriter::create_with_audio(
            VideoSpec {
                path: req.path.clone(),
                width,
                height,
                codec: req.codec,
                timescale: tps as i32,
                background: req.background,
                alpha: req.alpha,
                bit_rate,
                quality: Some(quality),
            },
            scratch.as_deref(),
        );
        let result = created.map_err(NativeVideoError::from).and_then(|writer| {
            self.append_frames(writer, &ticks, &view, width, height, geometry, stride)
        });
        if let Some(wav) = &scratch {
            let _ = std::fs::remove_file(wav);
        }
        result.map(|(frames, ticks_per_frame)| NativeVideo {
            path: req.path.clone(),
            frames,
            width,
            height,
            ticks_per_frame,
            ticks_per_second: tps,
        })
    }
}

impl NativeExportSource {
    #[allow(clippy::too_many_arguments)]
    fn append_frames(
        &self,
        mut writer: VideoWriter,
        ticks: &[u32],
        view: &vexel_timeline::TimelineView<'_>,
        width: u32,
        height: u32,
        geometry: super::native_drawing::SvgGeometry,
        stride: u64,
    ) -> Result<(usize, u32), NativeVideoError> {
        let assets = self.drawing_assets();
        let ticks_per_frame = u32::try_from(stride).map_err(|_| NativeExportError::Timeline)?;
        render_ordered(
            ticks.len(),
            worker_count(ticks.len()),
            |i| {
                render_native_frame(
                    self.svg_source(),
                    view,
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
        Ok((ticks.len(), ticks_per_frame))
    }
}
