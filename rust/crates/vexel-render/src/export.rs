//! Read-only exporters of the stroke and fill data the CPU rasterizer builds,
//! for the GPU display rasterizer (ADR-0035). Nothing here is used by the CPU
//! render paths; the exporters call the same builders (`JoinGeom::new`,
//! `dash::build_range`, `stroke::range_at`) with the same arithmetic and
//! return plain data. All numbers are `f64`, exactly as the CPU holds them;
//! the GPU narrows to `f32` after subtracting a tile-local origin.
//!
//! # Coordinate spaces
//!
//! * Device sample `(sx, sy)` of pixel `(x, y)`: `sx = x + SAMPLE_OFFSETS[i]`,
//!   `sy = y + SAMPLE_OFFSETS[j]`, 4x4 samples per pixel, row-major
//!   (`j * 4 + i`). A sample paints only when `ViewMap::sample_in_view`.
//! * User (document) space: `(ux, uy) = ViewMap::to_user(sx, sy)`. Fill edges
//!   and fill hit tests live here.
//! * Stroke local space: `local = affine_map_point(StrokeExport::inverse,
//!   (ux, uy))`. Stroke primitives live here. `affine_map_point` returns `None`
//!   (the sample is not stroked) when any coordinate leaves +-`MAX_COORDINATE`
//!   or turns non-finite.
//!
//! # Fill semantics (`fill.rs`, `FillExport`)
//!
//! For a sample `(px, py)` in user space, over the edges in export order:
//! ```text
//! winding = 0; parity = false
//! for e in edges:
//!     up   = e.y0 <= py && e.y1 >  py        // dir = +1
//!     down = e.y1 <= py && e.y0 >  py        // dir = -1
//!     if !up && !down: continue
//!     xcross = e.x0 + (py - e.y0) * (e.x1 - e.x0) / (e.y1 - e.y0)
//!     if xcross > px: winding += dir; parity = !parity
//! inside = evenodd ? parity : winding != 0
//! ```
//! Horizontal edges never count. Edge order does not change the result
//! (integer sums). The strict `xcross > px` makes a sample exactly on a
//! crossing count as right of it; in f32 this boundary is the only place a
//! GPU port may differ, and only for samples within rounding of an edge.
//!
//! # Stroke semantics (`stroke.rs`, `stroke_join.rs`, `StrokeExport`)
//!
//! A sample is stroked when ANY segment hits or ANY join hits (order is
//! irrelevant, the result is a logical or). Notation: `P = (px, py)` is the
//! local sample, `r = radius`, `reach = join_reach`.
//!
//! `segment_hit(s, P)`:
//! ```text
//! square = s.start_cap == Square || s.end_cap == Square
//! margin = square ? r * SQRT_2 : r                  // = s.margin
//! if !(px >= min(a.x,b.x) - margin && px <= max(a.x,b.x) + margin
//!      && py >= min(a.y,b.y) - margin && py <= max(a.y,b.y) + margin): return false
//! dx = b.x - a.x; dy = b.y - a.y; L = s.length     // hypot(dx, dy)
//! if L == 0:                                        // dot (zero-length segment)
//!     if start_cap == Round || end_cap == Round: return within(px-a.x, py-a.y, r)
//!     if start_cap == Square || end_cap == Square:
//!         return |px-a.x| <= r && |py-a.y| <= r
//!     return false
//! along  = ((px-a.x)*dx + (py-a.y)*dy) / L
//! across = |(px-a.x)*dy - (py-a.y)*dx| / L
//! low = 0; high = L
//! if start_cap == Square: low  -= r
//! if end_cap   == Square: high += r
//! if along >= low && along <= high && across <= r: return true
//! if start_cap == Round && within(px-a.x, py-a.y, r): return true
//! if end_cap   == Round && within(px-b.x, py-b.y, r): return true
//! return false
//! ```
//! `within(dx, dy, r)` is `hypot(dx, dy) <= r` (the CPU only skips `hypot`
//! when the squared distance is clearly on one side; the truth value is the
//! same, so MSL may use `dx*dx + dy*dy <= r*r` within f32 tolerance).
//! Butt caps add nothing beyond the body rectangle.
//!
//! `join_hit(j, P)`:
//! ```text
//! inside_box = j.tight is Some(b) ? (px >= b.x0 && px <= b.x1 && py >= b.y0 && py <= b.y1)
//!                                 : (|px - v.x| <= reach && |py - v.y| <= reach)
//! if !inside_box: return false
//! match j.kind:
//!   Never:    false
//!   Round:    within(px - v.x, py - v.y, r)
//!   Triangle: in_triangle(P, v, p1, p2)
//!   Miter:    in_triangle(P, v, p1, m) || in_triangle(P, v, m, p2)
//! ```
//! `in_triangle(P, a, b, c)` (boundary inclusive, orientation independent):
//! ```text
//! d1 = (px-b.x)*(a.y-b.y) - (a.x-b.x)*(py-b.y)
//! d2 = (px-c.x)*(b.y-c.y) - (b.x-c.x)*(py-c.y)
//! d3 = (px-a.x)*(c.y-a.y) - (c.x-a.x)*(py-a.y)
//! neg = d1 < 0 || d2 < 0 || d3 < 0
//! pos = d1 > 0 || d2 > 0 || d3 > 0
//! return !(neg && pos)
//! ```
//! The join kind, `p1`, `m`, `p2` and `tight` are precomputed by the CPU
//! (`JoinGeom::new`: `side` from the sign of the turn cross product, `p1`/`p2`
//! the offset points of the two edge normals, the miter apex clipped by
//! `ratio <= miter_limit`); the exporter hands them over verbatim, so the GPU
//! never recomputes them. `tight` is the padded hull of the join, absent for
//! nearly straight joins; `reach = r * miter_limit` for miter joins, else `r`.
//!
//! `cull_box` of every primitive is the CPU's candidate box (segments: body
//! box grown by `margin`; joins: `tight` or the reach box padded by
//! `(reach + max(|v.x|,|v.y|)) * 1e-9`). A sample inside no `cull_box` cannot
//! be hit; the boxes are for binning only and never change a result.

