//! Hover and selection outlines and resize handles, drawn in view space so
//! lines keep their width at any zoom. Rects are device pixels `(x0, y0, x1, y1)`.

use super::surface::{Surface, rgb};

pub type DeviceRect = (f64, f64, f64, f64);

/// Hover outline: 1 logical point at 60% accent.
pub const HOVER_WIDTH: f64 = 1.0;
pub const HOVER_OPACITY: f64 = 0.6;
/// Selection outline: 1.5 logical points.
pub const SELECTION_WIDTH: f64 = 1.5;
/// Handle size in logical points.
pub const HANDLE: i64 = 8;

fn span(lo: f64, hi: f64, p: i64) -> f64 {
    (hi.min((p + 1) as f64) - lo.max(p as f64)).clamp(0.0, 1.0)
}

fn area(r: DeviceRect, x: i64, y: i64) -> f64 {
    span(r.0, r.2, x) * span(r.1, r.3, y)
}

/// Strokes `rect` with a band `width` device pixels wide centred on its edge,
/// with exact pixel coverage, in `color` at `opacity`.
pub fn outline(s: &mut Surface<'_>, rect: DeviceRect, width: f64, color: u32, opacity: f64) {
    let h = width / 2.0;
    let outer = (rect.0 - h, rect.1 - h, rect.2 + h, rect.3 + h);
    let inner = (rect.0 + h, rect.1 + h, rect.2 - h, rect.3 - h);
    let has_inner = inner.2 > inner.0 && inner.3 > inner.1;
    let c = s.clip.intersect(
        outer.0.floor() as i64,
        outer.1.floor() as i64,
        outer.2.ceil() as i64,
        outer.3.ceil() as i64,
    );
    // Pixels wholly inside the inner rectangle carry no stroke.
    let (ix0, ix1) = (inner.0.ceil() as i64, inner.2.floor() as i64);
    let (iy0, iy1) = (inner.1.ceil() as i64, inner.3.floor() as i64);
    let color = rgb(color);
    for y in c.y0..c.y1 {
        let skip = has_inner && y >= iy0 && y < iy1;
        let mut x = c.x0;
        while x < c.x1 {
            if skip && x >= ix0 && x < ix1 {
                x = ix1;
                continue;
            }
            let inner_area = if has_inner { area(inner, x, y) } else { 0.0 };
            s.blend(x, y, color, (area(outer, x, y) - inner_area) * opacity);
            x += 1;
        }
    }
}

/// The 8 handle centres of a rect: corners then edge midpoints.
pub fn handle_points(r: DeviceRect) -> [(f64, f64); 8] {
    let (mx, my) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0);
    [
        (r.0, r.1),
        (r.2, r.1),
        (r.2, r.3),
        (r.0, r.3),
        (mx, r.1),
        (r.2, my),
        (mx, r.3),
        (r.0, my),
    ]
}

/// Device rect `(x0, y0, x1, y1)` of the handle centred on `centre`.
pub fn handle_rect(centre: (f64, f64), scale: i64) -> (i64, i64, i64, i64) {
    let size = HANDLE * scale;
    let x0 = (centre.0 - size as f64 / 2.0).round() as i64;
    let y0 = (centre.1 - size as f64 / 2.0).round() as i64;
    (x0, y0, x0 + size, y0 + size)
}

/// Eight handles: white fill, accent edge of 1 logical point.
pub fn handles(s: &mut Surface<'_>, rect: DeviceRect, accent: u32) {
    let edge = s.scale;
    for centre in handle_points(rect) {
        let (x0, y0, x1, y1) = handle_rect(centre, s.scale);
        s.fill(x0, y0, x1, y1, rgb(accent));
        s.fill(x0 + edge, y0 + edge, x1 - edge, y1 - edge, [255, 255, 255]);
    }
}

/// Paints the live rotated selection outline around its bounds centre.
pub fn rotation_preview(s: &mut Surface<'_>, rect: DeviceRect, degrees: f64, accent: u32) {
    let (cx, cy) = ((rect.0 + rect.2) / 2.0, (rect.1 + rect.3) / 2.0);
    let (sin, cos) = degrees.to_radians().sin_cos();
    let corners = [
        (rect.0, rect.1),
        (rect.2, rect.1),
        (rect.2, rect.3),
        (rect.0, rect.3),
    ];
    let points = corners.map(|(x, y)| {
        let (dx, dy) = (x - cx, y - cy);
        (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
    });
    for i in 0..4 {
        line(s, points[i], points[(i + 1) % 4], rgb(accent));
    }
}

fn line(s: &mut Surface<'_>, a: (f64, f64), b: (f64, f64), color: [u8; 3]) {
    let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil() as usize;
    for step in 0..=steps {
        let t = if steps == 0 {
            0.0
        } else {
            step as f64 / steps as f64
        };
        s.blend(
            (a.0 + (b.0 - a.0) * t).round() as i64,
            (a.1 + (b.1 - a.1) * t).round() as i64,
            color,
            1.0,
        );
    }
}
