use crate::*;
use vexel_geometry::{Edge, Point};
use vexel_scene::{
    DashCache, DashJoin, DashRange, DashSegment, Linecap, SceneView, ShapeId, WorkKind,
};

fn mul(a: usize, b: usize) -> Option<usize> {
    a.checked_mul(b)
}
fn grow<T>(v: &mut Vec<T>, needed: usize) -> Result<()> {
    if needed <= v.capacity() {
        return Ok(());
    }
    let mut cap = if v.capacity() == 0 { 32 } else { v.capacity() };
    while cap < needed {
        cap = cap.checked_add((cap / 8).max(1)).ok_or_else(limit)?
    }
    let additional = cap.saturating_sub(v.len());
    v.try_reserve_exact(additional).map_err(|_| oom())
}
fn can_grow(cache: &DashCache, which: u8, capacity: usize) -> bool {
    let segments = if which == 0 {
        capacity
    } else {
        cache.segments.capacity()
    };
    let joins = if which == 1 {
        capacity
    } else {
        cache.joins.capacity()
    };
    mul(cache.shapes.capacity(), std::mem::size_of::<DashRange>())
        .and_then(|n| n.checked_add(mul(segments, std::mem::size_of::<DashSegment>())?))
        .and_then(|n| n.checked_add(mul(joins, std::mem::size_of::<DashJoin>())?))
        .and_then(|n| n.checked_add(cache.grid_bytes))
        .is_some_and(|n| n <= DASH_CACHE_BYTE_LIMIT)
}
fn reserve_segments(c: &mut DashCache, n: usize) -> Result<()> {
    if n <= c.segments.capacity() {
        return Ok(());
    }
    let mut cap = if c.segments.capacity() == 0 {
        32
    } else {
        c.segments.capacity()
    };
    while cap < n {
        cap = cap.checked_add((cap / 8).max(1)).ok_or_else(limit)?
    }
    if !can_grow(c, 0, cap) {
        return Err(limit());
    }
    grow(&mut c.segments, n)?;
    if cache_used(c)
        .filter(|x| *x <= DASH_CACHE_BYTE_LIMIT)
        .is_none()
    {
        return Err(limit());
    }
    Ok(())
}
fn reserve_joins(c: &mut DashCache, n: usize) -> Result<()> {
    if n <= c.joins.capacity() {
        return Ok(());
    }
    let mut cap = if c.joins.capacity() == 0 {
        32
    } else {
        c.joins.capacity()
    };
    while cap < n {
        cap = cap.checked_add((cap / 8).max(1)).ok_or_else(limit)?
    }
    if !can_grow(c, 1, cap) {
        return Err(limit());
    }
    grow(&mut c.joins, n)?;
    if cache_used(c)
        .filter(|x| *x <= DASH_CACHE_BYTE_LIMIT)
        .is_none()
    {
        return Err(limit());
    }
    Ok(())
}
fn step(c: &mut DashCache, n: usize, control: &mut dyn WorkControl) -> Result<()> {
    if n > DASH_CACHE_MAX_STEPS.saturating_sub(c.build_steps) {
        return Err(limit());
    }
    control
        .charge(WorkKind::StrokeTest, n as u64)
        .map_err(stopped)?;
    c.build_steps += n;
    Ok(())
}
fn add_segment(c: &mut DashCache, e: Edge, sc: Linecap, ec: Linecap) -> Result<()> {
    let n = c.segments.len().checked_add(1).ok_or_else(limit)?;
    reserve_segments(c, n)?;
    c.segments.push(DashSegment {
        edge: e,
        start_cap: sc,
        end_cap: ec,
    });
    Ok(())
}
fn add_join(c: &mut DashCache, a: Edge, b: Edge) -> Result<()> {
    let n = c.joins.len().checked_add(1).ok_or_else(limit)?;
    reserve_joins(c, n)?;
    c.joins.push(DashJoin {
        previous: a.a,
        vertex: a.b,
        next: b.b,
    });
    Ok(())
}
fn build_dashed(
    c: &mut DashCache,
    stroke: &vexel_scene::Stroke,
    dashes: &[f64],
    edges: &[Edge],
    first: usize,
    count: usize,
    closed: bool,
    implicit: bool,
    seam: bool,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let mut n = count;
    if !closed && implicit && n > 0 {
        n -= 1
    }
    if n == 0 {
        return Ok(());
    }
    if dashes.is_empty() || dashes.len() > 16 {
        return Err(limit());
    }
    let exp = dashes.len() * (if dashes.len() % 2 == 1 { 2 } else { 1 });
    step(c, exp, control)?;
    let mut period = 0.0;
    for i in 0..exp {
        let d = dashes[i % dashes.len()];
        if !d.is_finite() || d <= 0.0 || !(period + d).is_finite() {
            return Err(limit());
        }
        period += d
    }
    if period <= 0.0 {
        return Err(limit());
    }
    let mut phase = stroke.dash_offset % period;
    if !phase.is_finite() {
        return Err(limit());
    }
    if phase < 0.0 {
        phase += period
    }
    let mut pat = 0usize;
    for _ in 0..=exp {
        if phase < dashes[pat % dashes.len()] {
            break;
        }
        step(c, 1, control)?;
        phase -= dashes[pat % dashes.len()];
        pat = (pat + 1) % exp
    }
    let mut remain = dashes[pat % dashes.len()] - phase;
    if !remain.is_finite() || remain <= 0.0 {
        return Err(limit());
    }
    let mut joins_prev = false;
    for i in 0..n {
        step(c, 1, control)?;
        let e = *edges.get(first + i).ok_or_else(limit)?;
        let dx = e.b.x - e.a.x;
        let dy = e.b.y - e.a.y;
        let len = dx.hypot(dy);
        if !len.is_finite() {
            return Err(limit());
        }
        if len == 0.0 {
            continue;
        }
        let (mut at, mut cont) = (0.0, false);
        while at < len {
            step(c, 1, control)?;
            let take = remain.min(len - at);
            let end = at + take;
            if take <= 0.0 || !end.is_finite() {
                return Err(limit());
            }
            let on = pat % 2 == 0;
            let continues = on && end == len && remain - take > DASH_EPSILON;
            let cross = continues && i + 1 < n;
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
                    stroke.cap
                };
                let ec = if cross || (end == len && seam && i + 1 == n) {
                    Linecap::Butt
                } else {
                    stroke.cap
                };
                add_segment(c, Edge { a, b }, sc, ec)?
            }
            if cross {
                cont = true
            }
            at = end;
            remain -= take;
            if remain <= DASH_EPSILON {
                pat = (pat + 1) % exp;
                remain = dashes[pat % dashes.len()]
            }
        }
        if cont && i + 1 < n {
            add_join(c, e, *edges.get(first + i + 1).ok_or_else(limit)?)?
        }
        joins_prev = cont;
    }
    if seam {
        add_join(
            c,
            *edges.get(first + n - 1).ok_or_else(limit)?,
            *edges.get(first).ok_or_else(limit)?,
        )?
    }
    Ok(())
}
fn build_solid(
    c: &mut DashCache,
    stroke: &vexel_scene::Stroke,
    edges: &[Edge],
    first: usize,
    count: usize,
    closed: bool,
    implicit: bool,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let mut n = count;
    if !closed && implicit && n > 0 {
        n -= 1
    }
    if n == 0 {
        return Ok(());
    }
    for i in 0..n {
        step(c, 1, control)?;
        let e = *edges.get(first + i).ok_or_else(limit)?;
        let cap = if !closed && (i == 0 || i + 1 == n) {
            stroke.cap
        } else {
            Linecap::Butt
        };
        add_segment(c, e, cap, cap)?
    }
    let joins = if closed { n } else { n - 1 };
    for i in 0..joins {
        step(c, 1, control)?;
        let next = if closed { (i + 1) % n } else { i + 1 };
        add_join(c, edges[first + (next + n - 1) % n], edges[first + next])?
    }
    Ok(())
}

