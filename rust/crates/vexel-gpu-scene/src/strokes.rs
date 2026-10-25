//! Stroke binning and primitive encoding (layout version 8). A stroke is the
//! exact CPU primitive set (segments with caps, precomputed joins) evaluated
//! per sample, so this module only decides which primitives each 16x16 tile
//! needs and writes them in the tile's local frame (see `layout.rs`): `f64`
//! arithmetic against the tile origin's stroke-local position, narrowed to
//! `f32` only after the subtraction.

use crate::error::UnsupportedReason as Reason;
use crate::input::{GpuCap, GpuJoinKind, GpuStroke, GpuStrokeJoin, GpuStrokeSegment};
use crate::layout::{
    GpuStrokePrim, JOIN_MITER, JOIN_ROUND, JOIN_TRIANGLE, MAX_STROKE_PRIMS, MAX_TILE_PRIMS,
    PRIM_JOIN, PRIM_SEG, SEG_DOT, SEG_DOT_SQUARE, SEG_ROUND_A, SEG_ROUND_B, TILE_SIZE,
};

const SQRT_2: f64 = std::f64::consts::SQRT_2;
/// Pixels of slack on tile borders when binning (conservative).
const SLACK: f64 = 1e-6;
/// Largest coordinate accepted in stroke-local space (keeps `f32` finite).
const MAX_LOCAL: f64 = 1e15;

/// `[a, b, c, d, e, f]` applied as `x' = a*x + c*y + e`, `y' = b*x + d*y + f`.
pub(crate) type Mat = [f64; 6];

pub(crate) fn apply(t: &Mat, x: f64, y: f64) -> [f64; 2] {
    [t[0] * x + t[2] * y + t[4], t[1] * x + t[3] * y + t[5]]
}

pub(crate) fn invert(t: &Mat) -> Option<Mat> {
    if t.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let det = t[0] * t[3] - t[1] * t[2];
    if !det.is_finite() || det == 0.0 {
        return None;
    }
    let (a, b, c, d) = (t[3] / det, -t[1] / det, -t[2] / det, t[0] / det);
    let inv = [
        a,
        b,
        c,
        d,
        -(a * t[4] + c * t[5]),
        -(b * t[4] + d * t[5]),
    ];
    inv.iter().all(|v| v.is_finite()).then_some(inv)
}

fn square(c: GpuCap) -> bool {
    c == GpuCap::Square
}

/// Candidate box of a segment in local space (`export.rs` `cull_box`).
fn segment_box(s: &GpuStrokeSegment, r: f64) -> [f64; 4] {
    let margin = if square(s.start_cap) || square(s.end_cap) {
        r * SQRT_2
    } else {
        r
    };
    [
        s.a[0].min(s.b[0]) - margin,
        s.a[1].min(s.b[1]) - margin,
        s.a[0].max(s.b[0]) + margin,
        s.a[1].max(s.b[1]) + margin,
    ]
}

/// Candidate box of a join (`None` for joins that never hit).
fn join_box(j: &GpuStrokeJoin, reach: f64) -> Option<[f64; 4]> {
    if j.kind == GpuJoinKind::Never {
        return None;
    }
    Some(j.tight.unwrap_or_else(|| {
        let pad = reach + (reach + j.vertex[0].abs().max(j.vertex[1].abs())) * 1e-9;
        [
            j.vertex[0] - pad,
            j.vertex[1] - pad,
            j.vertex[0] + pad,
            j.vertex[1] + pad,
        ]
    }))
}

/// Checks the numbers of a stroke are usable; `false` means the scene is invalid.
pub(crate) fn stroke_valid(s: &GpuStroke) -> bool {
    let ok = |v: f64| v.is_finite() && v.abs() <= MAX_LOCAL;
    s.radius.is_finite()
        && s.radius > 0.0
        && s.radius <= MAX_LOCAL
        && ok(s.join_reach)
        && s.transform.iter().all(|v| v.is_finite())
        && s.segments
            .iter()
            .all(|g| g.a.into_iter().chain(g.b).all(ok))
        && s.joins.iter().all(|j| {
            j.vertex
                .into_iter()
                .chain(j.p1)
                .chain(j.miter)
                .chain(j.p2)
                .chain(j.tight.unwrap_or([0.0; 4]))
                .all(ok)
        })
}

/// Calls `f(prim_index, local_box)` for every primitive that can hit. Segment
/// indices come first, then joins at `segments.len() + join_index`.
fn for_each_box(s: &GpuStroke, mut f: impl FnMut(u32, [f64; 4])) {
    for (i, g) in s.segments.iter().enumerate() {
        f(i as u32, segment_box(g, s.radius));
    }
    let base = s.segments.len() as u32;
    for (i, j) in s.joins.iter().enumerate() {
        if let Some(b) = join_box(j, s.join_reach) {
            f(base + i as u32, b);
        }
    }
}

