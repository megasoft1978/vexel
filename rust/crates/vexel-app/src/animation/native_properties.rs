//! Applies native timeline property tracks to an isolated SVG source snapshot.
//!
//! The snapshot keeps all bytes outside the edited presentation attributes
//! intact. This is the native integer-tick path; SVG SMIL evaluation remains
//! separate.

use std::collections::HashMap;
use std::ops::Range;
use vexel_timeline::{PropertyValue, TimelineView, sample_properties};
use vexel_xml::{TokenKind, XmlSpan, decode_attribute_value, tokenize_large};

const MAX_ID_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePropertyError {
    InvalidTick,
    InvalidSvg,
    MissingTarget,
    DuplicateTarget,
    UnsupportedProperty,
    /// A geometry property (for example `cy`) targets an element type that
    /// has no such attribute (for example a `path`).
    PropertyNotOnElement,
    InvalidValue,
    Allocation,
}

#[derive(Debug, Clone)]
struct Element {
    opening: XmlSpan,
    kind: TokenKind,
    tag: XmlSpan,
    id: Option<Vec<u8>>,
    attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Copy)]
struct Attribute {
    name: XmlSpan,
    value: XmlSpan,
    quote: u8,
}

/// Element table of one SVG source, built once per document revision so a
/// tick resolves each property write by id in O(1) instead of re-tokenizing
/// the source and scanning every element.
#[derive(Debug, Clone)]
pub struct SourceIndex {
    elements: Vec<Element>,
    by_id: HashMap<Vec<u8>, Vec<usize>>,
}

impl SourceIndex {
    pub fn build(source: &[u8]) -> Result<Self, NativePropertyError> {
        let elements = parse_elements(source)?;
        let mut by_id: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();
        for (index, element) in elements.iter().enumerate() {
            if let Some(id) = &element.id {
                by_id.entry(id.clone()).or_default().push(index);
            }
        }
        Ok(Self { elements, by_id })
    }
}

#[derive(Debug, Clone)]
struct Write {
    target: Vec<u8>,
    name: &'static [u8],
    value: Vec<u8>,
}

#[derive(Debug)]
struct Patch {
    range: Range<usize>,
    replacement: Vec<u8>,
}

/// Evaluates visible native property tracks at `tick` and returns an owned SVG
/// snapshot with those properties written in serialized track order.
pub fn apply_native_properties(
    source: &[u8],
    timeline: &TimelineView<'_>,
    tick: u32,
) -> Result<Vec<u8>, NativePropertyError> {
    apply_with_index(source, None, timeline, tick)
}

/// Like [`apply_native_properties`], resolving targets through a prebuilt
/// [`SourceIndex`] of the same `source`. Output is byte-identical.
pub fn apply_native_properties_indexed(
    source: &[u8],
    index: &SourceIndex,
    timeline: &TimelineView<'_>,
    tick: u32,
) -> Result<Vec<u8>, NativePropertyError> {
    apply_with_index(source, Some(index), timeline, tick)
}

