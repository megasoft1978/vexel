use crate::*;
use vexel_scene::{
    DashCache, DashRange, DashSegment, GridCell, Linecap, SceneView, WorkControl, WorkKind,
};

fn grid_axis(x: f64, origin: f64, size: f64, count: u16) -> Option<u16> {
    let mut o = (x - origin) / size;
    if !o.is_finite() || o < -1.0 {
        return None;
    }
    if o < 0.0 {
        o = 0.0
    }
    let cell = o.floor();
    let cell = cell.min(count as f64 - 1.0);
    if cell < 0.0 { None } else { Some(cell as u16) }
}
pub fn grid_lookup(range: &DashRange, x: f64, y: f64) -> Option<&GridCell> {
    if range.grid_columns == 0 || range.grid_rows == 0 {
        return None;
    }
    let col = grid_axis(
        x,
        range.grid_min_x,
        range.grid_cell_size,
        range.grid_columns,
    )?;
    let row = grid_axis(y, range.grid_min_y, range.grid_cell_size, range.grid_rows)?;
    range
        .grid_cells
        .get(row as usize * range.grid_columns as usize + col as usize)
}
fn bounds_segment(s: &DashSegment, r: f64) -> Option<[f64; 4]> {
    let m = if s.start_cap == Linecap::Square || s.end_cap == Linecap::Square {
        r * std::f64::consts::SQRT_2
    } else {
        r
    };
    Some([
        s.edge.a.x.min(s.edge.b.x) - m,
        s.edge.a.y.min(s.edge.b.y) - m,
        s.edge.a.x.max(s.edge.b.x) + m,
        s.edge.a.y.max(s.edge.b.y) + m,
    ])
}
fn bbox(range: &DashRange, b: [f64; 4]) -> Option<(usize, usize, usize, usize)> {
    Some((
        grid_axis(
            b[0],
            range.grid_min_x,
            range.grid_cell_size,
            range.grid_columns,
        )? as usize,
        grid_axis(
            b[1],
            range.grid_min_y,
            range.grid_cell_size,
            range.grid_rows,
        )? as usize,
        grid_axis(
            b[2],
            range.grid_min_x,
            range.grid_cell_size,
            range.grid_columns,
        )? as usize,
        grid_axis(
            b[3],
            range.grid_min_y,
            range.grid_cell_size,
            range.grid_rows,
        )? as usize,
    ))
}
pub fn build_stroke_grid(
    view: SceneView<'_>,
    cache: &mut DashCache,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if cache.shapes.len() != view.shapes.len() {
        return Err(vexel_scene::Diagnostic {
            kind: vexel_scene::ErrorKind::InvalidArgument,
            offset: 0,
        });
    }
    for range in &mut cache.shapes {
        control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
        range.grid_cells = Vec::new();
        range.grid_refs = Vec::new();
        range.grid_min_x = 0.0;
        range.grid_min_y = 0.0;
        range.grid_cell_size = 0.0;
        range.grid_columns = 0;
        range.grid_rows = 0;
    }
    cache.grid_cell_count = 0;
    cache.grid_ref_count = 0;
    cache.grid_bytes = 0;
    let mut cells_total = 0usize;
    let mut refs_total = 0usize;
    for i in 0..cache.shapes.len() {
        let base_used = cache_used(cache).ok_or_else(limit)?;
        let Some(s) = view.shapes.get(i) else {
            return Err(vexel_scene::Diagnostic {
                kind: vexel_scene::ErrorKind::InvalidArgument,
                offset: 0,
            });
        };
        let r = &mut cache.shapes[i];
        let segment_end = r.segments.end().ok_or_else(limit)?;
        let join_end = r.joins.end().ok_or_else(limit)?;
        if segment_end > cache.segments.len() || join_end > cache.joins.len() {
            return Err(vexel_scene::Diagnostic {
                kind: vexel_scene::ErrorKind::InvalidArgument,
                offset: 0,
            });
        }
        let primitive = r.segments.count + r.joins.count;
        if primitive == 0 {
            continue;
        }
        control.checkpoint().map_err(stopped)?;
        let radius = s.stroke.width * 0.5;
        let mut minx = f64::INFINITY;
        let mut miny = minx;
        let mut maxx = f64::NEG_INFINITY;
        let mut maxy = maxx;
        for seg in &cache.segments[r.segments.first..r.segments.first + r.segments.count] {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            let Some(b) = bounds_segment(seg, radius) else {
                continue;
            };
            minx = minx.min(b[0]);
            miny = miny.min(b[1]);
            maxx = maxx.max(b[2]);
            maxy = maxy.max(b[3])
        }
        let jr = if s.stroke.join == vexel_scene::Linejoin::Miter {
            radius * s.stroke.miter_limit
        } else {
            radius
        };
        for j in &cache.joins[r.joins.first..r.joins.first + r.joins.count] {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            minx = minx.min(j.vertex.x - jr);
            miny = miny.min(j.vertex.y - jr);
            maxx = maxx.max(j.vertex.x + jr);
            maxy = maxy.max(j.vertex.y + jr)
        }
        let sx = maxx - minx;
        let sy = maxy - miny;
        if !sx.is_finite() || !sy.is_finite() {
            continue;
        }
        let size = 1.0f64.max((sx.max(sy)) / STROKE_GRID_MAX_AXIS as f64);
        let cols = (sx / size).ceil().max(1.0);
        let rows = (sy / size).ceil().max(1.0);
        if cols > 128.0 || rows > 128.0 {
            continue;
        }
        r.grid_min_x = minx;
        r.grid_min_y = miny;
        r.grid_cell_size = size;
        r.grid_columns = cols as u16;
        r.grid_rows = rows as u16;
        let count = (cols as usize)
            .checked_mul(rows as usize)
            .ok_or_else(limit)?;
        if cells_total
            .checked_add(count)
            .is_none_or(|n| n > STROKE_GRID_MAX_CELLS)
        {
            r.grid_columns = 0;
            r.grid_rows = 0;
            continue;
        }
        let mut cells = Vec::new();
        cells.try_reserve_exact(count).map_err(|_| oom())?;
        cells.resize(count, GridCell { first: 0, count: 0 });
        let mut total = 0usize;
        let mut encoded_bounds = Vec::new();
        encoded_bounds
            .try_reserve_exact(primitive)
            .map_err(|_| oom())?;
        for (k, seg) in cache.segments[r.segments.first..r.segments.first + r.segments.count]
            .iter()
            .enumerate()
        {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            if let Some(b) = bounds_segment(seg, radius) {
                encoded_bounds.push(((r.segments.first + k) as u32, b));
            } else {
                encoded_bounds.clear();
                break;
            }
        }
        for (k, j) in cache.joins[r.joins.first..r.joins.first + r.joins.count]
            .iter()
            .enumerate()
        {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            encoded_bounds.push((
                STROKE_GRID_JOIN_BIT | ((r.joins.first + k) as u32),
                [
                    j.vertex.x - jr,
                    j.vertex.y - jr,
                    j.vertex.x + jr,
                    j.vertex.y + jr,
                ],
            ))
        }
        let mut fits = true;
        for (_, b) in &encoded_bounds {
            let Some((x0, y0, x1, y1)) = bbox(r, *b) else {
                fits = false;
                break;
            };
            for y in y0..=y1 {
                for x in x0..=x1 {
                    control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                    let c = &mut cells[y * cols as usize + x];
                    c.count = c.count.checked_add(1).ok_or_else(limit)?;
                    total = total.checked_add(1).ok_or_else(limit)?
                }
            }
        }
        if !fits
            || refs_total
                .checked_add(total)
                .is_none_or(|x| x > STROKE_GRID_MAX_REFS)
        {
            r.grid_columns = 0;
            r.grid_rows = 0;
            continue;
        }
        let cell_bytes = count
            .checked_mul(std::mem::size_of::<GridCell>())
            .ok_or_else(limit)?;
        let ref_bytes = total.checked_mul(4).ok_or_else(limit)?;
        if base_used
            .checked_add(cell_bytes + ref_bytes)
            .is_none_or(|x| x > DASH_CACHE_BYTE_LIMIT)
        {
            r.grid_columns = 0;
            r.grid_rows = 0;
            continue;
        }
        let mut offset = 0;
        for c in &mut cells {
            let num = c.count;
            c.first = offset;
            c.count = 0;
            offset = offset.checked_add(num).ok_or_else(limit)?
        }
        let mut refs = Vec::new();
        refs.try_reserve_exact(total).map_err(|_| oom())?;
        refs.resize(total, 0);
        for (encoded, b) in encoded_bounds {
            let Some((x0, y0, x1, y1)) = bbox(r, b) else {
                fits = false;
                break;
            };
            for y in y0..=y1 {
                for x in x0..=x1 {
                    control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                    let c = &mut cells[y * cols as usize + x];
                    refs[c.first + c.count] = encoded;
                    c.count += 1
                }
            }
        }
        if !fits {
            r.grid_columns = 0;
            r.grid_rows = 0;
            continue;
        }
        let actual_grid_bytes = bytes(&cells)
            .and_then(|x| bytes(&refs).and_then(|y| x.checked_add(y)))
            .ok_or_else(limit)?;
        if base_used
            .checked_add(actual_grid_bytes)
            .is_none_or(|x| x > DASH_CACHE_BYTE_LIMIT)
        {
            r.grid_columns = 0;
            r.grid_rows = 0;
            continue;
        }
        r.grid_cells = cells;
        r.grid_refs = refs;
        cells_total += count;
        refs_total += total;
        cache.grid_cell_count = cells_total;
        cache.grid_ref_count = refs_total;
        cache.grid_bytes = cache
            .grid_bytes
            .checked_add(actual_grid_bytes)
            .ok_or_else(limit)?
    }
    Ok(())
}