fn device_box(t: &Mat, b: [f64; 4]) -> [f64; 4] {
    let mut out = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for (x, y) in [(b[0], b[1]), (b[2], b[1]), (b[0], b[3]), (b[2], b[3])] {
        let p = apply(t, x, y);
        out = [out[0].min(p[0]), out[1].min(p[1]), out[2].max(p[0]), out[3].max(p[1])];
    }
    out
}

/// Inclusive tile span of `[lo, hi]` pixels on an axis of `count` tiles.
fn span(lo: f64, hi: f64, count: u32) -> Option<(u32, u32)> {
    let inv = 1.0 / f64::from(TILE_SIZE);
    let first = ((lo - SLACK) * inv).floor().max(0.0);
    let last = ((hi + SLACK) * inv).floor().min(f64::from(count) - 1.0);
    (first <= last).then_some((first as u32, last as u32))
}

/// Result of [`StrokeBinner::prepare`].
pub(crate) struct Prepared {
    /// Device to stroke-local.
    pub inv: Mat,
    /// `[a, b, c, d]` of `inv` as `f32`.
    pub m: [f32; 4],
    /// Device-space bounds `[x0, y0, x1, y1]` of all primitives.
    pub bounds: [f64; 4],
}

/// Cheap upper bound used by classification: (primitive count, tile visits
/// by the primitives' device boxes) of `s` on a `tiles_x * tiles_y` grid.
pub(crate) fn estimate(s: &GpuStroke, tiles_x: u32, tiles_y: u32) -> Option<(u64, u64, [f64; 4])> {
    let mut prims = 0u64;
    let mut work = 0u64;
    let mut bounds = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for_each_box(s, |_, b| {
        prims += 1;
        let d = device_box(&s.transform, b);
        bounds = [
            bounds[0].min(d[0]),
            bounds[1].min(d[1]),
            bounds[2].max(d[2]),
            bounds[3].max(d[3]),
        ];
        if let (Some((x0, x1)), Some((y0, y1))) =
            (span(d[0], d[2], tiles_x), span(d[1], d[3], tiles_y))
        {
            work += u64::from(x1 - x0 + 1) * u64::from(y1 - y0 + 1);
        }
    });
    (prims > 0).then_some((prims, work, bounds))
}

/// Scratch for binning one stroke at a time.
#[derive(Default)]
pub(crate) struct StrokeBinner {
    /// (tile index, primitive index), sorted.
    pairs: Vec<(u32, u32)>,
}

impl StrokeBinner {
    /// Bins the primitives of `s`; `None` when nothing can be drawn (singular
    /// transform, no primitive touches the canvas).
    pub fn prepare(
        &mut self,
        s: &GpuStroke,
        tiles_x: u32,
        tiles_y: u32,
    ) -> Result<Option<Prepared>, Reason> {
        self.pairs.clear();
        let Some(inv) = invert(&s.transform) else {
            return Ok(None);
        };
        let rotated = inv[1] != 0.0 || inv[2] != 0.0;
        let t = &s.transform;
        let mut bounds = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        let mut error = None;
        let pairs = &mut self.pairs;
        for_each_box(s, |index, b| {
            let d = device_box(t, b);
            let (Some((x0, x1)), Some((y0, y1))) =
                (span(d[0], d[2], tiles_x), span(d[1], d[3], tiles_y))
            else {
                return;
            };
            bounds = [
                bounds[0].min(d[0]),
                bounds[1].min(d[1]),
                bounds[2].max(d[2]),
                bounds[3].max(d[3]),
            ];
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    if rotated {
                        let (ox, oy) = (
                            f64::from(tx * TILE_SIZE) - SLACK,
                            f64::from(ty * TILE_SIZE) - SLACK,
                        );
                        let size = f64::from(TILE_SIZE) + 2.0 * SLACK;
                        let tb = device_box_inv(&inv, [ox, oy, ox + size, oy + size]);
                        if tb[2] < b[0] || tb[0] > b[2] || tb[3] < b[1] || tb[1] > b[3] {
                            continue;
                        }
                    }
                    if pairs.len() >= MAX_STROKE_PRIMS as usize {
                        error = Some(Reason::TooManyStrokePrims);
                        return;
                    }
                    pairs.push((ty * tiles_x + tx, index));
                }
            }
        });
        if let Some(e) = error {
            return Err(e);
        }
        if self.pairs.is_empty() {
            return Ok(None);
        }
        self.pairs.sort_unstable();
        Ok(Some(Prepared {
            inv,
            m: [inv[0] as f32, inv[1] as f32, inv[2] as f32, inv[3] as f32],
            bounds,
        }))
    }

    /// Writes the primitives of every tile in ascending tile order into
    /// `prims` and calls `sink(tile, start, count)` per tile.
    pub fn emit(
        &self,
        s: &GpuStroke,
        p: &Prepared,
        tiles_x: u32,
        prims: &mut Vec<GpuStrokePrim>,
        mut sink: impl FnMut(u32, u32, u32) -> Result<(), Reason>,
    ) -> Result<(), Reason> {
        let nseg = s.segments.len() as u32;
        let mut at = 0;
        while at < self.pairs.len() {
            let tile = self.pairs[at].0;
            let end = at + self.pairs[at..].iter().take_while(|q| q.0 == tile).count();
            let count = end - at;
            if count > MAX_TILE_PRIMS as usize
                || prims.len() + count > MAX_STROKE_PRIMS as usize
            {
                return Err(Reason::TooManyStrokePrims);
            }
            let (tx, ty) = (tile % tiles_x, tile / tiles_x);
            let origin = apply(&p.inv, f64::from(tx * TILE_SIZE), f64::from(ty * TILE_SIZE));
            let half = f64::from(TILE_SIZE) / 2.0;
            let centre = [
                p.inv[0] * half + p.inv[2] * half,
                p.inv[1] * half + p.inv[3] * half,
            ];
            let start = prims.len() as u32;
            for &(_, index) in &self.pairs[at..end] {
                let prim = if index < nseg {
                    segment_prim(&s.segments[index as usize], s.radius, origin, centre)
                } else {
                    join_prim(&s.joins[(index - nseg) as usize], s.join_reach, origin)
                };
                prims.push(prim?);
            }
            sink(tile, start, count as u32)?;
            at = end;
        }
        Ok(())
    }
}