use crate::dash::{self, Budget, Pattern};
use crate::scan::SAMPLE_LOCATIONS;
use crate::stroke::{self, Join, Segment};
use crate::stroke_join::{JoinGeom, Shape};
use crate::{Linecap, Linejoin, RenderError, RenderShape};
use vexel_geometry::{Affine, Point, affine_map_point};

/// Sample offsets inside a pixel along each axis (4x4 grid, row-major).
pub const SAMPLE_OFFSETS: [f64; 4] = SAMPLE_LOCATIONS;

const SQRT_2: f64 = 1.414_213_562_373_095_1;

/// One filled edge in user space, as the scanner consumes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillEdgeExport {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    /// `+1` when `y0 < y1` (counts for `y0 <= py < y1`), `-1` when `y1 < y0`
    /// (counts for `y1 <= py < y0`), `0` for horizontal / degenerate edges
    /// (never count).
    pub direction: i8,
}

/// Fill edges and rule of one shape. `edges` keep the shape's own order.
#[derive(Debug, Clone, PartialEq)]
pub struct FillExport {
    pub edges: Vec<FillEdgeExport>,
    pub evenodd: bool,
}

/// Exports the fill of `shape`; `None` when the shape has no fill (`filled`
/// is false) or no edges. Colour/paint is not exported.
pub fn export_fill(shape: &RenderShape<'_>) -> Option<FillExport> {
    if !shape.filled || shape.edges.is_empty() {
        return None;
    }
    let edges = shape
        .edges
        .iter()
        .map(|e| FillEdgeExport {
            x0: e.x0,
            y0: e.y0,
            x1: e.x1,
            y1: e.y1,
            direction: if e.y0 < e.y1 {
                1
            } else if e.y1 < e.y0 {
                -1
            } else {
                0
            },
        })
        .collect();
    Some(FillExport { edges, evenodd: shape.evenodd })
}

