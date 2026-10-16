use crate::{DASH_CACHE_MAX_STEPS, STROKE_GRID_JOIN_BIT, stopped};
use vexel_geometry::{Edge, Point};
use vexel_scene::{
    DashJoin, DashSegment, JobDashCache, JobDashRange, JobRasterScratch, Linecap, RasterJob,
    RasterPool, Result, SceneView, Stroke, WorkControl, WorkKind,
};

fn dash_limit() -> vexel_scene::Diagnostic {
    vexel_scene::Diagnostic {
        kind: vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::DashCache),
        offset: 0,
    }
}
fn range(first: usize, end: usize) -> Result<vexel_scene::TableRange> {
    Ok(vexel_scene::TableRange {
        first,
        count: end.checked_sub(first).ok_or_else(dash_limit)?,
    })
}
struct Writer<'a> {
    job: &'a mut RasterJob,
    cache: JobDashCache,
    steps: usize,
}
impl Writer<'_> {
    fn ensure<T: vexel_scene::JobElement>(
        job: &mut RasterJob,
        handle: vexel_scene::JobBuffer<T>,
        needed: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let current = job.capacity(handle)?;
        if needed <= current { return Ok(()); }
        let mut capacity = if current == 0 { 32 } else { current };
        while capacity < needed {
            capacity = capacity.checked_add((capacity / 8).max(1)).ok_or_else(dash_limit)?;
        }
        job.reserve(handle, capacity, control)
    }
    fn step(&mut self, n: usize, control: &mut dyn WorkControl) -> Result<()> {
        if n > DASH_CACHE_MAX_STEPS.saturating_sub(self.steps) {
            return Err(dash_limit());
        }
        control
            .charge(WorkKind::StrokeTest, n as u64)
            .map_err(stopped)?;
        self.steps += n;
        Ok(())
    }
    fn segment(
        &mut self,
        edge: Edge,
        start_cap: Linecap,
        end_cap: Linecap,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let needed = self.job.slice(self.cache.segments)?.len().checked_add(1).ok_or_else(dash_limit)?;
        Self::ensure(self.job, self.cache.segments, needed, control)?;
        let segment = DashSegment {
            edge,
            start_cap,
            end_cap,
        };
        self.job.extend(self.cache.segments, &[segment], control)
    }
    fn join(&mut self, a: Edge, b: Edge, control: &mut dyn WorkControl) -> Result<()> {
        let needed = self.job.slice(self.cache.joins)?.len().checked_add(1).ok_or_else(dash_limit)?;
        Self::ensure(self.job, self.cache.joins, needed, control)?;
        self.job.extend(
            self.cache.joins,
            &[DashJoin {
                previous: a.a,
                vertex: a.b,
                next: b.b,
            }],
            control,
        )
    }
}
fn dashed(
    w: &mut Writer<'_>,
    stroke: &Stroke,
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
        n -= 1;
    }
    if n == 0 {
        return Ok(());
    }
    if dashes.is_empty() || dashes.len() > 16 {
        return Err(dash_limit());
    }
    let exp = dashes
        .len()
        .checked_mul(if dashes.len() % 2 == 1 { 2 } else { 1 })
        .ok_or_else(dash_limit)?;
    w.step(exp, control)?;
    let mut period = 0.0;
    for i in 0..exp {
        let d = dashes[i % dashes.len()];
        if !d.is_finite() || d <= 0.0 || !(period + d).is_finite() {
            return Err(dash_limit());
        }
        period += d;
    }
    if period <= 0.0 {
        return Err(dash_limit());
    }
    let mut phase = stroke.dash_offset % period;
    if !phase.is_finite() {
        return Err(dash_limit());
    }
    if phase < 0.0 {
        phase += period;
    }
    let mut pat = 0usize;
    for _ in 0..=exp {
        if phase < dashes[pat % dashes.len()] {
            break;
        }
        w.step(1, control)?;
        phase -= dashes[pat % dashes.len()];
        pat = (pat + 1) % exp;
    }
    let mut remain = dashes[pat % dashes.len()] - phase;
    if !remain.is_finite() || remain <= 0.0 {
        return Err(dash_limit());
    }
    let mut joins_prev = false;
    for i in 0..n {
        w.step(1, control)?;
        let e = *edges
            .get(first.checked_add(i).ok_or_else(dash_limit)?)
            .ok_or_else(dash_limit)?;
        let dx = e.b.x - e.a.x;
        let dy = e.b.y - e.a.y;
        let len = dx.hypot(dy);
        if !len.is_finite() {
            return Err(dash_limit());
        }
        if len == 0.0 {
            continue;
        }
        let (mut at, mut cont) = (0.0, false);
        while at < len {
            w.step(1, control)?;
            let take = remain.min(len - at);
            let end = at + take;
            if take <= 0.0 || !end.is_finite() {
                return Err(dash_limit());
            }
            let on = pat % 2 == 0;
            let continues = on && end == len && remain - take > 1.0e-12;
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
                w.segment(Edge { a, b }, sc, ec, control)?;
            }
            if cross {
                cont = true;
            }
            at = end;
            remain -= take;
            if remain <= 1.0e-12 {
                pat = (pat + 1) % exp;
                remain = dashes[pat % dashes.len()];
            }
        }
        if cont && i + 1 < n {
            w.join(
                e,
                *edges.get(first + i + 1).ok_or_else(dash_limit)?,
                control,
            )?;
        }
        joins_prev = cont;
    }
    if seam {
        w.join(
            *edges.get(first + n - 1).ok_or_else(dash_limit)?,
            *edges.get(first).ok_or_else(dash_limit)?,
            control,
        )?;
    }
    Ok(())
}
fn solid(
    w: &mut Writer<'_>,
    stroke: &Stroke,
    edges: &[Edge],
    first: usize,
    count: usize,
    closed: bool,
    implicit: bool,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let mut n = count;
    if !closed && implicit && n > 0 {
        n -= 1;
    }
    if n == 0 {
        return Ok(());
    }
    for i in 0..n {
        w.step(1, control)?;
        let e = *edges
            .get(first.checked_add(i).ok_or_else(dash_limit)?)
            .ok_or_else(dash_limit)?;
        let cap = if !closed && (i == 0 || i + 1 == n) {
            stroke.cap
        } else {
            Linecap::Butt
        };
        w.segment(e, cap, cap, control)?;
    }
    let joins = if closed { n } else { n - 1 };
    for i in 0..joins {
        w.step(1, control)?;
        let next = if closed { (i + 1) % n } else { i + 1 };
        w.join(
            edges[first + (next + n - 1) % n],
            edges[first + next],
            control,
        )?;
    }
    Ok(())
}

