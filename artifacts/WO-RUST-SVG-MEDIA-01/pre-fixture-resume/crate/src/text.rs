use vexel_base::Span;
use vexel_scene::{Diagnostic, ErrorKind, LimitKind, Result, TextRequest, TextState, TextStyle};
use vexel_svg_css::{CssPaint, Property};
use vexel_xml::{XmlSpan, XmlToken};

pub const MAX_TEXT_BYTES: usize = 64 * 1024;
pub const MAX_POSITION_VALUES: usize = 65_536;
pub const MAX_FAMILIES: usize = 32;
pub(crate) fn fail(kind: ErrorKind, offset: usize) -> Diagnostic {
    Diagnostic { kind, offset }
}
fn whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyleValues {
    pub families: Vec<Vec<u8>>,
    pub size_points: f64,
    pub weight: f64,
    pub style: TextStyle,
    pub anchor: vexel_scene::TextAnchor,
    pub letter_spacing_points: f64,
    pub word_spacing: f64,
}
impl TextStyleValues {
    pub fn request<'a>(&'a self, text: &'a [u8], families: &'a [&'a [u8]]) -> TextRequest<'a> {
        TextRequest {
            text,
            font_families: families,
            size_points: self.size_points,
            weight: self.weight,
            style: self.style,
            letter_spacing_points: self.letter_spacing_points,
        }
    }
}

/// Decode entities then preserve or collapse XML whitespace with tspan state.
pub fn normalize_text(
    source: &[u8],
    raw: XmlSpan,
    preserve: bool,
    state: &mut TextState,
) -> Result<Vec<u8>> {
    let input = raw
        .slice(source)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, raw.offset))?;
    let mut decoded = Vec::new();
    decoded
        .try_reserve_exact(input.len())
        .map_err(|_| fail(ErrorKind::OutOfMemory, raw.offset))?;
    decoded.resize(input.len(), 0);
    let length = vexel_xml::decode_attribute_value(source, raw, &mut decoded)
        .map_err(|offset| fail(ErrorKind::Unsupported, raw.offset.saturating_add(offset)))?;
    decoded.truncate(length);
    if preserve {
        if !decoded.is_empty() {
            state.has_position = true;
        }
        return Ok(decoded);
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(decoded.len())
        .map_err(|_| fail(ErrorKind::OutOfMemory, raw.offset))?;
    for byte in decoded {
        if whitespace(byte) {
            if state.has_position || !result.is_empty() {
                state.pending_space = true;
            }
            continue;
        }
        if state.pending_space && (state.has_position || !result.is_empty()) {
            result.push(b' ');
        }
        state.pending_space = false;
        result.push(byte);
        state.has_position = true;
    }
    Ok(result)
}

/// Parse the x/y/dx/dy list grammar with percentages against the caller reference.
pub fn parse_position_list(source: &[u8], span: XmlSpan, reference: f64) -> Result<Vec<f64>> {
    let bytes = span
        .slice(source)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, span.offset))?;
    if bytes.is_empty() {
        return Err(fail(ErrorKind::Number, span.offset));
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(bytes.len().min(MAX_POSITION_VALUES))
        .map_err(|_| fail(ErrorKind::OutOfMemory, span.offset))?;
    let mut i = 0;
    let mut need_value = true;
    while i < bytes.len() {
        while i < bytes.len() && whitespace(bytes[i]) {
            i += 1;
        }
        if i == bytes.len() {
            break;
        }
        if bytes[i] == b',' {
            if need_value {
                return Err(fail(ErrorKind::Number, span.offset + i));
            }
            need_value = true;
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i] != b',' && !whitespace(bytes[i]) {
            i += 1;
        }
        if values.len() == MAX_POSITION_VALUES {
            return Err(fail(
                ErrorKind::Limit(LimitKind::TextBytes),
                span.offset + start,
            ));
        }
        let item = Span::from_offset_length(span.offset + start, i - start)
            .ok_or_else(|| fail(ErrorKind::Limit(LimitKind::Arithmetic), span.offset + start))?;
        let value = vexel_svg_values::parse_length(source, item, reference)
            .map_err(|_| fail(ErrorKind::Number, item.start))?;
        values.push(value);
        need_value = false;
    }
    if values.is_empty() || need_value {
        return Err(fail(ErrorKind::Number, span.offset + span.length));
    }
    Ok(values)
}

