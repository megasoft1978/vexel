//! SVG viewport and conditional switch helpers, preserving source offsets.
use vexel_base::Span;
use vexel_geometry::{affine_identity, Affine};
use vexel_scene::{Diagnostic, ErrorKind, Viewport};
use vexel_svg_values::Aspect;
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

fn s(v: XmlSpan) -> Span {
    Span {
        start: v.offset,
        end: v.offset.saturating_add(v.length),
    }
}
fn mapping(e: vexel_svg_values::MappingError) -> Diagnostic {
    use vexel_svg_values::MappingErrorKind as M;
    Diagnostic {
        kind: match e.kind {
            M::InvalidArgument => ErrorKind::InvalidArgument,
            M::Number => ErrorKind::Number,
            M::Viewport => ErrorKind::Viewport,
            M::Unsupported => ErrorKind::Unsupported,
        },
        offset: e.offset,
    }
}
pub fn parse_preserve_aspect(source: &[u8], span: XmlSpan) -> Result<Aspect, Diagnostic> {
    vexel_svg_values::parse_preserve_aspect(source, s(span)).map_err(mapping)
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
) -> Result<Affine, Diagnostic> {
    vexel_svg_values::viewbox_transform(vx, vy, vw, vh, x, y, w, h, a).map_err(mapping)
}
pub fn root_viewport(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
) -> Result<(Viewport, f64, f64), Diagnostic> {
    let t = tokens.get(element).ok_or(Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    })?;
    if let Some(namespace) = attr(source, tokens, element, b"xmlns") {
        if namespace.slice(source) != Some(b"http://www.w3.org/2000/svg") {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: namespace.offset,
            });
        }
    }
    let ws = attr(source, tokens, element, b"width").ok_or(Diagnostic {
        kind: ErrorKind::Viewport,
        offset: t.name.offset,
    })?;
    let hs = attr(source, tokens, element, b"height").ok_or(Diagnostic {
        kind: ErrorKind::Viewport,
        offset: t.name.offset,
    })?;
    let w = crate::read_dimension(source, ws)?;
    let h = crate::read_dimension(source, hs)?;
    if !(1.0..=4096.0).contains(&w) || !(1.0..=4096.0).contains(&h) {
        return Err(Diagnostic {
            kind: ErrorKind::Viewport,
            offset: if !(1.0..=4096.0).contains(&w) {
                ws.offset
            } else {
                hs.offset
            },
        });
    }
    let mut x = 0.0;
    let mut y = 0.0;
    let mut vw = w;
    let mut vh = h;
    if let Some(v) = attr(source, tokens, element, b"viewBox") {
        let a = crate::read_number_list(source, v, 4).map_err(|e| Diagnostic {
            kind: ErrorKind::Viewport,
            offset: if e.offset == 0 { v.offset } else { e.offset },
        })?;
        if a[2] <= 0.0 || a[3] <= 0.0 {
            return Err(Diagnostic {
                kind: ErrorKind::Viewport,
                offset: v.offset,
            });
        }
        x = a[0];
        y = a[1];
        vw = a[2];
        vh = a[3];
    }
    let aspect = match attr(source, tokens, element, b"preserveAspectRatio") {
        Some(v) => parse_preserve_aspect(source, v)?,
        None => Aspect {
            set: true,
            ..Aspect::default()
        },
    };
    let viewport = Viewport {
        width: w.ceil() as u32,
        height: h.ceil() as u32,
        view_x: x,
        view_y: y,
        view_width: vw,
        view_height: vh,
        aspect,
    };
    Ok((viewport, vw, vh))
}
pub fn nested_viewport(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    parent_width: f64,
    parent_height: f64,
    transform: Affine,
) -> Result<vexel_svg_values::NestedViewport, Diagnostic> {
    use vexel_geometry::{affine_compose, affine_map_point, Point};
    let t = tokens.get(element).ok_or(Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    })?;
    let at = t.name.offset;
    let x = attr(source, tokens, element, b"x")
        .map(|v| crate::read_length(source, v, parent_width))
        .transpose()?
        .unwrap_or(0.0);
    let y = attr(source, tokens, element, b"y")
        .map(|v| crate::read_length(source, v, parent_height))
        .transpose()?
        .unwrap_or(0.0);
    let w = attr(source, tokens, element, b"width")
        .map(|v| crate::read_length(source, v, parent_width))
        .transpose()?
        .unwrap_or(parent_width);
    let h = attr(source, tokens, element, b"height")
        .map(|v| crate::read_length(source, v, parent_height))
        .transpose()?
        .unwrap_or(parent_height);
    if w < 0.0 || h < 0.0 || [x, y, w, h].iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
        return Err(Diagnostic {
            kind: ErrorKind::Number,
            offset: at,
        });
    }
    let mut view_transform = Affine {
        e: x,
        f: y,
        ..vexel_geometry::affine_identity()
    };
    let mut cw = w;
    let mut ch = h;
    if let Some(v) = attr(source, tokens, element, b"viewBox") {
        let a = crate::read_number_list(source, v, 4).map_err(|_| Diagnostic {
            kind: ErrorKind::Viewport,
            offset: v.offset,
        })?;
        let aspect = match attr(source, tokens, element, b"preserveAspectRatio") {
            Some(a) => parse_preserve_aspect(source, a)?,
            None => Aspect::default(),
        };
        view_transform = viewbox_transform(a[0], a[1], a[2], a[3], x, y, w, h, aspect)?;
        cw = a[2];
        ch = a[3];
    }
    let child = affine_compose(transform, view_transform).ok_or(Diagnostic {
        kind: ErrorKind::Number,
        offset: at,
    })?;
    let local = [
        Point { x, y },
        Point { x: x + w, y },
        Point { x: x + w, y: y + h },
        Point { x, y: y + h },
    ];
    let mut clip = [Point { x: 0.0, y: 0.0 }; 4];
    for (i, p) in local.into_iter().enumerate() {
        clip[i] = affine_map_point(transform, p).ok_or(Diagnostic {
            kind: ErrorKind::Number,
            offset: at,
        })?
    }
    Ok(vexel_svg_values::NestedViewport {
        child_transform: child,
        viewport_transform: view_transform,
        clip_points: clip,
        child_width: cw,
        child_height: ch,
        empty: w == 0.0 || h == 0.0,
    })
}
pub fn symbol_mapping(
    source: &[u8],
    tokens: &[XmlToken],
    target: usize,
    use_element: usize,
    w: f64,
    h: f64,
) -> Result<Affine, Diagnostic> {
    let tv = attr(source, tokens, target, b"viewBox").ok_or(Diagnostic {
        kind: ErrorKind::Viewport,
        offset: tokens.get(target).map_or(0, |t| t.name.offset),
    })?;
    let v = crate::read_number_list(source, tv, 4).map_err(|_| Diagnostic {
        kind: ErrorKind::Viewport,
        offset: tv.offset,
    })?;
    let ue = tokens.get(use_element).map_or(0, |t| t.name.offset);
    let width = attr(source, tokens, use_element, b"width").ok_or(Diagnostic {
        kind: ErrorKind::Viewport,
        offset: ue,
    })?;
    let height = attr(source, tokens, use_element, b"height").ok_or(Diagnostic {
        kind: ErrorKind::Viewport,
        offset: ue,
    })?;
    let width = crate::read_length(source, width, w)?;
    let height = crate::read_length(source, height, h)?;
    let x = attr(source, tokens, use_element, b"x")
        .map(|s| crate::read_length(source, s, w))
        .transpose()?
        .unwrap_or(0.0);
    let y = attr(source, tokens, use_element, b"y")
        .map(|s| crate::read_length(source, s, h))
        .transpose()?
        .unwrap_or(0.0);
    let aspect = match attr(source, tokens, use_element, b"preserveAspectRatio")
        .or_else(|| attr(source, tokens, target, b"preserveAspectRatio"))
    {
        Some(a) => parse_preserve_aspect(source, a)?,
        None => Aspect::default(),
    };
    viewbox_transform(v[0], v[1], v[2], v[3], x, y, width, height, aspect)
}

