//! What the extended inspector rows show about the selected element: opacity,
//! fill kind with gradient stops and angle, dash preset and a transform
//! decomposed into rotation and scale about a pivot. Read from the source
//! document, never from the render path, so rows and edits agree.

use vexel_v1_document::{ElementInfo, V1Document};

/// Fill kind of the selected element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillKind {
    Solid,
    Linear,
    Radial,
}

/// Stroke dash preset; `Custom` is any other `stroke-dasharray`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dash {
    None,
    Dashed,
    Dotted,
    Custom,
}

/// A transform that is a pure rotate and scale about `pivot`, plus whatever
/// translation is left over once that pivot is fixed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformParts {
    /// Degrees in (-180, 180].
    pub rotation: f64,
    pub scale: (f64, f64),
    pub pivot: (f64, f64),
    pub translate: (f64, f64),
}

/// `Custom`: skew or a matrix that is not rotate and scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransformView {
    Parts(TransformParts),
    Custom,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeExt {
    /// The element's own `opacity`, 0..=1 (1 when absent).
    pub opacity: f64,
    pub fill: FillKind,
    /// First and last stop colour of the referenced gradient.
    pub stops: Option<[[u8; 3]; 2]>,
    /// Direction of a linear gradient in whole degrees.
    pub angle: Option<f64>,
    pub dash: Dash,
    pub transform: TransformView,
}

pub(crate) fn attr<'a>(info: &'a ElementInfo, name: &str) -> Option<&'a str> {
    info.attributes
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

fn num(info: &ElementInfo, name: &str, default: f64) -> f64 {
    attr(info, name)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}

/// `#rrggbb` to bytes.
pub fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let hex = text.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

