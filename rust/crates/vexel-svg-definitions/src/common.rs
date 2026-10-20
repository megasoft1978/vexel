use vexel_base::Span;
use vexel_scene::{Diagnostic, ErrorKind, LimitKind, Result};
use vexel_xml::XmlSpan;
fn map_value(error: vexel_svg_values::ValueError, offset: usize) -> Diagnostic {
    use vexel_svg_values::ValueError;
    match error {
        ValueError::Number(n) => Diagnostic {
            kind: ErrorKind::Number,
            offset: n.offset,
        },
        ValueError::OutOfMemory => Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset,
        },
        ValueError::SourceLimit => Diagnostic {
            kind: ErrorKind::Limit(LimitKind::SourceBytes),
            offset,
        },
        ValueError::UnsupportedUnit { offset } => Diagnostic {
            kind: ErrorKind::Unsupported,
            offset,
        },
        ValueError::InvalidSpan => Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset,
        },
    }
}
pub(crate) fn parse_number(source: &[u8], span: XmlSpan, fallback: f64) -> Result<f64> {
    if span.length == 0 {
        return Ok(fallback);
    }
    vexel_svg_values::parse_number(
        source,
        Span {
            start: span.offset,
            end: span
                .offset
                .checked_add(span.length)
                .ok_or_else(|| crate::invalid(span.offset))?,
        },
    )
    .map_err(|e| map_value(e, span.offset))
}
pub(crate) fn region_number(source: &[u8], span: XmlSpan, fallback: f64) -> Result<f64> {
    if span.length == 0 {
        return Ok(fallback);
    }
    let b = source
        .get(
            span.offset
                ..span
                    .offset
                    .checked_add(span.length)
                    .ok_or_else(|| crate::invalid(span.offset))?,
        )
        .ok_or_else(|| crate::invalid(span.offset))?;
    if b.last() == Some(&b'%') {
        let n = parse_number(
            source,
            XmlSpan {
                offset: span.offset,
                length: span.length - 1,
            },
            fallback,
        )?;
        Ok(n / 100.)
    } else {
        parse_number(source, span, fallback)
    }
}
pub(crate) fn number_list(source: &[u8], span: XmlSpan, max: usize) -> Result<Vec<f64>> {
    let end = span
        .offset
        .checked_add(span.length)
        .ok_or_else(|| crate::invalid(span.offset))?;
    let mut cursor = vexel_svg_values::NumberCursor::new(
        source,
        Span {
            start: span.offset,
            end,
        },
        // Whole-document policy cap, as the decoder uses; the 16 MiB default
        // rejected any number list located in a larger document.
        vexel_svg_values::ValueLimits {
            source_bytes: vexel_svg_values::MAX_POLICY_SOURCE_BYTES,
            attributes: vexel_svg_values::MAX_POLICY_ATTRIBUTES,
        },
    )
    .map_err(|e| map_value(e, span.offset))?;
    let mut out = Vec::new();
    out.try_reserve_exact(max).map_err(|_| Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: span.offset,
    })?;
    let first_at = cursor.absolute();
    if source
        .get(first_at..end)
        .is_none_or(|b| b.iter().all(|c| matches!(c, b' ' | b'\t' | b'\r' | b'\n')))
    {
        return Ok(out);
    }
    loop {
        if out.len() >= max {
            return Err(Diagnostic {
                kind: ErrorKind::Number,
                offset: cursor.absolute(),
            });
        }
        out.push(cursor.number().map_err(|e| map_value(e, span.offset))?);
        let at = cursor.absolute();
        let rest = source.get(at..end).ok_or_else(|| crate::invalid(at))?;
        if rest
            .iter()
            .all(|c| matches!(c, b' ' | b'\t' | b'\r' | b'\n'))
        {
            break;
        }
        if !cursor.separator() && !matches!(source.get(at), Some(b'+' | b'-')) {
            return Err(Diagnostic {
                kind: ErrorKind::Number,
                offset: at,
            });
        }
    }
    Ok(out)
}
pub(crate) fn parse_length(source: &[u8], span: XmlSpan, reference: f64) -> Result<f64> {
    let end = span
        .offset
        .checked_add(span.length)
        .ok_or_else(|| crate::invalid(span.offset))?;
    vexel_svg_values::parse_length(
        source,
        Span {
            start: span.offset,
            end,
        },
        reference,
    )
    .map_err(|e| map_value(e, span.offset))
}
