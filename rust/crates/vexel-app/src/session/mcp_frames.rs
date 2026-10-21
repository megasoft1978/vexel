//! `export_animation` as `apng` or `frames`: the native timeline loop as one
//! animated PNG, or as one PNG per sampled tick inside a granted directory.
//! Synchronous like `render_png`.
//!
//! An APNG is written atomically (temporary sibling, then rename). A frame
//! sequence refuses to start if any target file exists, creates files with
//! `create_new`, and removes the ones it wrote if a later frame fails.

use super::AppDocument;
use super::mcp::McpOutcome;
use super::mcp_png::{quoted, write_atomic};
use crate::animation::native_export::NativeExportSource;
use crate::animation::native_export_frames::{FramesPlan, NativeFramesError, NativeFramesRequest};
use crate::animation::native_export_message::{Tool, render_failure};
use crate::preview::FramesExport;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use vexel_mcp_access::{AccessError, AccessPolicy};
use vexel_png::{ApngFrame, Compression};

const NO_TIMELINE: &str = "export apng and frames need a native timeline; add a track with native_timeline_add_property_track first";
const BAD_EXTENSION: &str = "export apng path must end in .png or .apng";
const BAD_PATTERN: &str = "export frames path must be name_####.png (four or more # for the frame number) or a name prefix without extension";
const BAD_SIZE: &str = "export size must be 1 to 4096 pixels per side";
const TOO_LARGE: &str = "export too large; reduce size, fps or frame count";
const FAILED: &str = "file operation failed";
const BAD_ARGS: &str = "invalid preview tool arguments";
/// Most RGBA bytes an APNG export holds in memory at once.
const APNG_PIXEL_LIMIT: u64 = 256 * 1024 * 1024;
/// Largest APNG file written.
const APNG_FILE_LIMIT: usize = 256 * 1024 * 1024;
/// Most bytes one PNG sequence may write in total.
const FRAMES_TOTAL_LIMIT: u64 = 512 * 1024 * 1024;

fn access_error(error: AccessError) -> &'static str {
    match error {
        AccessError::NotGranted => "path not granted",
        AccessError::NotFound | AccessError::NotRegularFile | AccessError::NotDirectory => {
            "path not found"
        }
        AccessError::InvalidPath => BAD_ARGS,
    }
}

fn frames_error(error: NativeFramesError<&'static str>) -> &'static str {
    match error {
        NativeFramesError::Export(e) => render_failure(Tool::ExportGif, &e),
        NativeFramesError::BadSize => BAD_SIZE,
        NativeFramesError::Sink(message) => message,
    }
}

fn plan_error(error: NativeFramesError<()>) -> &'static str {
    match error {
        NativeFramesError::Export(e) => render_failure(Tool::ExportGif, &e),
        _ => BAD_SIZE,
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn leaf_has_png_extension(leaf: &str) -> bool {
    let bytes = leaf.as_bytes();
    let ends = |ext: &str| {
        bytes.len() > ext.len()
            && bytes[bytes.len() - ext.len()..].eq_ignore_ascii_case(ext.as_bytes())
    };
    ends(".png") || ends(".apng")
}

pub(super) fn export_frames(
    app: &AppDocument,
    policy: &AccessPolicy,
    request: &FramesExport,
) -> Result<McpOutcome, &'static str> {
    if request.apng {
        let leaf = request.path.rsplit('/').next().unwrap_or("");
        if !leaf_has_png_extension(leaf) {
            return Err(BAD_EXTENSION);
        }
        let target = policy
            .resolve_save_path(&request.path)
            .map_err(access_error)?;
        let native = NativeExportSource::from_document(app).ok_or(NO_TIMELINE)?;
        export_apng(&native, request, &target)
    } else {
        let names = Names::parse(&request.path)?;
        let directory = policy
            .resolve_granted_directory(names.directory)
            .map_err(access_error)?;
        let native = NativeExportSource::from_document(app).ok_or(NO_TIMELINE)?;
        export_sequence(&native, request, &directory, &names)
    }
}

fn sampling(request: &FramesExport) -> NativeFramesRequest {
    NativeFramesRequest {
        size: request.size,
        background: request.background,
        fps: request.fps,
    }
}

fn timing(plan: &FramesPlan) -> (f64, f64) {
    let fps = f64::from(plan.ticks_per_second) / f64::from(plan.stride);
    (fps, plan.frames as f64 / fps)
}

fn into_reply(reply: Vec<u8>) -> Result<McpOutcome, &'static str> {
    String::from_utf8(reply)
        .map(McpOutcome::Reply)
        .map_err(|_| "preview allocation failed")
}

