//! File > Export Video: encodes the native timeline loop as an H.264 MP4 at
//! the document's size, white behind transparency, or as HEVC with alpha in a
//! QuickTime `.mov`.
//!
//! Like the other exports it renders into a private staging directory beside
//! the chosen file and renames the finished video over it.

use crate::animation::native_export::NativeExportSource;
use crate::animation::native_export_video::NativeVideoRequest;
use crate::session::AppDocument;
use crate::toolbar_actions::Stage;
use std::path::PathBuf;
use vexel_platform_apple::VideoCodec;

const NEEDS_TIMELINE: &str = "Video export needs a native timeline";
const FAILED: &str = "Export could not render the video";

/// Writes `target` (an `.mp4` path); the status line on success.
pub fn write_video(doc: &AppDocument, target: PathBuf) -> Result<&'static str, &'static str> {
    write(doc, target, VideoCodec::H264, false).map(|()| "Exported video")
}

/// Writes `target` (a `.mov` path) as HEVC keeping transparency.
pub fn write_video_alpha(doc: &AppDocument, target: PathBuf) -> Result<&'static str, &'static str> {
    write(doc, target, VideoCodec::Hevc, true).map(|()| "Exported HEVC video with alpha")
}

fn write(
    doc: &AppDocument,
    target: PathBuf,
    codec: VideoCodec,
    alpha: bool,
) -> Result<(), &'static str> {
    let native = NativeExportSource::from_document(doc).ok_or(NEEDS_TIMELINE)?;
    let stage = Stage::create(&target)?;
    let encoded = native.export_video(&NativeVideoRequest {
        path: stage.file.clone(),
        codec,
        size: None,
        background: [255, 255, 255],
        alpha,
        bit_rate: None,
        quality: None,
        fps: None,
        loops: 1,
    });
    if encoded.is_err() {
        stage.discard();
        return Err(FAILED);
    }
    stage.publish(&target)
}