fn apply_with_index(
    source: &[u8],
    prebuilt: Option<&SourceIndex>,
    timeline: &TimelineView<'_>,
    tick: u32,
) -> Result<Vec<u8>, NativePropertyError> {
    if tick >= timeline.duration_ticks() {
        return Err(NativePropertyError::InvalidTick);
    }
    let samples =
        sample_properties(timeline, tick).map_err(|_| NativePropertyError::InvalidValue)?;
    let mut writes = Vec::new();
    for sample in samples {
        let target = decode_json_string(sample.target)?;
        let property = decode_json_string(sample.property)?;
        let (value_type, values) = match sample.value {
            PropertyValue::Scalar(value) => (b"scalar".as_slice(), vec![value]),
            PropertyValue::Transform2d(values) => (b"transform2d".as_slice(), values.to_vec()),
            PropertyValue::Trs(trs) => (b"transform2d".as_slice(), trs.to_matrix().to_vec()),
            PropertyValue::Rgba(values) => (b"rgba".as_slice(), values.to_vec()),
            PropertyValue::Boolean(value) => {
                (b"boolean".as_slice(), vec![if value { 1.0 } else { 0.0 }])
            }
        };
        append_writes(&mut writes, target, &property, value_type, &values)?;
    }

    let built;
    let index = match prebuilt {
        Some(index) => index,
        None => {
            built = SourceIndex::build(source)?;
            &built
        }
    };
    let mut patches = Vec::new();
    for write in writes {
        let matches = index
            .by_id
            .get(write.target.as_slice())
            .map_or(&[][..], Vec::as_slice);
        let element = match matches {
            [] => return Err(NativePropertyError::MissingTarget),
            [one] => &index.elements[*one],
            _ => return Err(NativePropertyError::DuplicateTarget),
        };
        let tag = element
            .tag
            .slice(source)
            .ok_or(NativePropertyError::InvalidSvg)?;
        if !property_fits_tag(tag, write.name) {
            return Err(NativePropertyError::PropertyNotOnElement);
        }
        let existing = element
            .attributes
            .iter()
            .find(|attribute| attribute.name.slice(source) == Some(write.name));
        if let Some(attribute) = existing {
            let quote_at = attribute
                .value
                .offset
                .checked_sub(1)
                .ok_or(NativePropertyError::InvalidSvg)?;
            let quote = attribute.quote;
            if source.get(quote_at) != Some(&quote) {
                return Err(NativePropertyError::InvalidSvg);
            }
            let encoded = escape_attribute(&write.value, quote)?;
            patches.push(Patch {
                range: attribute.value.offset..attribute.value.offset + attribute.value.length,
                replacement: encoded,
            });
        } else {
            let opening_end = element
                .opening
                .offset
                .checked_add(element.opening.length)
                .ok_or(NativePropertyError::InvalidSvg)?;
            let insert_at = match element.kind {
                TokenKind::EmptyElement => opening_end.checked_sub(2),
                TokenKind::StartElement => opening_end.checked_sub(1),
                _ => None,
            }
            .ok_or(NativePropertyError::InvalidSvg)?;
            let mut insertion = Vec::new();
            insertion
                .try_reserve(write.name.len() + write.value.len() + 5)
                .map_err(|_| NativePropertyError::Allocation)?;
            insertion.push(b' ');
            insertion.extend_from_slice(write.name);
            insertion.extend_from_slice(b"=\"");
            insertion.extend_from_slice(&escape_attribute(&write.value, b'"')?);
            insertion.push(b'"');
            patches.push(Patch {
                range: insert_at..insert_at,
                replacement: insertion,
            });
        }
    }
    apply_patches(source, patches)
}

fn append_writes(
    writes: &mut Vec<Write>,
    target: Vec<u8>,
    property: &[u8],
    value_type: &[u8],
    values: &[f64],
) -> Result<(), NativePropertyError> {
    let (name, value) = match (property, value_type) {
        (b"visibility", b"boolean") => (
            b"visibility".as_slice(),
            if values.first().copied().unwrap_or(0.0) != 0.0 {
                b"visible".to_vec()
            } else {
                b"hidden".to_vec()
            },
        ),
        (b"fill" | b"stroke", b"rgba") => {
            let color = rgba_color(values)?;
            let opacity = number_text(*values.get(3).ok_or(NativePropertyError::InvalidValue)?);
            push_write(writes, &target, property, color)?;
            let opacity_name = if property == b"fill" {
                b"fill-opacity".as_slice()
            } else {
                b"stroke-opacity".as_slice()
            };
            push_write(writes, &target, opacity_name, opacity)?;
            return Ok(());
        }
        (b"stop-color", b"rgba") => {
            let color = rgba_color(values)?;
            let opacity = number_text(*values.get(3).ok_or(NativePropertyError::InvalidValue)?);
            push_write(writes, &target, b"stop-color", color)?;
            push_write(writes, &target, b"stop-opacity", opacity)?;
            return Ok(());
        }
        (b"transform", b"transform2d") => {
            let mut value = b"matrix(".to_vec();
            for (index, component) in values.iter().enumerate() {
                if index != 0 {
                    value.push(b' ');
                }
                value.extend_from_slice(&number_text(*component));
            }
            value.push(b')');
            (b"transform".as_slice(), value)
        }
        (name, b"scalar") if scalar_property(name) => (name, number_text(values[0])),
        _ => return Err(NativePropertyError::UnsupportedProperty),
    };
    push_write(writes, &target, name, value)
}

