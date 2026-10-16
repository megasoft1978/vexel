#![forbid(unsafe_code)]
//! Source-borrowing planners and geometry helpers for SVG definitions.
mod color;
mod common;
pub use color::parse_color;
mod filter;
mod marker;
mod planner;
pub use marker::{
    MarkerAttributes, MarkerOrient, MarkerPlacement, MarkerPosition, compose_marker_transform,
    direction_angle, element_scale, middle_angle, parse_attributes as parse_marker_attributes,
    path_placements as marker_path_placements, primitive_placements as marker_primitive_placements,
};
pub use planner::DefinitionPlanner;
mod clip;
pub use clip::{
    ClipOverlay, ClipWorkCounts, apply_group, clone_clips, contains, create_rect, local_bounds,
    update_owner, visit_boundary_edges, work_counts, work_units,
};
pub use filter::{FilterPlan, parse_filter};
use vexel_scene::{Diagnostic, ErrorKind, LimitKind, Result};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub(crate) fn invalid(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset,
    }
}
pub(crate) fn unsupported(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Unsupported,
        offset,
    }
}
pub(crate) fn number(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Number,
        offset,
    }
}
pub(crate) fn limit(kind: LimitKind, offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(kind),
        offset,
    }
}
pub(crate) fn oom(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset,
    }
}

/// Checked byte view over an accepted immutable XML token stream.
#[derive(Clone, Copy)]
pub struct DefinitionSource<'a> {
    pub source: &'a [u8],
    pub tokens: &'a [XmlToken],
}
impl<'a> DefinitionSource<'a> {
    pub fn new(source: &'a [u8], tokens: &'a [XmlToken]) -> Result<Self> {
        for t in tokens {
            for s in [t.source, t.name, t.value, t.prefix, t.local_name] {
                if span_bytes(source, s).is_none() {
                    return Err(invalid(s.offset));
                }
            }
        }
        Ok(Self { source, tokens })
    }
    pub fn attribute(&self, element: usize, name: &[u8]) -> Option<XmlSpan> {
        self.tokens
            .get(element.checked_add(1)?..)?
            .iter()
            .take_while(|t| t.kind == TokenKind::Attribute)
            .find(|t| t.prefix.length == 0 && span_bytes(self.source, t.local_name) == Some(name))
            .map(|t| t.value)
    }
    pub fn element_end(&self, element: usize) -> Result<usize> {
        matching_end(self.tokens, element)
    }
    pub fn bytes(&self, span: XmlSpan) -> Option<&'a [u8]> {
        span_bytes(self.source, span)
    }
    pub fn equals(&self, span: XmlSpan, value: &[u8]) -> bool {
        self.bytes(span) == Some(value)
    }
}
pub(crate) fn span_bytes(source: &[u8], span: XmlSpan) -> Option<&[u8]> {
    source.get(span.offset..span.offset.checked_add(span.length)?)
}
pub(crate) fn matching_end(tokens: &[XmlToken], element: usize) -> Result<usize> {
    let Some(t) = tokens.get(element) else {
        return Err(invalid(0));
    };
    if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement) {
        return Err(invalid(t.source.offset));
    }
    if t.kind == TokenKind::EmptyElement {
        return Ok(element);
    }
    tokens
        .iter()
        .enumerate()
        .skip(element + 1)
        .find_map(|(i, x)| (x.kind == TokenKind::EndElement && x.depth == t.depth).then_some(i))
        .ok_or_else(|| Diagnostic {
            kind: ErrorKind::Xml(vexel_xml::XmlError::MalformedSyntax),
            offset: t.name.offset,
        })
}
pub(crate) fn attr(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    name: &[u8],
) -> Option<XmlSpan> {
    tokens
        .get(element.checked_add(1)?..)?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .find(|t| t.prefix.length == 0 && span_bytes(source, t.local_name) == Some(name))
        .map(|t| t.value)
}
pub(crate) fn ascii_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

/// Strict local URL fragment parser. Whitespace is permitted around url content.
pub fn parse_local_url(source: &[u8], value: XmlSpan) -> Result<XmlSpan> {
    let Some(bytes) = span_bytes(source, value) else {
        return Err(invalid(value.offset));
    };
    let mut a = 0;
    let mut b = bytes.len();
    while a < b && ascii_space(bytes[a]) {
        a += 1;
    }
    while b > a && ascii_space(bytes[b - 1]) {
        b -= 1;
    }
    if b - a < 7 || &bytes[a..a + 4] != b"url(" || bytes[b - 1] != b')' {
        return Err(unsupported(value.offset));
    }
    a += 4;
    b -= 1;
    while a < b && ascii_space(bytes[a]) {
        a += 1;
    }
    while b > a && ascii_space(bytes[b - 1]) {
        b -= 1;
    }
    if b - a < 2 || bytes[a] != b'#' {
        return Err(unsupported(value.offset));
    }
    Ok(XmlSpan {
        offset: value.offset + a + 1,
        length: b - a - 1,
    })
}

#[cfg(test)]
mod tests;
