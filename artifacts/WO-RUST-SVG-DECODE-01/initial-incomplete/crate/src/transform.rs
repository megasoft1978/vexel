//! SVG transform parsing and node composition helpers.
use vexel_base::Span;
use vexel_geometry::{affine_compose, Affine};
use vexel_scene::{Diagnostic, ErrorKind};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub fn affine_inverse(value: Affine) -> Option<Affine> {
    vexel_svg_values::affine_inverse(value)
}

pub fn parse_transform(source: &[u8], span: XmlSpan) -> Result<(Affine, usize), Diagnostic> {
    vexel_svg_values::parse_transform(
        source,
        Span {
            start: span.offset,
            end: span.offset.saturating_add(span.length),
        },
    )
    .map_err(|e| Diagnostic {
        kind: ErrorKind::Number,
        offset: e.offset,
    })
}

fn attr(source: &[u8], tokens: &[XmlToken], element: usize, name: &[u8]) -> Option<XmlSpan> {
    let token = tokens.get(element)?;
    if !matches!(
        token.kind,
        TokenKind::StartElement | TokenKind::EmptyElement
    ) {
        return None;
    }
    tokens
        .get(element + 1..)?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .find(|t| t.prefix.length == 0 && t.local_name.slice(source) == Some(name))
        .map(|t| t.value)
}

pub fn transform_node(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    parent: Affine,
) -> Result<Affine, Diagnostic> {
    let Some(value) = attr(source, tokens, element, b"transform") else {
        return Ok(parent);
    };
    let (local, _) = parse_transform(source, value)?;
    affine_compose(parent, local).ok_or(Diagnostic {
        kind: ErrorKind::Number,
        offset: value.offset,
    })
}
