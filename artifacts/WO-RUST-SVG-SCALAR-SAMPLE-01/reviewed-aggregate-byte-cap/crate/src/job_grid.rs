use crate::{
    STROKE_GRID_JOIN_BIT, STROKE_GRID_MAX_AXIS, STROKE_GRID_MAX_CELLS, STROKE_GRID_MAX_REFS,
    stopped,
};
use vexel_geometry::Point;
use vexel_scene::{
    DashSegment, GridCell, JobDashCache, JobDashRange, RasterJob, RasterPool, Result, SceneView,
    WorkControl, WorkKind,
};

fn cap_error() -> vexel_scene::Diagnostic {
    vexel_scene::Diagnostic {
        kind: vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::DashCache),
        offset: 0,
    }
}
fn capacity_fallback(error: vexel_scene::Diagnostic) -> bool {
    matches!(
        error.kind,
        vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::DashCache)
            | vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Grid)
    )
}
fn bounds_segment(s: &DashSegment, radius: f64) -> [f64; 4] {
    let margin = if s.start_cap == vexel_scene::Linecap::Square
        || s.end_cap == vexel_scene::Linecap::Square
    {
        radius * std::f64::consts::SQRT_2
    } else {
        radius
    };
    [
        s.edge.a.x.min(s.edge.b.x) - margin,
        s.edge.a.y.min(s.edge.b.y) - margin,
        s.edge.a.x.max(s.edge.b.x) + margin,
        s.edge.a.y.max(s.edge.b.y) + margin,
    ]
}
fn join_bounds(p: Point, radius: f64) -> [f64; 4] {
    [p.x - radius, p.y - radius, p.x + radius, p.y + radius]
}
fn axis(x: f64, range: &JobDashRange, horizontal: bool) -> Option<usize> {
    let (origin, size, count) = if horizontal {
        (range.grid_min_x, range.grid_cell_size, range.grid_columns)
    } else {
        (range.grid_min_y, range.grid_cell_size, range.grid_rows)
    };
    let mut value = (x - origin) / size;
    if !value.is_finite() || value < -1.0 || count == 0 {
        return None;
    }
    if value < 0.0 {
        value = 0.0;
    }
    let cell = value.floor().min(count as f64 - 1.0);
    (cell >= 0.0).then_some(cell as usize)
}
fn bbox(b: [f64; 4], r: &JobDashRange) -> Option<(usize, usize, usize, usize)> {
    Some((
        axis(b[0], r, true)?,
        axis(b[1], r, false)?,
        axis(b[2], r, true)?,
        axis(b[3], r, false)?,
    ))
}
/// Build every shape grid in admitted same-job buffers. Oversized cell/ref/byte
/// grids follow the frozen scalar fallback; cancellation and admission errors
/// propagate without publishing a partially filled range.
pub(crate) fn build_stroke_grid(
    view: SceneView<'_>,
    job: &mut RasterJob,
    cache: JobDashCache,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if job.slice(cache.shapes)?.len() != view.shapes.len() {
        return Err(vexel_scene::Diagnostic {
            kind: vexel_scene::ErrorKind::InvalidArgument,
            offset: 0,
        });
    }
    let mut total_cells = 0usize;
    let mut total_refs = 0usize;
    for i in 0..view.shapes.len() {
        control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
        let mut range = *job.slice(cache.shapes)?.get(i).ok_or_else(cap_error)?;
        range.grid_min_x = 0.0;
        range.grid_min_y = 0.0;
        range.grid_cell_size = 0.0;
        range.grid_columns = 0;
        range.grid_rows = 0;
        let primitive = range
            .segments
            .count
            .checked_add(range.joins.count)
            .ok_or_else(cap_error)?;
        if primitive == 0 {
            job.slice_mut(cache.shapes)?[i] = range;
            continue;
        }
        let shape = &view.shapes[i];
        let radius = shape.stroke.width * 0.5;
        let mut minx = f64::INFINITY;
        let mut miny = f64::INFINITY;
        let mut maxx = f64::NEG_INFINITY;
        let mut maxy = f64::NEG_INFINITY;
        let sr = range.segments.first..range.segments.end().ok_or_else(cap_error)?;
        let jr = range.joins.first..range.joins.end().ok_or_else(cap_error)?;
        if sr.end > job.slice(cache.segments)?.len() || jr.end > job.slice(cache.joins)?.len() {
            return Err(vexel_scene::Diagnostic {
                kind: vexel_scene::ErrorKind::InvalidArgument,
                offset: 0,
            });
        }
        for index in sr.clone() {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            let seg = job.slice(cache.segments)?[index];
            let b = bounds_segment(&seg, radius);
            minx = minx.min(b[0]);
            miny = miny.min(b[1]);
            maxx = maxx.max(b[2]);
            maxy = maxy.max(b[3]);
        }
        let jradii = if shape.stroke.join == vexel_scene::Linejoin::Miter {
            radius * shape.stroke.miter_limit
        } else {
            radius
        };
        for index in jr.clone() {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            let j = job.slice(cache.joins)?[index];
            let b = join_bounds(j.vertex, jradii);
            minx = minx.min(b[0]);
            miny = miny.min(b[1]);
            maxx = maxx.max(b[2]);
            maxy = maxy.max(b[3]);
        }
        let sx = maxx - minx;
        let sy = maxy - miny;
        if !sx.is_finite() || !sy.is_finite() {
            job.slice_mut(cache.shapes)?[i] = range;
            continue;
        }
        let size = 1.0f64.max(sx.max(sy) / STROKE_GRID_MAX_AXIS as f64);
        let cols = (sx / size).ceil().max(1.0);
        let rows = (sy / size).ceil().max(1.0);
        if cols > 128.0 || rows > 128.0 {
            job.slice_mut(cache.shapes)?[i] = range;
            continue;
        }
        range.grid_min_x = minx;
        range.grid_min_y = miny;
        range.grid_cell_size = size;
        range.grid_columns = cols as u16;
        range.grid_rows = rows as u16;
        let cell_count = (cols as usize)
            .checked_mul(rows as usize)
            .ok_or_else(cap_error)?;
        if total_cells
            .checked_add(cell_count)
            .is_none_or(|n| n > STROKE_GRID_MAX_CELLS)
        {
            range.grid_columns = 0;
            range.grid_rows = 0;
            job.slice_mut(cache.shapes)?[i] = range;
            continue;
        }
        let cells_handle =
            job.buffer::<GridCell>(RasterPool::Grid, STROKE_GRID_MAX_CELLS, control)?;
        let mut refs_handle = None;
        let built = (|| -> Result<Option<(vexel_scene::JobBuffer<u32>, usize)>> {
            if let Err(error) = job.reserve(cells_handle, cell_count, control) {
                if capacity_fallback(error) {
                    return Ok(None);
                }
                return Err(error);
            }
            let zero = GridCell { first: 0, count: 0 };
            for _ in 0..cell_count {
                job.extend(cells_handle, &[zero], control)?;
            }
            let mut count = 0usize;
            let mut fits = true;
            for p in 0..primitive {
                control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                let b = if p < range.segments.count {
                    let seg = job.slice(cache.segments)?[range.segments.first + p];
                    bounds_segment(&seg, radius)
                } else {
                    let join =
                        job.slice(cache.joins)?[range.joins.first + p - range.segments.count];
                    join_bounds(join.vertex, jradii)
                };
                if p < range.segments.count
                    && ![b[0], b[1], b[2], b[3]].into_iter().all(f64::is_finite)
                {
                    fits = false;
                    break;
                }
                let Some((x0, y0, x1, y1)) = bbox(b, &range) else {
                    fits = false;
                    break;
                };
                for cy in y0..=y1 {
                    for cx in x0..=x1 {
                        control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                        let index = cy * cols as usize + cx;
                        let next = job.slice(cells_handle)?[index]
                            .count
                            .checked_add(1)
                            .ok_or_else(cap_error)?;
                        job.slice_mut(cells_handle)?[index].count = next;
                        count = count.checked_add(1).ok_or_else(cap_error)?;
                    }
                }
            }
            if !fits
                || total_refs
                    .checked_add(count)
                    .is_none_or(|n| n > STROKE_GRID_MAX_REFS)
            {
                return Ok(None);
            }
            let refs = job.buffer::<u32>(RasterPool::Grid, STROKE_GRID_MAX_REFS, control)?;
            refs_handle = Some(refs);
            if let Err(error) = job.reserve(refs, count, control) {
                if capacity_fallback(error) {
                    return Ok(None);
                }
                return Err(error);
            }
            for _ in 0..count {
                job.extend(refs, &[0u32], control)?;
            }
            let mut offset = 0usize;
            for cell in job.slice_mut(cells_handle)? {
                let n = cell.count;
                cell.first = offset;
                cell.count = 0;
                offset = offset.checked_add(n).ok_or_else(cap_error)?;
            }
            for p in 0..primitive {
                control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                let b = if p < range.segments.count {
                    let seg = job.slice(cache.segments)?[range.segments.first + p];
                    bounds_segment(&seg, radius)
                } else {
                    let join =
                        job.slice(cache.joins)?[range.joins.first + p - range.segments.count];
                    join_bounds(join.vertex, jradii)
                };
                let (x0, y0, x1, y1) = bbox(b, &range).ok_or_else(cap_error)?;
                let encoded = if p < range.segments.count {
                    let index = range.segments.first + p;
                    if index >= STROKE_GRID_JOIN_BIT as usize {
                        return Err(cap_error());
                    }
                    u32::try_from(index).map_err(|_| cap_error())?
                } else {
                    let index = range.joins.first + p - range.segments.count;
                    if index >= STROKE_GRID_JOIN_BIT as usize {
                        return Err(cap_error());
                    }
                    STROKE_GRID_JOIN_BIT | index as u32
                };
                for cy in y0..=y1 {
                    for cx in x0..=x1 {
                        control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                        let cell_index = cy * cols as usize + cx;
                        let cell = job.slice(cells_handle)?[cell_index];
                        let ref_index = cell.first.checked_add(cell.count).ok_or_else(cap_error)?;
                        job.slice_mut(refs)?[ref_index] = encoded;
                        job.slice_mut(cells_handle)?[cell_index].count += 1;
                    }
                }
            }
            Ok(Some((refs, count)))
        })();
        match built {
            Ok(Some((refs, count))) => {
                range.grid_cells = cells_handle;
                range.grid_refs = refs;
                total_cells += cell_count;
                total_refs += count;
                job.slice_mut(cache.shapes)?[i] = range;
            }
            Ok(None) => {
                if let Some(refs) = refs_handle {
                    job.release(refs)?;
                }
                job.release(cells_handle)?;
                range.grid_columns = 0;
                range.grid_rows = 0;
                job.slice_mut(cache.shapes)?[i] = range;
            }
            Err(error) => {
                if let Some(refs) = refs_handle {
                    let _ = job.release(refs);
                }
                let _ = job.release(cells_handle);
                return Err(error);
            }
        }
    }
    Ok(())
}
