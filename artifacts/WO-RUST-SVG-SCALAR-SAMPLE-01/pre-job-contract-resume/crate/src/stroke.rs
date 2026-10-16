use crate::*;
use vexel_geometry::{Edge, Point, affine_map_point};
use vexel_scene::{Linecap, Linejoin, SceneView};

pub(super) fn segment_caps(
    a: Point,
    b: Point,
    px: f64,
    py: f64,
    r: f64,
    sc: Linecap,
    ec: Linecap,
) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = dx.hypot(dy);
    if len == 0.0 {
        return match (sc, ec) {
            (Linecap::Round, _) | (_, Linecap::Round) => (px - a.x).hypot(py - a.y) <= r,
            (Linecap::Square, _) | (_, Linecap::Square) => {
                (px - a.x).abs() <= r && (py - a.y).abs() <= r
            }
            _ => false,
        };
    }
    let along = ((px - a.x) * dx + (py - a.y) * dy) / len;
    let across = ((px - a.x) * dy - (py - a.y) * dx).abs() / len;
    let low = if sc == Linecap::Square { -r } else { 0.0 };
    let high = if ec == Linecap::Square { len + r } else { len };
    (along >= low && along <= high && across <= r)
        || (sc == Linecap::Round && (px - a.x).hypot(py - a.y) <= r)
        || (ec == Linecap::Round && (px - b.x).hypot(py - b.y) <= r)
}
fn triangle(px: f64, py: f64, a: Point, b: Point, c: Point) -> bool {
    let d1 = (px - b.x) * (a.y - b.y) - (a.x - b.x) * (py - b.y);
    let d2 = (px - c.x) * (b.y - c.y) - (b.x - c.x) * (py - c.y);
    let d3 = (px - a.x) * (c.y - a.y) - (c.x - a.x) * (py - a.y);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}
