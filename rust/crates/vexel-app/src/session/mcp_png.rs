//! `render_png`: render the open document to a PNG file in a granted
//! directory.
//!
//! Pixels come from the same scene render the GIF exporter uses at time 0
//! ([`AnimatedScene::render_sized`]); that tile is straight (non-premultiplied)
//! RGBA, which is exactly what PNG colour type 6 stores, so no conversion is
//! applied. The whole file is encoded in memory, written to a temporary
//! sibling and renamed over the target, so a failure leaves any existing file
//! unchanged and no temporary file behind.

use super::AppDocument;
use super::mcp::{McpOutcome, source};
use crate::animation::frame::{self, AnimationError};
use crate::animation::native_export::{NativeExportError, NativeExportSource};
use crate::animation::native_export_message::{Tool, render_failure};
use crate::preview::Operation;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use vexel_json::{JsonBuffer, JsonDocument, JsonType};
use vexel_mcp_access::{AccessError, AccessPolicy};
use vexel_png::{Compression, PngError};

const BAD_ARGS: &str = "invalid preview tool arguments";
const TOO_LARGE: &str = "export png too large";
const TIME_RANGE: &str = "render_png time is in seconds and is past the end of the timeline; use tick / ticksPerSecond (see native_timeline_get)";
const RENDER_FAILED: &str = "export png render failed";
/// Longest accepted `path` argument in bytes (matches the access policy cap).
const PATH_LIMIT: usize = 1023;
const MAX_SIDE: u32 = 4096;
/// Largest PNG file written.
const OUTPUT_LIMIT: usize = 64 * 1024 * 1024;

/// Validates the `path`, `width` and `height` arguments.
pub(crate) fn parse(doc: &JsonDocument<'_>, args: usize) -> Result<Operation, &'static str> {
    let path = doc
        .object_get(args, b"path")
        .filter(|t| {
            doc.token(*t).is_some_and(|k| k.kind == JsonType::String)
                && doc
                    .raw_token(*t)
                    .is_some_and(|raw| raw.len() <= PATH_LIMIT * 6 + 2)
        })
        .and_then(|t| doc.decoded_string(t))
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .filter(|p| !p.is_empty() && p.len() <= PATH_LIMIT && !p.contains('\0'))
        .ok_or(BAD_ARGS)?;
    let side = |key: &[u8]| -> Result<Option<u32>, &'static str> {
        let Some(token) = doc.object_get(args, key) else {
            return Ok(None);
        };
        doc.uint(token)
            .filter(|v| (1..=u64::from(MAX_SIDE)).contains(v))
            .map(|v| Some(v as u32))
            .ok_or(BAD_ARGS)
    };
    let time = match doc.object_get(args, b"time") {
        None => None,
        Some(token) => Some(
            doc.token(token)
                .filter(|k| k.kind == JsonType::Number)
                .and_then(|_| doc.raw_token(token))
                .and_then(|raw| std::str::from_utf8(raw).ok())
                .and_then(|text| text.parse::<f64>().ok())
                .filter(|t| t.is_finite() && (0.0..=3600.0).contains(t))
                .ok_or(BAD_ARGS)?,
        ),
    };
    Ok(Operation::RenderPng {
        path,
        width: side(b"width")?,
        height: side(b"height")?,
        time,
    })
}

fn access_error(error: AccessError) -> &'static str {
    match error {
        AccessError::NotGranted => "path not granted",
        AccessError::NotFound | AccessError::NotRegularFile | AccessError::NotDirectory => {
            "path not found"
        }
        AccessError::InvalidPath => BAD_ARGS,
    }
}

fn png_error(error: PngError) -> &'static str {
    match error {
        PngError::OutOfMemory => "file operation failed",
        _ => TOO_LARGE,
    }
}

fn leaf_is_png(path: &str) -> bool {
    let leaf = path.rsplit('/').next().unwrap_or("");
    leaf.len() > ".png".len() && leaf.as_bytes()[leaf.len() - 4..].eq_ignore_ascii_case(b".png")
}

