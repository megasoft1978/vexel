//! Original affine parser with controlled source reads and frozen operation limits.
use crate::numbers::NumberReader;
use vexel_scene::{Affine, Diagnostic, ErrorKind, Result, WorkControl, WorkKind};
use vexel_xml::{XmlSpan, XmlToken};
pub fn affine_inverse(matrix: Affine) -> Option<Affine> {
    vexel_svg_values::affine_inverse(matrix)
}
fn fail(span: XmlSpan) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Unsupported,
        offset: span.offset,
    }
}
pub fn parse_transform(source: &[u8], span: XmlSpan) -> Result<(Affine, usize)> {
    parse_transform_controlled(source, span, &mut NoStop)
}
pub fn parse_transform_controlled(
    source: &[u8],
    span: XmlSpan,
    control: &mut dyn WorkControl,
) -> Result<(Affine, usize)> {
    let mut r = NumberReader::new(source, span)?;
    let mut result = vexel_geometry::affine_identity();
    let mut operations = 0;
    loop {
        r.skip_space(control)?;
        if r.index == r.end {
            return Ok((result, operations));
        }
        let start = r.index;
        while r.peek().is_some_and(|b| b.is_ascii_alphabetic()) {
            crate::source::charge(control, WorkKind::Transform, 1, r.index)?;
            r.index += 1;
        }
        let name = &source[start..r.index];
        if name.is_empty() || r.peek() != Some(b'(') {
            return Err(fail(span));
        }
        crate::source::charge(control, WorkKind::Transform, 1, r.index)?;
        r.index += 1;
        let count = match name {
            b"matrix" => 6,
            b"translate" | b"scale" => 2,
            b"rotate" => 3,
            b"skewX" | b"skewY" => 1,
            _ => return Err(fail(span)),
        };
        let mut v = [0.0; 6];
        r.skip_space(control)?;
        if count == 2 || count == 3 {
            v[0] = r.number(control).map_err(|e| preserve_stop(e, span))?;
            let before = r.index;
            r.skip_space(control)?;
            if r.peek() != Some(b')') && r.index < r.end {
                r.index = before;
                if !r.separator(control)? {
                    return Err(fail(span));
                }
                v[1] = r.number(control).map_err(|e| preserve_stop(e, span))?;
                if count == 3 {
                    if !r.separator(control)? {
                        return Err(fail(span));
                    }
                    v[2] = r.number(control).map_err(|e| preserve_stop(e, span))?;
                }
            } else {
                v[1] = if name == b"scale" { v[0] } else { 0.0 };
            }
        } else {
            for (i, value) in v.iter_mut().enumerate().take(count) {
                if i != 0 && !r.separator(control)? {
                    return Err(fail(span));
                }
                *value = r.number(control).map_err(|e| preserve_stop(e, span))?;
            }
        }
        r.skip_space(control)?;
        if r.peek() != Some(b')') {
            return Err(fail(span));
        }
        crate::source::charge(control, WorkKind::Transform, 1, r.index)?;
        r.index += 1;
        let mut op = vexel_geometry::affine_identity();
        match name {
            b"matrix" => {
                op = Affine {
                    a: v[0],
                    b: v[1],
                    c: v[2],
                    d: v[3],
                    e: v[4],
                    f: v[5],
                }
            }
            b"translate" => {
                op.e = v[0];
                op.f = v[1]
            }
            b"scale" => {
                op.a = v[0];
                op.d = v[1]
            }
            b"skewX" | b"skewY" => {
                let tangent = (v[0] * 0.017453292519943295).tan();
                if !tangent.is_finite() || tangent.abs() > 1e6 {
                    return Err(fail(span));
                }
                if name == b"skewX" {
                    op.c = tangent
                } else {
                    op.b = tangent
                }
            }
            _ => {
                let angle = v[0] * 0.017453292519943295;
                let cosine = angle.cos();
                let sine = angle.sin();
                op = Affine {
                    a: cosine,
                    b: sine,
                    c: -sine,
                    d: cosine,
                    e: v[1] - cosine * v[1] + sine * v[2],
                    f: v[2] - sine * v[1] - cosine * v[2],
                };
            }
        }
        operations += 1;
        if operations > 64 || !vexel_geometry::affine_valid(op) {
            return Err(fail(span));
        }
        result = vexel_geometry::affine_compose(result, op).ok_or_else(|| fail(span))?;
        let after = r.index;
        r.skip_space(control)?;
        let spaced = after != r.index;
        if r.peek() == Some(b',') {
            crate::source::charge(control, WorkKind::Transform, 1, r.index)?;
            r.index += 1;
            r.skip_space(control)?;
            if r.index == r.end {
                return Err(fail(span));
            }
        } else if r.index < r.end && !spaced {
            return Err(fail(span));
        }
    }
}
fn preserve_stop(e: Diagnostic, span: XmlSpan) -> Diagnostic {
    if matches!(
        e.kind,
        ErrorKind::Canceled | ErrorKind::Limit(_) | ErrorKind::OutOfMemory
    ) {
        e
    } else {
        fail(span)
    }
}
pub fn transform_node(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    parent: Affine,
) -> Result<Affine> {
    let local = match crate::source::attribute(source, tokens, element, b"transform", None) {
        Some(span) => parse_transform(source, span)?.0,
        None => vexel_geometry::affine_identity(),
    };
    vexel_geometry::affine_compose(parent, local).ok_or(Diagnostic {
        kind: ErrorKind::Number,
        offset: tokens.get(element).map_or(0, |t| t.name.offset),
    })
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
