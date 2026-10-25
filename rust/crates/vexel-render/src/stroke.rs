//! Solid stroke hit testing: segments with caps, joins, miter limit.
//!
//! Ports `shape_render_stroke.c` for the non-dashed path and the solid range
//! builder of `shape_render_dash_cache.c`. The uniform acceleration grid of
//! `shape_render_stroke_grid.c` is a pure culling structure; the exact
//! segment and join predicates below decide every sample, so it is not
//! ported (see parity-map.json).

use crate::dash::{self, Budget, Pattern};
use crate::stroke_grid::{self, Grid};
use crate::stroke_join::{JoinGeom, within};
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

/// [`is_visible`] for a stroke whose colour may be a gradient: a gradient
/// stroke ignores the solid colour's alpha, so only its width decides.
pub(crate) fn is_painted(stroke: &StrokeStyle, gradient: bool) -> bool {
    stroke.width > 0.0 && (gradient || stroke.rgba[3] != 0)
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
        if !self.closed && self.implicit_close && self.count > 0 {
            self.count - 1
        } else {
            self.count
        }
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
    /// Per-join geometry, parallel to `joins`.
    geoms: Vec<JoinGeom>,
    /// Segment lengths (`hypot` of the endpoint difference), parallel to `segments`.
    lengths: Vec<f64>,
    radius: f64,
    join_reach: f64,
    grid: Option<Grid>,
    /// Hit box of every segment then every join, same arithmetic as their tests.
    boxes: Vec<[f64; 4]>,
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
        let join_reach =
            if stroke.join == Linejoin::Miter { radius * stroke.miter_limit } else { radius };
        let mut geoms = Vec::new();
        reserve(&mut geoms, joins.len())?;
        geoms.extend(
            joins.iter().map(|j| JoinGeom::new(j, radius, stroke.join, stroke.miter_limit)),
        );
        let mut lengths = Vec::new();
        reserve(&mut lengths, segments.len())?;
        lengths.extend(segments.iter().map(|s| (s.b.x - s.a.x).hypot(s.b.y - s.a.y)));
        let grid = Grid::build(&segments, &joins, &geoms, radius, join_reach)?;
        let mut boxes = Vec::new();
        {
            reserve(&mut boxes, segments.len() + joins.len())?;
            for segment in &segments {
                let square =
                    segment.start_cap == Linecap::Square || segment.end_cap == Linecap::Square;
                let margin = if square { radius * SQRT_2 } else { radius };
                let (a, b) = (segment.a, segment.b);
                boxes.push([
                    a.x.min(b.x) - margin,
                    a.y.min(b.y) - margin,
                    a.x.max(b.x) + margin,
                    a.y.max(b.y) + margin,
                ]);
            }
            for (join, geom) in joins.iter().zip(&geoms) {
                boxes.push(geom.tight.unwrap_or_else(|| {
                    let (vx, vy) = (join.vertex.x, join.vertex.y);
                    let pad = join_reach + (join_reach + vx.abs().max(vy.abs())) * 1e-9;
                    [vx - pad, vy - pad, vx + pad, vy + pad]
                }));
            }
        }
        Ok(Self {
            boxes,
            segments,
            joins,
            geoms,
            lengths,
            radius,
            join_reach,
            grid,
        })
    }

    /// C `dash_cache_contains` for a local-space sample: any segment or join
    /// hits. With a grid only the candidates of the sample's cell are tested.
    pub(crate) fn contains(&self, px: f64, py: f64) -> bool {
        let Some(grid) = self.grid.as_ref() else {
            return self.contains_linear(px, py);
        };
        grid.candidates(px, py).iter().any(|tag| {
            let (is_join, index) = stroke_grid::decode(*tag);
            if is_join {
                self.join_hit_at(index, px, py)
            } else {
                self.segment_hit_at(index, px, py)
            }
        })
    }

    /// Tags (segments first, then joins offset by the segment count) whose hit
    /// box meets the local rectangle, padded for rounding. A sample inside the
    /// rectangle can only be hit by one of these. With a grid only the cells
    /// the rectangle touches are scanned (a tag may repeat across cells).
    pub(crate) fn candidates_in(&self, rect: [f64; 4], out: &mut Vec<u32>) {
        out.clear();
        let pad = 1e-7 * (1.0 + rect.iter().fold(0.0f64, |m, v| m.max(v.abs())));
        let (x0, y0, x1, y1) = (rect[0] - pad, rect[1] - pad, rect[2] + pad, rect[3] + pad);
        let meets = |b: &[f64; 4]| b[0] <= x1 && b[2] >= x0 && b[1] <= y1 && b[3] >= y0;
        match self.grid.as_ref() {
            None => {
                for (tag, b) in self.boxes.iter().enumerate() {
                    if meets(b) {
                        out.push(tag as u32);
                    }
                }
            }
            Some(grid) => {
                let segments = self.segments.len();
                grid.for_each_in(x0, y0, x1, y1, |item| {
                    let (is_join, index) = stroke_grid::decode(item);
                    let tag = if is_join { segments + index } else { index };
                    if meets(&self.boxes[tag]) {
                        out.push(tag as u32);
                    }
                });
            }
        }
    }

    /// True when every point of the convex hull of `corners` is provably hit by
    /// one candidate segment's body (the rectangle between its end points,
    /// shrunk by a margin far above rounding error), so all samples between
    /// the corners hit without being tested.
    pub(crate) fn body_covers(&self, corners: &[Point; 4], tags: &[u32]) -> bool {
        let segments = self.segments.len();
        tags.iter().any(|tag| {
            let index = *tag as usize;
            if index >= segments {
                return false;
            }
            let segment = &self.segments[index];
            let length = self.lengths[index];
            let reach = self.radius;
            if !(length > 0.0) || !length.is_finite() {
                return false;
            }
            let margin = 1e-8 * (reach + length);
            let (dx, dy) = (segment.b.x - segment.a.x, segment.b.y - segment.a.y);
            let across_limit = (reach - margin) * length;
            corners.iter().all(|c| {
                let (rx, ry) = (c.x - segment.a.x, c.y - segment.a.y);
                let dot = rx * dx + ry * dy;
                let cross = (rx * dy - ry * dx).abs();
                cross < across_limit && dot > margin * length && dot < (length - margin) * length
            })
        })
    }

    /// [`Self::contains`] restricted to candidate tags from [`Self::candidates_in`].
    pub(crate) fn contains_among(&self, tags: &[u32], px: f64, py: f64) -> bool {
        let segments = self.segments.len();
        tags.iter().any(|tag| {
            let index = *tag as usize;
            if index < segments {
                self.segment_hit_at(index, px, py)
            } else {
                self.join_hit_at(index - segments, px, py)
            }
        })
    }

    /// Every primitive tested in order (the grid-free path).
    pub(crate) fn contains_linear(&self, px: f64, py: f64) -> bool {
        (0..self.segments.len()).any(|index| self.segment_hit_at(index, px, py))
            || (0..self.joins.len()).any(|index| self.join_hit_at(index, px, py))
    }

    fn segment_hit_at(&self, index: usize, px: f64, py: f64) -> bool {
        let segment = &self.segments[index];
        let radius = self.radius;
        let square = segment.start_cap == Linecap::Square || segment.end_cap == Linecap::Square;
        let margin = if square { radius * SQRT_2 } else { radius };
        edge_box_contains(segment.a, segment.b, px, py, margin)
            && segment_hit_caps(
                segment.a,
                segment.b,
                self.lengths[index],
                px,
                py,
                radius,
                segment.start_cap,
                segment.end_cap,
            )
    }

    fn join_hit_at(&self, index: usize, px: f64, py: f64) -> bool {
        let join = &self.joins[index];
        let geom = &self.geoms[index];
        let inside_box = match geom.tight {
            Some(b) => px >= b[0] && px <= b[2] && py >= b[1] && py <= b[3],
            None => {
                (px - join.vertex.x).abs() <= self.join_reach
                    && (py - join.vertex.y).abs() <= self.join_reach
            }
        };
        inside_box && geom.hit(join.vertex, px, py, self.radius)
    }
}

