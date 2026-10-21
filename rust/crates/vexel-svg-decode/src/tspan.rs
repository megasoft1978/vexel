//! `<tspan>` attributes the text path cannot honour. Each is a typed
//! `Unsupported` whose offset is the attribute name, so the failure names it.
use vexel_scene::{Diagnostic, ErrorKind, Result};
use vexel_xml::{TokenKind, XmlToken};

/// Reject `rotate`, `textLength`, `lengthAdjust` and `baseline-shift` as attributes,
/// and a `baseline-shift` style declaration other than `baseline`.
pub(crate) fn reject_unsupported(source: &[u8], tokens: &[XmlToken], element: usize) -> Result<()> {
    let attributes = tokens
        .iter()
        .skip(element + 1)
        .take_while(|t| t.kind == TokenKind::Attribute)
        .filter(|t| t.prefix.length == 0);
    for a in attributes {
        let name = a.local_name.slice(source).unwrap_or_default();
        let value = a.value.slice(source).unwrap_or_default();
        let bad = match name {
            b"rotate" | b"textLength" | b"lengthAdjust" | b"baseline-shift" => true,
            b"style" => value
                .split(|b| *b == b';')
                .filter_map(|d| d.iter().position(|b| *b == b':').map(|i| d.split_at(i)))
                .any(|(n, v)| {
                    n.trim_ascii().eq_ignore_ascii_case(b"baseline-shift") && !is_baseline(&v[1..])
                }),
            _ => false,
        };
        if bad {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: a.name.offset,
            });
        }
    }
    Ok(())
}

fn is_baseline(value: &[u8]) -> bool {
    value.trim_ascii().eq_ignore_ascii_case(b"baseline")
}