fn lower(b: u8) -> u8 {
    b.to_ascii_lowercase()
}
fn language_matches(range: &[u8], language: &[u8]) -> bool {
    range == b"*"
        || (!range.is_empty()
            && range.len() <= language.len()
            && range
                .iter()
                .zip(language)
                .all(|(a, b)| lower(*a) == lower(*b))
            && (range.len() == language.len() || language.get(range.len()) == Some(&b'-')))
}
fn attr(source: &[u8], tokens: &[XmlToken], element: usize, name: &[u8]) -> Option<XmlSpan> {
    tokens
        .get(element + 1..)?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .find(|t| t.prefix.length == 0 && t.local_name.slice(source) == Some(name))
        .map(|t| t.value)
}
fn attr_bytes<'a>(
    source: &'a [u8],
    tokens: &[XmlToken],
    element: usize,
    name: &[u8],
) -> Option<&'a [u8]> {
    attr(source, tokens, element, name)?.slice(source)
}
/// Return first matching direct child of `<switch>`, honoring ordered alternatives.
pub fn select_switch_child(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    end: usize,
    language: &[u8],
) -> Result<Option<(usize, usize)>, Diagnostic> {
    let depth = tokens
        .get(element)
        .ok_or(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: 0,
        })?
        .depth;
    let mut i = element + 1;
    while i < end {
        let t = &tokens[i];
        if !matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
            || t.depth != depth + 1
        {
            i += 1;
            continue;
        }
        if attr(source, tokens, i, b"requiredFeatures").is_some()
            || attr(source, tokens, i, b"requiredExtensions").is_some()
        {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset: t.name.offset,
            });
        }
        let child_end = if t.kind == TokenKind::EmptyElement {
            i
        } else {
            find_end(tokens, i, end).ok_or(Diagnostic {
                kind: ErrorKind::Xml(vexel_xml::XmlError::MismatchedTag),
                offset: t.name.offset,
            })?
        };
        let mut matched = true;
        if let Some(raw) = attr_bytes(source, tokens, i, b"systemLanguage") {
            if raw.is_empty() {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: t.name.offset,
                });
            }
            matched = false;
            let mut start = 0;
            for (p, b) in raw
                .iter()
                .copied()
                .enumerate()
                .chain(std::iter::once((raw.len(), b',')))
            {
                if b != b',' {
                    continue;
                }
                let mut a = start;
                let mut z = p;
                while raw.get(a).is_some_and(|v| v.is_ascii_whitespace()) {
                    a += 1
                }
                while z > a && raw[z - 1].is_ascii_whitespace() {
                    z -= 1
                }
                let range = &raw[a..z];
                if range.is_empty()
                    || range
                        .iter()
                        .any(|x| !(x.is_ascii_alphanumeric() || matches!(*x, b'-' | b'*')))
                {
                    return Err(Diagnostic {
                        kind: ErrorKind::Unsupported,
                        offset: t.name.offset + a,
                    });
                }
                matched |= language_matches(range, language);
                start = p + 1;
            }
        }
        if matched {
            return Ok(Some((i, child_end + 1)));
        }
        i = child_end + 1;
    }
    Ok(None)
}
fn find_end(tokens: &[XmlToken], start: usize, end: usize) -> Option<usize> {
    let t = tokens.get(start)?;
    if t.kind == TokenKind::EmptyElement {
        return Some(start);
    }
    tokens
        .get(start + 1..end)?
        .iter()
        .position(|x| x.kind == TokenKind::EndElement && x.depth == t.depth)
        .map(|n| start + 1 + n)
}
#[allow(dead_code)]
fn identity() -> Affine {
    affine_identity()
}
