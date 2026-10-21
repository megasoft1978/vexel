use super::*;
use std::sync::Arc;
use vexel_svg_animation::{Attribute, ValueKind, evaluate_view, parse_owned};
use vexel_xml::{XmlLimits, tokenize};

fn shape_rgb(shape: Shape) -> u32 {
    match shape {
        Shape::Rect { rgb, .. } | Shape::Ellipse { rgb, .. } => rgb,
    }
}
pub(super) fn rebuild_document(
    shapes: &[Shape],
    element_ids: &[String],
    animations: &[AnimationFragment],
) -> Result<V1Document, &'static str> {
    let mut doc = V1Document::new(512, 512).map_err(document_error)?;
    if shapes.len() != element_ids.len() {
        return Err("preview stable ID history is invalid");
    }
    for (shape, id) in shapes.iter().zip(element_ids) {
        add_shape(&mut doc, *shape, id)?;
    }
    validated(doc, shapes, element_ids, animations)
}

impl PreviewState {
    /// The document, replaying the scene on first use.
    pub(super) fn document(&self) -> Result<&V1Document, &'static str> {
        if self.document.get().is_none() {
            let built = rebuild_document(&self.shapes, &self.element_ids, &self.animations)?;
            let _ = self.document.set(built);
        }
        self.document.get().ok_or("preview allocation failed")
    }

    /// The document cell and source length after an edit that produced
    /// `shapes`. A plain add to a shape-only scene is O(1): the length grows
    /// by the new element's own bytes and the document is built when read.
    pub(super) fn next_document(
        &self,
        add: bool,
        shapes: &[Shape],
        element_ids: &[String],
        animations: &[AnimationFragment],
    ) -> Result<(std::cell::OnceCell<V1Document>, usize), &'static str> {
        let cell = std::cell::OnceCell::new();
        if add && animations.is_empty() {
            let (Some(shape), Some(id)) = (shapes.last(), element_ids.last()) else {
                return Err("preview stable ID history is invalid");
            };
            let mut blank = V1Document::new(512, 512).map_err(document_error)?;
            if shapes.len() > 1 {
                // Later siblings carry separator bytes the first one lacks.
                add_shape(&mut blank, *shape, "vexel-dummy")?;
            }
            let base = blank.source().map_err(document_error)?.len();
            add_shape(&mut blank, *shape, id)?;
            let grown = blank.source().map_err(document_error)?.len() - base;
            let len = self.source_len + grown;
            if len > SOURCE_LIMIT {
                return Err("preview source limit reached");
            }
            return Ok((cell, len));
        }
        let document = if add {
            extend_document(self.document()?, shapes, element_ids, animations)?
        } else {
            rebuild_document(shapes, element_ids, animations)?
        };
        let len = document.source().map_err(document_error)?.len();
        let _ = cell.set(document);
        Ok((cell, len))
    }
}

/// An independent copy of the built document, if there is one.
pub(super) fn copy_cell(
    cell: &std::cell::OnceCell<V1Document>,
) -> Result<std::cell::OnceCell<V1Document>, &'static str> {
    let copy = std::cell::OnceCell::new();
    if let Some(document) = cell.get() {
        let _ = copy.set(copy_document(document)?);
    }
    Ok(copy)
}

/// `previous` plus one more shape (the last of `shapes`), without replaying
/// the whole scene: O(source) instead of O(source x shapes).
pub(super) fn extend_document(
    previous: &V1Document,
    shapes: &[Shape],
    element_ids: &[String],
    animations: &[AnimationFragment],
) -> Result<V1Document, &'static str> {
    let (Some(shape), Some(id)) = (shapes.last(), element_ids.last()) else {
        return Err("preview stable ID history is invalid");
    };
    let mut doc = V1Document::from_source(&previous.source().map_err(document_error)?)
        .map_err(document_error)?;
    add_shape(&mut doc, *shape, id)?;
    validated(doc, shapes, element_ids, animations)
}

/// An independent copy of `document` (same source, fresh edit history).
pub(super) fn copy_document(document: &V1Document) -> Result<V1Document, &'static str> {
    V1Document::from_source(&document.source().map_err(document_error)?).map_err(document_error)
}

fn add_shape(doc: &mut V1Document, shape: Shape, id: &str) -> Result<(), &'static str> {
    {
        let shape = &shape;
        let mut color = [0u8; 7];
        color[0] = b'#';
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        let rgb = shape_rgb(*shape);
        for (index, shift) in [20, 16, 12, 8, 4, 0].into_iter().enumerate() {
            color[index + 1] = HEX[((rgb >> shift) & 0xf) as usize];
        }
        let color = std::str::from_utf8(&color).map_err(|_| "preview color encoding failed")?;
        match *shape {
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            } => doc.add_rectangle_with_id(x, y, x + width, y + height, &color, id),
            Shape::Ellipse { cx, cy, rx, ry, .. } => {
                doc.add_ellipse_with_id(cx - rx, cy - ry, cx + rx, cy + ry, &color, id)
            }
        }
        .map_err(document_error)?;
    }
    Ok(())
}

