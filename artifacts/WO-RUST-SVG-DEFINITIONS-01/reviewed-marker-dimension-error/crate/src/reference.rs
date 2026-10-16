//! Strict URL-fragment resolution through the canonical accepted reference index.
use crate::{parse_local_url, span_bytes, unsupported};
use vexel_base::WorkControl;
use vexel_scene::{DecodeSource, DefinitionKind, ElementId, Result};
use vexel_xml::{TokenKind, XmlSpan};

/// Resolves a local `url(#id)` to the expected unprefixed definition element.
/// The reference index owns duplicate-ID and visit-limit policy; this helper
/// never scans or rewrites the source independently.
pub fn resolve_local_definition(
    shared: &mut DecodeSource<'_>,
    value: XmlSpan,
    expected: DefinitionKind,
    control: &mut dyn WorkControl,
) -> Result<ElementId> {
    let source = shared.source();
    let id = if expected == DefinitionKind::Clip {
        parse_clip_reference(source, value)?
    } else {
        parse_local_url(source, value)?
    };
    let Some(element) = shared.find_reference(id, control)? else {
        return Err(unsupported(value.offset));
    };
    let Some(token) = shared.tokens().get(element.0) else {
        return Err(unsupported(value.offset));
    };
    let local = match expected {
        DefinitionKind::Clip => b"clipPath".as_slice(),
        DefinitionKind::Mask => b"mask",
        DefinitionKind::Pattern => b"pattern",
        DefinitionKind::Marker => b"marker",
        DefinitionKind::Filter => b"filter",
    };
    if !matches!(
        token.kind,
        TokenKind::StartElement | TokenKind::EmptyElement
    ) || token.prefix.length != 0
        || span_bytes(source, token.local_name) != Some(local)
    {
        return Err(unsupported(value.offset));
    }
    Ok(element)
}

fn parse_clip_reference(source: &[u8], value: XmlSpan) -> Result<XmlSpan> {
    let Some(end) = value.offset.checked_add(value.length) else {
        return Err(crate::invalid(value.offset));
    };
    let Some(bytes) = source.get(value.offset..end) else {
        return Err(crate::invalid(value.offset));
    };
    if bytes.len() < 7 || !bytes.starts_with(b"url(#") || bytes.last() != Some(&b')') {
        return Err(unsupported(value.offset));
    }
    Ok(XmlSpan {
        offset: value
            .offset
            .checked_add(5)
            .ok_or_else(|| crate::invalid(value.offset))?,
        length: bytes.len() - 6,
    })
}
