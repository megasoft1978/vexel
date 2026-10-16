//! Checked lexical adapters and actual work accounting.
use vexel_scene::*;
use vexel_xml::{TokenKind, XmlSpan, XmlToken};
pub fn stopped(stop: Stop, offset: usize) -> Diagnostic {
    Diagnostic {
        kind: match stop {
            Stop::Canceled => ErrorKind::Canceled,
            Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset,
    }
}
pub fn charge(control: &mut dyn WorkControl, kind: WorkKind, n: u64, offset: usize) -> Result<()> {
    control.charge(kind, n).map_err(|s| stopped(s, offset))
}
pub fn charge_bytes(
    control: &mut dyn WorkControl,
    kind: WorkKind,
    mut n: usize,
    offset: usize,
) -> Result<()> {
    while n != 0 {
        let batch = n.min(256);
        charge(control, kind, batch as u64, offset)?;
        n -= batch;
    }
    Ok(())
}
pub fn attribute(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    name: &[u8],
    prefix: Option<&[u8]>,
) -> Option<XmlSpan> {
    tokens
        .get(element + 1..)?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .find(|t| {
            t.local_name.slice(source) == Some(name)
                && match prefix {
                    Some(p) => t.prefix.slice(source) == Some(p),
                    None => t.prefix.length == 0,
                }
        })
        .map(|t| t.value)
}
pub fn element_name(source: &[u8], t: &XmlToken, name: &[u8]) -> bool {
    t.prefix.length == 0 && t.local_name.slice(source) == Some(name)
}
pub fn basic_shape(source: &[u8], t: &XmlToken) -> bool {
    [
        b"rect".as_slice(),
        b"path",
        b"circle",
        b"ellipse",
        b"line",
        b"polygon",
        b"polyline",
        b"image",
        b"text",
        b"tspan",
    ]
    .iter()
    .any(|n| element_name(source, t, n))
}
pub fn matching_end(
    tokens: &[XmlToken],
    element: usize,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    let t = tokens.get(element).ok_or(Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    })?;
    if t.kind == TokenKind::EmptyElement {
        return Ok(element);
    }
    for (i, x) in tokens.iter().enumerate().skip(element + 1) {
        charge(control, WorkKind::TokenVisit, 1, x.source.offset)?;
        if x.kind == TokenKind::EndElement && x.depth == t.depth {
            return Ok(i);
        }
    }
    Err(Diagnostic {
        kind: ErrorKind::Xml(vexel_xml::XmlError::MismatchedTag),
        offset: t.name.offset,
    })
}
pub fn element_span(
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    source: &[u8],
) -> Result<Span> {
    let first = tokens.get(element).ok_or(Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    })?;
    let last = tokens.get(end).ok_or(Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: first.source.offset,
    })?;
    let span = Span {
        start: first.source.offset,
        end: last.source.end().ok_or(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::Arithmetic),
            offset: last.source.offset,
        })?,
    };
    if span.start > span.end || span.slice(source).is_none() {
        return Err(Diagnostic {
            kind: ErrorKind::Xml(vexel_xml::XmlError::MismatchedTag),
            offset: first.name.offset,
        });
    }
    Ok(span)
}
