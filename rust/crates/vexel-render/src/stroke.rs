//! Solid stroke hit testing: segments with caps, joins, miter limit.
//!
//! Ports `shape_render_stroke.c` for the non-dashed path and the solid range
//! builder of `shape_render_dash_cache.c`. The uniform acceleration grid of
//! `shape_render_stroke_grid.c` is a pure culling structure; the exact
//! segment and join predicates below decide every sample, so it is not
//! ported (see parity-map.json).

use crate::dash::{self, Budget, Pattern};
use crate::{Linecap, Linejoin, RenderError, RenderShape, StrokeStyle};
use vexel_geometry::{Affine, Point, affine_map_point, affine_valid};

/// Bytes the C cache charges per segment, join and per-shape range record.
pub(crate) const SEGMENT_BYTES: usize = 40;
pub(crate) const JOIN_BYTES: usize = 48;
pub(crate) const RANGE_BYTES: usize = 96;

const SQRT_2: f64 = 1.414_213_562_373_095_1;

/// C `svg_shape_affine_inverse`.
pub(crate) fn invert(matrix: Affine) -> Option<Affine> {
    let determinant = matrix.a * matrix.d - matrix.b * matrix.c;
    if !affine_valid(matrix) || !determinant.is_finite() || determinant.abs() < 1e-15 {
        return None;
    }
    let reciprocal = 1.0 / determinant;
    let mut result = Affine {
        a: matrix.d * reciprocal,
        b: -matrix.b * reciprocal,
        c: -matrix.c * reciprocal,
        d: matrix.a * reciprocal,
        e: 0.0,
        f: 0.0,
    };
    result.e = -(result.a * matrix.e + result.c * matrix.f);
    result.f = -(result.b * matrix.e + result.d * matrix.f);
    affine_valid(result).then_some(result)
}

/// True when the stroke can paint anything (C: alpha != 0 and width > 0).
pub(crate) fn is_visible(stroke: &StrokeStyle) -> bool {
    stroke.rgba[3] != 0 && stroke.width > 0.0
}

/// One stroked edge range: first edge, edge count, closed, implicit-close.
pub(crate) struct StrokeRange {
    pub first: usize,
    pub count: usize,
    pub closed: bool,
    pub implicit_close: bool,
}

/// Number of ranges the C cache builder walks (per subpath, or the whole shape).
pub(crate) fn range_count(shape: &RenderShape<'_>) -> usize {
    shape.subpaths.len().max(1)
}

/// The `index`-th range; callers keep `index < range_count(shape)` and
/// `subpath_closed.len() == subpaths.len()` (checked by validation).
pub(crate) fn range_at(shape: &RenderShape<'_>, index: usize) -> StrokeRange {
    if shape.subpaths.is_empty() {
        return StrokeRange {
            first: 0,
            count: shape.edges.len(),
            closed: shape.stroke_closed,
            implicit_close: shape.filled,
        };
    }
    let range = &shape.subpaths[index];
    StrokeRange {
        first: range.start,
        count: range.end - range.start,
        closed: shape.subpath_closed[index],
        implicit_close: true,
    }
}

impl StrokeRange {
    /// Edge count after dropping the implicit fill-closing edge of an open path.
    pub fn stroked_count(&self) -> usize {
        if !self.closed && self.implicit_close && self.count > 0 { self.count - 1 } else { self.count }
    }