/// Parse a comma-separated CSS family list, removing paired quotes as the C implementation does.
pub fn parse_families(source: &[u8], style: &CssPaint<'_>) -> Result<Vec<Vec<u8>>> {
    let Some(value) = style.value(Property::FontFamily) else {
        return Ok(Vec::new());
    };
    let span = value.span;
    if span.slice(source) != Some(value.bytes) {
        return Err(fail(ErrorKind::InvalidArgument, span.offset));
    }
    let bytes = value.bytes;
    let mut i = 0;
    let mut families = Vec::new();
    families
        .try_reserve_exact(MAX_FAMILIES.min(bytes.len()))
        .map_err(|_| fail(ErrorKind::OutOfMemory, span.offset))?;
    while i < bytes.len() {
        while i < bytes.len() && whitespace(bytes[i]) {
            i += 1;
        }
        if i == bytes.len() {
            break;
        }
        if families.len() == MAX_FAMILIES {
            return Err(fail(
                ErrorKind::Limit(LimitKind::TextBytes),
                span.offset + i,
            ));
        }
        let start;
        let end;
        if matches!(bytes[i], b'\'' | b'"') {
            let quote = bytes[i];
            i += 1;
            start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            if i == bytes.len() {
                return Err(fail(ErrorKind::Unsupported, span.offset + start - 1));
            }
            end = i;
            i += 1;
        } else {
            start = i;
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            let mut trimmed = i;
            while trimmed > start && whitespace(bytes[trimmed - 1]) {
                trimmed -= 1;
            }
            end = trimmed;
        }
        if end == start {
            return Err(fail(ErrorKind::Unsupported, span.offset + start));
        }
        let mut family = Vec::new();
        family
            .try_reserve_exact(end - start)
            .map_err(|_| fail(ErrorKind::OutOfMemory, span.offset + start))?;
        family.extend_from_slice(&bytes[start..end]);
        families.push(family);
        while i < bytes.len() && bytes[i] != b',' {
            if !whitespace(bytes[i]) {
                return Err(fail(ErrorKind::Unsupported, span.offset + i));
            }
            i += 1;
        }
        if i < bytes.len() {
            i += 1;
        }
    }
    if families.is_empty() {
        Err(fail(ErrorKind::Unsupported, span.offset))
    } else {
        Ok(families)
    }
}

/// Apply the frozen CSS-to-CoreText style conversion and SVG defaults.
pub fn parse_text_style(
    source: &[u8],
    style: &CssPaint<'_>,
    viewport: [f64; 2],
) -> Result<TextStyleValues> {
    let read_length = |property, reference, default| -> Result<f64> {
        let Some(value) = style.value(property) else {
            return Ok(default);
        };
        vexel_svg_values::parse_length(
            source,
            Span::from_offset_length(value.span.offset, value.span.length)
                .ok_or_else(|| fail(ErrorKind::Number, value.span.offset))?,
            reference,
        )
        .map_err(|_| fail(ErrorKind::Number, value.span.offset))
    };
    let size = read_length(Property::FontSize, viewport[1], 16.)?;
    let size_span = style.value(Property::FontSize).map_or(0, |v| v.span.offset);
    if size <= 0. {
        return Err(fail(ErrorKind::Unsupported, size_span));
    }
    let weight = if let Some(v) = style.value(Property::FontWeight) {
        if v.bytes.eq_ignore_ascii_case(b"normal") {
            400.
        } else if v.bytes.eq_ignore_ascii_case(b"bold") {
            700.
        } else {
            let n = vexel_svg_values::parse_number(
                source,
                Span::from_offset_length(v.span.offset, v.span.length)
                    .ok_or_else(|| fail(ErrorKind::Number, v.span.offset))?,
            )
            .map_err(|_| fail(ErrorKind::Number, v.span.offset))?;
            if !(100. ..=900.).contains(&n) || n % 100. != 0. {
                return Err(fail(ErrorKind::Unsupported, v.span.offset));
            }
            n
        }
    } else {
        400.
    };
    let families = parse_families(source, style)?;
    let spacing = |property| -> Result<f64> {
        let Some(v) = style.value(property) else {
            return Ok(0.);
        };
        if v.bytes.eq_ignore_ascii_case(b"normal") {
            return Ok(0.);
        }
        vexel_svg_values::parse_length(
            source,
            Span::from_offset_length(v.span.offset, v.span.length)
                .ok_or_else(|| fail(ErrorKind::Number, v.span.offset))?,
            viewport[0],
        )
        .map_err(|_| fail(ErrorKind::Number, v.span.offset))
    };
    let text_style = if let Some(v) = style.value(Property::FontStyle) {
        if v.bytes.eq_ignore_ascii_case(b"normal") {
            TextStyle::Normal
        } else if v.bytes.eq_ignore_ascii_case(b"italic") {
            TextStyle::Italic
        } else if v.bytes.eq_ignore_ascii_case(b"oblique") {
            TextStyle::Oblique
        } else {
            return Err(fail(ErrorKind::Unsupported, v.span.offset));
        }
    } else {
        TextStyle::Normal
    };
    let anchor = if let Some(v) = style.value(Property::TextAnchor) {
        if v.bytes.eq_ignore_ascii_case(b"start") {
            vexel_scene::TextAnchor::Start
        } else if v.bytes.eq_ignore_ascii_case(b"middle") {
            vexel_scene::TextAnchor::Middle
        } else if v.bytes.eq_ignore_ascii_case(b"end") {
            vexel_scene::TextAnchor::End
        } else {
            return Err(fail(ErrorKind::Unsupported, v.span.offset));
        }
    } else {
        vexel_scene::TextAnchor::Start
    };
    let letter_spacing = spacing(Property::LetterSpacing)?;
    let word_spacing = spacing(Property::WordSpacing)?;
    Ok(TextStyleValues {
        families,
        size_points: size * 0.75,
        weight,
        style: text_style,
        anchor,
        letter_spacing_points: letter_spacing * 0.75,
        word_spacing,
    })
}