fn validated(
    mut doc: V1Document,
    shapes: &[Shape],
    element_ids: &[String],
    animations: &[AnimationFragment],
) -> Result<V1Document, &'static str> {
    for animation in animations {
        doc.add_animation_fragment(&animation.target_id, &animation.xml)
            .map_err(document_error)?;
    }
    if doc.source().map_err(document_error)?.len() > SOURCE_LIMIT {
        return Err("preview source limit reached");
    }
    if animations.is_empty() {
        // Only shapes this module wrote: no animation tracks to parse or check.
        return Ok(doc);
    }
    let parsed = parse_animation_document(&doc)?;
    let source = doc.source().map_err(document_error)?;
    if parsed.tracks().len() != animations.len() {
        return Err("unsupported or malformed preview animation");
    }
    validate_tracks(&parsed, &source)?;
    validate_track_targets(&parsed, &source, shapes, element_ids)?;
    Ok(doc)
}
fn parse_animation_document(
    document: &V1Document,
) -> Result<vexel_svg_animation::OwnedDocument, &'static str> {
    let source = document.source().map_err(document_error)?;
    let (tokens, result) = tokenize(
        &source,
        XmlLimits {
            input_bytes: SOURCE_LIMIT,
            ..XmlLimits::default()
        },
    );
    if result.diagnostic.category != vexel_xml::XmlError::None {
        return Err("invalid preview SVG source");
    }
    parse_owned(&Arc::new(source), &tokens).map_err(|_| "preview animation parsing failed")
}
pub(crate) fn validate_animation_fragment(xml: &[u8]) -> Result<(), &'static str> {
    if xml.is_empty() || xml.len() > ANIMATION_XML_LIMIT || !xml.starts_with(b"<animate") {
        return Err("invalid preview animation fragment");
    }
    let (tokens, result) = tokenize(
        xml,
        XmlLimits {
            input_bytes: ANIMATION_XML_LIMIT,
            ..XmlLimits::default()
        },
    );
    if result.diagnostic.category != vexel_xml::XmlError::None {
        return Err("invalid preview animation fragment");
    }
    let elements: Vec<_> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.kind,
                vexel_xml::TokenKind::StartElement | vexel_xml::TokenKind::EmptyElement
            )
        })
        .collect();
    if elements.len() != 1
        || elements[0].kind != vexel_xml::TokenKind::EmptyElement
        || elements[0].local_name.slice(xml) != Some(b"animate")
    {
        return Err("invalid preview animation fragment");
    }
    if tokens.iter().any(|t| {
        !matches!(
            t.kind,
            vexel_xml::TokenKind::EmptyElement | vexel_xml::TokenKind::Attribute
        )
    }) {
        return Err("invalid preview animation fragment");
    }
    const ALLOWED: &[&[u8]] = &[
        b"attributeName",
        b"values",
        b"dur",
        b"calcMode",
        b"keyTimes",
        b"keySplines",
        b"begin",
        b"repeatCount",
        b"fill",
    ];
    for token in tokens
        .iter()
        .filter(|t| t.kind == vexel_xml::TokenKind::Attribute)
    {
        let name = token
            .local_name
            .slice(xml)
            .ok_or("invalid preview animation fragment")?;
        if !ALLOWED.contains(&name) {
            return Err("external or unsupported preview animation attribute");
        }
        if token.value.slice(xml).is_none_or(|v| v.len() > 1024) {
            return Err("invalid preview animation fragment");
        }
    }
    Ok(())
}
fn validate_tracks(
    parsed: &vexel_svg_animation::OwnedDocument,
    source: &[u8],
) -> Result<(), &'static str> {
    if parsed.tracks().len() > ANIMATION_LIMIT {
        return Err("preview animation limit reached");
    }
    for track in parsed.tracks() {
        if !track.supported
            || track.kind != vexel_svg_animation::Kind::Animate
            || !matches!(
                track.calc_mode,
                vexel_svg_animation::CalcMode::Linear | vexel_svg_animation::CalcMode::Spline
            )
            || !matches!(
                track.attribute,
                Attribute::X | Attribute::Y | Attribute::Cx | Attribute::Cy | Attribute::Opacity
            )
            || track.keys.len() < 2
            || track.keys.len() > 16
            || track.keys.iter().any(|key| {
                key.value.kind != ValueKind::Number
                    || !key.value.number[0].is_finite()
                    || (track.attribute == Attribute::Opacity
                        && !(0.0..=1.0).contains(&key.value.number[0]))
            })
            || track
                .splines
                .iter()
                .flatten()
                .any(|control| !control.is_finite() || !(0.0..=1.0).contains(control))
            || !track.duration_seconds.is_finite()
            || !(0.1..=10.0).contains(&track.duration_seconds)
            || !track.begin_offset.is_finite()
            || !(0.0..=10.0).contains(&track.begin_offset)
            || track.begin_indefinite
            || track.has_begin_sync
            || track.repeat_indefinite
            || !track.repeat_count.is_finite()
            || !(1.0..=10.0).contains(&track.repeat_count)
            || track.begin_offset + track.duration_seconds * track.repeat_count > 10.0
        {
            return Err("unsupported or out-of-bound preview animation");
        }
    }
    if source.len() > SOURCE_LIMIT {
        return Err("preview source limit reached");
    }
    Ok(())
}
fn validate_track_targets(
    parsed: &vexel_svg_animation::OwnedDocument,
    source: &[u8],
    shapes: &[Shape],
    element_ids: &[String],
) -> Result<(), &'static str> {
    for track in parsed.tracks() {
        let target = track
            .target
            .slice(source)
            .ok_or("invalid preview animation target")?;
        let shape_index = shapes
            .iter()
            .enumerate()
            .find_map(|(i, _)| {
                let marker = format!("id=\"{}\"", element_ids.get(i)?);
                target
                    .windows(marker.len())
                    .any(|w| w == marker.as_bytes())
                    .then_some(i)
            })
            .ok_or("unknown preview animation target")?;
        let shape = shapes[shape_index];
        for key in &track.keys {
            let value = key.value.number[0];
            let inside = match (shape, track.attribute) {
                (Shape::Rect { width, .. }, Attribute::X) => (0.0..=512.0 - width).contains(&value),
                (Shape::Rect { height, .. }, Attribute::Y) => {
                    (0.0..=512.0 - height).contains(&value)
                }
                (Shape::Ellipse { rx, .. }, Attribute::Cx) => (rx..=512.0 - rx).contains(&value),
                (Shape::Ellipse { ry, .. }, Attribute::Cy) => (ry..=512.0 - ry).contains(&value),
                (_, Attribute::Opacity) => (0.0..=1.0).contains(&value),
                _ => false,
            };
            if !inside {
                return Err("animation value is outside preview bounds");
            }
        }
    }
    Ok(())
}
pub(crate) fn document_duration(document: &V1Document) -> Result<f64, &'static str> {
    let parsed = parse_animation_document(document)?;
    validate_tracks(&parsed, parsed.source())?;
    Ok(parsed
        .tracks()
        .iter()
        .map(|t| t.duration_seconds * t.repeat_count.min(10.0))
        .fold(0.0, f64::max)
        .min(10.0))
}
pub(crate) fn animation_list_json(
    document: &V1Document,
    element_ids: &[String],
) -> Result<String, &'static str> {
    let parsed = parse_animation_document(document)?;
    validate_tracks(&parsed, parsed.source())?;
    let mut json = String::from("{\"tracks\":[");
    for (track_index, track) in parsed.tracks().iter().enumerate() {
        if track_index > 0 {
            json.push(',');
        }
        let target = track
            .target
            .slice(parsed.source())
            .ok_or("invalid preview animation target")?;
        let id = element_ids
            .iter()
            .find(|id| {
                let marker = format!("id=\"{id}\"");
                target
                    .windows(marker.len())
                    .any(|window| window == marker.as_bytes())
            })
            .ok_or("unknown preview animation target")?;
        use std::fmt::Write as _;
        write!(
            json,
            "{{\"targetId\":\"{id}\",\"attributeName\":\"{}\",\"durationMs\":{:.0},\"values\":[",
            vexel_svg_animation::attribute_name(track.attribute),
            track.duration_seconds * 1000.0
        )
        .map_err(|_| "preview allocation failed")?;
        for (key_index, key) in track.keys.iter().enumerate() {
            if key_index > 0 {
                json.push(',');
            }
            write!(json, "{:.8}", key.value.number[0]).map_err(|_| "preview allocation failed")?;
        }
        json.push_str("]}");
    }
    json.push_str("]}");
    Ok(json)
}
pub(crate) fn validate_shape(shape: Shape) -> Result<(), &'static str> {
    let (l, t, r, b) = match shape {
        Shape::Rect {
            x,
            y,
            width,
            height,
            ..
        } => (x, y, x + width, y + height),
        Shape::Ellipse { cx, cy, rx, ry, .. } => (cx - rx, cy - ry, cx + rx, cy + ry),
    };
    if shape_rgb(shape) > 0x00ff_ffff
        || ![l, t, r, b].into_iter().all(f64::is_finite)
        || l < 0.0
        || t < 0.0
        || r > 512.0
        || b > 512.0
        || r <= l
        || b <= t
    {
        return Err("shape geometry must be finite, non-degenerate, and inside 512x512");
    }
    Ok(())
}

