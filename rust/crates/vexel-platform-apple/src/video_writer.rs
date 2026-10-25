//! Compressed video export through `AVAssetWriter` (H.264 or HEVC, BT.709).
//!
//! Frames arrive as straight-alpha sRGB RGBA, are composited onto an opaque
//! background colour (or premultiplied for HEVC with alpha) and appended with exact `CMTime(ticks, timescale)` times.
//! Encoding is synchronous and never touches the main queue: `finish` waits on
//! a channel with a timeout.

use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_av_foundation::{
    AVAssetWriter, AVAssetWriterInput, AVAssetWriterInputPixelBufferAdaptor, AVAssetWriterStatus,
    AVFileTypeMPEG4, AVFileTypeQuickTimeMovie, AVMediaTypeVideo, AVVideoAverageBitRateKey,
    AVVideoCodecKey, AVVideoCodecTypeH264, AVVideoCodecTypeHEVC, AVVideoCodecTypeHEVCWithAlpha,
    AVVideoColorPrimaries_ITU_R_709_2, AVVideoColorPrimariesKey, AVVideoColorPropertiesKey,
    AVVideoCompressionPropertiesKey, AVVideoHeightKey, AVVideoQualityKey,
    AVVideoTransferFunction_ITU_R_709_2, AVVideoTransferFunctionKey, AVVideoWidthKey,
    AVVideoYCbCrMatrix_ITU_R_709_2, AVVideoYCbCrMatrixKey,
};
use objc2_core_foundation::{CFRetained, CFString, CFType};
use objc2_core_media::CMTime;
use objc2_core_video::{
    CVAttachmentMode, CVBuffer, CVPixelBuffer, CVPixelBufferCreate, CVPixelBufferGetBaseAddress,
    CVPixelBufferGetBytesPerRow, CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags,
    CVPixelBufferUnlockBaseAddress, kCVImageBufferColorPrimaries_ITU_R_709_2,
    kCVImageBufferColorPrimariesKey, kCVImageBufferTransferFunction_ITU_R_709_2,
    kCVImageBufferTransferFunctionKey, kCVImageBufferYCbCrMatrix_ITU_R_709_2,
    kCVImageBufferYCbCrMatrixKey, kCVPixelBufferHeightKey, kCVPixelBufferPixelFormatTypeKey,
    kCVPixelBufferWidthKey, kCVPixelFormatType_32BGRA, kCVReturnSuccess,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

/// Largest accepted frame edge in pixels.
pub const MAX_VIDEO_DIMENSION: u32 = 4096;
const IO_TIMEOUT: Duration = Duration::from_secs(60);

/// Video codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VideoCodec {
    H264,
    Hevc,
}

/// Output description. The container follows the path extension (`mp4` or `mov`).
#[derive(Clone, Debug)]
pub struct VideoSpec {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub codec: VideoCodec,
    /// Ticks per second for every presentation time passed to `append`.
    pub timescale: i32,
    /// Opaque background (sRGB) frames are composited onto.
    pub background: [u8; 3],
    /// Encode HEVC with alpha (`.mov` + HEVC only); frames are premultiplied and
    /// `background` is ignored.
    pub alpha: bool,
    /// Average bit rate in bits per second. When set the encoder runs in
    /// bit-rate mode and `quality` is ignored.
    pub bit_rate: Option<u32>,
    /// Constant-quality level 0..1 (encoder `AVVideoQualityKey`), used when
    /// `bit_rate` is `None`; `None` means 1.0. Deterministic.
    pub quality: Option<f64>,
}

/// Video writer failure.
#[derive(Debug, Eq, PartialEq)]
pub enum VideoError {
    Invalid(String),
    Native(String),
    Timeout(&'static str),
}

impl std::fmt::Display for VideoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(m) => write!(f, "invalid video request: {m}"),
            Self::Native(m) => write!(f, "video encoder failed: {m}"),
            Self::Timeout(m) => write!(f, "video encoder timed out: {m}"),
        }
    }
}

impl std::error::Error for VideoError {}

/// Streaming MP4/MOV writer. One frame in memory at a time.
pub struct VideoWriter {
    writer: Retained<AVAssetWriter>,
    input: Retained<AVAssetWriterInput>,
    adaptor: Retained<AVAssetWriterInputPixelBufferAdaptor>,
    spec: VideoSpec,
    timescale: i32,
    last: Option<i64>,
    previous: Option<i64>,
}