fn push_write(
    writes: &mut Vec<Write>,
    target: &[u8],
    name: &[u8],
    value: Vec<u8>,
) -> Result<(), NativePropertyError> {
    let name: &'static [u8] = match name {
        b"x" => b"x",
        b"y" => b"y",
        b"width" => b"width",
        b"height" => b"height",
        b"cx" => b"cx",
        b"cy" => b"cy",
        b"r" => b"r",
        b"rx" => b"rx",
        b"ry" => b"ry",
        b"opacity" => b"opacity",
        b"fill-opacity" => b"fill-opacity",
        b"stroke-opacity" => b"stroke-opacity",
        b"stroke-width" => b"stroke-width",
        b"stroke-dashoffset" => b"stroke-dashoffset",
        b"offset" => b"offset",
        b"stop-color" => b"stop-color",
        b"stop-opacity" => b"stop-opacity",
        b"fill" => b"fill",
        b"stroke" => b"stroke",
        b"visibility" => b"visibility",
        b"transform" => b"transform",
        _ => return Err(NativePropertyError::UnsupportedProperty),
    };
    if let Some(previous) = writes
        .iter_mut()
        .find(|write| write.target == target && write.name == name)
    {
        previous.value = value;
        return Ok(());
    }
    let mut owned_target = Vec::new();
    owned_target
        .try_reserve_exact(target.len())
        .map_err(|_| NativePropertyError::Allocation)?;
    owned_target.extend_from_slice(target);
    writes
        .try_reserve(1)
        .map_err(|_| NativePropertyError::Allocation)?;
    writes.push(Write {
        target: owned_target,
        name,
        value,
    });
    Ok(())
}

/// Whether the geometry property `name` exists on elements named `tag`.
/// Only element types known to lack the attribute are rejected; every other
/// combination is passed through to the renderer.
pub fn property_fits_tag(tag: &[u8], name: &[u8]) -> bool {
    let geometry: &[&[u8]] = match tag {
        b"path" | b"line" | b"polyline" | b"polygon" | b"g" | b"a" | b"switch" => &[],
        b"circle" => &[b"cx", b"cy", b"r"],
        b"ellipse" => &[b"cx", b"cy", b"rx", b"ry"],
        b"rect" => &[b"x", b"y", b"width", b"height", b"rx", b"ry"],
        _ => return true,
    };
    let is_geometry = matches!(
        name,
        b"x" | b"y" | b"width" | b"height" | b"cx" | b"cy" | b"r" | b"rx" | b"ry"
    );
    !is_geometry || geometry.contains(&name)
}

/// Element type of the element with `id` in `source`, `None` when the source
/// cannot be read or has no such element.
pub fn element_tag(source: &[u8], id: &[u8]) -> Option<Vec<u8>> {
    let elements = parse_elements(source).ok()?;
    elements
        .iter()
        .find(|element| element.id.as_deref() == Some(id))
        .and_then(|element| element.tag.slice(source))
        .map(<[u8]>::to_vec)
}

fn scalar_property(name: &[u8]) -> bool {
    matches!(
        name,
        b"x" | b"y"
            | b"width"
            | b"height"
            | b"cx"
            | b"cy"
            | b"r"
            | b"rx"
            | b"ry"
            | b"opacity"
            | b"fill-opacity"
            | b"stroke-opacity"
            | b"stroke-width"
            | b"stroke-dashoffset"
            | b"offset"
            | b"stop-opacity"
    )
}

fn rgba_color(values: &[f64]) -> Result<Vec<u8>, NativePropertyError> {
    if values.len() != 4 {
        return Err(NativePropertyError::InvalidValue);
    }
    let mut color = b"#000000".to_vec();
    for (channel, value) in values[..3].iter().enumerate() {
        if !(0.0..=1.0).contains(value) {
            return Err(NativePropertyError::InvalidValue);
        }
        let byte = (value * 255.0 + 0.5).floor() as u8;
        const HEX: &[u8; 16] = b"0123456789abcdef";
        color[1 + channel * 2] = HEX[usize::from(byte >> 4)];
        color[2 + channel * 2] = HEX[usize::from(byte & 0x0f)];
    }
    if !(0.0..=1.0).contains(&values[3]) {
        return Err(NativePropertyError::InvalidValue);
    }
    Ok(color)
}

fn number_text(value: f64) -> Vec<u8> {
    value.to_string().into_bytes()
}