/// Deterministic fixed-size renderer for the intentionally limited preview scene.
// Test-only: the shell renders through `compose`, not this preview rasterizer.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn rasterize(shapes: &[Shape]) -> Result<Vec<u8>, &'static str> {
    rasterize_with_opacity(shapes, &vec![1.0; shapes.len()])
}
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn rasterize_at(
    state: &PreviewState,
    time_seconds: f64,
) -> Result<Vec<u8>, &'static str> {
    if !time_seconds.is_finite() || !(0.0..=10.0).contains(&time_seconds) {
        return Err("invalid preview animation time");
    }
    let parsed = parse_animation_document(state.document()?)?;
    validate_tracks(&parsed, parsed.source())?;
    let overlay =
        evaluate_view(parsed.view(), time_seconds).ok_or("preview animation evaluation failed")?;
    let mut shapes = clone_shapes(&state.shapes)?;
    let mut opacity = vec![1.0; shapes.len()];
    for item in overlay.items {
        let Some(track) = parsed.tracks().get(item.track_index) else {
            continue;
        };
        let Some(target) = track.target.slice(parsed.source()) else {
            continue;
        };
        let target_index = state.shapes.iter().enumerate().find_map(|(i, _)| {
            let id = state.element_ids.get(i)?;
            let quoted = format!("id=\"{id}\"");
            target
                .windows(quoted.len())
                .any(|window| window == quoted.as_bytes())
                .then_some(i)
        });
        let Some(index) = target_index else {
            continue;
        };
        let value = item.value.number[0];
        match (&mut shapes[index], track.attribute) {
            (Shape::Rect { x, .. }, Attribute::X) => *x = value,
            (Shape::Rect { y, .. }, Attribute::Y) => *y = value,
            (Shape::Ellipse { cx, .. }, Attribute::Cx) => *cx = value,
            (Shape::Ellipse { cy, .. }, Attribute::Cy) => *cy = value,
            (_, Attribute::Opacity) => opacity[index] = value.clamp(0.0, 1.0),
            _ => return Err("animation target property does not match shape"),
        }
    }
    rasterize_with_opacity(&shapes, &opacity)
}
#[cfg_attr(not(test), allow(dead_code))]
fn rasterize_with_opacity(shapes: &[Shape], opacity: &[f64]) -> Result<Vec<u8>, &'static str> {
    if shapes.len() > SCENE_LIMIT {
        return Err("preview scene limit reached");
    }
    if opacity.len() != shapes.len()
        || opacity
            .iter()
            .any(|o| !o.is_finite() || !(0.0..=1.0).contains(o))
    {
        return Err("invalid preview opacity");
    }
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(512 * 512 * 4)
        .map_err(|_| "preview allocation failed")?;
    pixels.resize(512 * 512 * 4, 255);
    for (shape_index, s) in shapes.iter().enumerate() {
        validate_shape(*s)?;
        let (l, t, r, b) = match *s {
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            } => (x, y, x + width, y + height),
            Shape::Ellipse { cx, cy, rx, ry, .. } => (cx - rx, cy - ry, cx + rx, cy + ry),
        };
        if l < 0.0 || t < 0.0 || r > 512.0 || b > 512.0 {
            return Err("animated geometry is outside preview bounds");
        }
        let rgb = shape_rgb(*s);
        let color = [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8];
        let (l, t, r, b) = match *s {
            Shape::Rect {
                x,
                y,
                width,
                height,
                ..
            } => (x, y, x + width, y + height),
            Shape::Ellipse { cx, cy, rx, ry, .. } => (cx - rx, cy - ry, cx + rx, cy + ry),
        };
        for y in t.floor().max(0.0) as u32..b.ceil().min(512.0) as u32 {
            for x in l.floor().max(0.0) as u32..r.ceil().min(512.0) as u32 {
                let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                let inside = match *s {
                    Shape::Rect { .. } => px >= l && px < r && py >= t && py < b,
                    Shape::Ellipse { cx, cy, rx, ry, .. } => {
                        ((px - cx) / rx).powi(2) + ((py - cy) / ry).powi(2) <= 1.0
                    }
                };
                if inside {
                    let at = (y as usize * 512 + x as usize) * 4;
                    let alpha = opacity[shape_index];
                    for channel in 0..3 {
                        pixels[at + channel] = (f64::from(color[channel]) * alpha
                            + f64::from(pixels[at + channel]) * (1.0 - alpha))
                            .round() as u8;
                    }
                    pixels[at + 3] = 255;
                }
            }
        }
    }
    Ok(pixels)
}
