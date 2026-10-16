use crate::util::*;
#[cfg(test)]
use vexel_scene::{
    BoundaryMask, FillCache, FillInterval, FillRowWork, KernelResources, LimitKind, PixelBounds,
    PixelRun, RowBuffers, TableRange, ViewMapping,
};
use vexel_scene::{FillCrossing, Result, SceneView, ShapeId, WorkControl, WorkKind};

#[cfg(test)]
const MAX_FAST_EDGES: usize = 4096;
#[cfg(test)]
const SAMPLE_Y: [f64; 5] = [0.125, 0.375, 0.5, 0.625, 0.875];
fn cross_less(a: &FillCrossing, b: &FillCrossing) -> bool {
    a.x < b.x || (a.x == b.x && a.edge_index < b.edge_index)
}
fn swap(a: &mut [FillCrossing], i: usize, j: usize) {
    a.swap(i, j);
}
fn sift_down(
    v: &mut [FillCrossing],
    mut root: usize,
    count: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    while root < count / 2 {
        poll(control, WorkKind::FillEdgeTest)?;
        let mut child = root * 2 + 1;
        if child + 1 < count && cross_less(&v[child], &v[child + 1]) {
            child += 1;
        }
        if !cross_less(&v[root], &v[child]) {
            return Ok(());
        }
        swap(v, root, child);
        root = child;
    }
    Ok(())
}
pub(super) fn heap_sort(v: &mut [FillCrossing], control: &mut dyn WorkControl) -> Result<()> {
    let n = v.len();
    for p in (0..n / 2).rev() {
        sift_down(v, p, n, control)?;
    }
    for end in (1..n).rev() {
        poll(control, WorkKind::FillEdgeTest)?;
        swap(v, 0, end);
        sift_down(v, 0, end, control)?;
    }
    Ok(())
}
#[cfg(test)]
fn ceil_log2(v: usize) -> usize {
    let mut n = 0;
    let mut p = 1;
    while p < v {
        if p > usize::MAX / 2 {
            return usize::BITS as usize;
        }
        p *= 2;
        n += 1;
    }
    n
}

#[cfg(test)]
fn uniform_fill(view: SceneView<'_>, shape: usize) -> bool {
    let Some(s) = view.shapes.get(shape) else {
        return false;
    };
    if !s.filled || s.stroke.rgba[3] != 0 && s.stroke.width > 0.0 {
        return false;
    }
    let Some(edges) = s.edges.slice(view.edges) else {
        return false;
    };
    if edges.is_empty() {
        return false;
    }
    let Some(first) = view.edge_colors.get(s.edges.first) else {
        return false;
    };
    edges
        .iter()
        .enumerate()
        .skip(1)
        .all(|(i, _)| view.edge_colors.get(s.edges.first + i) == Some(first))
}