/// Bounding box of the device-space box `b` mapped through `inv`.
fn device_box_inv(inv: &Mat, b: [f64; 4]) -> [f64; 4] {
    device_box(inv, b)
}

fn f32_checked(values: &[f64]) -> Result<[f32; 15], Reason> {
    let mut w = [0f32; 15];
    for (slot, v) in w.iter_mut().zip(values) {
        let n = *v as f32;
        if !n.is_finite() {
            return Err(Reason::InvalidScene);
        }
        *slot = n;
    }
    Ok(w)
}

fn segment_prim(
    s: &GpuStrokeSegment,
    r: f64,
    origin: [f64; 2],
    centre: [f64; 2],
) -> Result<GpuStrokePrim, Reason> {
    let a = [s.a[0] - origin[0], s.a[1] - origin[1]];
    let b = [s.b[0] - origin[0], s.b[1] - origin[1]];
    let (dx, dy) = (s.b[0] - s.a[0], s.b[1] - s.a[1]);
    let len = dx.hypot(dy);
    let round_a = s.start_cap == GpuCap::Round;
    let round_b = s.end_cap == GpuCap::Round;
    if len == 0.0 {
        let flags = if round_a || round_b {
            SEG_DOT | SEG_ROUND_A
        } else if square(s.start_cap) || square(s.end_cap) {
            SEG_DOT | SEG_DOT_SQUARE
        } else {
            SEG_DOT
        };
        let w = f32_checked(&[a[0], a[1], 0.0, 0.0, 0.0, 0.0, a[0], a[1], b[0], b[1]])?;
        return Ok(GpuStrokePrim {
            kind: PRIM_SEG | flags,
            w,
        });
    }
    let (ux, uy) = (dx / len, dy / len);
    let t = ((centre[0] - a[0]) * ux + (centre[1] - a[1]) * uy).clamp(0.0, len);
    let (fx, fy) = (a[0] + t * ux, a[1] + t * uy);
    let low = if square(s.start_cap) { -r } else { 0.0 };
    let high = len + if square(s.end_cap) { r } else { 0.0 };
    let flags = u32::from(round_a) * SEG_ROUND_A + u32::from(round_b) * SEG_ROUND_B;
    let w = f32_checked(&[
        fx,
        fy,
        ux,
        uy,
        low - t,
        high - t,
        a[0],
        a[1],
        b[0],
        b[1],
    ])?;
    Ok(GpuStrokePrim {
        kind: PRIM_SEG | flags,
        w,
    })
}

fn join_prim(j: &GpuStrokeJoin, reach: f64, origin: [f64; 2]) -> Result<GpuStrokePrim, Reason> {
    let rel = |p: [f64; 2]| [p[0] - origin[0], p[1] - origin[1]];
    let kind = match j.kind {
        GpuJoinKind::Never => return Ok(GpuStrokePrim::default()),
        GpuJoinKind::Round => JOIN_ROUND,
        GpuJoinKind::Triangle => JOIN_TRIANGLE,
        GpuJoinKind::Miter => JOIN_MITER,
    };
    let (v, p1, m, p2) = (rel(j.vertex), rel(j.p1), rel(j.miter), rel(j.p2));
    let hit = match j.tight {
        Some(t) => [t[0] - origin[0], t[1] - origin[1], t[2] - origin[0], t[3] - origin[1]],
        None => [v[0] - reach, v[1] - reach, v[0] + reach, v[1] + reach],
    };
    let w = f32_checked(&[
        v[0], v[1], p1[0], p1[1], m[0], m[1], p2[0], p2[1], hit[0], hit[1], hit[2], hit[3],
    ])?;
    Ok(GpuStrokePrim {
        kind: PRIM_JOIN | kind,
        w,
    })
}