/// One stroke segment in local space (`stroke::Segment` plus derived values).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentExport {
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub start_cap: Linecap,
    pub end_cap: Linecap,
    /// `(b.x - a.x, b.y - a.y)`.
    pub delta: [f64; 2],
    /// `hypot(delta)`, the exact value the hit test divides by.
    pub length: f64,
    /// Unit normal `(-dy, dx) / length`, `[0, 0]` for a zero-length segment.
    /// Informational: the exact test uses `delta` and `length` instead.
    pub normal: [f64; 2],
    /// Body box margin: `radius * sqrt(2)` with a square cap, else `radius`.
    pub margin: f64,
    /// `[x0, y0, x1, y1]` candidate box.
    pub cull_box: [f64; 4],
}

/// Kind of join geometry the CPU precomputed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    /// Degenerate or collinear: never hits.
    Never,
    /// Disc of `radius` around the vertex.
    Round,
    /// Triangle `(vertex, p1, p2)` (bevel, or miter beyond the limit).
    Triangle,
    /// Triangles `(vertex, p1, miter)` and `(vertex, miter, p2)`.
    Miter,
}

/// One join in local space with its exact precomputed hit geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoinExport {
    pub previous: [f64; 2],
    pub vertex: [f64; 2],
    pub next: [f64; 2],
    pub kind: JoinKind,
    /// Triangle corner `p1` (zeros for `Never` / `Round`).
    pub p1: [f64; 2],
    /// Miter apex (zeros unless `kind == Miter`).
    pub miter: [f64; 2],
    /// Triangle corner `p2` (zeros for `Never` / `Round`).
    pub p2: [f64; 2],
    /// Exact hit box `[x0, y0, x1, y1]` when present; otherwise the hit test
    /// uses `|p - vertex| <= join_reach` per axis.
    pub tight: Option<[f64; 4]>,
    /// `[x0, y0, x1, y1]` candidate box (`tight` or the padded reach box).
    pub cull_box: [f64; 4],
}

/// One stroked range after dashing: the primitives it owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeExport {
    pub first_segment: usize,
    pub segment_count: usize,
    pub first_join: usize,
    pub join_count: usize,
    /// The source range was closed (its seam joins).
    pub closed: bool,
}

/// Everything needed to evaluate the CPU stroke hit test of one shape.
/// Segments come first in CPU order, then joins in CPU order; `ranges` slice
/// both lists per subpath (empty subpaths contribute no range).
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeExport {
    /// Document (user) to stroke-local space; `affine_map_point` semantics.
    pub inverse: Affine,
    /// Local to document space (the shape transform).
    pub transform: Affine,
    pub radius: f64,
    pub join: Linejoin,
    pub miter_limit: f64,
    /// `radius * miter_limit` for miter joins, else `radius`.
    pub join_reach: f64,
    pub segments: Vec<SegmentExport>,
    pub joins: Vec<JoinExport>,
    pub ranges: Vec<RangeExport>,
    /// True when a dash pattern was applied (segments are the dashes).
    pub dashed: bool,
}

fn pt(p: Point) -> [f64; 2] {
    [p.x, p.y]
}

