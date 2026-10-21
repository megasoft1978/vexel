//! `export_animation` with `format` `apng` or `frames`: argument validation.
//!
//! Every message starts with `export ` so the error whitelist passes it on.

use super::video_args::{MAX_LOOPS, MAX_SIDE, bad, hex_colour, integer, text};
use super::{Operation, PreviewArgumentError};
use vexel_json::JsonDocument;

const MAX_FPS: u32 = 120;

/// Validated animated-PNG or PNG-sequence request; `path` is not yet
/// resolved against a grant.
#[derive(Clone, Debug, PartialEq)]
pub struct FramesExport {
    pub path: String,
    /// One animated PNG when true, else one PNG per frame.
    pub apng: bool,
    pub fps: Option<u32>,
    /// APNG play count, 0 for forever.
    pub loops: u32,
    pub size: Option<(u32, u32)>,
    /// Colour transparent areas are composited onto; `None` keeps alpha.
    pub background: Option<[u8; 3]>,
}

/// `path` is already validated by the caller.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    path: String,
    apng: bool,
) -> Result<Operation, PreviewArgumentError> {
    let kind = if apng { "apng" } else { "frames" };
    for key in [&b"codec"[..], b"quality", b"bitrate", b"alpha"] {
        if doc.object_get(args, key).is_some() {
            return Err(bad(&format!(
                "export {} applies to mp4 and mov only, not {kind}",
                String::from_utf8_lossy(key)
            )));
        }
    }
    if doc.object_get(args, b"duration").is_some() {
        return Err(bad(&format!(
            "export duration applies to gif only; {kind} export the whole timeline loop"
        )));
    }
    if !apng && doc.object_get(args, b"loops").is_some() {
        return Err(bad("export loops applies to apng only"));
    }
    const SIDE: &str = "export width and height must be integers from 1 to 4096";
    let side =
        |key: &[u8]| integer(doc, args, key, 1..=u64::from(MAX_SIDE)).map_err(|()| bad(SIDE));
    let size = match (side(b"width")?, side(b"height")?) {
        (None, None) => None,
        (Some(w), Some(h)) => Some((w, h)),
        _ => return Err(bad("export width and height must be given together")),
    };
    let fps = integer(doc, args, b"fps", 1..=u64::from(MAX_FPS))
        .map_err(|()| bad("export fps must be an integer from 1 to 120"))?;
    let loops = integer(doc, args, b"loops", 1..=u64::from(MAX_LOOPS))
        .map_err(|()| bad("export loops must be an integer from 1 to 100"))?
        .unwrap_or(0); // 0 = forever
    let background = match text(doc, args, b"background") {
        None => None,
        Some(value) => Some(
            hex_colour(&value)
                .ok_or_else(|| bad("export background must be a #rgb or #rrggbb colour"))?,
        ),
    };
    Ok(Operation::ExportFrames(Box::new(FramesExport {
        path,
        apng,
        fps,
        loops,
        size,
        background,
    })))
}
