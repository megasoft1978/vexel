//! `export_animation` with `format` `mp4` or `mov`: argument validation.
//!
//! Every message starts with `export ` so the error whitelist passes it on.

use super::{Operation, PreviewArgumentError};
use vexel_json::{JsonDocument, JsonType};
use vexel_platform_apple::VideoCodec;

/// Longest edge of an exported video, in pixels.
pub(crate) const MAX_SIDE: u32 = 4096;
/// Most times the loop may be repeated.
pub(crate) const MAX_LOOPS: u32 = 100;
const MAX_FPS: u32 = 120;
const MAX_BITRATE_KBPS: u64 = 200_000;

/// Validated video export request; `path` is not yet resolved against a grant.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoExport {
    pub path: String,
    /// `mov` when true, else `mp4`.
    pub mov: bool,
    pub codec: VideoCodec,
    pub quality: Option<f64>,
    pub bit_rate: Option<u32>,
    pub size: Option<(u32, u32)>,
    pub fps: Option<u32>,
    pub loops: u32,
    pub background: [u8; 3],
    /// Keep transparency (HEVC with alpha; `mov` + `hevc` only).
    pub alpha: bool,
}

pub(super) fn bad(message: &str) -> PreviewArgumentError {
    PreviewArgumentError::Actionable(message.to_owned())
}

pub(super) fn number(doc: &JsonDocument<'_>, args: usize, key: &[u8]) -> Result<Option<f64>, ()> {
    let Some(token) = doc.object_get(args, key) else {
        return Ok(None);
    };
    doc.token(token)
        .filter(|t| t.kind == JsonType::Number)
        .and_then(|_| doc.raw_token(token))
        .filter(|raw| raw.len() <= 32)
        .and_then(|raw| std::str::from_utf8(raw).ok()?.parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .map(Some)
        .ok_or(())
}

pub(super) fn integer(
    doc: &JsonDocument<'_>,
    args: usize,
    key: &[u8],
    range: std::ops::RangeInclusive<u64>,
) -> Result<Option<u32>, ()> {
    match number(doc, args, key)? {
        None => Ok(None),
        Some(n) if n.fract() == 0.0 && n >= 0.0 && range.contains(&(n as u64)) => {
            Ok(Some(n as u32))
        }
        Some(_) => Err(()),
    }
}

pub(super) fn text(doc: &JsonDocument<'_>, args: usize, key: &[u8]) -> Option<String> {
    let token = doc.object_get(args, key)?;
    if doc.token(token)?.kind != JsonType::String || doc.raw_token(token)?.len() > 64 {
        return None;
    }
    String::from_utf8(doc.decoded_string(token)?).ok()
}

pub(super) fn hex_colour(value: &str) -> Option<[u8; 3]> {
    let hex = value.strip_prefix('#')?;
    if !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    let one = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok().map(|d| d * 17);
    match hex.len() {
        6 => Some([pair(0)?, pair(2)?, pair(4)?]),
        3 => Some([one(0)?, one(1)?, one(2)?]),
        _ => None,
    }
}

/// `path` is already validated by the caller.
pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    path: String,
    mov: bool,
) -> Result<Operation, PreviewArgumentError> {
    if doc.object_get(args, b"duration").is_some() {
        return Err(bad(
            "export duration applies to gif only; mp4 and mov export the whole timeline loop (use loops to repeat it)",
        ));
    }
    let codec = match text(doc, args, b"codec").as_deref() {
        None | Some("h264") => VideoCodec::H264,
        Some("hevc") => VideoCodec::Hevc,
        Some(_) => return Err(bad("export codec must be h264 or hevc")),
    };
    let quality = number(doc, args, b"quality")
        .map_err(|()| bad("export quality must be a number from 0 to 1"))?;
    if quality.is_some_and(|q| !(0.0..=1.0).contains(&q)) {
        return Err(bad("export quality must be a number from 0 to 1"));
    }
    let bit_rate = integer(doc, args, b"bitrate", 50..=MAX_BITRATE_KBPS)
        .map_err(|()| bad("export bitrate is in kbps, an integer from 50 to 200000"))?
        .map(|kbps| kbps * 1000);
    if quality.is_some() && bit_rate.is_some() {
        return Err(bad("export takes quality or bitrate, not both"));
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
        .unwrap_or(1);
    let background = match text(doc, args, b"background") {
        None => [255, 255, 255],
        Some(value) => hex_colour(&value)
            .ok_or_else(|| bad("export background must be a #rgb or #rrggbb colour"))?,
    };
    let alpha = match doc.object_get(args, b"alpha") {
        None => false,
        Some(token) => match doc.token(token).map(|t| t.kind) {
            Some(JsonType::True) => true,
            Some(JsonType::False) => false,
            _ => return Err(bad("export alpha must be true or false")),
        },
    };
    if alpha && !mov {
        return Err(bad(
            "export alpha needs format mov (mp4 cannot carry alpha)",
        ));
    }
    if alpha && codec != VideoCodec::Hevc {
        return Err(bad(
            "export alpha needs codec hevc (h264 cannot carry alpha)",
        ));
    }
    Ok(Operation::ExportVideo(Box::new(VideoExport {
        path,
        mov,
        codec,
        quality,
        bit_rate,
        size,
        fps,
        loops,
        background,
        alpha,
    })))
}
