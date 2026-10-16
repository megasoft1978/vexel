//! Owned primitive/custom geometry. Only authored path data produces path metadata.
use crate::source::{attribute, charge};
use vexel_base::{ControlledError, Span, WorkControl, WorkKind};
use vexel_geometry::{Affine, Point};
use vexel_scene::{Diagnostic, ErrorKind, LimitKind, Result};
use vexel_svg_path::{ParseOptions, PathDocument};
use vexel_xml::{XmlSpan, XmlToken};
/// Device-pixel flatness rule shared with paths and arcs (C-reference).
const LEGACY_DEVICE_ERROR: f64 = 0.25;
/// Maximum radial error of a flattened circle or ellipse, in device px.
const CURVE_DEVICE_ERROR: f64 = 0.08;
#[derive(Debug)]
pub struct Geometry {
    pub path: PathDocument,
    pub filled: bool,
    pub stroke_closed: bool,
    pub point_count: usize,
}
pub fn empty_path() -> PathDocument {
    PathDocument {
        edges: Vec::new(),
        subpaths: Vec::new(),
        segments: Vec::new(),
        numbers: Vec::new(),
    }
}
pub fn path_options() -> ParseOptions {
    ParseOptions::default()
}
fn error(kind: ErrorKind, offset: usize) -> Diagnostic {
    Diagnostic { kind, offset }
}
pub fn controlled_path_error(
    e: ControlledError<vexel_svg_path::Diagnostic>,
    offset: usize,
) -> Diagnostic {
    match e {
        ControlledError::Original(e) => crate::map_path_error(e),
        ControlledError::Canceled => crate::source::stopped(vexel_base::Stop::Canceled, offset),
        ControlledError::Limit => crate::source::stopped(vexel_base::Stop::Limit, offset),
        ControlledError::Overflow => crate::source::stopped(vexel_base::Stop::Overflow, offset),
        ControlledError::OutOfMemory => error(ErrorKind::OutOfMemory, offset),
    }
}
/// Public owned custom geometry builder for real downstream media composition.
pub struct GeometryBuilder {
    pub path: PathDocument,
    pub transform: Affine,
    pub offset: usize,
}
impl GeometryBuilder {
    pub fn new(transform: Affine, offset: usize) -> Result<Self> {
        if !vexel_geometry::affine_valid(transform) {
            return Err(error(ErrorKind::Number, offset));
        }
        Ok(Self {
            path: empty_path(),
            transform,
            offset,
        })
    }
    pub fn add_edge(
        &mut self,
        from: Point,
        to: Point,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        charge(control, WorkKind::Edge, 1, self.offset)?;
        vexel_svg_path::append_edge(
            from,
            to,
            self.transform,
            4 * 1024 * 1024,
            self.offset,
            &mut self.path.edges,
        )
        .map_err(crate::map_path_error)
    }
    pub fn add_curve(&mut self, points: [Point; 4], control: &mut dyn WorkControl) -> Result<()> {
        vexel_svg_path::flatten_cubic_controlled(
            points,
            self.transform,
            path_options(),
            self.offset,
            &mut self.path.edges,
            control,
        )
        .map_err(|e| controlled_path_error(e, self.offset))
    }
    pub fn finish(self, filled: bool, stroke_closed: bool, point_count: usize) -> Geometry {
        Geometry {
            path: self.path,
            filled,
            stroke_closed,
            point_count,
        }
    }
}
fn number(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    name: &[u8],
    reference: f64,
    default: Option<f64>,
    control: &mut dyn WorkControl,
) -> Result<f64> {
    match attribute(source, tokens, element, name, None) {
        Some(s) => crate::read_length_controlled(source, s, reference, control)
            .map_err(|e| crate::numeric_error(e, s.offset)),
        None => default.ok_or_else(|| error(ErrorKind::Unsupported, tokens[element].name.offset)),
    }
}
pub fn element_geometry(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    transform: Affine,
    viewport: [f64; 2],
) -> Result<Geometry> {
    element_geometry_controlled(source, tokens, element, transform, viewport, &mut NoStop)
}
pub fn element_geometry_controlled(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    transform: Affine,
    viewport: [f64; 2],
    control: &mut dyn WorkControl,
) -> Result<Geometry> {
    let t = tokens
        .get(element)
        .ok_or_else(|| error(ErrorKind::InvalidArgument, 0))?;
    let name = t
        .local_name
        .slice(source)
        .ok_or_else(|| error(ErrorKind::InvalidArgument, t.name.offset))?;
    if t.prefix.length != 0 {
        return Err(error(ErrorKind::Unsupported, t.name.offset));
    }
    if !viewport.iter().all(|v| v.is_finite() && *v > 0.0) {
        return Err(error(ErrorKind::InvalidArgument, t.name.offset));
    }
    let mut b = GeometryBuilder::new(transform, t.name.offset)?;
    if name == b"path" {
        let d = attribute(source, tokens, element, b"d", None)
            .ok_or_else(|| error(ErrorKind::Path, t.name.offset))?;
        let end = d
            .end()
            .ok_or_else(|| error(ErrorKind::InvalidArgument, d.offset))?;
        b.path = vexel_svg_path::parse_path_controlled(
            source,
            Span {
                start: d.offset,
                end,
            },
            transform,
            path_options(),
            control,
        )
        .map_err(|e| controlled_path_error(e, d.offset))?;
        return Ok(b.finish(true, true, 0));
    }
    if name == b"rect" {
        let x = number(
            source,
            tokens,
            element,
            b"x",
            viewport[0],
            Some(0.0),
            control,
        )?;
        let y = number(
            source,
            tokens,
            element,
            b"y",
            viewport[1],
            Some(0.0),
            control,
        )?;
        let w = number(
            source,
            tokens,
            element,
            b"width",
            viewport[0],
            None,
            control,
        )?;
        let h = number(
            source,
            tokens,
            element,
            b"height",
            viewport[1],
            None,
            control,
        )?;
        if w < 0.0 || h < 0.0 {
            return Err(error(ErrorKind::Number, t.name.offset));
        }
        if w == 0.0 || h == 0.0 {
            return Ok(b.finish(true, true, 0));
        }
        let points = [
            Point { x, y },
            Point { x: x + w, y },
            Point { x: x + w, y: y + h },
            Point { x, y: y + h },
        ];
        if points.iter().any(|p| !crate::point_bounded(*p)) {
            return Err(error(ErrorKind::Number, t.name.offset));
        }
        let xs = attribute(source, tokens, element, b"rx", None);
        let ys = attribute(source, tokens, element, b"ry", None);
        let mut rx = number(
            source,
            tokens,
            element,
            b"rx",
            viewport[0],
            Some(0.0),
            control,
        )
        .map_err(|e| crate::numeric_error(e, xs.or(ys).map_or(t.name.offset, |s| s.offset)))?;
        let mut ry = number(
            source,
            tokens,
            element,
            b"ry",
            viewport[1],
            Some(0.0),
            control,
        )
        .map_err(|e| crate::numeric_error(e, xs.or(ys).map_or(t.name.offset, |s| s.offset)))?;
        if rx < 0.0 || ry < 0.0 {
            return Err(error(
                ErrorKind::Number,
                xs.or(ys).map_or(t.name.offset, |s| s.offset),
            ));
        }
        if xs.is_some() && ys.is_none() {
            ry = rx
        } else if xs.is_none() && ys.is_some() {
            rx = ry
        }
        rx = rx.min(w * 0.5);
        ry = ry.min(h * 0.5);
        if rx == 0.0 || ry == 0.0 {
            for i in 0..4 {
                b.add_edge(points[i], points[(i + 1) % 4], control)?
            }
        } else {
            let k = 0.5522847498307936;
            let p = [
                Point { x: x + rx, y },
                Point { x: x + w - rx, y },
                Point {
                    x: x + w,
                    y: y + ry,
                },
                Point {
                    x: x + w,
                    y: y + h - ry,
                },
                Point {
                    x: x + w - rx,
                    y: y + h,
                },
                Point {
                    x: x + rx,
                    y: y + h,
                },
                Point { x, y: y + h - ry },
                Point { x, y: y + ry },
            ];
            b.add_edge(p[0], p[1], control)?;
            b.add_curve(
                [
                    p[1],
                    Point {
                        x: p[1].x + k * rx,
                        y: p[1].y,
                    },
                    Point {
                        x: p[2].x,
                        y: p[2].y - k * ry,
                    },
                    p[2],
                ],
                control,
            )?;
            b.add_edge(p[2], p[3], control)?;
            b.add_curve(
                [
                    p[3],
                    Point {
                        x: p[3].x,
                        y: p[3].y + k * ry,
                    },
                    Point {
                        x: p[4].x + k * rx,
                        y: p[4].y,
                    },
                    p[4],
                ],
                control,
            )?;
            b.add_edge(p[4], p[5], control)?;
            b.add_curve(
                [
                    p[5],
                    Point {
                        x: p[5].x - k * rx,
                        y: p[5].y,
                    },
                    Point {
                        x: p[6].x,
                        y: p[6].y + k * ry,
                    },
                    p[6],
                ],
                control,
            )?;
            b.add_edge(p[6], p[7], control)?;
            b.add_curve(
                [
                    p[7],
                    Point {
                        x: p[7].x,
                        y: p[7].y - k * ry,
                    },
                    Point {
                        x: p[0].x - k * rx,
                        y: p[0].y,
                    },
                    p[0],
                ],
                control,
            )?;
        }
        return Ok(b.finish(true, true, 0));
    }
    if name == b"circle" || name == b"ellipse" {
        let cx = number(
            source,
            tokens,
            element,
            b"cx",
            viewport[0],
            Some(0.0),
            control,
        )
        .map_err(|e| crate::numeric_error(e, t.name.offset))?;
        let cy = number(
            source,
            tokens,
            element,
            b"cy",
            viewport[1],
            Some(0.0),
            control,
        )
        .map_err(|e| crate::numeric_error(e, t.name.offset))?;
        let rx = number(
            source,
            tokens,
            element,
            if name == b"circle" { b"r" } else { b"rx" },
            if name == b"circle" {
                viewport[0].hypot(viewport[1]) / 2.0f64.sqrt()
            } else {
                viewport[0]
            },
            Some(0.0),
            control,
        )
        .map_err(|e| crate::numeric_error(e, t.name.offset))?;
        let ry = if name == b"circle" {
            rx
        } else {
            number(
                source,
                tokens,
                element,
                b"ry",
                viewport[1],
                Some(0.0),
                control,
            )
            .map_err(|e| crate::numeric_error(e, t.name.offset))?
        };
        if rx < 0.0
            || ry < 0.0
            || rx > 1e6
            || ry > 1e6
            || !crate::point_bounded(Point { x: cx, y: cy })
        {
            return Err(error(ErrorKind::Number, t.name.offset));
        }
        if rx == 0.0 || ry == 0.0 {
            return Ok(b.finish(true, true, 0));
        }
        // Circle/ellipse flattening bound (supersedes the C-parity 0.25 px rule,
        // which produced visible facets): the sagitta of each chord, i.e. the
        // maximum radial error against the analytic curve, is at most
        // `CURVE_DEVICE_ERROR` (0.08) device px. The transform's scale bound
        // (sqrt(row_norm * column_norm) >= the largest singular value, floored
        // at 1) turns the device bound into local units, so the error stays
        // <= 0.08 px under any scale. With radius R and tolerance t, the step
        // angle is 2*acos(1 - t/R) and steps = ceil(2*pi / angle), floor 12,
        // cap 4096 edges (over the cap is the Limit error).
        let tolerance = crate::transformed_flatness_tolerance(transform)
            * (CURVE_DEVICE_ERROR / LEGACY_DEVICE_ERROR);
        let radius = rx.max(ry);
        let angle = if radius <= tolerance {
            6.283185307179586
        } else {
            2.0 * (1.0 - tolerance / radius).max(-1.0).acos()
        };
        let raw = (6.283185307179586 / angle).ceil();
        if !raw.is_finite() || raw > 4096.0 {
            return Err(error(ErrorKind::Limit(LimitKind::Edges), t.name.offset));
        }
        let steps = (raw as usize).max(12);
        let mut previous = Point { x: cx + rx, y: cy };
        for i in 1..=steps {
            let theta = 6.283185307179586 * i as f64 / steps as f64;
            let next = if i == steps {
                Point { x: cx + rx, y: cy }
            } else {
                Point {
                    x: cx + rx * theta.cos(),
                    y: cy + ry * theta.sin(),
                }
            };
            if !crate::point_bounded(next) {
                return Err(error(ErrorKind::Number, t.name.offset));
            }
            b.add_edge(previous, next, control)?;
            previous = next;
        }
        return Ok(b.finish(true, true, 0));
    }
    if name == b"line" {
        let mut n = [0.0; 4];
        for (i, key) in [b"x1".as_slice(), b"y1", b"x2", b"y2"]
            .into_iter()
            .enumerate()
        {
            n[i] = number(
                source,
                tokens,
                element,
                key,
                viewport[i % 2],
                Some(0.0),
                control,
            )
            .map_err(|e| crate::numeric_error(e, t.name.offset))?;
        }
        let a = Point { x: n[0], y: n[1] };
        let z = Point { x: n[2], y: n[3] };
        if !crate::point_bounded(a) || !crate::point_bounded(z) {
            return Err(error(ErrorKind::Number, t.name.offset));
        }
        b.add_edge(a, z, control)?;
        return Ok(b.finish(false, false, 0));
    }
    if name == b"polyline" || name == b"polygon" {
        let span = attribute(source, tokens, element, b"points", None)
            .ok_or_else(|| error(ErrorKind::Path, t.name.offset))?;
        return poly_points(source, span, viewport, b, name == b"polygon", control);
    }
    Err(error(ErrorKind::Unsupported, t.name.offset))
}
fn poly_points(
    source: &[u8],
    span: XmlSpan,
    viewport: [f64; 2],
    mut b: GeometryBuilder,
    closed: bool,
    control: &mut dyn WorkControl,
) -> Result<Geometry> {
    let mut cur = crate::numbers::NumberReader::new(source, span)?;
    let mut first = None;
    let mut previous = None;
    let mut count = 0;
    loop {
        let before = cur.index;
        cur.skip_space(control)?;
        let spaced = before != cur.index;
        if count != 0 {
            if cur.peek() == Some(b',') {
                if !cur.separator(control)? {
                    return Err(error(ErrorKind::Path, cur.index));
                }
            } else if cur.index < cur.end && !spaced {
                return Err(error(ErrorKind::Path, cur.index));
            }
        }
        if cur.index == cur.end {
            break;
        }
        let x = cur.length(viewport[0], control).map_err(|e| {
            if matches!(e.kind, ErrorKind::Canceled | ErrorKind::Limit(_)) {
                e
            } else {
                error(ErrorKind::Path, cur.index)
            }
        })?;
        if !cur.separator(control)? {
            return Err(error(ErrorKind::Path, cur.index));
        }
        let y = cur.length(viewport[1], control).map_err(|e| {
            if matches!(e.kind, ErrorKind::Canceled | ErrorKind::Limit(_)) {
                e
            } else {
                error(ErrorKind::Path, cur.index)
            }
        })?;
        let p = Point { x, y };
        if !crate::point_bounded(p) {
            return Err(error(ErrorKind::Number, cur.index));
        }
        if count >= 4 * 1024 * 1024 {
            return Err(error(ErrorKind::Limit(LimitKind::PathCommands), cur.index));
        }
        count += 1;
        if first.is_none() {
            first = Some(p)
        }
        if let Some(last) = previous {
            b.add_edge(last, p, control)?
        }
        previous = Some(p);
    }
    if count < 2 {
        return Err(error(ErrorKind::Path, span.offset));
    }
    b.add_edge(previous.unwrap(), first.unwrap(), control)?;
    Ok(b.finish(true, closed, count))
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