/// Output size: the requested sides, a missing side following the document's
/// aspect, and the document size capped to [`MAX_SIDE`] on its long axis when
/// neither is given.
fn target_size(
    document: (u32, u32),
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(u32, u32), &'static str> {
    let (dw, dh) = (u64::from(document.0.max(1)), u64::from(document.1.max(1)));
    let scaled = |side: u64, num: u64, den: u64| (side * num + den / 2) / den;
    let (w, h) = match (width, height) {
        (Some(w), Some(h)) => (u64::from(w), u64::from(h)),
        (Some(w), None) => (u64::from(w), scaled(u64::from(w), dh, dw).max(1)),
        (None, Some(h)) => (scaled(u64::from(h), dw, dh).max(1), u64::from(h)),
        (None, None) => {
            let long = dw.max(dh);
            let cap = u64::from(MAX_SIDE);
            if long <= cap {
                (dw, dh)
            } else {
                (scaled(dw, cap, long).max(1), scaled(dh, cap, long).max(1))
            }
        }
    };
    if w > u64::from(MAX_SIDE) || h > u64::from(MAX_SIDE) {
        return Err(BAD_ARGS);
    }
    Ok((w as u32, h as u32))
}

fn quoted(bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
    let failed = |_| "preview allocation failed";
    let mut out = JsonBuffer::new(bytes.len() + 16, 64 * 1024).map_err(failed)?;
    out.quoted(bytes).map_err(failed)?;
    Ok(out.as_bytes().to_vec())
}

pub(super) fn render_png(
    app: &AppDocument,
    policy: &AccessPolicy,
    path: &str,
    width: Option<u32>,
    height: Option<u32>,
    time: Option<f64>,
) -> Result<McpOutcome, &'static str> {
    if !leaf_is_png(path) {
        return Err("unsupported file type");
    }
    let target = policy.resolve_save_path(path).map_err(access_error)?;
    let mut tick = None;
    let (w, h, rgba) = if let Some(native) = NativeExportSource::from_document(app) {
        let (w, h, at, rgba) = native
            .render_still_at(time.unwrap_or(0.0), |doc| target_size(doc, width, height))
            .map_err(|error| match error {
                NativeExportError::TimeOutOfRange { .. } => TIME_RANGE,
                other => render_failure(Tool::RenderPng, &other),
            })?;
        tick = Some(at);
        (w, h, rgba)
    } else {
        let source = source(app)?;
        let scene = frame::decode(&source).map_err(|e| match e {
            AnimationError::Limit => TOO_LARGE,
            _ => RENDER_FAILED,
        })?;
        let (w, h) = target_size(scene.size(), width, height)?;
        let tile = scene
            .render_sized(time.unwrap_or(0.0), (w, h), &AtomicBool::new(false))
            .map_err(|e| match e {
                AnimationError::Limit => TOO_LARGE,
                _ => RENDER_FAILED,
            })?;
        (w, h, tile.rgba().to_vec())
    };
    let png = vexel_png::encode_rgba8(w, h, &rgba, Compression::Fixed).map_err(png_error)?;
    if png.len() > OUTPUT_LIMIT {
        return Err(TOO_LARGE);
    }
    write_atomic(&target, &png)?;
    let mut reply = br#"{"path":"#.to_vec();
    reply.extend(quoted(target.as_os_str().as_bytes())?);
    reply.extend_from_slice(
        format!(r#","width":{w},"height":{h},"bytes":{}"#, png.len()).as_bytes(),
    );
    if let Some(tick) = tick {
        reply.extend_from_slice(format!(r#","tick":{tick}"#).as_bytes());
    }
    reply.push(b'}');
    String::from_utf8(reply)
        .map(McpOutcome::Reply)
        .map_err(|_| "preview allocation failed")
}

/// Writes `bytes` to a temporary sibling of `target` and renames it over
/// `target`; the temporary file is removed on any failure.
fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), &'static str> {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let failed = "file operation failed";
    let name = target.file_name().and_then(OsStr::to_str).ok_or(failed)?;
    let temp = target.with_file_name(format!(
        ".{name}.vexel-png-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|_| failed)?;
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| {
            drop(file);
            fs::rename(&temp, target)
        });
    if result.is_err() {
        let _ = fs::remove_file(&temp);
        return Err(failed);
    }
    Ok(())
}