pub fn parse_xml_space(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    parent_preserve: bool,
) -> Result<bool> {
    let token = tokens
        .get(element)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, 0))?;
    for attr in tokens
        .get(element + 1..end.min(tokens.len()))
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, token.name.offset))?
        .iter()
        .take_while(|t| t.kind == vexel_xml::TokenKind::Attribute)
    {
        if attr.local_name.slice(source) != Some(b"space")
            || attr.prefix.slice(source) != Some(b"xml")
        {
            continue;
        }
        return match attr.value.slice(source) {
            Some(v) if v.eq_ignore_ascii_case(b"preserve") => Ok(true),
            Some(v) if v.eq_ignore_ascii_case(b"default") => Ok(false),
            _ => Err(fail(ErrorKind::Unsupported, attr.value.offset)),
        };
    }
    Ok(parent_preserve)
}

/// Legacy helper name retained as a public pure operation for parity.
pub fn text_span_equal(source: &[u8], span: XmlSpan, literal: &[u8]) -> bool {
    span.slice(source) == Some(literal)
}

pub fn text_attribute(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    local: &[u8],
    prefix: Option<&[u8]>,
) -> Option<XmlSpan> {
    super::image::attribute(source, tokens, element, end, local, prefix)
}

pub fn parse_size(source: &[u8], style: &CssPaint<'_>, viewport_height: f64) -> Result<f64> {
    let Some(v) = style.value(Property::FontSize) else {
        return Ok(16.);
    };
    let value = vexel_svg_values::parse_length(
        source,
        Span::from_offset_length(v.span.offset, v.span.length)
            .ok_or_else(|| fail(ErrorKind::Number, v.span.offset))?,
        viewport_height,
    )
    .map_err(|_| fail(ErrorKind::Number, v.span.offset))?;
    if value <= 0. {
        return Err(fail(ErrorKind::Number, v.span.offset));
    }
    Ok(value)
}

pub fn parse_weight(source: &[u8], style: &CssPaint<'_>) -> Result<f64> {
    let Some(v) = style.value(Property::FontWeight) else {
        return Ok(400.);
    };
    if v.bytes.eq_ignore_ascii_case(b"normal") {
        return Ok(400.);
    }
    if v.bytes.eq_ignore_ascii_case(b"bold") {
        return Ok(700.);
    }
    let n = vexel_svg_values::parse_number(
        source,
        Span::from_offset_length(v.span.offset, v.span.length)
            .ok_or_else(|| fail(ErrorKind::Number, v.span.offset))?,
    )
    .map_err(|_| fail(ErrorKind::Number, v.span.offset))?;
    if !(100. ..=900.).contains(&n) || n % 100. != 0. {
        return Err(fail(ErrorKind::Number, v.span.offset));
    }
    Ok(n)
}

pub fn parse_spacing(
    source: &[u8],
    style: &CssPaint<'_>,
    property: Property,
    reference: f64,
) -> Result<f64> {
    let Some(v) = style.value(property) else {
        return Ok(0.);
    };
    if v.bytes.eq_ignore_ascii_case(b"normal") {
        return Ok(0.);
    }
    vexel_svg_values::parse_length(
        source,
        Span::from_offset_length(v.span.offset, v.span.length)
            .ok_or_else(|| fail(ErrorKind::Number, v.span.offset))?,
        reference,
    )
    .map_err(|_| fail(ErrorKind::Number, v.span.offset))
}