/// Build dash records directly in the admitted same-job buffers. The returned
/// descriptor has no borrowed source or temporary cache storage.
pub fn build_dash_cache(
    view: SceneView<'_>,
    local_edges: &[Edge],
    seams: &[bool],
    job: &mut RasterJob,
    scratch: &mut JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<JobDashCache> {
    if view.shapes.len() > 65_536 {
        return Err(vexel_scene::Diagnostic {
            kind: vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Shapes),
            offset: 0,
        });
    }
    job.validate_source(view.source_owner())?;
    // Build into new admitted buffers. The published cache remains intact until
    // dash records and every grid finish successfully; growth accounts for the
    // real old/new overlap through RasterJob's ChargeLedger admission.
    let segments = job.buffer(RasterPool::Dash, usize::MAX, control)?;
    let joins = match job.buffer(RasterPool::Dash, usize::MAX, control) {
        Ok(handle) => handle,
        Err(error) => { let _ = job.release(segments); return Err(error); }
    };
    let shapes = match job.buffer(RasterPool::Dash, 65_536, control) {
        Ok(handle) => handle,
        Err(error) => {
            let _ = job.release(segments);
            let _ = job.release(joins);
            return Err(error);
        }
    };
    let desc = JobDashCache { segments, joins, shapes, build_steps: 0 };
    if let Err(error) = job.reserve(desc.shapes, view.shapes.len(), control) {
        let _ = release_cache(job, desc);
        return Err(error);
    }
    let empty_cells = if view.shapes.is_empty() {
        None
    } else {
        match job.buffer::<vexel_scene::GridCell>(RasterPool::Grid, 0, control) {
            Ok(handle) => Some(handle),
            Err(error) => { let _ = release_cache(job, desc); return Err(error); }
        }
    };
    let empty_refs = if view.shapes.is_empty() {
        None
    } else {
        match job.buffer::<u32>(RasterPool::Grid, 0, control) {
            Ok(handle) => Some(handle),
            Err(error) => {
                if let Some(cells) = empty_cells { let _ = job.release(cells); }
                let _ = release_cache(job, desc);
                return Err(error);
            }
        }
    };
    let result = (|| {
    let mut w = Writer {
        job,
        cache: desc,
        steps: 0,
    };
    for (i, shape) in view.shapes.iter().enumerate() {
        w.step(1, control)?;
        let segment_first = w.job.slice(desc.segments)?.len();
        let join_first = w.job.slice(desc.joins)?.len();
        if shape.stroke.rgba[3] != 0 && shape.stroke.width > 0.0 {
            let dashes = shape
                .stroke
                .dash_values
                .slice(view.dash_values)
                .ok_or_else(dash_limit)?;
            let subpaths = shape
                .subpaths
                .slice(view.path_subpaths)
                .ok_or_else(dash_limit)?;
            if !subpaths.is_empty() {
                for (si, sub) in subpaths.iter().enumerate() {
                    let seam = seams
                        .get(view.shapes.len() + shape.subpaths.first + si)
                        .copied()
                        .unwrap_or(false);
                    if dashes.is_empty() {
                        solid(
                            &mut w,
                            &shape.stroke,
                            local_edges,
                            sub.first_edge,
                            sub.edge_count,
                            sub.closed,
                            true,
                            control,
                        )?;
                    } else {
                        dashed(
                            &mut w,
                            &shape.stroke,
                            dashes,
                            local_edges,
                            sub.first_edge,
                            sub.edge_count,
                            sub.closed,
                            true,
                            seam,
                            control,
                        )?;
                    }
                }
            } else {
                let seam = seams.get(i).copied().unwrap_or(false);
                if dashes.is_empty() {
                    solid(
                        &mut w,
                        &shape.stroke,
                        local_edges,
                        shape.edges.first,
                        shape.edges.count,
                        shape.stroke_closed,
                        shape.filled,
                        control,
                    )?;
                } else {
                    dashed(
                        &mut w,
                        &shape.stroke,
                        dashes,
                        local_edges,
                        shape.edges.first,
                        shape.edges.count,
                        shape.stroke_closed,
                        shape.filled,
                        seam,
                        control,
                    )?;
                }
            }
        }
        let segment_end = w.job.slice(desc.segments)?.len();
        let join_end = w.job.slice(desc.joins)?.len();
        let range = JobDashRange {
            segments: range(segment_first, segment_end)?,
            joins: range(join_first, join_end)?,
            grid_cells: empty_cells.ok_or_else(dash_limit)?,
            grid_refs: empty_refs.ok_or_else(dash_limit)?,
            grid_min_x: 0.0,
            grid_min_y: 0.0,
            grid_cell_size: 0.0,
            grid_columns: 0,
            grid_rows: 0,
        };
        w.job.extend(desc.shapes, &[range], control)?;
    }
    let cache = JobDashCache {
        build_steps: w.steps,
        ..desc
    };
    crate::job_grid::build_stroke_grid(view, w.job, cache, control)?;
    Ok(JobDashCache {
        build_steps: w.steps,
        ..desc
    })
    })();
    if let Err(error) = result {
        let _ = release_cache(job, desc);
        if let Some(handle) = empty_cells { let _ = job.release(handle); }
        if let Some(handle) = empty_refs { let _ = job.release(handle); }
        return Err(error);
    }
    let cache = result?;
    release_cache(job, scratch.dash)?;
    scratch.dash = cache;
    Ok(cache)
}

fn release_cache(job: &mut RasterJob, cache: JobDashCache) -> Result<()> {
    let mut empty = None;
    let count = job.slice(cache.shapes)?.len();
    for i in 0..count {
        let range = job.slice(cache.shapes)?[i];
        if range.grid_columns != 0 {
            job.release(range.grid_cells)?;
            job.release(range.grid_refs)?;
        } else if empty.is_none() {
            empty = Some((range.grid_cells, range.grid_refs));
        }
    }
    if let Some((cells, refs)) = empty {
        job.release(cells)?;
        job.release(refs)?;
    }
    job.release(cache.segments)?;
    job.release(cache.joins)?;
    job.release(cache.shapes)
}

pub(crate) fn tagged_index(reference: u32) -> (bool, usize) {
    (
        reference & STROKE_GRID_JOIN_BIT != 0,
        (reference & !STROKE_GRID_JOIN_BIT) as usize,
    )
}