/// Extra local-space padding a stroke adds to the shape bounds; see
/// `stroke_padding` in `shape_bounds.c` (computed from the stroke inverse).
pub(crate) fn bounds_padding(
    stroke: &StrokeStyle,
    inverse: Affine,
    closed_whole: bool,
    dashed: bool,
) -> Option<(f64, f64)> {
    let determinant = inverse.a * inverse.d - inverse.b * inverse.c;
    let forward_a = inverse.d / determinant;
    let forward_b = -inverse.c / determinant;
    let forward_c = -inverse.b / determinant;
    let forward_d = inverse.a / determinant;
    let radius = stroke.width * 0.5;
    let join_factor = if stroke.join == Linejoin::Miter { stroke.miter_limit } else { 1.0 };
    // Every dash of a closed range carries its own caps, so a dashed
    // stroke needs the square-cap corner reach even when the range is closed.
    let cap_factor = if (!closed_whole || dashed) && stroke.cap == Linecap::Square { 2f64.sqrt() } else { 1.0 };
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
    length: f64,
    px: f64,
    py: f64,
    radius: f64,
    start_cap: Linecap,
    end_cap: Linecap,
) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if length == 0.0 {
        if start_cap == Linecap::Round || end_cap == Linecap::Round {
            return within(px - a.x, py - a.y, radius);
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
    if start_cap == Linecap::Round && within(px - a.x, py - a.y, radius) {
        return true;
    }
    if end_cap == Linecap::Round && within(px - b.x, py - b.y, radius) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlatEdge;
    use vexel_geometry::affine_identity;

    /// xorshift64*: deterministic samples without a dev-dependency.
    struct Rng(u64);

    impl Rng {
        fn unit(&mut self) -> f64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
        }

        fn range(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (hi - lo) * self.unit()
        }
    }

    fn style(cap: Linecap, join: Linejoin, width: f64) -> StrokeStyle {
        StrokeStyle {
            rgba: [0, 0, 0, 255],
            width,
            miter_limit: 4.0,
            dash_offset: 0.0,
            cap,
            join,
            crisp_edges: false,
        }
    }

    fn build(points: &[(f64, f64)], closed: bool, stroke: StrokeStyle) -> StrokePrims {
        let edges: Vec<FlatEdge> = points
            .windows(2)
            .map(|w| FlatEdge { x0: w[0].0, y0: w[0].1, x1: w[1].0, y1: w[1].1 })
            .collect();
        let shape = RenderShape {
            fill_gradient: None,
            stroke_gradient: None,
            edges: &edges,
            subpaths: &[],
            subpath_closed: &[],
            stroke_closed: closed,
            evenodd: false,
            filled: false,
            fill_rgba: [0; 4],
            stroke: Some(stroke),
            transform: affine_identity(),
        };
        let mut budget = Budget { steps: 0, bytes: 0 };
        StrokePrims::build(&shape, &stroke, affine_identity(), None, &mut budget).unwrap()
    }

    /// Samples inside the box around `points` plus samples near the vertices.
    fn samples(rng: &mut Rng, points: &[(f64, f64)], count: usize, slack: f64) -> Vec<(f64, f64)> {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(x, y) in points {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        (0..count)
            .map(|i| {
                if i % 3 == 0 {
                    let index = ((rng.unit() * points.len() as f64) as usize).min(points.len() - 1);
                    let (x, y) = points[index];
                    (x + rng.range(-slack, slack), y + rng.range(-slack, slack))
                } else {
                    (rng.range(x0 - slack, x1 + slack), rng.range(y0 - slack, y1 + slack))
                }
            })
            .collect()
    }

    fn assert_equal_everywhere(prims: &StrokePrims, samples: &[(f64, f64)]) {
        for &(px, py) in samples {
            assert_eq!(
                prims.contains(px, py),
                prims.contains_linear(px, py),
                "grid and linear scan disagree at ({px}, {py})"
            );
        }
    }

    /// Short segments: the grid is built here, so the grid path is exercised.
    #[test]
    fn grid_contains_equals_linear_scan_on_dense_path() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut points = vec![(0.0, 0.0)];
        for _ in 0..400 {
            let (x, y) = *points.last().unwrap();
            points.push((x + rng.range(-3.0, 3.0), y + rng.range(-3.0, 3.0)));
        }
        for (cap, join) in [(Linecap::Butt, Linejoin::Miter), (Linecap::Round, Linejoin::Round)] {
            let prims = build(&points, false, style(cap, join, 0.8));
            assert!(prims.grid.is_some(), "dense path must get a grid");
            let probes = samples(&mut rng, &points, 20_000, 2.0);
            assert_equal_everywhere(&prims, &probes);
        }
    }

    /// Long, wide, and closed strokes: whether or not a grid is built, the
    /// answer must match the linear scan at every sample.
    #[test]
    fn grid_contains_equals_linear_scan_on_long_strokes() {
        let mut rng = Rng(0x0123_4567_89AB_CDEF);
        let mut points = vec![(0.0, 0.0)];
        for _ in 0..500 {
            let (x, y) = *points.last().unwrap();
            points.push((x + rng.range(-40.0, 40.0), y + rng.range(-40.0, 40.0)));
        }
        let cases = [
            (false, Linecap::Butt, Linejoin::Miter, 0.8),
            (true, Linecap::Square, Linejoin::Bevel, 2.5),
            (false, Linecap::Round, Linejoin::Round, 6.0),
        ];
        for (closed, cap, join, width) in cases {
            let prims = build(&points, closed, style(cap, join, width));
            let probes = samples(&mut rng, &points, 8_000, 2.0 * width);
            assert_equal_everywhere(&prims, &probes);
        }
    }
}