/// Admit simple uniform fills up to the source's 4096-edge fast-path bound.
#[cfg(test)]
pub fn create_fill_cache(
    view: SceneView<'_>,
    _resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<FillCache> {
    let mut c = FillCache::default();
    reserve(&mut c.eligible_shapes, view.shapes.len())?;
    let mut max = 0;
    for (i, s) in view.shapes.iter().enumerate() {
        poll(control, WorkKind::ShapeRowVisit)?;
        let count = s.edges.count;
        if count <= MAX_FAST_EDGES && uniform_fill(view, i) {
            c.eligible_shapes.push(true);
            max = max.max(count);
        } else {
            c.eligible_shapes.push(false);
        }
    }
    if max == 0 {
        return Ok(c);
    }
    if (view.viewport.width as usize) > PIXEL_RUN_LIMIT / std::mem::size_of::<PixelRun>() {
        c.eligible_shapes.fill(false);
        return Ok(c);
    }
    c.max_edge_count = max;
    c.interval_stride = add(max, 1)?;
    reserve(&mut c.crossings, max)?;
    c.crossings.resize(
        max,
        FillCrossing {
            x: 0.0,
            winding_delta: 0,
            edge_index: 0,
        },
    );
    let intervals = mul(c.interval_stride, 5)?;
    reserve(&mut c.intervals, intervals)?;
    c.intervals.resize(intervals, FillInterval::default());
    reserve(&mut c.pixel_runs, view.viewport.width as usize)?;
    Ok(c)
}

#[cfg(test)]
pub fn eligible(cache: &FillCache, shape: ShapeId) -> bool {
    cache
        .eligible_shapes
        .get(shape.index())
        .copied()
        .unwrap_or(false)
}
/// Read the first edge color for a validated uniform-fill shape without allocation.
pub fn fill_color(view: SceneView<'_>, shape: ShapeId) -> Option<[u8; 4]> {
    let s = view.shape(shape)?;
    let i = s.edges.first;
    view.edge_colors.get(i).copied()
}
/// Accepted interface spelling for the uniform cached color.
#[cfg(test)]
pub fn color(view: SceneView<'_>, shape: ShapeId) -> Option<[u8; 4]> {
    fill_color(view, shape)
}

#[cfg(test)]
fn build_intervals(
    view: SceneView<'_>,
    shape: usize,
    sample_y: f64,
    active: &[u32],
    cache: &mut FillCache,
    row: usize,
    control: &mut dyn WorkControl,
) -> Result<(usize, usize, usize)> {
    let required_intervals = cache
        .interval_stride
        .checked_mul(5)
        .ok_or_else(|| limit(LimitKind::Arithmetic))?;
    if active.len() > cache.max_edge_count
        || cache.crossings.len() < cache.max_edge_count
        || cache.interval_stride == 0
        || cache.intervals.len() < required_intervals
        || row >= 5
    {
        return Err(limit(LimitKind::Scratch));
    }
    let mut crossing_count = 0;
    for &edge_id in active {
        poll(control, WorkKind::FillEdgeTest)?;
        let edge = *view
            .edges
            .get(edge_id as usize)
            .ok_or_else(|| invalid(source_offset(view, shape)))?;
        if view
            .edge_shape(edge_id as usize)
            .is_none_or(|id| id.index() != shape)
        {
            return Err(invalid(source_offset(view, shape)));
        }
        let up = edge.a.y <= sample_y && edge.b.y > sample_y;
        let down = edge.b.y <= sample_y && edge.a.y > sample_y;
        if !up && !down {
            continue;
        }
        let x = edge.a.x + (sample_y - edge.a.y) * (edge.b.x - edge.a.x) / (edge.b.y - edge.a.y);
        if !x.is_finite() {
            return Err(invalid(source_offset(view, shape)));
        }
        cache.crossings[crossing_count] = FillCrossing {
            x,
            winding_delta: if up { 1 } else { -1 },
            edge_index: edge_id,
        };
        crossing_count += 1;
    }
    if crossing_count > 1 {
        heap_sort(&mut cache.crossings[..crossing_count], control)?;
    }
    let crossings = &cache.crossings[..crossing_count];
    let mut winding: i64 = 0;
    let mut parity = 0u8;
    for c in crossings {
        winding += c.winding_delta as i64;
        parity ^= 1;
    }
    let offset = row * cache.interval_stride;
    let mut count = 0;
    if crossing_count != 0
        && (if view.shapes[shape].evenodd {
            parity != 0
        } else {
            winding != 0
        })
    {
        cache.intervals[offset + count] = FillInterval {
            left: f64::NEG_INFINITY,
            right: crossings[0].x,
        };
        count += 1;
    }
    let mut first = 0;
    while first < crossing_count {
        poll(control, WorkKind::FillEdgeTest)?;
        let mut end = first + 1;
        let mut gw = crossings[first].winding_delta as i64;
        let mut gp = 1u8;
        while end < crossing_count && crossings[end].x == crossings[first].x {
            gw += crossings[end].winding_delta as i64;
            gp ^= 1;
            end += 1;
        }
        winding -= gw;
        parity ^= gp;
        let right = if end < crossing_count {
            crossings[end].x
        } else {
            f64::INFINITY
        };
        let inside = if view.shapes[shape].evenodd {
            parity != 0
        } else {
            winding != 0
        };
        if inside && crossings[first].x < right {
            if count >= cache.interval_stride {
                return Err(limit(LimitKind::Scratch));
            }
            cache.intervals[offset + count] = FillInterval {
                left: crossings[first].x,
                right,
            };
            count += 1;
        }
        first = end;
    }
    cache.interval_counts[row] = count;
    let log = ceil_log2(crossing_count.max(1));
    let work = add(
        mul(crossing_count, add(mul(log, 2)?, 6)?)?,
        mul(active.len(), 2)?,
    )?;
    Ok((active.len(), work, crossing_count))
}

/// Build all five fill interval rows from current scanline edge lists.
#[cfg(test)]
pub fn build_fill_row(
    cache: &mut FillCache,
    view: SceneView<'_>,
    shape: ShapeId,
    y: u32,
    map: ViewMapping,
    rows: &RowBuffers,
    _resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<FillRowWork> {
    let si = shape.index();
    if !eligible_shapes(cache, si)
        || y >= view.viewport.height
        || !map.scale_y.is_finite()
        || map.scale_y <= 0.0
    {
        return Err(invalid(source_offset(view, si)));
    }
    let stride = add(view.shapes.len(), 1)?;
    let required = mul(stride, 5)?;
    if rows.shape_offsets.len() < required
        || cache.interval_stride == 0
        || cache.intervals.len() < mul(cache.interval_stride, 5)?
        || cache.crossings.len() < cache.max_edge_count
    {
        return Err(limit(LimitKind::Scratch));
    }
    let mut result = FillRowWork::default();
    for sample in 0..5 {
        let base = sample * stride + si;
        let first = rows.shape_offsets[base];
        let end = rows.shape_offsets[base + 1];
        if end < first || end > rows.active_edges.len() {
            return Err(invalid(source_offset(view, si)));
        }
        let sy =
            view.viewport.view_y + ((y as f64 + SAMPLE_Y[sample]) - map.offset_y) / map.scale_y;
        let (visits, work, _) = build_intervals(
            view,
            si,
            sy,
            &rows.active_edges[first..end],
            cache,
            sample,
            control,
        )?;
        result.edge_visits = add(result.edge_visits, visits)?;
        result.work_units = add(result.work_units, work)?;
    }
    Ok(result)
}
#[cfg(test)]
fn eligible_shapes(cache: &FillCache, shape: usize) -> bool {
    cache.eligible_shapes.get(shape).copied().unwrap_or(false)
}

#[cfg(test)]
fn pixel_fully_in_view(view: SceneView<'_>, x: u32, y: u32, map: ViewMapping) -> bool {
    let w = view.viewport.view_width * map.scale_x;
    let h = view.viewport.view_height * map.scale_y;
    x as f64 >= map.offset_x
        && y as f64 >= map.offset_y
        && (x as f64) + 1.0 <= map.offset_x + w
        && (y as f64) + 1.0 <= map.offset_y + h
}

/// Build center-only versus multisample runs for one bounded pixel row.
#[cfg(test)]
pub fn build_fill_runs(
    cache: &mut FillCache,
    view: SceneView<'_>,
    bounds: PixelBounds,
    y: u32,
    boundary: &BoundaryMask,
    map: ViewMapping,
    _resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    if !bounds.visible
        || bounds.x_first > bounds.x_last
        || bounds.x_last >= view.viewport.width
        || bounds.y_first > bounds.y_last
        || bounds.y_last >= view.viewport.height
        || y < bounds.y_first
        || y > bounds.y_last
        || y >= view.viewport.height
        || boundary.bit_count
            != (view.viewport.width as usize)
                .checked_mul(view.viewport.height as usize)
                .ok_or_else(|| limit(LimitKind::Arithmetic))?
        || !map.scale_x.is_finite()
        || map.scale_x <= 0.0
        || !map.scale_y.is_finite()
        || map.scale_y <= 0.0
    {
        return Err(invalid(0));
    }
    if boundary.bits.len() < add(boundary.bit_count, 7)? / 8 {
        return Err(invalid(0));
    }
    cache.pixel_runs.clear();
    if cache.pixel_runs.capacity() < view.viewport.width as usize {
        return Err(limit(LimitKind::PixelRuns));
    }
    let mut first = bounds.x_first;
    let mut current_center = None;
    let mut work = 0;
    for x in bounds.x_first..=bounds.x_last {
        poll(control, WorkKind::ShapePixel)?;
        work = add(work, 1)?;
        let idx = (y as usize) * view.viewport.width as usize + x as usize;
        let center = pixel_fully_in_view(view, x, y, map) && !crate::boundary_test(boundary, idx);
        match current_center {
            None => {
                first = x;
                current_center = Some(center);
            }
            Some(old) if old != center => {
                cache.pixel_runs.push(PixelRun {
                    first_x: first,
                    end_x: x,
                    center_only: old,
                });
                first = x;
                current_center = Some(center);
            }
            _ => {}
        }
    }
    if let Some(center_only) = current_center {
        cache.pixel_runs.push(PixelRun {
            first_x: first,
            end_x: bounds
                .x_last
                .checked_add(1)
                .ok_or_else(|| limit(LimitKind::Arithmetic))?,
            center_only,
        });
    }
    Ok(work)
}
/// Accepted interface spelling for building cached pixel runs.
#[cfg(test)]
pub fn build_runs(
    cache: &mut FillCache,
    view: SceneView<'_>,
    bounds: PixelBounds,
    y: u32,
    boundary: &BoundaryMask,
    map: ViewMapping,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    build_fill_runs(cache, view, bounds, y, boundary, map, resources, control)
}
#[cfg(test)]
pub fn pixel_runs(cache: &FillCache) -> &[PixelRun] {
    &cache.pixel_runs
}

#[cfg(test)]
fn first_sample_x(
    view: SceneView<'_>,
    bounds: PixelBounds,
    map: ViewMapping,
    offset: f64,
    target: f64,
    comparisons: &mut usize,
) -> usize {
    let mut low = bounds.x_first as usize;
    let mut high = bounds.x_last as usize + 1;
    while low < high {
        let mid = low + (high - low) / 2;
        let ux = view.viewport.view_x + ((mid as f64 + offset) - map.offset_x) / map.scale_x;
        *comparisons += 1;
        if ux < target {
            low = mid + 1
        } else {
            high = mid;
        }
    }
    low
}

/// Convert a user-space fill interval to a half-open device pixel sample range.
#[cfg(test)]
pub fn interval_pixel_range(
    view: SceneView<'_>,
    bounds: PixelBounds,
    map: ViewMapping,
    sample_offset: f64,
    interval: FillInterval,
) -> Result<(TableRange, usize)> {
    if !bounds.visible
        || bounds.x_first > bounds.x_last
        || bounds.x_last >= view.viewport.width
        || !map.scale_x.is_finite()
        || map.scale_x <= 0.0
        || !map.offset_x.is_finite()
        || !sample_offset.is_finite()
        || interval.left.is_nan()
        || interval.right.is_nan()
        || interval.right < interval.left
    {
        return Err(invalid(0));
    }
    let mut comparisons = 0;
    let first = if interval.left == f64::NEG_INFINITY {
        bounds.x_first as usize
    } else {
        first_sample_x(
            view,
            bounds,
            map,
            sample_offset,
            interval.left,
            &mut comparisons,
        )
    };
    let end = if interval.right == f64::INFINITY {
        bounds.x_last as usize + 1
    } else {
        first_sample_x(
            view,
            bounds,
            map,
            sample_offset,
            interval.right,
            &mut comparisons,
        )
    };
    if end < first {
        return Err(invalid(0));
    }
    Ok((
        TableRange {
            first,
            count: end - first,
        },
        comparisons,
    ))
}

#[cfg(test)]
pub fn fill_contains(cache: &FillCache, sample_row: usize, x: f64) -> (bool, usize) {
    if sample_row >= 5 || x.is_nan() || cache.interval_stride == 0 {
        return (false, 0);
    }
    let Some(start) = sample_row.checked_mul(cache.interval_stride) else {
        return (false, 0);
    };
    let Some(end) = start.checked_add(cache.interval_stride) else {
        return (false, 0);
    };
    let Some(intervals) = cache.intervals.get(start..end) else {
        return (false, 0);
    };
    let count = cache.interval_counts[sample_row];
    if count > intervals.len() {
        return (false, 0);
    }
    let mut low = 0;
    let mut high = count;
    let mut comparisons = 1;
    while low < high {
        let mid = low + (high - low) / 2;
        comparisons += 1;
        if intervals[mid].right <= x {
            low = mid + 1
        } else {
            high = mid;
        }
    }
    (
        low < count && intervals[low].left <= x && x < intervals[low].right,
        comparisons,
    )
}
