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
    pub family_spans: Vec<Span>,
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
    normalize_text_kind(source, raw, preserve, state, true)
}

pub(crate) fn normalize_text_kind(
    source: &[u8],
    raw: XmlSpan,
    preserve: bool,
    state: &mut TextState,
    decode_entities: bool,
) -> Result<Vec<u8>> {
    normalize_text_kind_controlled(source, raw, preserve, state, decode_entities, &mut NoStop)
}

pub(crate) fn normalize_text_kind_controlled(
    source: &[u8],
    raw: XmlSpan,
    preserve: bool,
    state: &mut TextState,
    decode_entities: bool,
    control: &mut dyn vexel_base::WorkControl,
) -> Result<Vec<u8>> {
    let input = raw
        .slice(source)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, raw.offset))?;
    let mut decoded = Vec::new();
    let mut decoded_offsets = Vec::new();
    decoded
        .try_reserve_exact(input.len())
        .map_err(|_| fail(ErrorKind::OutOfMemory, raw.offset))?;
    decoded_offsets
        .try_reserve_exact(input.len())
        .map_err(|_| fail(ErrorKind::OutOfMemory, raw.offset))?;
    if decode_entities {
        decoded.resize(input.len(), 0);
        let mut input_at = 0usize;
        let mut out_at = 0usize;
        while input_at < input.len() {
            scan_charge(control, raw.offset + input_at)?;
            if input[input_at] == b'&' {
                let end = input[input_at..]
                    .iter()
                    .take(18)
                    .position(|b| *b == b';')
                    .map(|n| input_at + n + 1)
                    .ok_or_else(|| fail(ErrorKind::Unsupported, raw.offset + input_at))?;
                let mut tmp = [0u8; 4];
                let n = vexel_xml::decode_attribute_value(
                    source,
                    XmlSpan {
                        offset: raw.offset + input_at,
                        length: end - input_at,
                    },
                    &mut tmp,
                )
                .map_err(|_| fail(ErrorKind::Unsupported, raw.offset + input_at))?;
                decoded[out_at..out_at + n].copy_from_slice(&tmp[..n]);
                decoded_offsets.resize(out_at + n, raw.offset + input_at);
                out_at += n;
                input_at = end;
            } else {
                let width = scalar_width(input, input_at).ok_or_else(|| {
                    fail(ErrorKind::Unsupported, raw.offset.saturating_add(input_at))
                })?;
                let end = input_at
                    .checked_add(width)
                    .filter(|&end| end <= input.len())
                    .ok_or_else(|| {
                        fail(ErrorKind::Unsupported, raw.offset.saturating_add(input_at))
                    })?;
                if !vexel_xml::utf8_next(input, input_at)
                    .is_some_and(|(cp, _)| vexel_xml::is_xml_char(cp))
                {
                    return Err(fail(
                        ErrorKind::Unsupported,
                        raw.offset.saturating_add(input_at),
                    ));
                }
                decoded[out_at..out_at + width].copy_from_slice(&input[input_at..end]);
                decoded_offsets.resize(out_at + width, raw.offset + input_at);
                out_at += width;
                input_at += width;
            }
        }
        decoded.truncate(out_at);
    } else {
        let mut i = 0;
        while i < input.len() {
            scan_charge(control, raw.offset.saturating_add(i))?;
            let width = scalar_width(input, i)
                .ok_or_else(|| fail(ErrorKind::Unsupported, raw.offset.saturating_add(i)))?;
            let end = i
                .checked_add(width)
                .filter(|&end| end <= input.len())
                .ok_or_else(|| fail(ErrorKind::Unsupported, raw.offset.saturating_add(i)))?;
            if !vexel_xml::utf8_next(input, i).is_some_and(|(cp, _)| vexel_xml::is_xml_char(cp)) {
                return Err(fail(ErrorKind::Unsupported, raw.offset.saturating_add(i)));
            }
            decoded.extend_from_slice(&input[i..end]);
            decoded_offsets.resize(decoded.len(), raw.offset + i);
            i = end;
        }
    }
    if preserve {
        if !decoded.is_empty() {
            state.has_position = true;
        }
        return Ok(decoded);
    }
    let mut result = Vec::new();
    let capacity = decoded
        .len()
        .checked_add(1)
        .ok_or_else(|| fail(ErrorKind::Limit(LimitKind::Arithmetic), raw.offset))?;
    result
        .try_reserve_exact(capacity)
        .map_err(|_| fail(ErrorKind::OutOfMemory, raw.offset))?;
    for (index, byte) in decoded.into_iter().enumerate() {
        scan_charge(control, decoded_offsets[index])?;
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

fn scalar_width(input: &[u8], at: usize) -> Option<usize> {
    let byte = *input.get(at)?;
    Some(match byte {
        0x00..=0x7f => 1,
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => return None,
    })
}

#[cfg(test)]
mod controlled_scan_tests {
    use super::*;
    struct StopAt {
        left: usize,
        stop: vexel_base::Stop,
    }
    impl vexel_base::WorkControl for StopAt {
        fn charge(
            &mut self,
            _: vexel_base::WorkKind,
            _: u64,
        ) -> std::result::Result<(), vexel_base::Stop> {
            if self.left == 0 {
                Err(self.stop)
            } else {
                self.left -= 1;
                Ok(())
            }
        }
        fn checkpoint(&mut self) -> std::result::Result<(), vexel_base::Stop> {
            Ok(())
        }
    }
    fn assert_stop(
        result: Result<impl Sized + std::fmt::Debug>,
        stop: vexel_base::Stop,
        offset: usize,
    ) {
        let error = result.unwrap_err();
        let kind = match stop {
            vexel_base::Stop::Canceled => ErrorKind::Canceled,
            vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        };
        assert_eq!((error.kind, error.offset), (kind, offset));
    }

    #[test]
    fn each_controlled_scan_stops_at_the_exact_scanned_byte() {
        for stop in [vexel_base::Stop::Canceled, vexel_base::Stop::Limit] {
            let mut control = StopAt { left: 2, stop };
            let mut state = TextState::default();
            assert_stop(
                normalize_text_kind_controlled(
                    b"xxxx x &amp; y",
                    XmlSpan {
                        offset: 5,
                        length: 9,
                    },
                    false,
                    &mut state,
                    true,
                    &mut control,
                ),
                stop,
                7,
            );

            let mut control = StopAt { left: 2, stop };
            assert_stop(
                parse_position_list_controlled(
                    b"xxxx1,2,3",
                    XmlSpan {
                        offset: 4,
                        length: 5,
                    },
                    100.,
                    &mut control,
                ),
                stop,
                5,
            );

            let mut control = StopAt { left: 5, stop };
            let mut state = TextState::default();
            assert_stop(
                normalize_text_kind_controlled(
                    b"xxxxA  B",
                    XmlSpan {
                        offset: 4,
                        length: 4,
                    },
                    false,
                    &mut state,
                    false,
                    &mut control,
                ),
                stop,
                5,
            );
        }
    }

    #[test]
    fn family_scan_cancellation_and_limit_report_the_interrupted_byte() {
        let source = std::sync::Arc::new(
            b"<svg><text style=\"font-family:AlphaBeta\">X</text></svg>".to_vec(),
        );
        let mut run = NoStop;
        let lexical =
            vexel_scene::LexicalDocument::parse(source.clone(), 1 << 20, &mut run).unwrap();
        let element = lexical
            .tokens()
            .iter()
            .position(|token| token.local_name.slice(&source) == Some(b"text"))
            .unwrap();
        let mut css = vexel_scene::DecodeSource::new(&lexical, &mut run).unwrap();
        let style = css
            .compute_style(vexel_scene::ElementId(element), None, &mut run)
            .unwrap();
        let expected = style.value(Property::FontFamily).unwrap().span.offset + 1;
        for stop in [vexel_base::Stop::Canceled, vexel_base::Stop::Limit] {
            let mut control = StopAt { left: 2, stop };
            assert_stop(
                parse_families_with_spans_controlled(&source, &style, &mut control),
                stop,
                expected,
            );
        }
    }

    #[test]
    fn malformed_utf8_returns_unsupported_at_the_first_bad_byte_without_mutating_state() {
        for bytes in [&b"A\xf0\x9f"[..], &b"A\xc0\xaf"[..], &b"A\xe2(\xa1"[..]] {
            let mut state = TextState::default();
            let before = (
                state.pending_space,
                state.has_position,
                state.cursor_x,
                state.cursor_y,
                state.text_bytes,
                state.glyph_count,
            );
            let mut source = b"xxxx".to_vec();
            source.extend_from_slice(bytes);
            let error = normalize_text(
                &source,
                XmlSpan {
                    offset: 4,
                    length: bytes.len(),
                },
                false,
                &mut state,
            )
            .unwrap_err();
            assert_eq!((error.kind, error.offset), (ErrorKind::Unsupported, 5));
            assert_eq!(
                (
                    state.pending_space,
                    state.has_position,
                    state.cursor_x,
                    state.cursor_y,
                    state.text_bytes,
                    state.glyph_count
                ),
                before
            );
        }
    }
}

struct NoStop;
impl vexel_base::WorkControl for NoStop {
    fn charge(
        &mut self,
        _: vexel_base::WorkKind,
        _: u64,
    ) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
}

fn scan_charge(control: &mut dyn vexel_base::WorkControl, offset: usize) -> Result<()> {
    control
        .charge(vexel_base::WorkKind::Media, 1)
        .map_err(|stop| Diagnostic {
            kind: match stop {
                vexel_base::Stop::Canceled => ErrorKind::Canceled,
                vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
                vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
            },
            offset,
        })
}

/// Parse the x/y/dx/dy list grammar with percentages against the caller reference.
pub fn parse_position_list(source: &[u8], span: XmlSpan, reference: f64) -> Result<Vec<f64>> {
    parse_position_list_controlled(source, span, reference, &mut NoStop)
}
pub(crate) fn parse_position_list_controlled(
    source: &[u8],
    span: XmlSpan,
    reference: f64,
    control: &mut dyn vexel_base::WorkControl,
) -> Result<Vec<f64>> {
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
        scan_charge(control, span.offset.saturating_add(i))?;
        while i < bytes.len() && whitespace(bytes[i]) {
            scan_charge(control, span.offset.saturating_add(i))?;
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
            scan_charge(control, span.offset.saturating_add(i))?;
            i += 1;
        }
        if values.len() == MAX_POSITION_VALUES {
            return Err(fail(ErrorKind::Number, span.offset + start));
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
    parse_families_with_spans_controlled(source, style, &mut NoStop).map(|(families, _)| families)
}

fn parse_families_with_spans_controlled(
    source: &[u8],
    style: &CssPaint<'_>,
    control: &mut dyn vexel_base::WorkControl,
) -> Result<(Vec<Vec<u8>>, Vec<Span>)> {
    let Some(value) = style.value(Property::FontFamily) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let span = value.span;
    if span.slice(source) != Some(value.bytes) {
        return Err(fail(ErrorKind::Unsupported, span.offset));
    }
    let bytes = value.bytes;
    let mut i = 0;
    let mut families = Vec::new();
    let mut spans = Vec::new();
    families
        .try_reserve_exact(MAX_FAMILIES.min(bytes.len()))
        .map_err(|_| fail(ErrorKind::OutOfMemory, span.offset))?;
    spans
        .try_reserve_exact(MAX_FAMILIES.min(bytes.len()))
        .map_err(|_| fail(ErrorKind::OutOfMemory, span.offset))?;
    while i < bytes.len() {
        scan_charge(control, span.offset.saturating_add(i))?;
        while i < bytes.len() && whitespace(bytes[i]) {
            scan_charge(control, span.offset.saturating_add(i))?;
            i += 1;
        }
        if i == bytes.len() {
            break;
        }
        if families.len() == MAX_FAMILIES {
            return Err(fail(ErrorKind::Unsupported, span.offset + i));
        }
        let start;
        let end;
        if matches!(bytes[i], b'\'' | b'"') {
            let quote = bytes[i];
            i += 1;
            start = i;
            while i < bytes.len() && bytes[i] != quote {
                scan_charge(control, span.offset.saturating_add(i))?;
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
                scan_charge(control, span.offset.saturating_add(i))?;
                i += 1;
            }
            let mut trimmed = i;
            while trimmed > start && whitespace(bytes[trimmed - 1]) {
                scan_charge(control, span.offset.saturating_add(trimmed - 1))?;
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
        let absolute_start = span
            .offset
            .checked_add(start)
            .ok_or_else(|| fail(ErrorKind::Limit(LimitKind::Arithmetic), span.offset))?;
        let absolute_end = span
            .offset
            .checked_add(end)
            .ok_or_else(|| fail(ErrorKind::Limit(LimitKind::Arithmetic), span.offset))?;
        spans.push(Span {
            start: absolute_start,
            end: absolute_end,
        });
        while i < bytes.len() && bytes[i] != b',' {
            scan_charge(control, span.offset.saturating_add(i))?;
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
        Ok((families, spans))
    }
}

/// Apply the frozen CSS-to-CoreText style conversion and SVG defaults.
pub fn parse_text_style(
    source: &[u8],
    style: &CssPaint<'_>,
    viewport: [f64; 2],
) -> Result<TextStyleValues> {
    parse_text_style_controlled(source, style, viewport, &mut NoStop)
}
pub(crate) fn parse_text_style_controlled(
    source: &[u8],
    style: &CssPaint<'_>,
    viewport: [f64; 2],
    control: &mut dyn vexel_base::WorkControl,
) -> Result<TextStyleValues> {
    // Keep the original failure order and Unsupported classification. Position
    // attributes use Number errors; font style values do not.
    charge_style_value(style, Property::FontSize, control)?;
    let size = parse_size(source, style, viewport[1])?;
    charge_style_value(style, Property::FontWeight, control)?;
    let weight = parse_weight(source, style)?;
    let (families, family_spans) = parse_families_with_spans_controlled(source, style, control)?;
    charge_style_value(style, Property::LetterSpacing, control)?;
    let letter_spacing = parse_spacing(source, style, Property::LetterSpacing, viewport[0])?;
    charge_style_value(style, Property::WordSpacing, control)?;
    let word_spacing = parse_spacing(source, style, Property::WordSpacing, viewport[0])?;
    charge_style_value(style, Property::FontStyle, control)?;
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
    charge_style_value(style, Property::TextAnchor, control)?;
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
    Ok(TextStyleValues {
        families,
        family_spans,
        size_points: size * 0.75,
        weight,
        style: text_style,
        anchor,
        letter_spacing_points: letter_spacing * 0.75,
        word_spacing,
    })
}

fn charge_style_value(
    style: &CssPaint<'_>,
    property: Property,
    control: &mut dyn vexel_base::WorkControl,
) -> Result<()> {
    if let Some(value) = style.value(property) {
        for (i, _) in value.bytes.iter().enumerate() {
            scan_charge(control, value.span.offset.saturating_add(i))?;
        }
    }
    Ok(())
}

pub fn parse_xml_space(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    parent_preserve: bool,
) -> Result<bool> {
    parse_xml_space_controlled(source, tokens, element, end, parent_preserve, &mut NoStop)
}

pub(crate) fn parse_xml_space_controlled(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    parent_preserve: bool,
    control: &mut dyn vexel_base::WorkControl,
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
        scan_charge(control, attr.name.offset)?;
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
            .ok_or_else(|| fail(ErrorKind::Unsupported, v.span.offset))?,
        viewport_height,
    )
    .map_err(|_| fail(ErrorKind::Unsupported, v.span.offset))?;
    if value <= 0. {
        return Err(fail(ErrorKind::Unsupported, v.span.offset));
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
            .ok_or_else(|| fail(ErrorKind::Unsupported, v.span.offset))?,
    )
    .map_err(|_| fail(ErrorKind::Unsupported, v.span.offset))?;
    if !(100. ..=900.).contains(&n) || n % 100. != 0. {
        return Err(fail(ErrorKind::Unsupported, v.span.offset));
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
            .ok_or_else(|| fail(ErrorKind::Unsupported, v.span.offset))?,
        reference,
    )
    .map_err(|_| fail(ErrorKind::Unsupported, v.span.offset))
}
