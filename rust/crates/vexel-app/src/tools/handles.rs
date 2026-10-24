//! Selection handle geometry: rotate corners, resize handles and their hit zones.

use super::controller::Point;

pub type Bounds = (f64, f64, f64, f64);

/// Screen pixels around a handle that start a resize.
pub const HANDLE_PX: f64 = 6.0;
/// Reach of the rotate zone outside a corner, in document units (at least).
const ROTATE_REACH: f64 = 16.0;

/// The rotate zone: outside the box, near a corner but beyond the resize
/// handle's grab radius `tol` (document units).
pub fn rotate_start(bounds: Bounds, p: Point, tol: f64) -> Option<Point> {
    let corners = [
        (bounds.0, bounds.1),
        (bounds.2, bounds.1),
        (bounds.2, bounds.3),
        (bounds.0, bounds.3),
    ];
    let reach = ROTATE_REACH.max(tol * 2.0);
    corners.into_iter().find(|corner| {
        let dx = (p.0 - corner.0).abs();
        let dy = (p.1 - corner.1).abs();
        let outside = (p.0 < bounds.0 || p.0 > bounds.2) && (p.1 < bounds.1 || p.1 > bounds.3);
        outside && dx <= reach && dy <= reach && (dx > tol || dy > tol)
    })
}

fn handle_points(b: Bounds) -> [Point; 8] {
    let mx = (b.0 + b.2) / 2.0;
    let my = (b.1 + b.3) / 2.0;
    [
        (b.0, b.1),
        (b.2, b.1),
        (b.2, b.3),
        (b.0, b.3),
        (mx, b.1),
        (b.2, my),
        (mx, b.3),
        (b.0, my),
    ]
}

/// The nearest handle within `tol` document units on both axes.
pub fn handle_at(b: Bounds, p: Point, tol: f64) -> Option<usize> {
    handle_points(b)
        .iter()
        .enumerate()
        .filter(|(_, h)| (h.0 - p.0).abs() <= tol && (h.1 - p.1).abs() <= tol)
        .min_by(|(_, a), (_, c)| {
            let (da, dc) = ((a.0 - p.0).hypot(a.1 - p.1), (c.0 - p.0).hypot(c.1 - p.1));
            da.total_cmp(&dc)
        })
        .map(|(index, _)| index)
}

pub fn resize_bounds(b: Bounds, h: usize, p: Point) -> Bounds {
    let (mut l, mut t, mut r, mut bot) = b;
    if matches!(h, 0 | 3 | 7) {
        l = p.0;
    }
    if matches!(h, 1 | 2 | 5) {
        r = p.0;
    }
    if matches!(h, 0 | 1 | 4) {
        t = p.1;
    }
    if matches!(h, 2 | 3 | 6) {
        bot = p.1;
    }
    let (l, r) = if l <= r { (l, r) } else { (r, l) };
    let (t, bot) = if t <= bot { (t, bot) } else { (bot, t) };
    (l, t, r, bot)
}