fn export_apng(
    native: &NativeExportSource,
    request: &FramesExport,
    target: &Path,
) -> Result<McpOutcome, &'static str> {
    let sampling = sampling(request);
    let plan = native.plan_frames(&sampling).map_err(plan_error)?;
    let frame_bytes = u64::from(plan.width) * u64::from(plan.height) * 4;
    if frame_bytes * plan.frames as u64 > APNG_PIXEL_LIMIT {
        return Err(TOO_LARGE);
    }
    let divisor = gcd(plan.stride, plan.ticks_per_second).max(1);
    let (Ok(delay_num), Ok(delay_den)) = (
        u16::try_from(plan.stride / divisor),
        u16::try_from(plan.ticks_per_second / divisor),
    ) else {
        return Err("export apng timeline tick rate is too high for a frame delay");
    };
    let mut pixels: Vec<Vec<u8>> = Vec::new();
    pixels.try_reserve_exact(plan.frames).map_err(|_| FAILED)?;
    native
        .render_frames(&sampling, |_, rgba| {
            let mut copy = Vec::new();
            copy.try_reserve_exact(rgba.len()).map_err(|_| FAILED)?;
            copy.extend_from_slice(rgba);
            pixels.push(copy);
            Ok::<(), &'static str>(())
        })
        .map_err(frames_error)?;
    let frames: Vec<ApngFrame<'_>> = pixels
        .iter()
        .map(|rgba| ApngFrame {
            rgba,
            delay_num,
            delay_den,
        })
        .collect();
    let png = vexel_png::encode_apng(&frames, plan.width, plan.height, request.loops)
        .map_err(|_| TOO_LARGE)?;
    drop(frames);
    drop(pixels);
    if png.len() > APNG_FILE_LIMIT {
        return Err(TOO_LARGE);
    }
    write_atomic(target, &png)?;
    let (fps, seconds) = timing(&plan);
    let mut reply = br#"{"path":"#.to_vec();
    reply.extend(quoted(target.as_os_str().as_bytes())?);
    reply.extend_from_slice(
        format!(
            r#","format":"apng","width":{},"height":{},"frames":{},"fps":{fps:.3},"durationSeconds":{seconds:.3},"loops":{},"bytes":{}}}"#,
            plan.width,
            plan.height,
            plan.frames,
            request.loops,
            png.len()
        )
        .as_bytes(),
    );
    into_reply(reply)
}

/// File names of a PNG sequence: `prefix` + zero-padded number + `.png`.
struct Names<'a> {
    directory: &'a str,
    prefix: String,
    digits: usize,
}

impl<'a> Names<'a> {
    /// `name_####.png` (one run of 4 or more `#`) or `name` (no extension,
    /// files `name_0000.png`).
    fn parse(path: &'a str) -> Result<Self, &'static str> {
        let (directory, leaf) = match path.rsplit_once('/') {
            Some(("", leaf)) => ("/", leaf),
            Some(parts) => parts,
            None => return Err(BAD_ARGS),
        };
        if leaf.is_empty() {
            return Err(BAD_PATTERN);
        }
        let Some(start) = leaf.find('#') else {
            if leaf_has_png_extension(leaf) {
                return Err(BAD_PATTERN);
            }
            return Ok(Self {
                directory,
                prefix: format!("{leaf}_"),
                digits: 4,
            });
        };
        let digits = leaf[start..].bytes().take_while(|&b| b == b'#').count();
        let (head, tail) = (&leaf[..start], &leaf[start + digits..]);
        if digits < 4 || tail.contains('#') || !tail.eq_ignore_ascii_case(".png") {
            return Err(BAD_PATTERN);
        }
        Ok(Self {
            directory,
            prefix: head.to_owned(),
            digits,
        })
    }

    fn name(&self, index: usize) -> String {
        format!("{}{index:0width$}.png", self.prefix, width = self.digits)
    }
}

fn export_sequence(
    native: &NativeExportSource,
    request: &FramesExport,
    directory: &Path,
    names: &Names<'_>,
) -> Result<McpOutcome, &'static str> {
    let sampling = sampling(request);
    let plan = native.plan_frames(&sampling).map_err(plan_error)?;
    let targets: Vec<PathBuf> = (0..plan.frames)
        .map(|i| directory.join(names.name(i)))
        .collect();
    if targets.iter().any(|t| fs::symlink_metadata(t).is_ok()) {
        return Err(
            "export frames would overwrite existing files; use a new name or an empty directory",
        );
    }
    let mut written: Vec<&PathBuf> = Vec::new();
    let mut total = 0u64;
    let result = native.render_frames(&sampling, |index, rgba| {
        let png = vexel_png::encode_rgba8(plan.width, plan.height, rgba, Compression::Fixed)
            .map_err(|_| TOO_LARGE)?;
        total += png.len() as u64;
        if total > FRAMES_TOTAL_LIMIT {
            return Err(TOO_LARGE);
        }
        let target = &targets[index];
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
            .map_err(|_| FAILED)?;
        written.push(target);
        file.write_all(&png)
            .and_then(|()| file.sync_all())
            .map_err(|_| FAILED)
    });
    if let Err(error) = result {
        for path in written {
            let _ = fs::remove_file(path);
        }
        return Err(frames_error(error));
    }
    let (fps, seconds) = timing(&plan);
    let mut reply = br#"{"directory":"#.to_vec();
    reply.extend(quoted(directory.as_os_str().as_bytes())?);
    reply.extend_from_slice(br#","first":"#);
    reply.extend(quoted(names.name(0).as_bytes())?);
    reply.extend_from_slice(br#","last":"#);
    reply.extend(quoted(names.name(plan.frames - 1).as_bytes())?);
    reply.extend_from_slice(
        format!(
            r#","format":"frames","width":{},"height":{},"frames":{},"fps":{fps:.3},"durationSeconds":{seconds:.3},"bytes":{total}}}"#,
            plan.width, plan.height, plan.frames
        )
        .as_bytes(),
    );
    into_reply(reply)
}