pub fn dash_range_joins_seam(
    stroke: &vexel_scene::Stroke,
    dashes: &[f64],
    edges: &[Edge],
    first: usize,
    count: usize,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    if dashes.is_empty()
        || dashes.len() > 16
        || count == 0
        || dashes.iter().any(|d| !d.is_finite() || *d <= 0.0)
    {
        return Ok(false);
    }
    let exp = dashes.len() * (if dashes.len() % 2 == 1 { 2 } else { 1 });
    let mut period = 0.0;
    for i in 0..exp {
        control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
        period += dashes[i % dashes.len()]
    }
    if !period.is_finite() || period <= 0.0 {
        return Ok(false);
    }
    let mut phase = stroke.dash_offset % period;
    if !phase.is_finite() {
        return Ok(false);
    }
    if phase < 0.0 {
        phase += period
    }
    let mut pat = 0;
    while phase >= dashes[pat % dashes.len()] {
        phase -= dashes[pat % dashes.len()];
        pat = (pat + 1) % exp
    }
    let starts = pat % 2 == 0;
    let mut remain = dashes[pat % dashes.len()] - phase;
    if !remain.is_finite() || remain <= 0.0 {
        return Ok(false);
    }
    let mut ends = false;
    for i in 0..count {
        control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
        let Some(e) = edges.get(first + i) else {
            return Ok(false);
        };
        let mut len = (e.b.x - e.a.x).hypot(e.b.y - e.a.y);
        while len > 0.0 {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            let take = remain.min(len);
            ends = pat % 2 == 0;
            len -= take;
            remain -= take;
            if remain <= DASH_EPSILON {
                pat = (pat + 1) % exp;
                remain = dashes[pat % dashes.len()]
            }
        }
    }
    Ok(starts && ends)
}
pub fn subpath_closed_seam(
    view: SceneView<'_>,
    id: ShapeId,
    subpath: usize,
    edges: &[Edge],
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(shape) = view.shape(id) else {
        return Ok(false);
    };
    let Some(path) = shape
        .subpaths
        .slice(view.path_subpaths)
        .and_then(|v| v.get(subpath))
    else {
        return Ok(false);
    };
    let Some(dashes) = shape.stroke.dash_values.slice(view.dash_values) else {
        return Ok(false);
    };
    if !path.closed {
        return Ok(false);
    }
    dash_range_joins_seam(
        &shape.stroke,
        dashes,
        edges,
        path.first_edge,
        path.edge_count,
        control,
    )
}
pub fn shape_closed_seam(
    view: SceneView<'_>,
    id: ShapeId,
    edges: &[Edge],
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(shape) = view.shape(id) else {
        return Ok(false);
    };
    let Some(dashes) = shape.stroke.dash_values.slice(view.dash_values) else {
        return Ok(false);
    };
    if !shape
        .subpaths
        .slice(view.path_subpaths)
        .unwrap_or(&[])
        .is_empty()
    {
        for (i, subpath) in shape
            .subpaths
            .slice(view.path_subpaths)
            .unwrap()
            .iter()
            .enumerate()
        {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            if subpath.closed && subpath_closed_seam(view, id, i, edges, control)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    if !shape.stroke_closed {
        return Ok(false);
    }
    dash_range_joins_seam(
        &shape.stroke,
        dashes,
        edges,
        shape.edges.first,
        shape.edges.count,
        control,
    )
}

pub fn build_dash_cache(
    view: SceneView<'_>,
    local_edges: &[Edge],
    seams: &[bool],
    control: &mut dyn WorkControl,
) -> Result<DashCache> {
    let mut c = DashCache::default();
    let n = view.shapes.len();
    let range_bytes = n
        .checked_mul(std::mem::size_of::<DashRange>())
        .ok_or_else(limit)?;
    if range_bytes > DASH_CACHE_BYTE_LIMIT {
        return Err(limit());
    }
    reserve(&mut c.shapes, n)?;
    if cache_used(&c).is_none_or(|used| used > DASH_CACHE_BYTE_LIMIT) {
        return Err(limit());
    }
    for (i, s) in view.shapes.iter().enumerate() {
        let start_s = c.segments.len();
        let start_j = c.joins.len();
        if s.stroke.rgba[3] != 0 && s.stroke.width > 0.0 {
            let d = s
                .stroke
                .dash_values
                .slice(view.dash_values)
                .ok_or_else(limit)?;
            if !s
                .subpaths
                .slice(view.path_subpaths)
                .ok_or_else(limit)?
                .is_empty()
            {
                for (si, p) in s
                    .subpaths
                    .slice(view.path_subpaths)
                    .unwrap()
                    .iter()
                    .enumerate()
                {
                    let seam = seams
                        .get(n + s.subpaths.first + si)
                        .copied()
                        .unwrap_or(false);
                    if d.is_empty() {
                        build_solid(
                            &mut c,
                            &s.stroke,
                            local_edges,
                            p.first_edge,
                            p.edge_count,
                            p.closed,
                            true,
                            control,
                        )?
                    } else {
                        build_dashed(
                            &mut c,
                            &s.stroke,
                            d,
                            local_edges,
                            p.first_edge,
                            p.edge_count,
                            p.closed,
                            true,
                            seam,
                            control,
                        )?
                    }
                }
            } else {
                let seam = seams.get(i).copied().unwrap_or(false);
                if d.is_empty() {
                    build_solid(
                        &mut c,
                        &s.stroke,
                        local_edges,
                        s.edges.first,
                        s.edges.count,
                        s.stroke_closed,
                        s.filled,
                        control,
                    )?
                } else {
                    build_dashed(
                        &mut c,
                        &s.stroke,
                        d,
                        local_edges,
                        s.edges.first,
                        s.edges.count,
                        s.stroke_closed,
                        s.filled,
                        seam,
                        control,
                    )?
                }
            }
        }
        c.shapes.push(DashRange {
            segments: vexel_scene::TableRange {
                first: start_s,
                count: c.segments.len() - start_s,
            },
            joins: vexel_scene::TableRange {
                first: start_j,
                count: c.joins.len() - start_j,
            },
            grid_cells: Vec::new(),
            grid_refs: Vec::new(),
            grid_min_x: 0.0,
            grid_min_y: 0.0,
            grid_cell_size: 0.0,
            grid_columns: 0,
            grid_rows: 0,
        });
        if cache_used(&c).ok_or_else(limit)? > DASH_CACHE_BYTE_LIMIT {
            return Err(limit());
        }
    }
    build_stroke_grid(view, &mut c, control)?;
    Ok(c)
}
