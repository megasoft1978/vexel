//! Source-offset-preserving scalar and path-number adapters.
use vexel_base::Span;
use vexel_scene::{Diagnostic, ErrorKind};
use vexel_xml::XmlSpan;

fn span(value: XmlSpan) -> Span {
    Span {
        start: value.offset,
        end: value.offset.saturating_add(value.length),
    }
}
fn err(error: vexel_svg_values::ValueError) -> Diagnostic {
    use vexel_svg_values::ValueError as V;
    let (kind, offset) = match error {
        V::InvalidSpan => (ErrorKind::InvalidArgument, 0),
        V::Number(e) => (ErrorKind::Number, e.offset),
        V::SourceLimit => (ErrorKind::Limit(vexel_scene::LimitKind::SourceBytes), 0),
        V::OutOfMemory => (ErrorKind::OutOfMemory, 0),
        V::UnsupportedUnit { offset } => (ErrorKind::Unsupported, offset),
    };
    Diagnostic { kind, offset }
}

pub fn skip_number_space(source: &[u8], mut at: usize) -> usize {
    while source
        .get(at)
        .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0c))
    {
        at += 1;
    }
    at
}

pub fn next_number_separator(source: &[u8], at: usize) -> bool {
    source
        .get(at)
        .is_some_and(|b| matches!(b, b',' | b' ' | b'\t' | b'\r' | b'\n' | 0x0c))
}

pub fn read_number(source: &[u8], value: XmlSpan) -> Result<f64, Diagnostic> {
    vexel_svg_values::parse_number(source, span(value)).map_err(err)
}

pub fn read_number_list(
    source: &[u8],
    value: XmlSpan,
    count: usize,
) -> Result<Vec<f64>, Diagnostic> {
    vexel_svg_values::parse_number_list(source, span(value), count).map_err(err)
}

pub fn read_length(source: &[u8], value: XmlSpan, reference: f64) -> Result<f64, Diagnostic> {
    vexel_svg_values::parse_length(source, span(value), reference).map_err(err)
}

pub fn read_dimension(source: &[u8], value: XmlSpan) -> Result<f64, Diagnostic> {
    // Root SVG dimensions use absolute CSS lengths. Percentage dimensions have no root reference.
    read_length(source, value, 0.0)
}

pub fn read_arc_flag(source: &[u8], at: usize) -> Result<(bool, usize), Diagnostic> {
    match source.get(at) {
        Some(b'0') => Ok((false, at + 1)),
        Some(b'1') => Ok((true, at + 1)),
        _ => Err(Diagnostic {
            kind: ErrorKind::Path,
            offset: at,
        }),
    }
}
