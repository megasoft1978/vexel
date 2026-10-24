//! Native timeline-v1 drawing-cel asset validation and frame composition.
//!
//! Tracks are supplied in serialized timeline order. This module stays below
//! playback and does not apply loop, onion-skin, or clock policy.

use crate::compose::{Frame, compose_svg};
use std::collections::BTreeSet;

pub const MAX_FRAME_PIXEL_BYTES: usize = 512 * 1024 * 1024;
pub const SVG_MEDIA_TYPE: &[u8] = b"image/svg+xml";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgGeometry {
    pub width: f64,
    pub height: f64,
    pub view_box: [f64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawingCel<'a> {
    pub tick: u32,
    pub asset: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawingTrack<'a> {
    pub visible: bool,
    pub cels: &'a [DrawingCel<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawingAsset<'a> {
    pub name: &'a [u8],
    pub media_type: &'a [u8],
    pub source: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingError {
    InvalidFrame,
    InvalidTimeline,
    MissingAsset,
    InvalidMediaType,
    InvalidSvg,
    UnsafeSvg,
    GeometryMismatch,
    Limit,
    Render,
}

/// Return the last cel at or before `tick`, preserving authored order checks.
pub fn active_cel<'a>(
    cels: &'a [DrawingCel<'a>],
    tick: u32,
) -> Result<Option<&'a DrawingCel<'a>>, DrawingError> {
    if cels.windows(2).any(|pair| pair[0].tick >= pair[1].tick) {
        return Err(DrawingError::InvalidTimeline);
    }
    Ok(cels.iter().take_while(|cel| cel.tick <= tick).last())
}

/// Validate a complete SVG drawing document against the scene root geometry.
/// The local tokenizer is used only for structural inspection; rendering still
/// goes through the app's owned SVG decoder and renderer.
pub fn validate_drawing_svg(source: &[u8], expected: SvgGeometry) -> Result<(), DrawingError> {
    use vexel_xml::{TokenKind, tokenize_large};

    if !expected.width.is_finite()
        || !expected.height.is_finite()
        || expected.width <= 0.0
        || expected.height <= 0.0
        || expected.view_box.iter().any(|value| !value.is_finite())
        || expected.view_box[2] <= 0.0
        || expected.view_box[3] <= 0.0
    {
        return Err(DrawingError::GeometryMismatch);
    }
    let (tokens, result) = tokenize_large(source);
    if result.diagnostic.category != vexel_xml::XmlError::None || tokens.is_empty() {
        return Err(DrawingError::InvalidSvg);
    }
    let mut ids = BTreeSet::<Vec<u8>>::new();
    let mut refs = Vec::<Vec<u8>>::new();
    let mut root_width = None;
    let mut root_height = None;
    let mut root_view_box = None;
    let mut root_seen = false;
    for token in &tokens {
        let local = token.local_name.slice(source).unwrap_or_default();
        match token.kind {
            TokenKind::StartElement | TokenKind::EmptyElement => {
                if token.depth == 0 {
                    if root_seen || local != b"svg" {
                        return Err(DrawingError::InvalidSvg);
                    }
                    root_seen = true;
                }
                if local.eq_ignore_ascii_case(b"script") {
                    return Err(DrawingError::UnsafeSvg);
                }
            }
            TokenKind::Attribute => {
                let name = token.name.slice(source).unwrap_or_default();
                let value = token.value.slice(source).unwrap_or_default();
                if name.eq_ignore_ascii_case(b"xml:base") {
                    return Err(DrawingError::UnsafeSvg);
                }
                if name.len() > 2 && name[..2].eq_ignore_ascii_case(b"on") {
                    return Err(DrawingError::UnsafeSvg);
                }
                if name.eq_ignore_ascii_case(b"id") {
                    if value.is_empty() || !ids.insert(value.to_vec()) {
                        return Err(DrawingError::UnsafeSvg);
                    }
                }
                if name.eq_ignore_ascii_case(b"href")
                    || name.ends_with(b":href")
                    || name.eq_ignore_ascii_case(b"src")
                {
                    collect_href(value, &mut refs)?;
                }
                if name.eq_ignore_ascii_case(b"style")
                    || name.eq_ignore_ascii_case(b"fill")
                    || name.eq_ignore_ascii_case(b"stroke")
                    || name.eq_ignore_ascii_case(b"filter")
                    || name.eq_ignore_ascii_case(b"clip-path")
                    || name.eq_ignore_ascii_case(b"mask")
                    || name.eq_ignore_ascii_case(b"marker")
                    || name.eq_ignore_ascii_case(b"cursor")
                {
                    collect_urls(value, &mut refs)?;
                }
                // Root attributes follow the root element token in the token stream.
                if token.depth == 0 && (name == b"width" || name == b"height" || name == b"viewBox")
                {
                    match name {
                        b"width" => {
                            root_width = Some(parse_length(value).ok_or(DrawingError::InvalidSvg)?)
                        }
                        b"height" => {
                            root_height = Some(parse_length(value).ok_or(DrawingError::InvalidSvg)?)
                        }
                        b"viewBox" => {
                            root_view_box =
                                Some(parse_view_box(value).ok_or(DrawingError::InvalidSvg)?)
                        }
                        _ => {}
                    }
                }
            }
            TokenKind::Text | TokenKind::Cdata => {
                let value = token.value.slice(source).unwrap_or_default();
                if contains_ascii_case_insensitive(value, b"@import") || value.contains(&b'\\') {
                    return Err(DrawingError::UnsafeSvg);
                }
                if value
                    .windows(4)
                    .any(|part| part.eq_ignore_ascii_case(b"url("))
                {
                    collect_urls(value, &mut refs)?;
                }
            }
            TokenKind::Doctype => return Err(DrawingError::UnsafeSvg),
            _ => {}
        }
    }
    if !root_seen {
        return Err(DrawingError::InvalidSvg);
    }
    let width = root_width.ok_or(DrawingError::InvalidSvg)?;
    let height = root_height.ok_or(DrawingError::InvalidSvg)?;
    let view_box = root_view_box.ok_or(DrawingError::InvalidSvg)?;
    if width != expected.width || height != expected.height || view_box != expected.view_box {
        return Err(DrawingError::GeometryMismatch);
    }
    if refs.iter().any(|id| !ids.contains(id)) {
        return Err(DrawingError::UnsafeSvg);
    }
    Ok(())
}

/// Render active visible drawing layers and composite in track order.
/// Failure is atomic: the base frame is returned only after every layer passes.
pub fn compose_drawing_tracks(
    base: &Frame,
    tracks: &[DrawingTrack<'_>],
    assets: &[DrawingAsset<'_>],
    tick: u32,
    geometry: SvgGeometry,
) -> Result<Frame, DrawingError> {
    let expected = frame_bytes(base.width, base.height)?;
    if base.rgba.len() != expected {
        return Err(DrawingError::InvalidFrame);
    }
    let mut cumulative = expected;
    let mut layers = Vec::new();
    for track in tracks {
        let Some(cel) = active_cel(track.cels, tick)? else {
            continue;
        };
        if !track.visible {
            continue;
        }
        let asset = assets
            .iter()
            .find(|asset| asset.name == cel.asset)
            .ok_or(DrawingError::MissingAsset)?;
        if asset.media_type != SVG_MEDIA_TYPE {
            return Err(DrawingError::InvalidMediaType);
        }
        validate_drawing_svg(asset.source, geometry)?;
        cumulative = admit_frame_bytes(cumulative, expected)?;
        let rendered =
            compose_svg(asset.source, base.width, base.height).map_err(|_| DrawingError::Render)?;
        if rendered.width != base.width
            || rendered.height != base.height
            || rendered.rgba.len() != expected
        {
            return Err(DrawingError::Render);
        }
        layers.try_reserve(1).map_err(|_| DrawingError::Limit)?;
        layers.push(rendered);
    }

    let mut output = Vec::new();
    output
        .try_reserve_exact(expected)
        .map_err(|_| DrawingError::Limit)?;
    output.extend_from_slice(&base.rgba);
    if layers.is_empty() {
        // Nothing to overlay: only the premultiply round trip remains.
        for pixel in output.chunks_exact_mut(4) {
            premultiply(pixel);
            unpremultiply(pixel);
        }
        return Ok(Frame {
            width: base.width,
            height: base.height,
            rgba: output,
            disclosures: Vec::new(),
        });
    }
    for pixel in output.chunks_exact_mut(4) {
        premultiply(pixel);
    }
    for layer in layers {
        for (destination, source) in output.chunks_exact_mut(4).zip(layer.rgba.chunks_exact(4)) {
            let alpha = u32::from(source[3]);
            let inverse = 255 - alpha;
            for channel in 0..3 {
                let src = (u32::from(source[channel]) * alpha + 127) / 255;
                destination[channel] =
                    (src + (u32::from(destination[channel]) * inverse + 127) / 255) as u8;
            }
            destination[3] = (alpha + (u32::from(destination[3]) * inverse + 127) / 255) as u8;
        }
    }
    for pixel in output.chunks_exact_mut(4) {
        unpremultiply(pixel);
    }
    Ok(Frame {
        width: base.width,
        height: base.height,
        rgba: output,
        disclosures: Vec::new(),
    })
}

fn frame_bytes(width: u32, height: u32) -> Result<usize, DrawingError> {
    usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(DrawingError::Limit)
}
pub(super) fn admit_frame_bytes(cumulative: usize, next: usize) -> Result<usize, DrawingError> {
    let total = cumulative.checked_add(next).ok_or(DrawingError::Limit)?;
    if total > MAX_FRAME_PIXEL_BYTES {
        return Err(DrawingError::Limit);
    }
    Ok(total)
}
fn premultiply(pixel: &mut [u8]) {
    let alpha = u32::from(pixel[3]);
    // Exact: (c * 255 + 127) / 255 == c.
    if alpha == 255 {
        return;
    }
    for channel in 0..3 {
        pixel[channel] = ((u32::from(pixel[channel]) * alpha + 127) / 255) as u8;
    }
}
fn unpremultiply(pixel: &mut [u8]) {
    let alpha = u32::from(pixel[3]);
    // Exact: (c * 255 + 127) / 255 == c.
    if alpha == 255 {
        return;
    }
    if alpha == 0 {
        pixel[..3].fill(0);
        return;
    }
    for channel in 0..3 {
        pixel[channel] = ((u32::from(pixel[channel]) * 255 + alpha / 2) / alpha).min(255) as u8;
    }
}
fn collect_href(value: &[u8], refs: &mut Vec<Vec<u8>>) -> Result<(), DrawingError> {
    let value = trim(value);
    if let Some(id) = value.strip_prefix(b"#") {
        if id.is_empty() {
            return Err(DrawingError::UnsafeSvg);
        }
        refs.push(id.to_vec());
        Ok(())
    } else {
        Err(DrawingError::UnsafeSvg)
    }
}
fn collect_urls(value: &[u8], refs: &mut Vec<Vec<u8>>) -> Result<(), DrawingError> {
    let mut rest = value;
    while let Some(index) = find_ascii_case_insensitive(rest, b"url(") {
        rest = &rest[index + 4..];
        let close = rest
            .iter()
            .position(|byte| *byte == b')')
            .ok_or(DrawingError::UnsafeSvg)?;
        let target = trim(
            rest[..close]
                .strip_prefix(b"'")
                .or_else(|| rest[..close].strip_prefix(b"\""))
                .unwrap_or(&rest[..close]),
        );
        let target = target
            .strip_suffix(b"'")
            .or_else(|| target.strip_suffix(b"\""))
            .unwrap_or(target);
        collect_href(target, refs)?;
        rest = &rest[close + 1..];
    }
    if contains_ascii_case_insensitive(value, b"url(") && !value.contains(&b')') {
        return Err(DrawingError::UnsafeSvg);
    }
    Ok(())
}
fn find_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|part| part.eq_ignore_ascii_case(needle))
}
fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    find_ascii_case_insensitive(haystack, needle).is_some()
}
fn trim(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }
    while bytes.last().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}
fn parse_length(bytes: &[u8]) -> Option<f64> {
    let bytes = trim(bytes);
    let number = bytes.strip_suffix(b"px").unwrap_or(bytes);
    let value = std::str::from_utf8(number).ok()?.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}
fn parse_view_box(bytes: &[u8]) -> Option<[f64; 4]> {
    let text = std::str::from_utf8(bytes).ok()?;
    let values = text
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let [x, y, width, height]: [f64; 4] = values.try_into().ok()?;
    [x, y, width, height]
        .iter()
        .all(|n| n.is_finite())
        .then_some([x, y, width, height])
}