pub(super) fn join_hit(
    prev: Point,
    v: Point,
    next: Point,
    px: f64,
    py: f64,
    r: f64,
    join: Linejoin,
    miter: f64,
) -> bool {
    if join == Linejoin::Round {
        return (px - v.x).hypot(py - v.y) <= r;
    }
    let ax = v.x - prev.x;
    let ay = v.y - prev.y;
    let bx = next.x - v.x;
    let by = next.y - v.y;
    let al = ax.hypot(ay);
    let bl = bx.hypot(by);
    if al == 0.0 || bl == 0.0 {
        return false;
    }
    let den = ax * by - ay * bx;
    if den.abs() < 1e-12 {
        return false;
    }
    let side = if den > 0.0 { -1.0 } else { 1.0 };
    let p1 = Point {
        x: v.x + side * (-ay / al) * r,
        y: v.y + side * (ax / al) * r,
    };
    let p2 = Point {
        x: v.x + side * (-by / bl) * r,
        y: v.y + side * (bx / bl) * r,
    };
    if join == Linejoin::Bevel {
        return triangle(px, py, v, p1, p2);
    }
    let qx = p2.x - p1.x;
    let qy = p2.y - p1.y;
    let t = (qx * by - qy * bx) / den;
    let mt = Point {
        x: p1.x + t * ax,
        y: p1.y + t * ay,
    };
    let ratio = (mt.x - v.x).hypot(mt.y - v.y) / r;
    if !ratio.is_finite() || ratio > miter {
        return triangle(px, py, v, p1, p2);
    }
    triangle(px, py, v, p1, mt) || triangle(px, py, v, mt, p2)
}
fn range_contains(
    view: SceneView<'_>,
    shape: &vexel_scene::Shape,
    first: usize,
    count: usize,
    closed: bool,
    implicit: bool,
    seam: bool,
    edges: &[Edge],
    px: f64,
    py: f64,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let s = &shape.stroke;
    if s.rgba[3] == 0 || s.width <= 0.0 || count == 0 {
        return Ok(false);
    }
    let mut n = count;
    if !closed && implicit {
        n -= 1
    }
    if n == 0 {
        return Ok(false);
    }
    let p = Point { x: px, y: py };
    let Some(local) = affine_map_point(shape.stroke_inverse, p) else {
        return Ok(false);
    };
    let (px, py) = (local.x, local.y);
    let r = s.width * 0.5;
    let dashes = s.dash_values.slice(view.dash_values).unwrap_or(&[]);
    if dashes.is_empty() {
        for i in 0..n {
            control
                .charge(vexel_scene::WorkKind::StrokeTest, 1)
                .map_err(crate::stopped)?;
            let Some(e) = edges.get(first + i) else {
                return Ok(false);
            };
            let cap = if !closed && (i == 0 || i + 1 == n) {
                s.cap
            } else {
                Linecap::Butt
            };
            if segment_caps(e.a, e.b, px, py, r, cap, cap) {
                return Ok(true);
            }
        }
        let joins = if closed { n } else { n.saturating_sub(1) };
        for i in 0..joins {
            control
                .charge(vexel_scene::WorkKind::StrokeTest, 1)
                .map_err(crate::stopped)?;
            let ni = if closed { (i + 1) % n } else { i + 1 };
            let Some(a) = edges.get(first + (ni + n - 1) % n) else {
                return Ok(false);
            };
            let Some(b) = edges.get(first + ni) else {
                return Ok(false);
            };
            if join_hit(a.a, a.b, b.b, px, py, r, s.join, s.miter_limit) {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    if dashes.len() > 16 || dashes.iter().any(|d| !d.is_finite() || *d <= 0.0) {
        return Ok(false);
    }
    let expanded = dashes.len() * (if dashes.len() % 2 == 1 { 2 } else { 1 });
    let mut period = 0.0;
    for i in 0..expanded {
        control
            .charge(vexel_scene::WorkKind::StrokeTest, 1)
            .map_err(crate::stopped)?;
        period += dashes[i % dashes.len()]
    }
    if !period.is_finite() || period <= 0.0 {
        return Ok(false);
    }
    let mut phase = s.dash_offset % period;
    if phase < 0.0 {
        phase += period
    }
    let mut pat = 0usize;
    let mut advances = 0;
    while phase >= dashes[pat % dashes.len()] {
        control
            .charge(vexel_scene::WorkKind::StrokeTest, 1)
            .map_err(crate::stopped)?;
        if advances >= expanded {
            return Ok(false);
        }
        phase -= dashes[pat % dashes.len()];
        pat = (pat + 1) % expanded;
        advances += 1
    }
    let mut remain = dashes[pat % dashes.len()] - phase;
    let mut joins_prev = false;
    for i in 0..n {
        control
            .charge(vexel_scene::WorkKind::StrokeTest, 1)
            .map_err(crate::stopped)?;
        let Some(e) = edges.get(first + i) else {
            return Ok(false);
        };
        let dx = e.b.x - e.a.x;
        let dy = e.b.y - e.a.y;
        let len = dx.hypot(dy);
        if len == 0.0 {
            continue;
        }
        let (mut at, mut continues) = (0.0, false);
        while at < len {
            control
                .charge(vexel_scene::WorkKind::StrokeTest, 1)
                .map_err(crate::stopped)?;
            let step = remain.min(len - at);
            let end = at + step;
            if step <= 0.0 || !end.is_finite() {
                return Ok(false);
            }
            let on = pat % 2 == 0;
            let crosses = on && end == len && remain - step > DASH_EPSILON && i + 1 < n;
            if on {
                let a = Point {
                    x: e.a.x + dx * (at / len),
                    y: e.a.y + dy * (at / len),
                };
                let b = Point {
                    x: e.a.x + dx * (end / len),
                    y: e.a.y + dy * (end / len),
                };
                let sc = if at == 0.0 && (joins_prev || (seam && i == 0)) {
                    Linecap::Butt
                } else {
                    s.cap
                };
                let ec = if crosses || (seam && end == len && i + 1 == n) {
                    Linecap::Butt
                } else {
                    s.cap
                };
                if segment_caps(a, b, px, py, r, sc, ec) {
                    return Ok(true);
                }
            }
            if crosses {
                continues = true
            }
            at = end;
            remain -= step;
            if remain <= DASH_EPSILON {
                pat = (pat + 1) % expanded;
                remain = dashes[pat % dashes.len()]
            }
        }
        if continues && i + 1 < n {
            let Some(next) = edges.get(first + i + 1) else {
                return Ok(false);
            };
            if join_hit(e.a, e.b, next.b, px, py, r, s.join, s.miter_limit) {
                return Ok(true);
            }
        }
        joins_prev = continues;
    }
    if closed && seam {
        if let (Some(a), Some(b)) = (edges.get(first + n - 1), edges.get(first)) {
            return Ok(join_hit(a.a, a.b, b.b, px, py, r, s.join, s.miter_limit));
        }
    }
    Ok(false)
}

pub fn stroke_contains(
    view: SceneView<'_>,
    id: ShapeId,
    input: &SampleInput<'_>,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(shape) = view.shape(id) else {
        return Ok(false);
    };
    let s = &shape.stroke;
    if s.rgba[3] == 0 || s.width <= 0.0 {
        return Ok(false);
    }
    if input
        .dash_cache
        .shapes
        .get(id.index())
        .is_some_and(|r| r.grid_columns != 0)
    {
        return dash_contains(
            view,
            shape,
            id.index(),
            input.dash_cache,
            input.px,
            input.py,
            control,
        );
    }
    let Some(_edge_range) = shape.edges.slice(input.local_edges) else {
        return Ok(false);
    };
    if !shape
        .subpaths
        .slice(view.path_subpaths)
        .unwrap_or(&[])
        .is_empty()
    {
        for (i, sub) in shape
            .subpaths
            .slice(view.path_subpaths)
            .unwrap()
            .iter()
            .enumerate()
        {
            control
                .charge(vexel_scene::WorkKind::StrokeTest, 1)
                .map_err(crate::stopped)?;
            let seam = input
                .dash_seams
                .get(view.shapes.len() + shape.subpaths.first + i)
                .copied()
                .unwrap_or(false);
            if range_contains(
                view,
                shape,
                sub.first_edge,
                sub.edge_count,
                sub.closed,
                true,
                seam,
                input.local_edges,
                input.px,
                input.py,
                control,
            )? {
                return Ok(true);
            }
        }
        Ok(false)
    } else {
        range_contains(
            view,
            shape,
            shape.edges.first,
            shape.edges.count,
            shape.stroke_closed,
            shape.filled,
            input.dash_seams.get(id.index()).copied().unwrap_or(false),
            input.local_edges,
            input.px,
            input.py,
            control,
        )
    }
}
fn dash_contains(
    _view: SceneView<'_>,
    shape: &vexel_scene::Shape,
    index: usize,
    cache: &DashCache,
    px: f64,
    py: f64,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(local) = affine_map_point(shape.stroke_inverse, Point { x: px, y: py }) else {
        return Ok(false);
    };
    let Some(range) = cache.shapes.get(index) else {
        return Ok(false);
    };
    let r = shape.stroke.width * 0.5;
    if let Some(cell) = grid_lookup(range, local.x, local.y) {
        for &encoded in range
            .grid_refs
            .get(cell.first..cell.first.saturating_add(cell.count))
            .unwrap_or(&[])
        {
            control
                .charge(vexel_scene::WorkKind::StrokeTest, 1)
                .map_err(crate::stopped)?;
            if encoded & STROKE_GRID_JOIN_BIT != 0 {
                let j = encoded & !STROKE_GRID_JOIN_BIT;
                if let Some(join) = cache.joins.get(j as usize) {
                    let reach = if shape.stroke.join == Linejoin::Miter {
                        r * shape.stroke.miter_limit
                    } else {
                        r
                    };
                    if (local.x - join.vertex.x).abs() <= reach
                        && (local.y - join.vertex.y).abs() <= reach
                        && join_hit(
                            join.previous,
                            join.vertex,
                            join.next,
                            local.x,
                            local.y,
                            r,
                            shape.stroke.join,
                            shape.stroke.miter_limit,
                        )
                    {
                        return Ok(true);
                    }
                }
            } else if let Some(seg) = cache.segments.get(encoded as usize) {
                if segment_caps(
                    seg.edge.a,
                    seg.edge.b,
                    local.x,
                    local.y,
                    r,
                    seg.start_cap,
                    seg.end_cap,
                ) {
                    return Ok(true);
                }
            }
        }
        return Ok(false);
    } else {
        return Ok(false);
    }
}
pub fn sample_work(
    view: SceneView<'_>,
    id: ShapeId,
    cache: &DashCache,
    x: f64,
    y: f64,
) -> (usize, bool, bool) {
    let Some(shape) = view.shape(id) else {
        return (0, false, false);
    };
    let Some(range) = cache.shapes.get(id.index()) else {
        return (0, false, false);
    };
    if shape.stroke.rgba[3] == 0 || shape.stroke.width <= 0.0 {
        return (0, false, false);
    }
    if range.grid_columns == 0 {
        return (
            range.segments.count + range.joins.count,
            false,
            range.segments.count + range.joins.count != 0,
        );
    }
    let Some(p) = affine_map_point(shape.stroke_inverse, Point { x, y }) else {
        return (0, true, false);
    };
    let n = grid_lookup(range, p.x, p.y).map_or(1, |c| c.count + 1);
    (n, true, n > 1)
}