    /// Number of segments and joins this range contributes.
    pub fn primitive_counts(&self) -> (usize, usize) {
        let count = self.stroked_count();
        if count == 0 {
            return (0, 0);
        }
        (count, if self.closed { count } else { count - 1 })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Segment {
    pub a: Point,
    pub b: Point,
    pub start_cap: Linecap,
    pub end_cap: Linecap,
}

#[derive(Clone, Copy)]
pub(crate) struct Join {
    pub previous: Point,
    pub vertex: Point,
    pub next: Point,
}

/// Local-space stroke primitives of one shape.
pub(crate) struct StrokePrims {
    segments: Vec<Segment>,
    joins: Vec<Join>,
    radius: f64,
    join: Linejoin,
    miter_limit: f64,
    join_reach: f64,
}

pub(crate) fn reserve<T>(vec: &mut Vec<T>, additional: usize) -> Result<(), RenderError> {
    vec.try_reserve_exact(additional).map_err(|_| RenderError::LimitExceeded)
}

impl StrokePrims {
    /// Maps edges through the stroke inverse (C `prepare_local_stroke_edges`)
    /// and builds the segment and join lists: solid ranges (C
    /// `build_solid_range`) or, with a `pattern`, dashed ranges (C
    /// `dash_cache_build_range`).
    pub(crate) fn build(
        shape: &RenderShape<'_>,
        stroke: &StrokeStyle,
        inverse: Affine,
        pattern: Option<&Pattern>,
        budget: &mut Budget,
    ) -> Result<Self, RenderError> {
        let mut local: Vec<(Point, Point)> = Vec::new();
        reserve(&mut local, shape.edges.len())?;
        for edge in shape.edges {
            let a = affine_map_point(inverse, Point { x: edge.x0, y: edge.y0 });
            let b = affine_map_point(inverse, Point { x: edge.x1, y: edge.y1 });
            match (a, b) {
                (Some(a), Some(b)) => local.push((a, b)),
                _ => return Err(RenderError::InvalidGeometry),
            }
        }
        let mut segments = Vec::new();
        let mut joins = Vec::new();
        for range_index in 0..range_count(shape) {
            let range = range_at(shape, range_index);
            let count = range.stroked_count();
            if count == 0 {
                continue;
            }
            if let Some(pattern) = pattern {
                let edges = &local[range.first..range.first + range.count];
                let seam = range.closed && dash::joins_seam(pattern, edges);
                dash::build_range(
                    &mut segments,
                    &mut joins,
                    budget,
                    pattern,
                    stroke.cap,
                    &edges[..count],
                    seam,
                )?;
                continue;
            }
            let (segment_count, join_count) = range.primitive_counts();
            reserve(&mut segments, segment_count)?;
            reserve(&mut joins, join_count)?;
            for index in 0..count {
                let cap = if !range.closed && (index == 0 || index + 1 == count) {
                    stroke.cap
                } else {
                    Linecap::Butt
                };
                let (a, b) = local[range.first + index];
                segments.push(Segment { a, b, start_cap: cap, end_cap: cap });
            }
            for index in 0..join_count {
                let next = if range.closed { (index + 1) % count } else { index + 1 };
                let previous = local[range.first + (next + count - 1) % count];
                let following = local[range.first + next];
                joins.push(Join { previous: previous.0, vertex: previous.1, next: following.1 });
            }
        }
        let radius = stroke.width * 0.5;
        let join_reach = if stroke.join == Linejoin::Miter { radius * stroke.miter_limit } else { radius };
        Ok(Self { segments, joins, radius, join: stroke.join, miter_limit: stroke.miter_limit, join_reach })
    }

    /// C `dash_cache_contains` (non-grid branch) for a local-space sample.
    pub(crate) fn contains(&self, px: f64, py: f64) -> bool {
        let radius = self.radius;
        for segment in &self.segments {
            let square = segment.start_cap == Linecap::Square || segment.end_cap == Linecap::Square;
            let margin = if square { radius * SQRT_2 } else { radius };
            if edge_box_contains(segment.a, segment.b, px, py, margin)
                && segment_hit_caps(
                    segment.a,
                    segment.b,
                    px,
                    py,
                    radius,
                    segment.start_cap,
                    segment.end_cap,
                )
            {
                return true;
            }
        }
        for join in &self.joins {
            if (px - join.vertex.x).abs() <= self.join_reach
                && (py - join.vertex.y).abs() <= self.join_reach
                && join_hit(join, px, py, radius, self.join, self.miter_limit)
            {
                return true;
            }
        }
        false
    }
}

/// Extra local-space padding a stroke adds to the shape bounds; see
/// `stroke_padding` in `shape_bounds.c` (computed from the stroke inverse).
pub(crate) fn bounds_padding(
    stroke: &StrokeStyle,
    inverse: Affine,
    closed_whole: bool,
) -> Option<(f64, f64)> {
    let determinant = inverse.a * inverse.d - inverse.b * inverse.c;
    let forward_a = inverse.d / determinant;
    let forward_b = -inverse.c / determinant;
    let forward_c = -inverse.b / determinant;
    let forward_d = inverse.a / determinant;
    let radius = stroke.width * 0.5;
    let join_factor = if stroke.join == Linejoin::Miter { stroke.miter_limit } else { 1.0 };
    let cap_factor = if !closed_whole && stroke.cap == Linecap::Square { 2f64.sqrt() } else { 1.0 };
    let extent = radius * join_factor.max(cap_factor);
    let pad_x = extent * forward_a.hypot(forward_b);
    let pad_y = extent * forward_c.hypot(forward_d);
    (pad_x.is_finite() && pad_y.is_finite()).then_some((pad_x, pad_y))
}

fn edge_box_contains(a: Point, b: Point, px: f64, py: f64, margin: f64) -> bool {
    px >= a.x.min(b.x) - margin
        && px <= a.x.max(b.x) + margin
        && py >= a.y.min(b.y) - margin
        && py <= a.y.max(b.y) + margin
}

fn segment_hit_caps(
    a: Point,
    b: Point,
    px: f64,
    py: f64,
    radius: f64,
    start_cap: Linecap,
    end_cap: Linecap,
) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length = dx.hypot(dy);
    if length == 0.0 {
        if start_cap == Linecap::Round || end_cap == Linecap::Round {
            return (px - a.x).hypot(py - a.y) <= radius;
        }
        if start_cap == Linecap::Square || end_cap == Linecap::Square {
            return (px - a.x).abs() <= radius && (py - a.y).abs() <= radius;
        }
        return false;
    }
    let along = ((px - a.x) * dx + (py - a.y) * dy) / length;
    let across = ((px - a.x) * dy - (py - a.y) * dx).abs() / length;
    let mut low = 0.0;
    let mut high = length;
    if start_cap == Linecap::Square {
        low -= radius;
    }
    if end_cap == Linecap::Square {
        high += radius;
    }
    if along >= low && along <= high && across <= radius {
        return true;
    }
    if start_cap == Linecap::Round && (px - a.x).hypot(py - a.y) <= radius {
        return true;
    }
    if end_cap == Linecap::Round && (px - b.x).hypot(py - b.y) <= radius {
        return true;
    }
    false
}

fn in_triangle(px: f64, py: f64, a: Point, b: Point, c: Point) -> bool {
    let d1 = (px - b.x) * (a.y - b.y) - (a.x - b.x) * (py - b.y);
    let d2 = (px - c.x) * (b.y - c.y) - (b.x - c.x) * (py - c.y);
    let d3 = (px - a.x) * (c.y - a.y) - (c.x - a.x) * (py - a.y);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

fn join_hit(join: &Join, px: f64, py: f64, radius: f64, style: Linejoin, miter_limit: f64) -> bool {
    let (previous, vertex, next) = (join.previous, join.vertex, join.next);
    if style == Linejoin::Round {
        return (px - vertex.x).hypot(py - vertex.y) <= radius;
    }
    let ax = vertex.x - previous.x;
    let ay = vertex.y - previous.y;
    let bx = next.x - vertex.x;
    let by = next.y - vertex.y;
    let al = ax.hypot(ay);
    let bl = bx.hypot(by);
    if al == 0.0 || bl == 0.0 {
        return false;
    }
    let cross = ax * by - ay * bx;
    if cross.abs() < 1e-12 {
        return false;
    }
    let side = if cross > 0.0 { -1.0 } else { 1.0 };
    let p1 = Point { x: vertex.x + side * (-ay / al) * radius, y: vertex.y + side * (ax / al) * radius };
    let p2 = Point { x: vertex.x + side * (-by / bl) * radius, y: vertex.y + side * (bx / bl) * radius };
    if style == Linejoin::Bevel {
        return in_triangle(px, py, vertex, p1, p2);
    }
    let denom = ax * by - ay * bx;
    let qx = p2.x - p1.x;
    let qy = p2.y - p1.y;
    let t = (qx * by - qy * bx) / denom;
    let miter = Point { x: p1.x + t * ax, y: p1.y + t * ay };
    let ratio = (miter.x - vertex.x).hypot(miter.y - vertex.y) / radius;
    if !ratio.is_finite() || ratio > miter_limit {
        return in_triangle(px, py, vertex, p1, p2);
    }
    in_triangle(px, py, vertex, p1, miter) || in_triangle(px, py, vertex, miter, p2)
}
