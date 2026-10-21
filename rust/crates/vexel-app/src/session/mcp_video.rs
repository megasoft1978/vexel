//! `export_animation` as `mp4`/`mov`: encode the native timeline loop into a
//! file inside a granted directory, synchronously like `render_png`.
//!
//! The encoder writes a temporary sibling (same extension, so the container
//! follows) that is renamed over the target; a failure leaves any existing
//! file unchanged and no temporary file behind.

use super::AppDocument;
use super::mcp::McpOutcome;
use super::mcp_png::quoted;
use crate::animation::native_export::NativeExportSource;
use crate::animation::native_export_message::{Tool, render_failure};
use crate::animation::native_export_video::{NativeVideoError, NativeVideoRequest};
use crate::preview::VideoExport;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::sync::atomic::{AtomicU32, Ordering};
use vexel_mcp_access::{AccessError, AccessPolicy};
use vexel_platform_apple::{VideoCodec, VideoError};

const NO_TIMELINE: &str = "export mp4 and mov need a native timeline; add a track with native_timeline_add_property_track first";
const BAD_EXTENSION: &str = "export path extension must match the format (.mp4 or .mov)";
const BAD_SIZE: &str = "export video size must be 1 to 4096 pixels per side";
const FAILED: &str = "file operation failed";

fn leaf_has_extension(path: &str, extension: &str) -> bool {
    let leaf = path.rsplit('/').next().unwrap_or("").as_bytes();
    let dot = extension.len() + 1;
    leaf.len() > dot
        && leaf[leaf.len() - dot] == b'.'
        && leaf[leaf.len() - extension.len()..].eq_ignore_ascii_case(extension.as_bytes())
}

fn access_error(error: AccessError) -> &'static str {
    match error {
        AccessError::NotGranted => "path not granted",
        AccessError::NotFound | AccessError::NotRegularFile | AccessError::NotDirectory => {
            "path not found"
        }
        AccessError::InvalidPath => "invalid preview tool arguments",
    }
}

fn video_error(error: NativeVideoError) -> &'static str {
    match error {
        NativeVideoError::Export(e) => render_failure(Tool::ExportGif, &e),
        NativeVideoError::BadSize { .. } => BAD_SIZE,
        NativeVideoError::BadLoops => "export loops must be at least 1",
        NativeVideoError::Video(VideoError::Invalid(_)) => {
            "export video request was rejected by the encoder"
        }
        NativeVideoError::Video(VideoError::Native(_)) => "export video encoder failed",
        NativeVideoError::Video(VideoError::Timeout(_)) => "export video encoder timed out",
    }
}

pub(super) fn export_video(
    app: &AppDocument,
    policy: &AccessPolicy,
    request: &VideoExport,
) -> Result<McpOutcome, &'static str> {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let extension = if request.mov { "mov" } else { "mp4" };
    if !leaf_has_extension(&request.path, extension) {
        return Err(BAD_EXTENSION);
    }
    let target = policy
        .resolve_save_path(&request.path)
        .map_err(access_error)?;
    let native = NativeExportSource::from_document(app).ok_or(NO_TIMELINE)?;
    let name = target.file_name().and_then(OsStr::to_str).ok_or(FAILED)?;
    let temp = target.with_file_name(format!(
        ".{name}.vexel-video-{}-{}.{extension}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = native.export_video(&NativeVideoRequest {
        path: temp.clone(),
        codec: request.codec,
        size: request.size,
        background: request.background,
        bit_rate: request.bit_rate,
        quality: request.quality,
        fps: request.fps,
        loops: request.loops,
    });
    let video = match result {
        Ok(video) => video,
        Err(error) => {
            let _ = fs::remove_file(&temp);
            return Err(video_error(error));
        }
    };
    let bytes = fs::metadata(&temp).map(|m| m.len()).map_err(|_| FAILED);
    if let Err(message) = bytes.and(fs::rename(&temp, &target).map_err(|_| FAILED)) {
        let _ = fs::remove_file(&temp);
        return Err(message);
    }
    let fps = f64::from(video.ticks_per_second) / f64::from(video.ticks_per_frame);
    let seconds = video.frames as f64 / fps;
    let mut reply = br#"{"path":"#.to_vec();
    reply.extend(quoted(target.as_os_str().as_bytes())?);
    reply.extend_from_slice(
        format!(
            r#","format":"{extension}","codec":"{}","width":{},"height":{},"frames":{},"fps":{fps:.3},"durationSeconds":{seconds:.3},"bytes":{}}}"#,
            if request.codec == VideoCodec::Hevc { "hevc" } else { "h264" },
            video.width,
            video.height,
            video.frames,
            bytes.unwrap_or(0)
        )
        .as_bytes(),
    );
    String::from_utf8(reply)
        .map(McpOutcome::Reply)
        .map_err(|_| "preview allocation failed")
}