pub fn hex_text(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

pub(crate) fn round_to(value: f64, places: i32) -> f64 {
    let k = 10f64.powi(places);
    let r = (value * k).round() / k;
    if r == 0.0 { 0.0 } else { r }
}

/// Degrees folded into (-180, 180].
pub fn normalize_degrees(deg: f64) -> f64 {
    let mut d = deg % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d <= -180.0 {
        d += 360.0;
    }
    round_to(d, 3)
}

impl ShapeExt {
    /// Reads the rows of element `id`; `None` when it does not exist.
    pub fn read(doc: &V1Document, id: &str) -> Option<Self> {
        let info = doc.element_get(id).ok()?;
        let opacity = num(&info, "opacity", 1.0).clamp(0.0, 1.0);
        let (fill, stops, angle) = read_fill(doc, &info);
        Some(Self {
            opacity,
            fill,
            stops,
            angle,
            dash: read_dash(&info),
            transform: read_transform(doc, id, &info),
        })
    }
}

type FillRead = (FillKind, Option<[[u8; 3]; 2]>, Option<f64>);

fn read_fill(doc: &V1Document, info: &ElementInfo) -> FillRead {
    let solid = (FillKind::Solid, None, None);
    let Some(gid) = attr(info, "fill")
        .and_then(|v| v.trim().strip_prefix("url(#"))
        .and_then(|v| v.strip_suffix(')'))
    else {
        return solid;
    };
    let Ok(gradient) = doc.element_get(gid) else {
        return solid;
    };
    let kind = match gradient.tag.as_str() {
        "linearGradient" => FillKind::Linear,
        "radialGradient" => FillKind::Radial,
        _ => return solid,
    };
    let colour = |stop: Option<&String>| {
        let stop = doc.element_get(stop?).ok()?;
        parse_hex(attr(&stop, "stop-color")?)
    };
    let stops = match (
        colour(gradient.children.first()),
        colour(gradient.children.last()),
    ) {
        (Some(a), Some(b)) => Some([a, b]),
        _ => None,
    };
    let angle = (kind == FillKind::Linear).then(|| {
        let (x1, y1) = (num(&gradient, "x1", 0.0), num(&gradient, "y1", 0.0));
        let (x2, y2) = (num(&gradient, "x2", 1.0), num(&gradient, "y2", 0.0));
        round_to((y2 - y1).atan2(x2 - x1).to_degrees(), 0)
    });
    (kind, stops, angle)
}

fn read_dash(info: &ElementInfo) -> Dash {
    let Some(raw) = attr(info, "stroke-dasharray") else {
        return Dash::None;
    };
    if raw.trim() == "none" {
        return Dash::None;
    }
    let values: Vec<f64> = raw
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    match values[..] {
        [a, b] if a == 0.0 && b > 0.0 => Dash::Dotted,
        [a, b] if a > 0.0 && b > 0.0 => Dash::Dashed,
        _ => Dash::Custom,
    }
}

fn read_transform(doc: &V1Document, id: &str, info: &ElementInfo) -> TransformView {
    let Ok(m) = doc.get_transform_matrix(id) else {
        return TransformView::Custom;
    };
    let [a, b, c, d, e, f] = m;
    let sx = a.hypot(b);
    if !m.iter().all(|v| v.is_finite()) || sx < 1e-9 {
        return TransformView::Custom;
    }
    let theta = b.atan2(a);
    let (sin, cos) = theta.sin_cos();
    let sy = -c * sin + d * cos;
    let tol = 1e-6 * sy.abs().max(1.0);
    if sy.abs() < 1e-9 || (c + sin * sy).abs() > tol || (d - cos * sy).abs() > tol {
        return TransformView::Custom;
    }
    let pivot = pivot_for(info, m);
    let translate = (
        e - (pivot.0 - a * pivot.0 - c * pivot.1),
        f - (pivot.1 - b * pivot.0 - d * pivot.1),
    );
    let snap = |v: f64| if v.abs() < 1e-9 { 0.0 } else { v };
    TransformView::Parts(TransformParts {
        rotation: normalize_degrees(theta.to_degrees()),
        scale: (round_to(sx, 6), round_to(sy, 6)),
        pivot,
        translate: (snap(translate.0), snap(translate.1)),
    })
}

/// The pivot the rows show. A rotate/scale that is well conditioned has a
/// fixed point, which is the pivot it was written with: that way an edited
/// pivot reads back and `translate` comes out zero. Otherwise (no rotate or
/// scale) the element's bounds centre, else its local centre, else the
/// origin, with any leftover shown as `translate`.
fn pivot_for(info: &ElementInfo, m: [f64; 6]) -> (f64, f64) {
    let [a, b, c, d, e, f] = m;
    let (p, q, r, s) = (1.0 - a, -c, -b, 1.0 - d);
    let det = p * s - q * r;
    if det.abs() > 1e-3 {
        return ((s * e - q * f) / det, (-r * e + p * f) / det);
    }
    let identity_linear =
        (a - 1.0).abs() < 1e-12 && b.abs() < 1e-12 && c.abs() < 1e-12 && (d - 1.0).abs() < 1e-12;
    // Bounds are only reported for no transform or a pure `translate(..)`.
    if let (Some((x0, y0, x1, y1)), true) = (info.bounds, identity_linear) {
        return ((x0 + x1) / 2.0 - e, (y0 + y1) / 2.0 - f);
    }
    if let Some(centre) = local_centre(info) {
        return centre;
    }
    if det.abs() > 1e-9 {
        return ((s * e - q * f) / det, (-r * e + p * f) / det);
    }
    (0.0, 0.0)
}

fn local_centre(info: &ElementInfo) -> Option<(f64, f64)> {
    let n = |name: &str| num(info, name, 0.0);
    match info.tag.as_str() {
        "rect" => Some((n("x") + n("width") / 2.0, n("y") + n("height") / 2.0)),
        "ellipse" | "circle" => Some((n("cx"), n("cy"))),
        "line" => Some(((n("x1") + n("x2")) / 2.0, (n("y1") + n("y2")) / 2.0)),
        _ => None,
    }
}
