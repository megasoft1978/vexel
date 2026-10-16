//! Uniform fill eligibility, crossings, intervals and runs in admitted buffers.
use crate::fill::heap_sort;
use crate::job_rows::JobRowsState;
use crate::job_storage::{initialize, restore, save};
use crate::util::*;
use vexel_scene::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JobFillState {
    pub max_edge_count: usize,
    pub interval_stride: usize,
    pub interval_counts: [usize; 5],
    pub run_count: usize,
}

fn uniform(view: SceneView<'_>, shape: usize, control: &mut dyn WorkControl) -> Result<bool> {
    let s = &view.shapes[shape];
    if !s.filled || s.stroke.rgba[3] != 0 && s.stroke.width > 0.0 || s.edges.count == 0 {
        return Ok(false);
    }
    if s.edges.slice(view.edges).is_none() {
        return Ok(false);
    }
    let Some(color) = view.edge_colors.get(s.edges.first) else {
        return Ok(false);
    };
    for i in 1..s.edges.count {
        poll(control, WorkKind::FillEdgeTest)?;
        if view.edge_colors.get(s.edges.first + i) != Some(color) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn create_fill_cache_job(
    view: SceneView<'_>,
    job: &mut RasterJob,
    s: &JobRasterScratch,
    backup: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<JobFillState> {
    job.validate_source(view.source_owner())?;
    s.validate(job, control)?;
    backup.validate(job, control)?;
    if s.crossings == backup.crossings
        || s.intervals == backup.intervals
        || s.pixel_runs == backup.pixel_runs
    {
        return Err(invalid(0));
    }
    if !job.slice(s.eligible_shapes)?.is_empty() {
        return Err(invalid(0));
    }
    job.reserve(s.eligible_shapes, view.shapes.len(), control)?;
    let mut max = 0usize;
    let result = (|| {
        for (i, shape) in view.shapes.iter().enumerate() {
            poll(control, WorkKind::ShapeRowVisit)?;
            let yes = shape.edges.count <= 4096 && uniform(view, i, control)?;
            job.extend(s.eligible_shapes, &[yes], control)?;
            if yes {
                max = max.max(shape.edges.count);
            }
        }
        if max == 0 {
            return Ok(JobFillState::default());
        }
        if view.viewport.width as usize > PIXEL_RUN_LIMIT / std::mem::size_of::<PixelRun>() {
            for i in 0..view.shapes.len() {
                poll(control, WorkKind::ShapeRowVisit)?;
                job.slice_mut(s.eligible_shapes)?[i] = false;
            }
            return Ok(JobFillState::default());
        }
        let stride = add(max, 1)?;
        let cross = FillCrossing {
            x: 0.0,
            winding_delta: 0,
            edge_index: 0,
        };
        let run = PixelRun {
            first_x: 0,
            end_x: 0,
            center_only: false,
        };
        for scratch in [s, backup] {
            initialize(job, scratch.crossings, max, cross, control)?;
            initialize(
                job,
                scratch.intervals,
                mul(stride, 5)?,
                FillInterval::default(),
                control,
            )?;
            job.reserve(scratch.pixel_runs, view.viewport.width as usize, control)?;
            if mul(
                job.capacity(scratch.pixel_runs)?,
                std::mem::size_of::<PixelRun>(),
            )? > PIXEL_RUN_LIMIT
            {
                return Err(limit(LimitKind::PixelRuns));
            }
            initialize(
                job,
                scratch.pixel_runs,
                view.viewport.width as usize,
                run,
                control,
            )?;
        }
        Ok(JobFillState {
            max_edge_count: max,
            interval_stride: stride,
            interval_counts: [0; 5],
            run_count: 0,
        })
    })();
    if result.is_err() {
        job.truncate(s.eligible_shapes, 0)?;
    }
    result
}

pub fn eligible_job(job: &RasterJob, s: &JobRasterScratch, shape: ShapeId) -> Result<bool> {
    Ok(job
        .slice(s.eligible_shapes)?
        .get(shape.index())
        .copied()
        .unwrap_or(false))
}

fn intervals(
    view: SceneView<'_>,
    shape: usize,
    y: f64,
    first: usize,
    end: usize,
    row: usize,
    state: &JobFillState,
    job: &mut RasterJob,
    s: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<(usize, usize, usize)> {
    let active = end
        .checked_sub(first)
        .ok_or_else(|| invalid(source_offset(view, shape)))?;
    if active > state.max_edge_count {
        return Err(limit(LimitKind::Scratch));
    }
    let mut count = 0usize;
    for i in first..end {
        poll(control, WorkKind::FillEdgeTest)?;
        let id = *job
            .slice(s.active_edges)?
            .get(i)
            .ok_or_else(|| invalid(0))?;
        let edge = *view
            .edges
            .get(id as usize)
            .ok_or_else(|| invalid(source_offset(view, shape)))?;
        if view
            .edge_shape(id as usize)
            .is_none_or(|id| id.index() != shape)
        {
            return Err(invalid(source_offset(view, shape)));
        }
        let up = edge.a.y <= y && edge.b.y > y;
        let down = edge.b.y <= y && edge.a.y > y;
        if !up && !down {
            continue;
        }
        let x = edge.a.x + (y - edge.a.y) * (edge.b.x - edge.a.x) / (edge.b.y - edge.a.y);
        if !x.is_finite() {
            return Err(invalid(source_offset(view, shape)));
        }
        job.slice_mut(s.crossings)?[count] = FillCrossing {
            x,
            winding_delta: if up { 1 } else { -1 },
            edge_index: id,
        };
        count += 1;
    }
    if count > 1 {
        heap_sort(&mut job.slice_mut(s.crossings)?[..count], control)?;
    }
    let mut winding = 0i64;
    let mut parity = 0u8;
    for i in 0..count {
        poll(control, WorkKind::FillEdgeTest)?;
        winding += job.slice(s.crossings)?[i].winding_delta as i64;
        parity ^= 1;
    }
    let offset = mul(row, state.interval_stride)?;
    let mut n = 0usize;
    if count != 0
        && (if view.shapes[shape].evenodd {
            parity != 0
        } else {
            winding != 0
        })
    {
        job.slice_mut(s.intervals)?[offset] = FillInterval {
            left: f64::NEG_INFINITY,
            right: job.slice(s.crossings)?[0].x,
        };
        n = 1;
    }
    let mut first = 0usize;
    while first < count {
        poll(control, WorkKind::FillEdgeTest)?;
        let c = job.slice(s.crossings)?[first];
        let mut end = first + 1;
        let mut gw = c.winding_delta as i64;
        let mut gp = 1u8;
        while end < count && job.slice(s.crossings)?[end].x == c.x {
            poll(control, WorkKind::FillEdgeTest)?;
            gw += job.slice(s.crossings)?[end].winding_delta as i64;
            gp ^= 1;
            end += 1;
        }
        winding -= gw;
        parity ^= gp;
        let right = if end < count {
            job.slice(s.crossings)?[end].x
        } else {
            f64::INFINITY
        };
        if (if view.shapes[shape].evenodd {
            parity != 0
        } else {
            winding != 0
        }) && c.x < right
        {
            if n >= state.interval_stride {
                return Err(limit(LimitKind::Scratch));
            }
            job.slice_mut(s.intervals)?[offset + n] = FillInterval { left: c.x, right };
            n += 1;
        }
        first = end;
    }
    let mut log = 0usize;
    let mut power = 1usize;
    while power < count.max(1) {
        poll(control, WorkKind::FillEdgeTest)?;
        power = mul(power, 2)?;
        log += 1;
    }
    Ok((
        n,
        active,
        add(mul(count, add(mul(log, 2)?, 6)?)?, mul(active, 2)?)?,
    ))
}

pub fn build_fill_row_job(
    state: &mut JobFillState,
    view: SceneView<'_>,
    shape: ShapeId,
    y: u32,
    map: ViewMapping,
    rows: &JobRowsState,
    job: &mut RasterJob,
    s: &JobRasterScratch,
    backup: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<FillRowWork> {
    job.validate_source(view.source_owner())?;
    let si = shape.index();
    if view.shape(shape).is_none()
        || s.crossings == backup.crossings
        || s.intervals == backup.intervals
    {
        return Err(invalid(source_offset(view, si)));
    }
    if !eligible_job(job, s, shape)?
        || y >= view.viewport.height
        || !map.scale_y.is_finite()
        || map.scale_y <= 0.0
        || !map.offset_y.is_finite()
    {
        return Err(invalid(source_offset(view, si)));
    }
    let stride = add(view.shapes.len(), 1)?;
    if state.interval_stride == 0
        || job.slice(s.shape_offsets)?.len() < mul(stride, 5)?
        || job.slice(s.crossings)?.len() < state.max_edge_count
        || job.slice(s.intervals)?.len() < mul(state.interval_stride, 5)?
    {
        return Err(limit(LimitKind::Scratch));
    }
    save(job, s.crossings, backup.crossings, control)?;
    save(job, s.intervals, backup.intervals, control)?;
    let mut next = *state;
    let result = (|| {
        let mut work = FillRowWork::default();
        for (sample, offset) in [0.125, 0.375, 0.5, 0.625, 0.875].iter().enumerate() {
            poll(control, WorkKind::FillEdgeTest)?;
            let base = sample * stride + si;
            let first = job.slice(s.shape_offsets)?[base];
            let end = job.slice(s.shape_offsets)?[base + 1];
            if end < first || end > rows.edge_count {
                return Err(invalid(source_offset(view, si)));
            }
            let sy = view.viewport.view_y + ((y as f64 + offset) - map.offset_y) / map.scale_y;
            let (count, visits, units) =
                intervals(view, si, sy, first, end, sample, state, job, s, control)?;
            next.interval_counts[sample] = count;
            work.edge_visits = add(work.edge_visits, visits)?;
            work.work_units = add(work.work_units, units)?;
        }
        Ok(work)
    })();
    if result.is_err() {
        restore(job, backup.crossings, s.crossings)?;
        restore(job, backup.intervals, s.intervals)?;
    } else {
        *state = next;
    }
    result
}

pub fn boundary_test_job(
    job: &RasterJob,
    s: &JobRasterScratch,
    bit_count: usize,
    pixel: usize,
) -> Result<bool> {
    Ok(pixel >= bit_count
        || job
            .slice(s.boundary_bits)?
            .get(pixel >> 3)
            .is_none_or(|byte| byte & (1 << (pixel & 7)) != 0))
}

pub(crate) fn fully_in_view(view: SceneView<'_>, x: u32, y: u32, map: ViewMapping) -> bool {
    x as f64 >= map.offset_x
        && y as f64 >= map.offset_y
        && x as f64 + 1.0 <= map.offset_x + view.viewport.view_width * map.scale_x
        && y as f64 + 1.0 <= map.offset_y + view.viewport.view_height * map.scale_y
}

pub fn build_fill_runs_job(
    state: &mut JobFillState,
    view: SceneView<'_>,
    bounds: PixelBounds,
    y: u32,
    map: ViewMapping,
    job: &mut RasterJob,
    s: &JobRasterScratch,
    backup: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    crate::bounds::validate_map(map)?;
    job.validate_source(view.source_owner())?;
    if !bounds.visible
        || y < bounds.y_first
        || y > bounds.y_last
        || bounds.x_first > bounds.x_last
        || bounds.x_last >= view.viewport.width
        || y >= view.viewport.height
    {
        return Err(invalid(0));
    }
    if s.pixel_runs == backup.pixel_runs {
        return Err(invalid(0));
    }
    if job.slice(s.pixel_runs)?.len() < view.viewport.width as usize {
        return Err(limit(LimitKind::Scratch));
    }
    save(job, s.pixel_runs, backup.pixel_runs, control)?;
    let pixels = mul(view.viewport.width as usize, view.viewport.height as usize)?;
    let result = (|| {
        let mut count = 0usize;
        let mut first = bounds.x_first;
        let mut last_center = None;
        for x in bounds.x_first..=bounds.x_last {
            poll(control, WorkKind::FillIntervalTest)?;
            let pixel = add(mul(y as usize, view.viewport.width as usize)?, x as usize)?;
            let center =
                fully_in_view(view, x, y, map) && !boundary_test_job(job, s, pixels, pixel)?;
            if last_center.is_some_and(|last| last != center) {
                job.slice_mut(s.pixel_runs)?[count] = PixelRun {
                    first_x: first,
                    end_x: x,
                    center_only: last_center.unwrap_or(false),
                };
                count += 1;
                first = x;
            }
            last_center = Some(center);
        }
        job.slice_mut(s.pixel_runs)?[count] = PixelRun {
            first_x: first,
            end_x: bounds
                .x_last
                .checked_add(1)
                .ok_or_else(|| limit(LimitKind::Arithmetic))?,
            center_only: last_center.unwrap_or(false),
        };
        count += 1;
        Ok(count)
    })();
    match result {
        Ok(count) => {
            state.run_count = count;
            Ok(bounds.x_last as usize - bounds.x_first as usize + 1)
        }
        Err(e) => {
            restore(job, backup.pixel_runs, s.pixel_runs)?;
            Err(e)
        }
    }
}

pub fn pixel_runs_job<'a>(
    state: &JobFillState,
    job: &'a RasterJob,
    s: &JobRasterScratch,
) -> Result<&'a [PixelRun]> {
    job.slice(s.pixel_runs)?
        .get(..state.run_count)
        .ok_or_else(|| invalid(0))
}

pub fn fill_contains_job(
    state: &JobFillState,
    job: &RasterJob,
    s: &JobRasterScratch,
    row: usize,
    x: f64,
    control: &mut dyn WorkControl,
) -> Result<(bool, usize)> {
    if row >= 5 || x.is_nan() {
        return Ok((false, 0));
    }
    let offset = mul(row, state.interval_stride)?;
    let count = state.interval_counts[row];
    let data = job
        .slice(s.intervals)?
        .get(offset..add(offset, count)?)
        .ok_or_else(|| invalid(0))?;
    let mut low = 0usize;
    let mut high = count;
    let mut comparisons = 1usize;
    while low < high {
        poll(control, WorkKind::FillIntervalTest)?;
        let mid = low + (high - low) / 2;
        comparisons += 1;
        if data[mid].right <= x {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    Ok((
        low < count && data[low].left <= x && x < data[low].right,
        comparisons,
    ))
}

pub fn interval_pixel_range_controlled(
    view: SceneView<'_>,
    bounds: PixelBounds,
    map: ViewMapping,
    sample: f64,
    interval: FillInterval,
    control: &mut dyn WorkControl,
) -> Result<(TableRange, usize)> {
    if !bounds.visible
        || !map.scale_x.is_finite()
        || map.scale_x <= 0.0
        || !map.offset_x.is_finite()
        || !sample.is_finite()
        || interval.left.is_nan()
        || interval.right.is_nan()
        || interval.right < interval.left
    {
        return Err(invalid(0));
    }
    let mut comparisons = 0usize;
    let mut search = |target: f64| -> Result<usize> {
        let mut low = bounds.x_first as usize;
        let mut high = add(bounds.x_last as usize, 1)?;
        while low < high {
            poll(control, WorkKind::FillIntervalTest)?;
            let mid = low + (high - low) / 2;
            comparisons = add(comparisons, 1)?;
            let x = view.viewport.view_x + ((mid as f64 + sample) - map.offset_x) / map.scale_x;
            if x < target {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        Ok(low)
    };
    let first = if interval.left == f64::NEG_INFINITY {
        bounds.x_first as usize
    } else {
        search(interval.left)?
    };
    let end = if interval.right == f64::INFINITY {
        add(bounds.x_last as usize, 1)?
    } else {
        search(interval.right)?
    };
    Ok((
        TableRange {
            first,
            count: end.checked_sub(first).ok_or_else(|| invalid(0))?,
        },
        comparisons,
    ))
}