fn cf_as_ns(s: &CFString) -> &NSString {
    // SAFETY: CFString and NSString are toll-free bridged.
    unsafe { &*(s as *const CFString).cast::<NSString>() }
}

fn dict(keys: &[&NSString], values: &[&AnyObject]) -> Retained<NSDictionary<NSString, AnyObject>> {
    NSDictionary::from_slices(keys, values)
}

fn unavailable<T>(v: Option<T>, what: &str) -> Result<T, VideoError> {
    v.ok_or_else(|| VideoError::Native(format!("{what} unavailable")))
}

fn native(context: &str, writer: &AVAssetWriter) -> VideoError {
    // SAFETY: plain property read on a live writer.
    let detail = unsafe { writer.error() }
        .map(|e| e.localizedDescription().to_string())
        .unwrap_or_else(|| "unknown error".to_owned());
    VideoError::Native(format!("{context}: {detail}"))
}

impl VideoWriter {
    /// Creates the output file and starts the writing session at time zero.
    pub fn create(spec: VideoSpec) -> Result<Self, VideoError> {
        let (w, h) = (spec.width, spec.height);
        if w == 0 || h == 0 || w > MAX_VIDEO_DIMENSION || h > MAX_VIDEO_DIMENSION {
            return Err(VideoError::Invalid(format!(
                "dimensions {w}x{h} must be 1..={MAX_VIDEO_DIMENSION}"
            )));
        }
        if w % 2 != 0 || h % 2 != 0 {
            return Err(VideoError::Invalid(format!(
                "dimensions {w}x{h} must be even for 4:2:0 video"
            )));
        }
        if spec.timescale <= 0 {
            return Err(VideoError::Invalid("timescale must be positive".into()));
        }
        let ext = spec
            .path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        let path_str = spec
            .path
            .to_str()
            .ok_or_else(|| VideoError::Invalid("path is not UTF-8".into()))?;
        // SAFETY: read of immutable framework constants.
        let file_type = unsafe {
            match ext.as_deref() {
                Some("mp4") => AVFileTypeMPEG4,
                Some("mov") => AVFileTypeQuickTimeMovie,
                _ => return Err(VideoError::Invalid("extension must be .mp4 or .mov".into())),
            }
        };
        if spec.alpha && (spec.codec != VideoCodec::Hevc || ext.as_deref() != Some("mov")) {
            return Err(VideoError::Invalid(
                "alpha needs HEVC in a .mov container".into(),
            ));
        }
        // AVAssetWriter refuses to overwrite.
        match std::fs::remove_file(&spec.path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(VideoError::Invalid(format!("cannot replace output: {e}"))),
        }
        // SAFETY: statics are valid framework constants; all objects are
        // retained values created and used on this thread.
        unsafe {
            let file_type = unavailable(file_type, "file type")?;
            let url = NSURL::fileURLWithPath(&NSString::from_str(path_str));
            let writer =
                AVAssetWriter::initWithURL_fileType_error(AVAssetWriter::alloc(), &url, file_type)
                    .map_err(|e| VideoError::Native(e.localizedDescription().to_string()))?;

            let codec = unavailable(
                match (spec.codec, spec.alpha) {
                    (VideoCodec::H264, _) => AVVideoCodecTypeH264,
                    (VideoCodec::Hevc, false) => AVVideoCodecTypeHEVC,
                    (VideoCodec::Hevc, true) => AVVideoCodecTypeHEVCWithAlpha,
                },
                "codec",
            )?;
            let n_w = NSNumber::new_u32(w);
            let n_h = NSNumber::new_u32(h);
            let n_rc = match spec.bit_rate {
                Some(rate) => NSNumber::new_u32(rate),
                None => NSNumber::new_f64(spec.quality.unwrap_or(1.0).clamp(0.0, 1.0)),
            };
            let color = dict(
                &[
                    unavailable(AVVideoColorPrimariesKey, "key")?,
                    unavailable(AVVideoTransferFunctionKey, "key")?,
                    unavailable(AVVideoYCbCrMatrixKey, "key")?,
                ],
                &[
                    unavailable(AVVideoColorPrimaries_ITU_R_709_2, "value")?,
                    unavailable(AVVideoTransferFunction_ITU_R_709_2, "value")?,
                    unavailable(AVVideoYCbCrMatrix_ITU_R_709_2, "value")?,
                ],
            );
            let compression = dict(
                // Either key, never both: with a quality key the encoder
                // ignores the bit rate.
                &[match spec.bit_rate {
                    Some(_) => unavailable(AVVideoAverageBitRateKey, "key")?,
                    None => unavailable(AVVideoQualityKey, "key")?,
                }],
                &[&n_rc],
            );
            let settings = dict(
                &[
                    unavailable(AVVideoCodecKey, "key")?,
                    unavailable(AVVideoWidthKey, "key")?,
                    unavailable(AVVideoHeightKey, "key")?,
                    unavailable(AVVideoColorPropertiesKey, "key")?,
                    unavailable(AVVideoCompressionPropertiesKey, "key")?,
                ],
                &[codec, &n_w, &n_h, &color, &compression],
            );
            let media = unavailable(AVMediaTypeVideo, "media type")?;
            let input = AVAssetWriterInput::assetWriterInputWithMediaType_outputSettings(
                media,
                Some(&settings),
            );
            input.setExpectsMediaDataInRealTime(false);
            let src_attrs = dict(
                &[
                    cf_as_ns(kCVPixelBufferPixelFormatTypeKey),
                    cf_as_ns(kCVPixelBufferWidthKey),
                    cf_as_ns(kCVPixelBufferHeightKey),
                ],
                &[&NSNumber::new_u32(kCVPixelFormatType_32BGRA), &n_w, &n_h],
            );
            let adaptor = AVAssetWriterInputPixelBufferAdaptor::assetWriterInputPixelBufferAdaptorWithAssetWriterInput_sourcePixelBufferAttributes(
                &input,
                Some(&src_attrs),
            );
            if !writer.canAddInput(&input) {
                return Err(VideoError::Native("writer rejected video input".into()));
            }
            writer.addInput(&input);
            if !writer.startWriting() {
                return Err(native("startWriting", &writer));
            }
            writer.startSessionAtSourceTime(CMTime::new(0, spec.timescale));
            Ok(Self {
                writer,
                input,
                adaptor,
                timescale: spec.timescale,
                spec,
                last: None,
                previous: None,
            })
        }
    }

