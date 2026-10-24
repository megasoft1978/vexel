//! Join hit tests with the per-join geometry computed once. `JoinGeom::new`
//! evaluates exactly the expressions the former per-sample `join_hit` did, in
//! the same order, so every hit result is bit-for-bit the same; only the
//! repeated `hypot` and division work moves out of the sample loop.
//!
//! The same geometry gives a tight hit box (the hull of the join's points)
//! instead of the `radius * miter_limit` square around the vertex. The tight
//! box is only used when the join is well conditioned (see [`TIGHT_SIN`]):
//! then a point outside the padded hull is outside every triangle by far more
//! than the rounding error of `in_triangle`, so the box never changes a result.

use crate::Linejoin;
use crate::stroke::Join;
use vexel_geometry::Point;

/// Smallest `|sin(turn angle)|` of a join that gets a tight box. Thinner
/// triangles (almost straight or reversing joins) keep the reach box.
const TIGHT_SIN: f64 = 1e-5;

#[derive(Clone, Copy)]
pub(crate) enum Shape {
    /// Never hits (degenerate or collinear join).
    Never,
    Round,
    Triangle(Point, Point),
    Miter(Point, Point, Point),
}

#[derive(Clone, Copy)]
pub(crate) struct JoinGeom {
    pub shape: Shape,
    /// Padded hull box `[x0, y0, x1, y1]` when the join is well conditioned.
    pub tight: Option<[f64; 4]>,
}

impl JoinGeom {
    pub(crate) fn new(join: &Join, radius: f64, style: Linejoin, miter_limit: f64) -> Self {
        let (previous, vertex, next) = (join.previous, join.vertex, join.next);
        if style == Linejoin::Round {
            let pad = radius + (radius + vertex.x.abs().max(vertex.y.abs())) * 1e-9;
            let tight = [vertex.x - pad, vertex.y - pad, vertex.x + pad, vertex.y + pad];
            return Self { shape: Shape::Round, tight: tight.iter().all(|v| v.is_finite()).then_some(tight) };
        }
        let ax = vertex.x - previous.x;
        let ay = vertex.y - previous.y;
        let bx = next.x - vertex.x;
        let by = next.y - vertex.y;
        let al = ax.hypot(ay);
        let bl = bx.hypot(by);
        if al == 0.0 || bl == 0.0 {
            return Self { shape: Shape::Never, tight: None };
        }
        let cross = ax * by - ay * bx;
        if cross.abs() < 1e-12 {
            return Self { shape: Shape::Never, tight: None };
        }
        let side = if cross > 0.0 { -1.0 } else { 1.0 };
        let p1 = Point {
            x: vertex.x + side * (-ay / al) * radius,
            y: vertex.y + side * (ax / al) * radius,
        };
        let p2 = Point {
            x: vertex.x + side * (-by / bl) * radius,
            y: vertex.y + side * (bx / bl) * radius,
        };
        let mut shape = Shape::Triangle(p1, p2);
        if style != Linejoin::Bevel {
            let denom = ax * by - ay * bx;
            let qx = p2.x - p1.x;
            let qy = p2.y - p1.y;
            let t = (qx * by - qy * bx) / denom;
            let miter = Point { x: p1.x + t * ax, y: p1.y + t * ay };
            let ratio = (miter.x - vertex.x).hypot(miter.y - vertex.y) / radius;
            if ratio.is_finite() && ratio <= miter_limit {
                shape = Shape::Miter(p1, miter, p2);
            }
        }
        let tight = (cross.abs() / (al * bl) >= TIGHT_SIN)
            .then(|| hull_box(vertex, &shape, radius))
            .flatten();
        Self { shape, tight }
    }

    /// Same truth value as the former `join_hit` for the join this was built from.
    pub(crate) fn hit(&self, vertex: Point, px: f64, py: f64, radius: f64) -> bool {
        match self.shape {
            Shape::Never => false,
            Shape::Round => within(px - vertex.x, py - vertex.y, radius),
            Shape::Triangle(p1, p2) => in_triangle(px, py, vertex, p1, p2),
            Shape::Miter(p1, miter, p2) => {
                in_triangle(px, py, vertex, p1, miter) || in_triangle(px, py, vertex, miter, p2)
            }
        }
    }
}

fn hull_box(vertex: Point, shape: &Shape, radius: f64) -> Option<[f64; 4]> {
    let mut points = [vertex; 4];
    let count = match *shape {
        Shape::Triangle(p1, p2) => {
            points[1] = p1;
            points[2] = p2;
            3
        }
        Shape::Miter(p1, miter, p2) => {
            points[1] = p1;
            points[2] = miter;
            points[3] = p2;
            4
        }
        _ => return None,
    };
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut scale = radius;
    for p in &points[..count] {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
        scale = scale.max(p.x.abs()).max(p.y.abs());
    }
    let pad = 1e-4 * radius + 1e-6 * scale;
    let tight = [x0 - pad, y0 - pad, x1 + pad, y1 + pad];
    tight.iter().all(|v| v.is_finite()).then_some(tight)
}

/// `hypot(dx, dy) <= radius`, skipping the `hypot` call whenever the squared
/// distance is clearly on one side (relative margin far above rounding error).
pub(crate) fn within(dx: f64, dy: f64, radius: f64) -> bool {
    if radius > 1e-100 && radius < 1e100 {
        let squared = dx * dx + dy * dy;
        let limit = radius * radius;
        if squared > limit * (1.0 + 1e-9) {
            return false;
        }
        if squared < limit * (1.0 - 1e-9) {
            return true;
        }
    }
    dx.hypot(dy) <= radius
}

fn in_triangle(px: f64, py: f64, a: Point, b: Point, c: Point) -> bool {
    let d1 = (px - b.x) * (a.y - b.y) - (a.x - b.x) * (py - b.y);
    let d2 = (px - c.x) * (b.y - c.y) - (b.x - c.x) * (py - c.y);
    let d3 = (px - a.x) * (c.y - a.y) - (c.x - a.x) * (py - a.y);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}
