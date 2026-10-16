//! GIF export of the current document's loop into a granted directory.
//!
//! The whole stream is rendered and encoded in memory first; only then is a
//! temporary sibling written and linked into place, so any failure or
//! cancellation leaves no partial file and an existing file is never replaced
//! unless `overwrite` is set.

use super::frame::{self, AnimationError, over_white};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use vexel_gif::{Frame, GifError, MAX_FRAMES, MAX_TOTAL_PIXELS};
use vexel_mcp_access::{AccessError, AccessPolicy};

#[derive(Debug, Clone, PartialEq)]
pub enum ExportError {
    /// `fps` outside the tool schema range 1..=30.
    InvalidFps,
    /// `duration` not finite or outside 0.01..=3600.
    InvalidDuration,
    /// The document has no animation to export.
    NotAnimated,
    /// `duration` is longer than the document's loop.
    DurationExceedsLoop,
    Access(AccessError),
    /// Target exists and `overwrite` is false.
    Exists,
    Frame(AnimationError),
    Encode(GifError),
    Cancelled,
    Io(std::io::ErrorKind),
}

/// GIF is the only format; frame sequences are a deferred non-goal.
pub struct ExportRequest<'a> {
    pub path: &'a OsStr,
    pub fps: u32,
    pub duration: f64,
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportSummary {
    pub path: PathBuf,
    pub bytes: usize,
    pub frames: usize,
    pub delay_cs: u16,
    pub width: u32,
    pub height: u32,
}

/// Number of frames for a duration and fps: `ceil(duration * fps)` as in the C
/// exporter, with a 1e-9 tolerance so `0.3 s * 10 fps` is 3, not 4.
pub fn frame_count(duration: f64, fps: u32) -> usize {
    ((duration * f64::from(fps) - 1e-9).ceil()).max(1.0) as usize
}

/// Renders frames `i / fps` for `i` in `0..frame_count` of the SVG `source`
/// and writes one GIF. `progress(done, total)` runs after every rendered frame.
pub fn export_gif(
    source: &[u8],
    policy: &AccessPolicy,
    request: &ExportRequest<'_>,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<ExportSummary, ExportError> {
    if !(1..=30).contains(&request.fps) {
        return Err(ExportError::InvalidFps);
    }
    if !request.duration.is_finite() || !(0.01..=3600.0).contains(&request.duration) {
        return Err(ExportError::InvalidDuration);
    }
    let target = policy
        .resolve_save_path(request.path)
        .map_err(ExportError::Access)?;
    if target.exists() && !request.overwrite {
        return Err(ExportError::Exists);
    }
    let count = frame_count(request.duration, request.fps);
    if count > MAX_FRAMES {
        return Err(ExportError::Encode(GifError::TooManyFrames));
    }
    let scene = frame::decode(source).map_err(ExportError::Frame)?;
    if scene.loop_seconds() <= 0.0 {
        return Err(ExportError::NotAnimated);
    }
    if request.duration > scene.loop_seconds() + 1e-9 {
        return Err(ExportError::DurationExceedsLoop);
    }
    let (width, height) = scene.size();
    if (u64::from(width) * u64::from(height)).saturating_mul(count as u64) > MAX_TOTAL_PIXELS {
        return Err(ExportError::Encode(GifError::TooManyPixels));
    }

    let mut rendered: Vec<Vec<u8>> = Vec::new();
    rendered
        .try_reserve_exact(count)
        .map_err(|_| ExportError::Encode(GifError::OutOfMemory))?;
    for index in 0..count {
        if cancel.load(Ordering::Relaxed) {
            return Err(ExportError::Cancelled);
        }
        let time = index as f64 / f64::from(request.fps);
        let tile = scene.render(time, cancel).map_err(|e| match e {
            AnimationError::Cancelled => ExportError::Cancelled,
            other => ExportError::Frame(other),
        })?;
        rendered.push(over_white(&tile));
        progress(index + 1, count);
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(ExportError::Cancelled);
    }
    let frames: Vec<Frame<'_>> = rendered
        .iter()
        .map(|rgba| Frame {
            width,
            height,
            rgba,
            delay_cs: None,
        })
        .collect();
    let bytes = vexel_gif::encode(&frames, request.fps).map_err(ExportError::Encode)?;
    let delay_cs = vexel_gif::delay_cs_for_fps(request.fps).map_err(ExportError::Encode)?;
    if cancel.load(Ordering::Relaxed) {
        return Err(ExportError::Cancelled);
    }
    write_new(&target, &bytes, request.overwrite)?;
    Ok(ExportSummary {
        path: target,
        bytes: bytes.len(),
        frames: count,
        delay_cs,
        width,
        height,
    })
}

fn write_new(target: &std::path::Path, bytes: &[u8], overwrite: bool) -> Result<(), ExportError> {
    let io = |e: std::io::Error| ExportError::Io(e.kind());
    let name = target
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or(ExportError::Io(std::io::ErrorKind::InvalidInput))?;
    let temp = target.with_file_name(format!(".{name}.vexel-export-{}", std::process::id()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(io)?;
        file.write_all(bytes).map_err(io)?;
        file.sync_all().map_err(io)?;
        drop::<File>(file);
        if overwrite {
            fs::rename(&temp, target).map_err(io)
        } else {
            // hard_link fails if `target` exists, so a concurrent file is never clobbered.
            match fs::hard_link(&temp, target) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(ExportError::Exists),
                Err(e) => Err(io(e)),
            }
        }
    })();
    if temp.exists() {
        let _ = fs::remove_file(&temp);
    }
    result
}
