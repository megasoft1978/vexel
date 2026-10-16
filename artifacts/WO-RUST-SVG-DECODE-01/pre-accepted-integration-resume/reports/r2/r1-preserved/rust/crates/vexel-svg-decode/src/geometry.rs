//! Primitive geometry adapters. Authored numbers retain their original byte spans.
use vexel_base::Span;
use vexel_geometry::{Affine, Point};
use vexel_scene::{Diagnostic, ErrorKind};
use vexel_svg_path::{ParseOptions, PathDocument, PathNumber, PathSubpath};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

#[derive(Debug)]
pub struct Geometry {
    pub path: PathDocument,
    pub filled: bool,
    pub stroke_closed: bool,
}
fn attr(source: &[u8], tokens: &[XmlToken], element: usize, name: &[u8]) -> Option<XmlSpan> {
    tokens
        .get(element + 1..)?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .find(|t| t.prefix.length == 0 && t.local_name.slice(source) == Some(name))
        .map(|t| t.value)
}
fn number(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    name: &[u8],
    reference: f64,
    default: Option<f64>,
) -> Result<(f64, Option<XmlSpan>), Diagnostic> {
    match attr(source, tokens, element, name) {
        Some(s) => Ok((
            vexel_svg_values::parse_length(
                source,
                Span {
                    start: s.offset,
                    end: s.offset + s.length,
                },
                reference,
            )
            .map_err(|_| Diagnostic {
                kind: ErrorKind::Number,
                offset: s.offset,
            })?,
            Some(s),
        )),
        None => default.map(|v| (v, None)).ok_or(Diagnostic {
            kind: ErrorKind::Number,
            offset: tokens.get(element).map_or(0, |t| t.name.offset),
        }),
    }
}
fn number_record(
    out: &mut PathDocument,
    source: &[u8],
    span: Option<XmlSpan>,
    value: f64,
) -> Result<(), Diagnostic> {
    if let Some(s) = span {
        out.numbers.try_reserve(1).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: s.offset,
        })?;
        out.numbers.push(PathNumber {
            source: Span {
                start: s.offset,
                end: s.offset + s.length,
            },
            value,
        });
        let _ = source;
    }
    Ok(())
}
fn edge(
    out: &mut PathDocument,
    a: Point,
    b: Point,
    m: Affine,
    offset: usize,
) -> Result<(), Diagnostic> {
    vexel_svg_path::append_edge(
        a,
        b,
        m,
        ParseOptions::default().max_edges,
        offset,
        &mut out.edges,
    )
    .map_err(crate::map_path_error)
}
fn finish(out: &mut PathDocument, closed: bool, filled: bool) -> Result<Geometry, Diagnostic> {
    out.subpaths.try_reserve(1).map_err(|_| Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: 0,
    })?;
    out.subpaths.push(PathSubpath {
        first_edge: 0,
        edge_count: out.edges.len(),
        closed,
    });
    Ok(Geometry {
        path: std::mem::replace(
            out,
            PathDocument {
                edges: Vec::new(),
                subpaths: Vec::new(),
                segments: Vec::new(),
                numbers: Vec::new(),
            },
        ),
        filled,
        stroke_closed: closed,
    })
}
/// Decode rect, circle, ellipse, line, polyline, or polygon using the frozen coordinate policy.
pub fn element_geometry(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    transform: Affine,
    viewport: [f64; 2],
) -> Result<Geometry, Diagnostic> {
    let name = tokens
        .get(element)
        .and_then(|t| t.local_name.slice(source))
        .ok_or(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: 0,
        })?;
    let offset = tokens[element].name.offset;
    let mut out = PathDocument {
        edges: Vec::new(),
        subpaths: Vec::new(),
        segments: Vec::new(),
        numbers: Vec::new(),
    };
    if name == b"line" {
        let mut p = [0.0; 4];
        for (i, n) in [b"x1".as_slice(), b"y1", b"x2", b"y2"].iter().enumerate() {
            let refv = if i % 2 == 0 { viewport[0] } else { viewport[1] };
            let (v, s) = number(source, tokens, element, n, refv, Some(0.0))?;
            p[i] = v;
            number_record(&mut out, source, s, v)?
        }
        if p.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
            return Err(Diagnostic {
                kind: ErrorKind::Number,
                offset,
            });
        }
        edge(
            &mut out,
            Point { x: p[0], y: p[1] },
            Point { x: p[2], y: p[3] },
            transform,
            offset,
        )?;
        return finish(&mut out, false, false);
    }
    if name == b"rect" {
        let (x, xs) = number(source, tokens, element, b"x", viewport[0], Some(0.0))?;
        let (y, ys) = number(source, tokens, element, b"y", viewport[1], Some(0.0))?;
        let (w, ws) = number(source, tokens, element, b"width", viewport[0], None)?;
        let (h, hs) = number(source, tokens, element, b"height", viewport[1], None)?;
        for (s, v) in [(xs, x), (ys, y), (ws, w), (hs, h)] {
            number_record(&mut out, source, s, v)?
        }
        let (rx, _) = number(source, tokens, element, b"rx", viewport[0], Some(0.0))?;
        let (ry, _) = number(source, tokens, element, b"ry", viewport[1], Some(0.0))?;
        if rx != 0.0 || ry != 0.0 {
            return Err(Diagnostic {
                kind: ErrorKind::Unsupported,
                offset,
            });
        }
        if w < 0.0 || h < 0.0 || [x, y, w, h].iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
            return Err(Diagnostic {
                kind: ErrorKind::Number,
                offset,
            });
        }
        let pts = [
            Point { x, y },
            Point { x: x + w, y },
            Point { x: x + w, y: y + h },
            Point { x, y: y + h },
            Point { x, y },
        ];
        for p in pts.windows(2) {
            edge(&mut out, p[0], p[1], transform, offset)?
        }
        return finish(&mut out, true, true);
    }
    if name == b"circle" || name == b"ellipse" {
        let (cx, cxs) = number(source, tokens, element, b"cx", viewport[0], Some(0.0))?;
        let (cy, cys) = number(source, tokens, element, b"cy", viewport[1], Some(0.0))?;
        let (rx, rxs) = if name == b"circle" {
            number(
                source,
                tokens,
                element,
                b"r",
                viewport[0].hypot(viewport[1]) / std::f64::consts::SQRT_2,
                None,
            )?
        } else {
            number(source, tokens, element, b"rx", viewport[0], None)?
        };
        let (ry, rys) = if name == b"circle" {
            (rx, rxs)
        } else {
            number(source, tokens, element, b"ry", viewport[1], None)?
        };
        for (s, v) in [(cxs, cx), (cys, cy), (rxs, rx), (rys, ry)] {
            number_record(&mut out, source, s, v)?
        }
        if rx < 0.0
            || ry < 0.0
            || [cx, cy, rx, ry]
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 1e6)
        {
            return Err(Diagnostic {
                kind: ErrorKind::Number,
                offset,
            });
        }
        if rx == 0.0 || ry == 0.0 {
            return finish(&mut out, true, true);
        }
        let tolerance = vexel_svg_path::transformed_tolerance(transform);
        let radius = rx.max(ry);
        let angle = if radius <= tolerance {
            2.0 * std::f64::consts::PI
        } else {
            2.0 * (1.0 - tolerance / radius).clamp(-1.0, 1.0).acos()
        };
        let raw = (2.0 * std::f64::consts::PI / angle).ceil();
        if !raw.is_finite() || raw > 4096.0 {
            return Err(Diagnostic {
                kind: ErrorKind::Limit(vexel_scene::LimitKind::Edges),
                offset,
            });
        }
        let steps = (raw as usize).max(12);
        let mut previous = Point { x: cx + rx, y: cy };
        for i in 1..=steps {
            let theta = 2.0 * std::f64::consts::PI * i as f64 / steps as f64;
            let next = if i == steps {
                Point { x: cx + rx, y: cy }
            } else {
                Point {
                    x: cx + rx * theta.cos(),
                    y: cy + ry * theta.sin(),
                }
            };
            if next.x.abs() > 1e6 || next.y.abs() > 1e6 {
                return Err(Diagnostic {
                    kind: ErrorKind::Number,
                    offset,
                });
            }
            edge(&mut out, previous, next, transform, offset)?;
            previous = next
        }
        return finish(&mut out, true, true);
    }
    if name == b"polyline" || name == b"polygon" {
        let data = attr(source, tokens, element, b"points").ok_or(Diagnostic {
            kind: ErrorKind::Path,
            offset,
        })?;
        let span = Span {
            start: data.offset,
            end: data.offset + data.length,
        };
        let mut cursor = vexel_svg_values::NumberCursor::new(
            source,
            span,
            vexel_svg_values::ValueLimits::default(),
        )
        .map_err(|_| Diagnostic {
            kind: ErrorKind::Number,
            offset: data.offset,
        })?;
        let mut points: Vec<(Point, usize, usize, usize)> = Vec::new();
        loop {
            let at = cursor.absolute();
            let x = cursor.number().map_err(|_| Diagnostic {
                kind: ErrorKind::Number,
                offset: at,
            })?;
            let after_x = cursor.absolute();
            let y = cursor.number().map_err(|_| Diagnostic {
                kind: ErrorKind::Number,
                offset: after_x,
            })?;
            let end = cursor.absolute();
            if x.abs() > 1e6 || y.abs() > 1e6 {
                return Err(Diagnostic {
                    kind: ErrorKind::Number,
                    offset: at,
                });
            }
            points.try_reserve(1).map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset: at,
            })?;
            points.push((Point { x, y }, at, after_x, end));
            if cursor.finished() {
                break;
            }
            if !cursor.separator() {
                return Err(Diagnostic {
                    kind: ErrorKind::Number,
                    offset: cursor.absolute(),
                });
            }
        }
        if points.len() < 2 {
            return Err(Diagnostic {
                kind: ErrorKind::Path,
                offset: data.offset,
            });
        }
        for (i, (p, a, x_end, b)) in points.iter().copied().enumerate() {
            out.numbers.try_reserve(2).map_err(|_| Diagnostic {
                kind: ErrorKind::OutOfMemory,
                offset: a,
            })?;
            out.numbers.push(PathNumber {
                source: Span {
                    start: a,
                    end: x_end,
                },
                value: p.x,
            });
            out.numbers.push(PathNumber {
                source: Span {
                    start: x_end,
                    end: b,
                },
                value: p.y,
            });
            if i > 0 {
                edge(&mut out, points[i - 1].0, p, transform, data.offset)?
            }
        }
        let closed = name == b"polygon";
        if closed {
            edge(
                &mut out,
                points.last().unwrap().0,
                points[0].0,
                transform,
                data.offset,
            )?
        }
        return finish(&mut out, closed, closed);
    }
    Err(Diagnostic {
        kind: ErrorKind::Unsupported,
        offset,
    })
}