/// Exports the stroke primitives of `shape` exactly as `StrokePrims::build`
/// would build them for this shape. `dashes` is the dash array (empty: solid);
/// the offset is `shape.stroke.dash_offset`. Returns `Ok(None)` when the shape
/// paints no stroke (no style, zero width, invisible, no edges, singular
/// transform) and `Err` where the CPU build errors (limits, invalid geometry).
/// `gradient` is true when the stroke colour is a gradient (alpha is then
/// ignored for visibility, as in the renderer).
pub fn export_stroke(
    shape: &RenderShape<'_>,
    dashes: &[f64],
    gradient: bool,
) -> Result<Option<StrokeExport>, RenderError> {
    let Some(style) =
        shape.stroke.filter(|s| stroke::is_painted(s, gradient) && !shape.edges.is_empty())
    else {
        return Ok(None);
    };
    let Some(inverse) = stroke::invert(shape.transform) else {
        return Ok(None);
    };
    let pattern = Pattern::new(dashes, style.dash_offset)?;
    let mut budget = Budget::default();
    // Same body as `StrokePrims::build`, recording range boundaries.
    let mut local: Vec<(Point, Point)> = Vec::new();
    stroke::reserve(&mut local, shape.edges.len())?;
    for edge in shape.edges {
        let a = affine_map_point(inverse, Point { x: edge.x0, y: edge.y0 });
        let b = affine_map_point(inverse, Point { x: edge.x1, y: edge.y1 });
        match (a, b) {
            (Some(a), Some(b)) => local.push((a, b)),
            _ => return Err(RenderError::InvalidGeometry),
        }
    }
    let mut segments: Vec<Segment> = Vec::new();
    let mut joins: Vec<Join> = Vec::new();
    let mut ranges = Vec::new();
    for range_index in 0..stroke::range_count(shape) {
        let range = stroke::range_at(shape, range_index);
        let count = range.stroked_count();
        if count == 0 {
            continue;
        }
        let (segment_start, join_start) = (segments.len(), joins.len());
        if let Some(pattern) = pattern.as_ref() {
            let edges = &local[range.first..range.first + range.count];
            let seam = range.closed && dash::joins_seam(pattern, edges);
            dash::build_range(
                &mut segments,
                &mut joins,
                &mut budget,
                pattern,
                style.cap,
                &edges[..count],
                seam,
            )?;
        } else {
            let (segment_count, join_count) = range.primitive_counts();
            stroke::reserve(&mut segments, segment_count)?;
            stroke::reserve(&mut joins, join_count)?;
            for index in 0..count {
                let cap = if !range.closed && (index == 0 || index + 1 == count) {
                    style.cap
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
        ranges.push(RangeExport {
            first_segment: segment_start,
            segment_count: segments.len() - segment_start,
            first_join: join_start,
            join_count: joins.len() - join_start,
            closed: range.closed,
        });
    }
    let radius = style.width * 0.5;
    let join_reach = if style.join == Linejoin::Miter { radius * style.miter_limit } else { radius };
    let segments_out = segments
        .iter()
        .map(|s| {
            let square = s.start_cap == Linecap::Square || s.end_cap == Linecap::Square;
            let margin = if square { radius * SQRT_2 } else { radius };
            let (dx, dy) = (s.b.x - s.a.x, s.b.y - s.a.y);
            let length = dx.hypot(dy);
            let normal = if length > 0.0 { [-dy / length, dx / length] } else { [0.0, 0.0] };
            SegmentExport {
                a: pt(s.a),
                b: pt(s.b),
                start_cap: s.start_cap,
                end_cap: s.end_cap,
                delta: [dx, dy],
                length,
                normal,
                margin,
                cull_box: [
                    s.a.x.min(s.b.x) - margin,
                    s.a.y.min(s.b.y) - margin,
                    s.a.x.max(s.b.x) + margin,
                    s.a.y.max(s.b.y) + margin,
                ],
            }
        })
        .collect();
    let joins_out = joins
        .iter()
        .map(|j| {
            let geom = JoinGeom::new(j, radius, style.join, style.miter_limit);
            let zero = [0.0; 2];
            let (kind, p1, miter, p2) = match geom.shape {
                Shape::Never => (JoinKind::Never, zero, zero, zero),
                Shape::Round => (JoinKind::Round, zero, zero, zero),
                Shape::Triangle(p1, p2) => (JoinKind::Triangle, pt(p1), zero, pt(p2)),
                Shape::Miter(p1, m, p2) => (JoinKind::Miter, pt(p1), pt(m), pt(p2)),
            };
            let (vx, vy) = (j.vertex.x, j.vertex.y);
            let cull_box = geom.tight.unwrap_or_else(|| {
                let pad = join_reach + (join_reach + vx.abs().max(vy.abs())) * 1e-9;
                [vx - pad, vy - pad, vx + pad, vy + pad]
            });
            JoinExport {
                previous: pt(j.previous),
                vertex: pt(j.vertex),
                next: pt(j.next),
                kind,
                p1,
                miter,
                p2,
                tight: geom.tight,
                cull_box,
            }
        })
        .collect();
    Ok(Some(StrokeExport {
        inverse,
        transform: shape.transform,
        radius,
        join: style.join,
        miter_limit: style.miter_limit,
        join_reach,
        segments: segments_out,
        joins: joins_out,
        ranges,
        dashed: pattern.is_some(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite::SAMPLE_COUNT;
    use crate::{DashedShape, FlatEdge, StrokeStyle, ViewBox, ViewMap, render_dashed_shapes};
    use std::ops::Range;
    use vexel_raster::Tile;

    /// xorshift64*: deterministic without a dev-dependency.
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

        fn pick(&mut self, n: usize) -> usize {
            ((self.unit() * n as f64) as usize).min(n - 1)
        }
    }

    // ---- reference hit tests over exported data only ----

    fn within(dx: f64, dy: f64, r: f64) -> bool {
        dx.hypot(dy) <= r
    }

    fn in_triangle(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
        let d1 = (p[0] - b[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (p[1] - b[1]);
        let d2 = (p[0] - c[0]) * (b[1] - c[1]) - (b[0] - c[0]) * (p[1] - c[1]);
        let d3 = (p[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (p[1] - a[1]);
        let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(neg && pos)
    }

    fn segment_hit(s: &SegmentExport, r: f64, px: f64, py: f64) -> bool {
        let (a, b) = (s.a, s.b);
        if !(px >= a[0].min(b[0]) - s.margin
            && px <= a[0].max(b[0]) + s.margin
            && py >= a[1].min(b[1]) - s.margin
            && py <= a[1].max(b[1]) + s.margin)
        {
            return false;
        }
        let (dx, dy, l) = (s.delta[0], s.delta[1], s.length);
        if l == 0.0 {
            if s.start_cap == Linecap::Round || s.end_cap == Linecap::Round {
                return within(px - a[0], py - a[1], r);
            }
            if s.start_cap == Linecap::Square || s.end_cap == Linecap::Square {
                return (px - a[0]).abs() <= r && (py - a[1]).abs() <= r;
            }
            return false;
        }
        let along = ((px - a[0]) * dx + (py - a[1]) * dy) / l;
        let across = ((px - a[0]) * dy - (py - a[1]) * dx).abs() / l;
        let (mut low, mut high) = (0.0, l);
        if s.start_cap == Linecap::Square {
            low -= r;
        }
        if s.end_cap == Linecap::Square {
            high += r;
        }
        (along >= low && along <= high && across <= r)
            || (s.start_cap == Linecap::Round && within(px - a[0], py - a[1], r))
            || (s.end_cap == Linecap::Round && within(px - b[0], py - b[1], r))
    }

    fn join_hit(j: &JoinExport, reach: f64, r: f64, px: f64, py: f64) -> bool {
        let inside = match j.tight {
            Some(b) => px >= b[0] && px <= b[2] && py >= b[1] && py <= b[3],
            None => (px - j.vertex[0]).abs() <= reach && (py - j.vertex[1]).abs() <= reach,
        };
        if !inside {
            return false;
        }
        let p = [px, py];
        match j.kind {
            JoinKind::Never => false,
            JoinKind::Round => within(px - j.vertex[0], py - j.vertex[1], r),
            JoinKind::Triangle => in_triangle(p, j.vertex, j.p1, j.p2),
            JoinKind::Miter => {
                in_triangle(p, j.vertex, j.p1, j.miter) || in_triangle(p, j.vertex, j.miter, j.p2)
            }
        }
    }

    fn stroke_hit(e: &StrokeExport, px: f64, py: f64) -> bool {
        e.segments.iter().any(|s| segment_hit(s, e.radius, px, py))
            || e.joins.iter().any(|j| join_hit(j, e.join_reach, e.radius, px, py))
    }

    fn fill_inside(e: &FillExport, px: f64, py: f64) -> bool {
        let (mut winding, mut parity) = (0i32, false);
        for edge in &e.edges {
            let up = edge.y0 <= py && edge.y1 > py;
            let down = edge.y1 <= py && edge.y0 > py;
            if !up && !down {
                continue;
            }
            let dir = if up { 1 } else { -1 };
            assert_eq!(i32::from(edge.direction), dir);
            let xcross = edge.x0 + (py - edge.y0) * (edge.x1 - edge.x0) / (edge.y1 - edge.y0);
            if xcross > px {
                winding += dir;
                parity = !parity;
            }
        }
        if e.evenodd { parity } else { winding != 0 }
    }

    // ---- harness ----

    const SIZE: u32 = 64;

    fn view() -> ViewMap {
        let side = f64::from(SIZE);
        ViewMap::from_viewbox(ViewBox { x: 0.0, y: 0.0, width: side, height: side }, SIZE, SIZE, None)
            .unwrap()
    }

    fn alpha_of(count: u32) -> u8 {
        ((255 * count + SAMPLE_COUNT as u32 / 2) / SAMPLE_COUNT as u32) as u8
    }

    /// Renders `shape` alone (white, opaque) and returns the alpha plane.
    fn render_alpha(shape: RenderShape<'_>, dashes: &[f64]) -> Vec<u8> {
        let mut tile =
            Tile::from_owned_rgba(SIZE, SIZE, vec![0; (SIZE * SIZE * 4) as usize]).unwrap();
        render_dashed_shapes(&[DashedShape { shape, dashes }], view(), &mut tile, &|| false)
            .unwrap();
        tile.rgba().chunks_exact(4).map(|p| p[3]).collect()
    }

    /// Per-pixel hit counts from the exported data.
    fn reference_counts(
        stroke: Option<&StrokeExport>,
        fill: Option<&FillExport>,
    ) -> Vec<u32> {
        let view = view();
        let mut counts = Vec::new();
        for y in 0..SIZE {
            for x in 0..SIZE {
                let mut count = 0;
                for oy in SAMPLE_OFFSETS {
                    for ox in SAMPLE_OFFSETS {
                        let (sx, sy) = (f64::from(x) + ox, f64::from(y) + oy);
                        if !view.sample_in_view(sx, sy) {
                            continue;
                        }
                        let (ux, uy) = view.to_user(sx, sy);
                        let mut hit = fill.is_some_and(|f| fill_inside(f, ux, uy));
                        if let Some(s) = stroke {
                            hit |= affine_map_point(s.inverse, Point { x: ux, y: uy })
                                .is_some_and(|p| stroke_hit(s, p.x, p.y));
                        }
                        count += u32::from(hit);
                    }
                }
                counts.push(count);
            }
        }
        counts
    }

    fn style(rng: &mut Rng) -> StrokeStyle {
        let caps = [Linecap::Butt, Linecap::Round, Linecap::Square];
        let joins = [Linejoin::Miter, Linejoin::Round, Linejoin::Bevel];
        StrokeStyle {
            rgba: [255, 255, 255, 255],
            width: rng.range(0.4, 9.0),
            miter_limit: rng.range(1.0, 6.0),
            dash_offset: rng.range(-4.0, 12.0),
            cap: caps[rng.pick(3)],
            join: joins[rng.pick(3)],
            crisp_edges: false,
        }
    }

    fn transform(rng: &mut Rng) -> Affine {
        match rng.pick(3) {
            0 => vexel_geometry::affine_identity(),
            1 => Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 },
            _ => Affine {
                a: rng.range(0.7, 1.3),
                b: rng.range(-0.4, 0.4),
                c: rng.range(-0.4, 0.4),
                d: rng.range(0.7, 1.3),
                e: rng.range(-3.0, 3.0),
                f: rng.range(-3.0, 3.0),
            },
        }
    }

    fn polyline(rng: &mut Rng, points: usize, closed: bool) -> Vec<FlatEdge> {
        let pts: Vec<(f64, f64)> =
            (0..points).map(|_| (rng.range(6.0, 58.0), rng.range(6.0, 58.0))).collect();
        let mut edges: Vec<FlatEdge> = pts
            .windows(2)
            .map(|w| FlatEdge { x0: w[0].0, y0: w[0].1, x1: w[1].0, y1: w[1].1 })
            .collect();
        if closed {
            let (f, l) = (pts[0], pts[points - 1]);
            edges.push(FlatEdge { x0: l.0, y0: l.1, x1: f.0, y1: f.1 });
        }
        edges
    }

    fn check(shape: RenderShape<'_>, dashes: &[f64], label: &str) {
        let stroke = export_stroke(&shape, dashes, false).unwrap();
        let fill = export_fill(&shape);
        let expected = reference_counts(stroke.as_ref(), fill.as_ref());
        let actual = render_alpha(shape, dashes);
        for (i, (count, alpha)) in expected.iter().zip(&actual).enumerate() {
            assert_eq!(
                alpha_of(*count),
                *alpha,
                "{label}: pixel ({}, {}) count {count}",
                i as u32 % SIZE,
                i as u32 / SIZE
            );
        }
    }

    fn base_shape<'a>(edges: &'a [FlatEdge], stroke: StrokeStyle, tf: Affine) -> RenderShape<'a> {
        RenderShape {
            edges,
            subpaths: &[],
            subpath_closed: &[],
            stroke_closed: false,
            evenodd: false,
            filled: false,
            fill_rgba: [0; 4],
            stroke: Some(stroke),
            transform: tf,
            fill_gradient: None,
            stroke_gradient: None,
        }
    }

    #[test]
    fn solid_strokes_match_renderer() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for case in 0..60 {
            let closed = case % 2 == 0;
            let points = 2 + rng.pick(6);
            let edges = polyline(&mut rng, points, false);
            let mut shape = base_shape(&edges, style(&mut rng), transform(&mut rng));
            shape.stroke_closed = closed;
            // Edges are in document space; the transform only moves the stroke space.
            check(shape, &[], &format!("solid case {case}"));
        }
    }

    #[test]
    fn dashed_strokes_match_renderer() {
        let mut rng = Rng(0x0123_4567_89AB_CDEF);
        for case in 0..40 {
            let points = 2 + rng.pick(5);
            let edges = polyline(&mut rng, points, case % 3 == 0);
            let mut shape = base_shape(&edges, style(&mut rng), transform(&mut rng));
            shape.stroke_closed = case % 3 == 0;
            let dashes: Vec<f64> = match case % 4 {
                0 => vec![rng.range(1.0, 6.0), rng.range(1.0, 6.0)],
                1 => vec![rng.range(2.0, 8.0)],
                2 => vec![0.0, rng.range(2.0, 5.0)],
                _ => vec![rng.range(1.0, 4.0), rng.range(1.0, 3.0), rng.range(0.5, 2.0)],
            };
            let export = export_stroke(&shape, &dashes, false).unwrap().unwrap();
            assert!(export.dashed);
            let total: usize = export.ranges.iter().map(|r| r.segment_count).sum();
            assert_eq!(total, export.segments.len());
            check(shape, &dashes, &format!("dashed case {case}"));
        }
    }

    #[test]
    fn subpath_strokes_and_fills_match_renderer() {
        let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
        for case in 0..30 {
            let mut edges = Vec::new();
            let mut subpaths: Vec<Range<usize>> = Vec::new();
            let mut closed = Vec::new();
            for _ in 0..1 + rng.pick(3) {
                let start = edges.len();
                let is_closed = rng.pick(2) == 0;
                // Open subpaths carry the implicit fill-closing edge, as the flattener emits.
                let n = 3 + rng.pick(4);
                edges.extend(polyline(&mut rng, n, true));
                subpaths.push(start..edges.len());
                closed.push(is_closed);
            }
            let mut shape = base_shape(&edges, style(&mut rng), transform(&mut rng));
            shape.subpaths = &subpaths;
            shape.subpath_closed = &closed;
            shape.filled = case % 2 == 0;
            shape.evenodd = case % 4 == 0;
            shape.fill_rgba = [255, 255, 255, 255];
            let dashes: &[f64] = if case % 3 == 0 { &[3.0, 2.0] } else { &[] };
            check(shape, dashes, &format!("subpath case {case}"));
        }
    }

    #[test]
    fn invisible_or_edgeless_exports_nothing() {
        let edges = [FlatEdge { x0: 1.0, y0: 1.0, x1: 9.0, y1: 9.0 }];
        let mut rng = Rng(7);
        let mut st = style(&mut rng);
        st.width = 0.0;
        let shape = base_shape(&edges, st, vexel_geometry::affine_identity());
        assert!(export_stroke(&shape, &[], false).unwrap().is_none());
        assert!(export_fill(&shape).is_none());
        let none = base_shape(&[], style(&mut rng), vexel_geometry::affine_identity());
        assert!(export_stroke(&none, &[], false).unwrap().is_none());
    }
}
