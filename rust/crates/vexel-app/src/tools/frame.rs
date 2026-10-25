//! Selection frame of a shape that may carry a matrix transform, and the
//! matrix helpers the rotate gesture composes with.
//!
//! `V1Document::geometry_bounds` only understands `translate(..)`. A shape that
//! was rotated or scaled has a `matrix(..)`, so its frame is rebuilt here from
//! the untransformed geometry and the matrix.

use vexel_v1_document::{ElementKind, TransformSpec, V1Document, V1DocumentError};

pub type Bounds = (f64, f64, f64, f64);
pub type Matrix = [f64; 6];

/// Where a selected shape is on the artboard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Axis-aligned bounds in document space.
    pub bounds: Bounds,
    /// Centre of the untransformed box, in document space (the rotation pivot).
    pub centre: (f64, f64),
    /// True when the shape has a non-translation matrix (resize is unavailable).
    pub transformed: bool,
}

pub fn shape_frame(doc: &V1Document, id: &str) -> Option<Frame> {
    if let Ok(bounds) = doc.geometry_bounds(id) {
        let centre = ((bounds.0 + bounds.2) / 2.0, (bounds.1 + bounds.3) / 2.0);
        return Some(Frame {
            bounds,
            centre,
            transformed: false,
        });
    }
    let matrix = doc.get_transform_matrix(id).ok()?;
    let local = local_bounds(doc, id)?;
    let corners = [
        (local.0, local.1),
        (local.2, local.1),
        (local.2, local.3),
        (local.0, local.3),
    ]
    .map(|(x, y)| apply(&matrix, (x, y)));
    let xs = corners.map(|c| c.0);
    let ys = corners.map(|c| c.1);
    let min = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
    let max = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
    let bounds = (min(xs), min(ys), max(xs), max(ys));
    let centre = apply(
        &matrix,
        ((local.0 + local.2) / 2.0, (local.1 + local.3) / 2.0),
    );
    bounds.0.is_finite().then_some(Frame {
        bounds,
        centre,
        transformed: true,
    })
}

pub fn apply(m: &Matrix, p: (f64, f64)) -> (f64, f64) {
    (
        m[0] * p.0 + m[2] * p.1 + m[4],
        m[1] * p.0 + m[3] * p.1 + m[5],
    )
}

/// `outer * inner`: `inner` is applied first.
pub fn compose(outer: &Matrix, inner: &Matrix) -> Matrix {
    [
        outer[0] * inner[0] + outer[2] * inner[1],
        outer[1] * inner[0] + outer[3] * inner[1],
        outer[0] * inner[2] + outer[2] * inner[3],
        outer[1] * inner[2] + outer[3] * inner[3],
        outer[0] * inner[4] + outer[2] * inner[5] + outer[4],
        outer[1] * inner[4] + outer[3] * inner[5] + outer[5],
    ]
}

/// Moves a shape by a document delta. Plain shapes use `V1Document::translate`;
/// a shape that already carries a non-translation transform (after rotate or
/// scale) gets the translation pre-multiplied onto its matrix instead.
pub fn translate_shape(
    doc: &mut V1Document,
    id: &str,
    dx: f64,
    dy: f64,
) -> Result<(), V1DocumentError> {
    match doc.translate(id, dx, dy) {
        Err(V1DocumentError::UnsupportedTransform) => {
            let old = doc.get_transform_matrix(id)?;
            let moved = compose(&[1.0, 0.0, 0.0, 1.0, dx, dy], &old);
            doc.set_transform(id, &decompose(&moved)?)
        }
        other => other,
    }
}

/// Rotation by `degrees` about the document point `pivot`.
pub fn rotation_about(degrees: f64, pivot: (f64, f64)) -> Matrix {
    TransformSpec {
        rotate_deg: degrees,
        pivot,
        ..TransformSpec::default()
    }
    .to_matrix()
}

/// Splits a matrix into the translate/rotate/scale/skew form `set_transform`
/// writes (pivot at the origin). Refuses degenerate matrices.
pub fn decompose(m: &Matrix) -> Result<TransformSpec, V1DocumentError> {
    let scale_x = m[0].hypot(m[1]);
    if scale_x < 1e-9 || m.iter().any(|v| !v.is_finite()) {
        return Err(V1DocumentError::UnsupportedTransform);
    }
    let rotation = m[1].atan2(m[0]);
    let (sin, cos) = rotation.sin_cos();
    let scale_y = -m[2] * sin + m[3] * cos;
    if scale_y.abs() < 1e-9 {
        return Err(V1DocumentError::UnsupportedTransform);
    }
    let skew = ((m[2] * cos + m[3] * sin) / scale_y).atan();
    Ok(TransformSpec {
        translate: (m[4], m[5]),
        rotate_deg: rotation.to_degrees(),
        pivot: (0.0, 0.0),
        scale: (scale_x, scale_y),
        skew_x_deg: skew.to_degrees(),
    })
}