    /// Output path.
    pub fn path(&self) -> &Path {
        &self.spec.path
    }

    /// Appends one straight-alpha RGBA frame at `pts` ticks. Times must increase.
    pub fn append(&mut self, rgba: &[u8], pts: i64) -> Result<(), VideoError> {
        let (w, h) = (self.spec.width as usize, self.spec.height as usize);
        if rgba.len() != w * h * 4 {
            return Err(VideoError::Invalid(format!(
                "frame has {} bytes, expected {}",
                rgba.len(),
                w * h * 4
            )));
        }
        if pts < 0 || self.last.is_some_and(|l| pts <= l) {
            return Err(VideoError::Invalid(
                "presentation times must be non-negative and strictly increasing".into(),
            ));
        }
        let start = Instant::now();
        // SAFETY: property reads on a live input and writer.
        while !unsafe { self.input.isReadyForMoreMediaData() } {
            if unsafe { self.writer.status() } == AVAssetWriterStatus::Failed {
                return Err(native("append", &self.writer));
            }
            if start.elapsed() > IO_TIMEOUT {
                return Err(VideoError::Timeout("waiting for encoder input"));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let buffer = make_pixel_buffer(rgba, w, h, self.spec.background, self.spec.alpha)?;
        // SAFETY: buffer is a valid BGRA pixel buffer of the adaptor's size and
        // the input is ready; the time is a valid numeric CMTime.
        let ok = unsafe {
            self.adaptor
                .appendPixelBuffer_withPresentationTime(&buffer, CMTime::new(pts, self.timescale))
        };
        if !ok {
            return Err(native("appendPixelBuffer", &self.writer));
        }
        self.previous = self.last;
        self.last = Some(pts);
        Ok(())
    }

    /// Finishes using the last frame interval as the final frame's duration.
    pub fn finish(self) -> Result<(), VideoError> {
        let end = match (self.previous, self.last) {
            (Some(p), Some(l)) => l + (l - p),
            (None, Some(l)) => l + i64::from(self.timescale / 30).max(1),
            _ => return Err(VideoError::Invalid("no frames appended".into())),
        };
        self.finish_at(end)
    }

    /// Finishes with an explicit end time in ticks (the last frame's end).
    pub fn finish_at(self, end_ticks: i64) -> Result<(), VideoError> {
        let Some(last) = self.last else {
            return Err(VideoError::Invalid("no frames appended".into()));
        };
        if end_ticks <= last {
            return Err(VideoError::Invalid(
                "end time must follow last frame".into(),
            ));
        }
        let (tx, rx) = mpsc::channel::<()>();
        let block = RcBlock::new(move || {
            let _ = tx.send(());
        });
        // SAFETY: the completion block only sends on a channel and may run on
        // any queue; the writer stays alive until the channel fires or times out.
        unsafe {
            self.input.markAsFinished();
            self.writer
                .endSessionAtSourceTime(CMTime::new(end_ticks, self.timescale));
            self.writer.finishWritingWithCompletionHandler(&block);
        }
        rx.recv_timeout(IO_TIMEOUT)
            .map_err(|_| VideoError::Timeout("finishWriting"))?;
        // SAFETY: property read on a live writer.
        if unsafe { self.writer.status() } == AVAssetWriterStatus::Completed {
            Ok(())
        } else {
            Err(native("finishWriting", &self.writer))
        }
    }
}

fn make_pixel_buffer(
    rgba: &[u8],
    w: usize,
    h: usize,
    bg: [u8; 3],
    alpha: bool,
) -> Result<CFRetained<CVPixelBuffer>, VideoError> {
    let mut raw: *mut CVPixelBuffer = std::ptr::null_mut();
    // SAFETY: out pointer is valid; the created buffer is owned (+1).
    let status = unsafe {
        CVPixelBufferCreate(
            None,
            w,
            h,
            kCVPixelFormatType_32BGRA,
            None,
            NonNull::from(&mut raw),
        )
    };
    let ptr = NonNull::new(raw).filter(|_| status == kCVReturnSuccess);
    let ptr = ptr.ok_or_else(|| VideoError::Native(format!("CVPixelBufferCreate: {status}")))?;
    // SAFETY: Create returned a +1 retained object.
    let buffer = unsafe { CFRetained::from_raw(ptr) };
    // SAFETY: lock/unlock bracket all base-address access; rows are bounded by
    // bytes-per-row which is checked to be at least w*4.
    unsafe {
        let flags = CVPixelBufferLockFlags(0);
        if CVPixelBufferLockBaseAddress(&buffer, flags) != kCVReturnSuccess {
            return Err(VideoError::Native("CVPixelBufferLockBaseAddress".into()));
        }
        let base = CVPixelBufferGetBaseAddress(&buffer).cast::<u8>();
        let stride = CVPixelBufferGetBytesPerRow(&buffer);
        if base.is_null() || stride < w * 4 {
            CVPixelBufferUnlockBaseAddress(&buffer, flags);
            return Err(VideoError::Native("pixel buffer layout".into()));
        }
        let dst = std::slice::from_raw_parts_mut(base, stride * h);
        for y in 0..h {
            let src_row = &rgba[y * w * 4..(y + 1) * w * 4];
            let dst_row = &mut dst[y * stride..y * stride + w * 4];
            for (s, d) in src_row.chunks_exact(4).zip(dst_row.chunks_exact_mut(4)) {
                if alpha {
                    // Premultiplied BGRA keeps transparency for the encoder.
                    let a = u32::from(s[3]);
                    let pm = |c: u8| ((u32::from(c) * a + 127) / 255) as u8;
                    d.copy_from_slice(&[pm(s[2]), pm(s[1]), pm(s[0]), s[3]]);
                    continue;
                }
                let a = u32::from(s[3]);
                let mix = |c: u8, b: u8| {
                    ((u32::from(c) * a + u32::from(b) * (255 - a) + 127) / 255) as u8
                };
                d[0] = mix(s[2], bg[2]);
                d[1] = mix(s[1], bg[1]);
                d[2] = mix(s[0], bg[0]);
                d[3] = 255;
            }
        }
        CVPixelBufferUnlockBaseAddress(&buffer, flags);
        let b: &CVBuffer = &buffer;
        let tag = |k: &CFString, v: &CFString| {
            let v: &CFType = v.as_ref();
            b.set_attachment(k, v, CVAttachmentMode::ShouldPropagate);
        };
        tag(
            kCVImageBufferColorPrimariesKey,
            kCVImageBufferColorPrimaries_ITU_R_709_2,
        );
        tag(
            kCVImageBufferTransferFunctionKey,
            kCVImageBufferTransferFunction_ITU_R_709_2,
        );
        tag(
            kCVImageBufferYCbCrMatrixKey,
            kCVImageBufferYCbCrMatrix_ITU_R_709_2,
        );
    }
    Ok(buffer)
}
