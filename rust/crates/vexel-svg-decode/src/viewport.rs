//! Canonical XML attribute adapters for accepted viewport mathematics.
use crate::source::{attribute, charge, matching_end};
use vexel_base::{Span, WorkControl, WorkKind};
use vexel_geometry::Affine;
use vexel_scene::{Diagnostic, ErrorKind, LimitKind, Result, Viewport};
use vexel_svg_values::{
    Aspect, Attribute, AttributeList, MappingError, MappingErrorKind, NestedViewport,
};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};
fn span(v: XmlSpan) -> Result<Span> {
    Ok(Span {
        start: v.offset,
        end: v.end().ok_or(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: v.offset,
        })?,
    })
}
fn mapping(e: MappingError) -> Diagnostic {
    Diagnostic {
        kind: match e.kind {
            MappingErrorKind::InvalidArgument => ErrorKind::InvalidArgument,
            MappingErrorKind::Number => ErrorKind::Number,
            MappingErrorKind::Viewport => ErrorKind::Viewport,
            MappingErrorKind::Unsupported => ErrorKind::Unsupported,
        },
        offset: e.offset,
    }
}
fn with_attrs<T>(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    control: &mut dyn WorkControl,
    f: impl for<'a> FnOnce(AttributeList<'a>) -> Result<T>,
) -> Result<T> {
    let t = tokens.get(element).ok_or(Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    })?;
    if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement) {
        return Err(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: t.source.offset,
        });
    }
    let mut attrs = [EMPTY_ATTRIBUTE; 4096];
    let mut count = 0;
    for a in tokens
        .iter()
        .skip(element + 1)
        .take_while(|t| t.kind == TokenKind::Attribute)
    {
        charge(control, WorkKind::TokenVisit, 1, a.name.offset)?;
        if count >= attrs.len() {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(LimitKind::Attributes),
                offset: a.name.offset,
            });
        }
        attrs[count] = Attribute {
            name: span(a.name)?,
            value: span(a.value)?,
            prefix: if a.prefix.length == 0 {
                None
            } else {
                Some(span(a.prefix)?)
            },
        };
        count += 1;
    }
    let list = AttributeList::new_with_limits(
        source,
        span(t.name)?,
        &attrs[..count],
        crate::numbers::value_limits(),
    )
    .map_err(|_| Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: t.name.offset,
    })?;
    f(list)
}
pub fn parse_preserve_aspect(source: &[u8], value: XmlSpan) -> Result<Aspect> {
    vexel_svg_values::parse_preserve_aspect_with_limits(
        source,
        span(value)?,
        crate::numbers::value_limits(),
    )
    .map_err(mapping)
}
pub fn viewbox_transform(
    vx: f64,
    vy: f64,
    vw: f64,
    vh: f64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    a: Aspect,
) -> Result<Affine> {
    vexel_svg_values::viewbox_transform(vx, vy, vw, vh, x, y, w, h, a).map_err(mapping)
}
pub fn root_viewport(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
) -> Result<(Viewport, f64, f64)> {
    root_viewport_controlled(source, tokens, element, &mut NoStop)
}
pub fn root_viewport_controlled(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    control: &mut dyn WorkControl,
) -> Result<(Viewport, f64, f64)> {
    let r = with_attrs(source, tokens, element, control, |a| {
        vexel_svg_values::parse_root_viewport(a).map_err(mapping)
    })?;
    Ok((
        Viewport {
            width: r.width,
            height: r.height,
            view_x: r.view_x,
            view_y: r.view_y,
            view_width: r.view_width,
            view_height: r.view_height,
            aspect: r.preserve_aspect,
        },
        r.view_width,
        r.view_height,
    ))
}
pub fn nested_viewport(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    w: f64,
    h: f64,
    t: Affine,
) -> Result<NestedViewport> {
    nested_viewport_controlled(source, tokens, element, w, h, t, &mut NoStop)
}
pub fn nested_viewport_controlled(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    w: f64,
    h: f64,
    t: Affine,
    control: &mut dyn WorkControl,
) -> Result<NestedViewport> {
    with_attrs(source, tokens, element, control, |a| {
        vexel_svg_values::nested_viewport(a, w, h, t).map_err(mapping)
    })
}
pub fn symbol_attributes_supported(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    with_attrs(source, tokens, element, control, |a| {
        vexel_svg_values::symbol_attributes_supported(a).map_err(mapping)
    })
}
pub fn symbol_mapping(
    source: &[u8],
    tokens: &[XmlToken],
    target: usize,
    use_element: usize,
    w: f64,
    h: f64,
) -> Result<Affine> {
    symbol_mapping_controlled(source, tokens, target, use_element, w, h, &mut NoStop)
}
pub fn symbol_mapping_controlled(
    source: &[u8],
    tokens: &[XmlToken],
    target: usize,
    use_element: usize,
    w: f64,
    h: f64,
    control: &mut dyn WorkControl,
) -> Result<Affine> {
    // Each list borrows a separate bounded attribute buffer. Both retain the same source.
    if target >= tokens.len() || use_element >= tokens.len() {
        return Err(crate::invalid(0));
    }
    let mut ta = [EMPTY_ATTRIBUTE; 4096];
    let mut ua = [EMPTY_ATTRIBUTE; 4096];
    let mut tc = 0;
    let mut uc = 0;
    for (e, out, count) in [(target, &mut ta, &mut tc), (use_element, &mut ua, &mut uc)] {
        for a in tokens
            .iter()
            .skip(e + 1)
            .take_while(|t| t.kind == TokenKind::Attribute)
        {
            charge(control, WorkKind::TokenVisit, 1, a.name.offset)?;
            if *count >= out.len() {
                return Err(Diagnostic {
                    kind: ErrorKind::Limit(LimitKind::Attributes),
                    offset: a.name.offset,
                });
            }
            out[*count] = Attribute {
                name: span(a.name)?,
                value: span(a.value)?,
                prefix: if a.prefix.length == 0 {
                    None
                } else {
                    Some(span(a.prefix)?)
                },
            };
            *count += 1;
        }
    }
    let a = AttributeList::new_with_limits(
        source,
        span(
            tokens
                .get(target)
                .ok_or(Diagnostic {
                    kind: ErrorKind::InvalidArgument,
                    offset: 0,
                })?
                .name,
        )?,
        &ta[..tc],
        crate::numbers::value_limits(),
    )
    .map_err(|_| Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    })?;
    let u = AttributeList::new_with_limits(
        source,
        span(
            tokens
                .get(use_element)
                .ok_or(Diagnostic {
                    kind: ErrorKind::InvalidArgument,
                    offset: 0,
                })?
                .name,
        )?,
        &ua[..uc],
        crate::numbers::value_limits(),
    )
    .map_err(|_| Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    })?;
    vexel_svg_values::parse_symbol_mapping(a, u, w, h).map_err(mapping)
}
fn ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}
pub fn language_range_matches(range: &[u8], language: &[u8]) -> bool {
    range == b"*"
        || (!range.is_empty()
            && range.len() <= language.len()
            && range
                .iter()
                .zip(language)
                .all(|(a, b)| a.eq_ignore_ascii_case(b))
            && (range.len() == language.len() || language[range.len()] == b'-'))
}
pub fn switch_child_matches(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    language: &[u8],
    control: &mut dyn WorkControl,
) -> Result<bool> {
    for name in [b"requiredFeatures".as_slice(), b"requiredExtensions"] {
        if let Some(v) = attribute(source, tokens, element, name, None) {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: v.offset,
            });
        }
    }
    let Some(v) = attribute(source, tokens, element, b"systemLanguage", None) else {
        return Ok(true);
    };
    let raw = v
        .slice(source)
        .filter(|v| !v.is_empty())
        .ok_or(Diagnostic {
            kind: ErrorKind::Unsupported,
            offset: v.offset,
        })?;
    let mut cursor = 0;
    let mut any = false;
    while cursor < raw.len() {
        while cursor < raw.len() && ws(raw[cursor]) {
            cursor += 1
        }
        let first = cursor;
        while cursor < raw.len() && raw[cursor] != b',' {
            cursor += 1
        }
        let mut last = cursor;
        while last > first && ws(raw[last - 1]) {
            last -= 1
        }
        if last == first {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: v.offset + first,
            });
        }
        for (i, b) in raw[first..last].iter().enumerate() {
            charge(control, WorkKind::SourceRead, 1, v.offset + first + i)?;
            if !(b.is_ascii_alphanumeric() || matches!(b, b'-' | b'*')) {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: v.offset + first + i,
                });
            }
        }
        any |= language_range_matches(&raw[first..last], language);
        if cursor < raw.len() {
            cursor += 1;
            if cursor == raw.len() {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: v.offset + cursor - 1,
                });
            }
        }
    }
    Ok(any)
}
pub fn select_switch_child(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    language: &[u8],
) -> Result<Option<(usize, usize)>> {
    select_switch_child_controlled(source, tokens, element, end, language, &mut NoStop)
}
pub fn select_switch_child_controlled(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    language: &[u8],
    control: &mut dyn WorkControl,
) -> Result<Option<(usize, usize)>> {
    let depth = tokens
        .get(element)
        .ok_or(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: 0,
        })?
        .depth;
    let mut i = element + 1;
    while i < end {
        let t = tokens.get(i).ok_or(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: 0,
        })?;
        charge(control, WorkKind::TokenVisit, 1, t.source.offset)?;
        if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
            || t.depth != depth + 1
        {
            i += 1;
            continue;
        }
        let close = matching_end(tokens, i, control)?;
        if close >= end {
            return Err(Diagnostic {
                kind: ErrorKind::Xml(vexel_xml::XmlError::MismatchedTag),
                offset: t.name.offset,
            });
        }
        if switch_child_matches(source, tokens, i, language, control)? {
            return Ok(Some((i, close + 1)));
        }
        i = close + 1;
    }
    Ok(None)
}
struct NoStop;
impl WorkControl for NoStop {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
}

const EMPTY_ATTRIBUTE: Attribute = Attribute {
    name: Span { start: 0, end: 0 },
    value: Span { start: 0, end: 0 },
    prefix: None,
};

/// Fixed original viewBox arity; no temporary heap allocation in symbol entry.
pub(crate) fn read_viewbox_controlled(
    source: &[u8],
    span: XmlSpan,
    control: &mut dyn WorkControl,
) -> Result<[f64; 4]> {
    let mut reader = crate::NumberReader::new(source, span)?;
    let mut values = [0.; 4];
    for (i, value) in values.iter_mut().enumerate() {
        if i != 0 && !reader.separator(control)? {
            return Err(Diagnostic {
                kind: ErrorKind::Number,
                offset: reader.position(),
            });
        }
        *value = reader.number(control)?;
    }
    if !reader.finished(control)? {
        return Err(Diagnostic {
            kind: ErrorKind::Number,
            offset: reader.position(),
        });
    }
    Ok(values)
}
