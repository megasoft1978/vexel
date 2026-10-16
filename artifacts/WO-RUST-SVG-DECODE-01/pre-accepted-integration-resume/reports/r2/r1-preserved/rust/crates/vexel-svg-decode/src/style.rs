//! Source-backed color and opacity style value parsing.
use vexel_base::Span;
use vexel_scene::{Diagnostic, ErrorKind, Result};
use vexel_svg_css::{CssPaint, Property};
use vexel_xml::XmlSpan;

use crate::invalid;

pub(crate) fn parse_opacity(
    source: &[u8],
    value: Option<vexel_svg_css::CssValue<'_>>,
) -> Result<f64> {
    let Some(value) = value else { return Ok(1.0) };
    let Some(mut bytes) = value.span.slice(source) else {
        return Err(invalid(value.span.offset));
    };
    let mut start = 0;
    let mut end = bytes.len();
    while start < end && matches!(bytes[start], b' ' | b'\t' | b'\r' | b'\n') {
        start += 1
    }
    while end > start && matches!(bytes[end - 1], b' ' | b'\t' | b'\r' | b'\n') {
        end -= 1
    }
    bytes = &bytes[start..end];
    let percent = bytes.last() == Some(&b'%');
    let number_len = bytes.len() - usize::from(percent);
    let span = XmlSpan {
        offset: value.span.offset + start,
        length: number_len,
    };
    let mut parsed = crate::read_number(source, span)?;
    if percent {
        parsed /= 100.0
    }
    if !(0.0..=1.0).contains(&parsed) {
        return Err(Diagnostic {
            kind: ErrorKind::Number,
            offset: value.span.offset,
        });
    }
    Ok(parsed)
}
pub(crate) fn parse_style_color(
    source: &[u8],
    style: &CssPaint<'_>,
    property: Property,
    span: XmlSpan,
) -> Result<[u8; 4]> {
    let raw = span.slice(source).ok_or_else(|| invalid(span.offset))?;
    let trimmed = raw
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .map(|first| {
            &raw[first
                ..raw
                    .iter()
                    .rposition(|b| !b.is_ascii_whitespace())
                    .unwrap_or(first)
                    + 1]
        })
        .unwrap_or(&[]);
    if trimmed
        .get(..4)
        .is_some_and(|v| v.eq_ignore_ascii_case(b"url("))
    {
        return Err(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: span.offset,
        });
    }
    if trimmed.eq_ignore_ascii_case(b"currentcolor") {
        if property == Property::Color {
            return Ok([0, 0, 0, 255]);
        }
        if let Some(color) = style.value(Property::Color) {
            let c = color
                .span
                .slice(source)
                .ok_or_else(|| invalid(color.span.offset))?;
            if !c.eq_ignore_ascii_case(b"currentcolor") {
                return vexel_svg_color::parse_color(
                    source,
                    Span {
                        start: color.span.offset,
                        end: color.span.offset + color.span.length,
                    },
                )
                .ok_or(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: color.span.offset,
                });
            }
        }
        return Ok([0, 0, 0, 255]);
    }
    vexel_svg_color::parse_color(
        source,
        Span {
            start: span.offset,
            end: span.offset + span.length,
        },
    )
    .ok_or(Diagnostic {
        kind: ErrorKind::Unsupported,
        offset: span.offset,
    })
}