/// Bounds of the shape's own geometry, ignoring its `transform`.
fn local_bounds(doc: &V1Document, id: &str) -> Option<Bounds> {
    let info = doc.element_get(id).ok()?;
    let attr = |name: &str| -> Option<f64> {
        match info.attributes.iter().find(|(key, _)| key == name) {
            Some((_, value)) => value.trim().parse::<f64>().ok().filter(|v| v.is_finite()),
            None => Some(0.0),
        }
    };
    let bounds = match info.kind {
        ElementKind::Rect => {
            let (x, y) = (attr("x")?, attr("y")?);
            (x, y, x + attr("width")?, y + attr("height")?)
        }
        ElementKind::Ellipse => {
            let (cx, cy, rx, ry) = (attr("cx")?, attr("cy")?, attr("rx")?, attr("ry")?);
            (cx - rx, cy - ry, cx + rx, cy + ry)
        }
        ElementKind::Circle => {
            let (cx, cy, r) = (attr("cx")?, attr("cy")?, attr("r")?);
            (cx - r, cy - r, cx + r, cy + r)
        }
        ElementKind::Line => {
            let (x1, y1, x2, y2) = (attr("x1")?, attr("y1")?, attr("x2")?, attr("y2")?);
            (x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2))
        }
        ElementKind::Path => {
            let d = info.attributes.iter().find(|(key, _)| key == "d")?;
            path_bounds(&d.1)?
        }
        ElementKind::Group | ElementKind::Text | ElementKind::Other => return None,
    };
    Some(bounds)
}

/// Bounds of the points and control points of a path (arcs use their end
/// points), enough to place the handles and the rotation pivot.
fn path_bounds(d: &str) -> Option<Bounds> {
    let mut tokens = Tokens { text: d, at: 0 };
    let mut bounds: Option<Bounds> = None;
    let mut add = |p: (f64, f64)| {
        bounds = Some(match bounds {
            None => (p.0, p.1, p.0, p.1),
            Some(b) => (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)),
        });
    };
    let (mut cur, mut start) = ((0.0, 0.0), (0.0, 0.0));
    let mut command = b'M';
    while let Some(next) = tokens.command() {
        if next.is_none() && command.eq_ignore_ascii_case(&b'Z') {
            return None;
        }
        command = next.unwrap_or(command);
        let rel = command.is_ascii_lowercase();
        let origin = if rel { cur } else { (0.0, 0.0) };
        let take = |t: &mut Tokens<'_>, count: usize| -> Option<Vec<f64>> {
            (0..count).map(|_| t.number()).collect()
        };
        match command.to_ascii_uppercase() {
            b'Z' => cur = start,
            b'H' => cur.0 = take(&mut tokens, 1)?[0] + origin.0,
            b'V' => cur.1 = take(&mut tokens, 1)?[0] + origin.1,
            b'M' | b'L' | b'T' => {
                let v = take(&mut tokens, 2)?;
                cur = (v[0] + origin.0, v[1] + origin.1);
                if command.eq_ignore_ascii_case(&b'M') {
                    start = cur;
                    command = if rel { b'l' } else { b'L' };
                }
            }
            b'C' | b'S' | b'Q' => {
                let count = if command.eq_ignore_ascii_case(&b'C') {
                    6
                } else {
                    4
                };
                let v = take(&mut tokens, count)?;
                for pair in v.chunks(2) {
                    add((pair[0] + origin.0, pair[1] + origin.1));
                }
                cur = (v[count - 2] + origin.0, v[count - 1] + origin.1);
            }
            b'A' => {
                let v = take(&mut tokens, 7)?;
                cur = (v[5] + origin.0, v[6] + origin.1);
            }
            _ => return None,
        }
        add(cur);
    }
    bounds
}

struct Tokens<'a> {
    text: &'a str,
    at: usize,
}

impl Tokens<'_> {
    fn skip(&mut self) {
        let bytes = self.text.as_bytes();
        while bytes
            .get(self.at)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b',')
        {
            self.at += 1;
        }
    }

    /// `Some(Some(letter))` for a new command, `Some(None)` when numbers repeat
    /// the previous command, `None` at the end.
    fn command(&mut self) -> Option<Option<u8>> {
        self.skip();
        let b = *self.text.as_bytes().get(self.at)?;
        if b.is_ascii_alphabetic() {
            self.at += 1;
            Some(Some(b))
        } else {
            Some(None)
        }
    }

    fn number(&mut self) -> Option<f64> {
        self.skip();
        let bytes = self.text.as_bytes();
        let start = self.at;
        let mut end = start;
        let mut seen_dot = false;
        let mut seen_exp = false;
        while let Some(&b) = bytes.get(end) {
            let ok = b.is_ascii_digit()
                || (b == b'.' && !seen_dot && !seen_exp)
                || ((b == b'-' || b == b'+')
                    && (end == start || matches!(bytes[end - 1], b'e' | b'E')))
                || ((b == b'e' || b == b'E') && !seen_exp && end > start);
            if !ok {
                break;
            }
            seen_dot |= b == b'.';
            seen_exp |= b == b'e' || b == b'E';
            end += 1;
        }
        self.at = end;
        self.text
            .get(start..end)?
            .parse()
            .ok()
            .filter(|v: &f64| v.is_finite())
    }
}
