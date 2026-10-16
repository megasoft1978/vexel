use crate::MAX_GRADIENT_STOPS;
use crate::cache::{self, invalid, limit, oom, unsupported};
use crate::control::PaintControl;
use crate::resolver::{eq, keyword, remap};
use vexel_base::Span;
use vexel_scene::{DecodeSource, ElementId, Paint, PaintKind, PaintStop, Result, Spread};
use vexel_svg_color::parse_color;
use vexel_svg_css::{CssPaint, Property};
use vexel_svg_values::{affine_inverse, parse_length, parse_transform};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub(crate) fn inherited_gradient_color(
    shared: &mut DecodeSource<'_>,
    gradient: ElementId,
    cache: &mut vexel_scene::PaintCache,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<Option<XmlSpan>> {
    let tokens = shared.tokens();
    let mut ancestors = [0usize; 64];
    let mut count = 0;
    for i in 0..=gradient.0 {
        cache::token_visit(cache, control, offset)?;
        let Some(token) = tokens.get(i) else {
            return Err(invalid(offset));
        };
        if !matches!(
            token.kind,
            TokenKind::StartElement | TokenKind::EmptyElement
        ) {
            continue;
        }
        while count > 0 && tokens[ancestors[count - 1]].depth >= token.depth {
            count -= 1;
        }
        if count == ancestors.len() {
            return Err(limit(vexel_scene::LimitKind::Depth, offset));
        }
        ancestors[count] = i;
        count += 1;
    }
    let mut parent: Option<CssPaint<'_>> = None;
    for index in &ancestors[..count] {
        let style = shared
            .compute_style(ElementId(*index), parent.as_ref(), control)
            .map_err(|error| remap(error, offset))?;
        parent = Some(style);
    }
    Ok(parent.and_then(|style| style.value(Property::Color).map(|value| value.span)))
}

pub(crate) fn collect_stops(
    shared: &mut DecodeSource<'_>,
    chain: &[ElementId],
    cache: &mut vexel_scene::PaintCache,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<(Vec<PaintStop>, usize)> {
    for gradient in chain {
        let inherited_color = inherited_gradient_color(shared, *gradient, cache, control, offset)?;
        let end = element_end(shared.tokens(), gradient.0, cache, control, offset)?;
        let mut values = [PaintStop {
            offset: 0.0,
            rgba: [0; 4],
        }; MAX_GRADIENT_STOPS];
        let mut count = 0;
        let mut has_children = false;
        for i in gradient.0.saturating_add(1)..end {
            cache::token_visit(cache, control, offset)?;
            let token = &shared.tokens()[i];
            if !matches!(
                token.kind,
                TokenKind::StartElement | TokenKind::EmptyElement
            ) || token.depth != shared.tokens()[gradient.0].depth + 1
            {
                continue;
            }
            has_children = true;
            if token.prefix.length != 0 || !eq(shared.source(), token.local_name, b"stop") {
                return Err(unsupported(offset));
            }
            if count == MAX_GRADIENT_STOPS {
                return Err(limit(vexel_scene::LimitKind::PaintStops, offset));
            }
            let source = shared.source();
            let tokens = shared.tokens();
            let offset_span = attr(source, tokens, i, b"offset");
            let color_span = attr(source, tokens, i, b"stop-color");
            let opacity_span = attr(source, tokens, i, b"stop-opacity");
            if let Some(style) = attr(source, tokens, i, b"style") {
                if !supported_stop_inline_style(source, style) {
                    return Err(unsupported(offset));
                }
            }
            let computed = shared
                .compute_style(ElementId(i), None, control)
                .map_err(|error| remap(error, offset))?;
            for property in Property::ALL {
                if !matches!(
                    property,
                    Property::StopColor | Property::StopOpacity | Property::Color
                ) && computed.value(property).is_some()
                {
                    return Err(unsupported(offset));
                }
            }
            let computed_offset = computed
                .value(Property::StopColor)
                .map(|v| v.span)
                .or(color_span);
            let computed_opacity = computed
                .value(Property::StopOpacity)
                .map(|v| v.span)
                .or(opacity_span);
            let position = offset_span
                .filter(|span| span.length != 0)
                .map_or(Some(0.0), |v| number(shared.source(), v))
                .ok_or_else(|| unsupported(offset))?;
            let alpha = computed_opacity
                .map_or(Some(1.0), |v| number(shared.source(), v))
                .ok_or_else(|| unsupported(offset))?;
            if !(0.0..=1.0).contains(&position)
                || !(0.0..=1.0).contains(&alpha)
                || (count > 0 && position < values[count - 1].offset)
            {
                return Err(unsupported(offset));
            }
            let color = if let Some(span) = computed_offset {
                resolve_stop_color(
                    shared.source(),
                    span,
                    computed
                        .value(Property::Color)
                        .map(|v| v.span)
                        .or(inherited_color),
                )?
            } else {
                [0, 0, 0, 255]
            };
            values[count] = PaintStop {
                offset: position,
                rgba: [
                    color[0],
                    color[1],
                    color[2],
                    (f64::from(color[3]) * alpha).round().clamp(0.0, 255.0) as u8,
                ],
            };
            count += 1;
        }
        if count >= 2 {
            let mut owned = Vec::new();
            owned.try_reserve_exact(count).map_err(|_| oom(offset))?;
            owned.extend_from_slice(&values[..count]);
            return Ok((owned, count));
        }
        if has_children {
            return Err(unsupported(offset));
        }
    }
    Err(unsupported(offset))
}

fn element_end(
    tokens: &[XmlToken],
    start: usize,
    cache: &mut vexel_scene::PaintCache,
    control: &mut PaintControl<'_>,
    offset: usize,
) -> Result<usize> {
    let token = tokens.get(start).ok_or_else(|| invalid(offset))?;
    if token.kind == TokenKind::EmptyElement {
        return Ok(start);
    }
    for i in start.saturating_add(1)..tokens.len() {
        cache::token_visit(cache, control, offset)?;
        if tokens[i].kind == TokenKind::EndElement && tokens[i].depth == token.depth {
            return Ok(i);
        }
    }
    Ok(start)
}

pub(crate) fn read_definition_attributes(
    source: &[u8],
    tokens: &[XmlToken],
    chain: &[ElementId],
    paint: &mut Paint,
    offset: usize,
) -> Result<()> {
    let (mut units, mut spread, mut transform) = (false, false, false);
    for element in chain {
        if !units {
            if let Some(value) = attr(source, tokens, element.0, b"gradientUnits") {
                if eq(source, value, b"userSpaceOnUse") {
                    paint.object_bounding_box = false;
                } else if !eq(source, value, b"objectBoundingBox") {
                    return Err(unsupported(offset));
                }
                units = true;
            }
        }
        if !spread {
            if let Some(value) = attr(source, tokens, element.0, b"spreadMethod") {
                paint.spread = if eq(source, value, b"reflect") {
                    Spread::Reflect
                } else if eq(source, value, b"repeat") {
                    Spread::Repeat
                } else if eq(source, value, b"pad") {
                    Spread::Pad
                } else {
                    return Err(unsupported(offset));
                };
                spread = true;
            }
        }
        if !transform {
            if let Some(value) = attr(source, tokens, element.0, b"gradientTransform") {
                let span = Span::from_offset_length(value.offset, value.length)
                    .ok_or_else(|| invalid(offset))?;
                let (matrix, _) =
                    parse_transform(source, span).map_err(|error| unsupported(error.offset))?;
                paint.gradient_inverse =
                    affine_inverse(matrix).ok_or_else(|| unsupported(offset))?;
                transform = true;
            }
        }
    }
    Ok(())
}

pub(crate) fn read_coordinates(
    source: &[u8],
    tokens: &[XmlToken],
    chain: &[ElementId],
    viewport: [f64; 2],
    paint: &mut Paint,
    offset: usize,
) -> Result<()> {
    let linear = paint.kind == PaintKind::Linear;
    let names: [&[u8]; 5] = if linear {
        [b"x1", b"y1", b"x2", b"y2", b""]
    } else {
        [b"cx", b"cy", b"r", b"fx", b"fy"]
    };
    let diagonal = viewport[0].hypot(viewport[1]) / std::f64::consts::SQRT_2;
    if linear {
        paint.coordinates[2] = if paint.object_bounding_box {
            1.0
        } else {
            viewport[0]
        };
    } else {
        paint.coordinates[0] = if paint.object_bounding_box {
            0.5
        } else {
            viewport[0] * 0.5
        };
        paint.coordinates[1] = if paint.object_bounding_box {
            0.5
        } else {
            viewport[1] * 0.5
        };
        paint.coordinates[2] = if paint.object_bounding_box {
            0.5
        } else {
            diagonal * 0.5
        };
        paint.coordinates[3] = paint.coordinates[0];
        paint.coordinates[4] = paint.coordinates[1];
    }
    let count = if linear { 4 } else { 5 };
    let mut found = [false; 5];
    for coordinate in 0..count {
        for element in chain {
            let Some(value) = attr(source, tokens, element.0, names[coordinate]) else {
                continue;
            };
            found[coordinate] = true;
            let reference = if paint.object_bounding_box {
                1.0
            } else if linear {
                if coordinate % 2 == 0 {
                    viewport[0]
                } else {
                    viewport[1]
                }
            } else if coordinate == 2 {
                diagonal
            } else if coordinate == 0 || coordinate == 3 {
                viewport[0]
            } else {
                viewport[1]
            };
            if paint.object_bounding_box
                && value
                    .slice(source)
                    .is_some_and(|raw| raw.iter().any(|byte| byte.is_ascii_alphabetic()))
            {
                return Err(unsupported(offset));
            }
            let span = Span::from_offset_length(value.offset, value.length)
                .ok_or_else(|| invalid(offset))?;
            paint.coordinates[coordinate] =
                parse_length(source, span, reference).map_err(|_| unsupported(offset))?;
            break;
        }
    }
    if !linear {
        if !found[3] {
            paint.coordinates[3] = paint.coordinates[0];
        }
        if !found[4] {
            paint.coordinates[4] = paint.coordinates[1];
        }
        if paint.coordinates[2] <= 0.0 || !paint.coordinates[2].is_finite() {
            return Err(unsupported(offset));
        }
    }
    Ok(())
}

fn supported_stop_inline_style(source: &[u8], span: XmlSpan) -> bool {
    let Some(bytes) = span.slice(source) else {
        return false;
    };
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n' | b';') {
            i += 1;
        }
        if i == bytes.len() {
            break;
        }
        let start = i;
        while i < bytes.len() && bytes[i] != b':' && bytes[i] != b';' {
            i += 1;
        }
        if i == bytes.len() || bytes[i] != b':' {
            return false;
        }
        let mut end = i;
        while end > start && matches!(bytes[end - 1], b' ' | b'\t') {
            end -= 1;
        }
        if !matches!(
            &bytes[start..end],
            b"stop-color" | b"stop-opacity" | b"color"
        ) {
            return false;
        }
        i += 1;
        while i < bytes.len() && bytes[i] != b';' {
            i += 1;
        }
        if i < bytes.len() {
            i += 1;
        }
    }
    true
}