fn decode_json_string(encoded: &[u8]) -> Result<Vec<u8>, NativePropertyError> {
    let mut decoded = Vec::new();
    decoded
        .try_reserve_exact(MAX_ID_BYTES)
        .map_err(|_| NativePropertyError::Allocation)?;
    decoded.resize(MAX_ID_BYTES, 0);
    let length = vexel_timeline::string_copy(encoded, &mut decoded)
        .ok_or(NativePropertyError::InvalidValue)?;
    decoded.truncate(length);
    Ok(decoded)
}

fn parse_elements(source: &[u8]) -> Result<Vec<Element>, NativePropertyError> {
    let (tokens, result) = tokenize_large(source);
    if result.diagnostic.category != vexel_xml::XmlError::None {
        return Err(NativePropertyError::InvalidSvg);
    }
    let mut elements = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if !matches!(
            token.kind,
            TokenKind::StartElement | TokenKind::EmptyElement
        ) {
            index += 1;
            continue;
        }
        token
            .name
            .slice(source)
            .ok_or(NativePropertyError::InvalidSvg)?;
        let mut element = Element {
            opening: token.source,
            kind: token.kind,
            tag: token.name,
            id: None,
            attributes: Vec::new(),
        };
        index += 1;
        while let Some(attribute) = tokens
            .get(index)
            .filter(|next| next.kind == TokenKind::Attribute)
        {
            let name = attribute
                .name
                .slice(source)
                .ok_or(NativePropertyError::InvalidSvg)?;
            let name_span = attribute.name;
            let quote_at = attribute
                .value
                .offset
                .checked_sub(1)
                .ok_or(NativePropertyError::InvalidSvg)?;
            let quote = *source
                .get(quote_at)
                .ok_or(NativePropertyError::InvalidSvg)?;
            if name == b"id" {
                let raw = attribute
                    .value
                    .slice(source)
                    .ok_or(NativePropertyError::InvalidSvg)?;
                let mut decoded = Vec::new();
                decoded
                    .try_reserve_exact(raw.len().max(1))
                    .map_err(|_| NativePropertyError::Allocation)?;
                decoded.resize(raw.len().max(1), 0);
                let length = decode_attribute_value(source, attribute.value, &mut decoded)
                    .map_err(|_| NativePropertyError::InvalidSvg)?;
                decoded.truncate(length);
                element.id = Some(decoded);
            }
            element.attributes.push(Attribute {
                name: name_span,
                value: attribute.value,
                quote,
            });
            index += 1;
        }
        elements.push(element);
    }
    Ok(elements)
}

fn escape_attribute(value: &[u8], quote: u8) -> Result<Vec<u8>, NativePropertyError> {
    let mut output = Vec::new();
    output
        .try_reserve(value.len())
        .map_err(|_| NativePropertyError::Allocation)?;
    for byte in value {
        match *byte {
            b'&' => output.extend_from_slice(b"&amp;"),
            b'<' => output.extend_from_slice(b"&lt;"),
            b'"' if quote == b'"' => output.extend_from_slice(b"&quot;"),
            b'\'' if quote == b'\'' => output.extend_from_slice(b"&apos;"),
            b'\t' => output.extend_from_slice(b"&#9;"),
            b'\n' => output.extend_from_slice(b"&#10;"),
            b'\r' => output.extend_from_slice(b"&#13;"),
            byte if byte.is_ascii() => output.push(byte),
            _ => return Err(NativePropertyError::InvalidValue),
        }
    }
    Ok(output)
}

fn apply_patches(source: &[u8], mut patches: Vec<Patch>) -> Result<Vec<u8>, NativePropertyError> {
    patches.sort_by_key(|patch| patch.range.start);
    let mut output_len = source.len();
    let mut previous_end = 0;
    for patch in &patches {
        if patch.range.start < previous_end || patch.range.end > source.len() {
            return Err(NativePropertyError::InvalidSvg);
        }
        output_len = output_len
            .checked_sub(patch.range.end - patch.range.start)
            .and_then(|length| length.checked_add(patch.replacement.len()))
            .ok_or(NativePropertyError::Allocation)?;
        previous_end = patch.range.end;
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(output_len)
        .map_err(|_| NativePropertyError::Allocation)?;
    let mut cursor = 0;
    for patch in patches {
        output.extend_from_slice(&source[cursor..patch.range.start]);
        output.extend_from_slice(&patch.replacement);
        cursor = patch.range.end;
    }
    output.extend_from_slice(&source[cursor..]);
    Ok(output)
}