fn resolve_stop_color(source: &[u8], stop: XmlSpan, current: Option<XmlSpan>) -> Result<[u8; 4]> {
    if !keyword(source, stop, b"currentColor") {
        return parse_css_color(source, stop);
    }
    let Some(color) = current else {
        return Ok([0, 0, 0, 255]);
    };
    if keyword(source, color, b"currentColor") {
        return Ok([0, 0, 0, 255]);
    }
    parse_css_color(source, color)
}

fn parse_css_color(source: &[u8], span: XmlSpan) -> Result<[u8; 4]> {
    let base =
        Span::from_offset_length(span.offset, span.length).ok_or_else(|| invalid(span.offset))?;
    parse_color(source, base).ok_or_else(|| unsupported(span.offset))
}

fn number(source: &[u8], span: XmlSpan) -> Option<f64> {
    let raw = span.slice(source)?;
    if raw.is_empty() || raw.len() >= 96 {
        return None;
    }
    let text = std::str::from_utf8(raw).ok()?;
    let text = text.trim_start_matches(|c| matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{000c}'));
    let (numeric, percent) = text.strip_suffix('%').map_or((text, false), |v| (v, true));
    let value: f64 = numeric.parse().ok()?;
    if !value.is_finite() || value.abs() > 1_000_000.0 {
        return None;
    }
    Some(if percent { value / 100.0 } else { value })
}

fn attr(source: &[u8], tokens: &[XmlToken], element: usize, name: &[u8]) -> Option<XmlSpan> {
    let mut i = element.checked_add(1)?;
    while tokens
        .get(i)
        .is_some_and(|token| token.kind == TokenKind::Attribute)
    {
        let token = &tokens[i];
        if token.prefix.length == 0 && eq(source, token.local_name, name) {
            return Some(token.value);
        }
        i += 1;
    }
    None
}
