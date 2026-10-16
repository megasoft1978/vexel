use crate::sample::{RasterSampleInput, paint_sample_inside};
use crate::{FILL_SAMPLE_COUNT, FILL_SAMPLES_PER_AXIS, stopped};
use vexel_scene::{
    AlphaMode, ControlledKernelResources, Diagnostic, ErrorKind, FillInterval, JobRasterScratch,
    LimitKind, PixelRun, RasterJob, Result, SceneView, ShapeBounds, ShapeId, ViewMapping,
    WorkControl, WorkKind,
};

fn invalid() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    }
}
fn arithmetic() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(LimitKind::Arithmetic),
        offset: 0,
    }
}

/// Device tile dimensions round fractional viewport extents upward.
pub fn tile_extent_ceil(extent: f64) -> Result<u32> {
    if !extent.is_finite() || extent <= 0.0 {
        return Err(invalid());
    }
    let rounded = extent.ceil();
    if rounded > u32::MAX as f64 {
        return Err(arithmetic());
    }
    Ok(rounded as u32)
}

/// Scalar metadata produced by the lower fill-index builder for one shape row.
/// The arrays and traversal records themselves remain in admitted job buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillRowLayout {
    pub interval_counts: [usize; 5],
    pub interval_stride: usize,
    pub max_edge_count: usize,
    pub run_count: usize,
}

pub(super) fn sample_is_in_view(
    viewport: vexel_scene::Viewport,
    x: f64,
    y: f64,
    map: ViewMapping,
) -> bool {
    x >= map.offset_x
        && y >= map.offset_y
        && x < map.offset_x + viewport.view_width * map.scale_x
        && y < map.offset_y + viewport.view_height * map.scale_y
}
fn user_point(view: SceneView<'_>, x: f64, y: f64, map: ViewMapping) -> (f64, f64) {
    (
        view.viewport.view_x + (x - map.offset_x) / map.scale_x,
        view.viewport.view_y + (y - map.offset_y) / map.scale_y,
    )
}
fn checked_row(
    bounds: ShapeBounds,
    y: u32,
    width: usize,
    intervals: usize,
    crossings: usize,
    runs: usize,
    layout: FillRowLayout,
) -> Result<()> {
    let p = bounds.pixels;
    if !p.visible
        || y < p.y_first
        || y > p.y_last
        || p.x_first > p.x_last
        || p.x_last as usize >= width
        || layout.max_edge_count > crossings
        || layout.run_count == 0
        || layout.run_count > runs
    {
        return Err(invalid());
    }
    let required = layout
        .interval_stride
        .checked_mul(5)
        .ok_or_else(arithmetic)?;
    if required > intervals
        || layout
            .interval_counts
            .iter()
            .any(|n| *n > layout.interval_stride)
    {
        return Err(invalid());
    }
    Ok(())
}

fn interval_at(
    composition: &mut vexel_scene::RasterComposition<'_>,
    scratch: JobRasterScratch,
    index: usize,
    control: &mut dyn WorkControl,
) -> Result<FillInterval> {
    composition.with_buffers(
        scratch.intervals,
        scratch.pixel_runs,
        control,
        |_cache, intervals, _runs, _meter| intervals.get(index).copied().ok_or_else(invalid),
    )
}

fn range_for(
    composition: &mut vexel_scene::RasterComposition<'_>,
    view: SceneView<'_>,
    bounds: ShapeBounds,
    interval: FillInterval,
    sample_offset: f64,
    map: ViewMapping,
    control: &mut dyn WorkControl,
) -> Result<(usize, usize)> {
    if !bounds.pixels.visible
        || !map.scale_x.is_finite()
        || map.scale_x <= 0.0
        || !map.offset_x.is_finite()
        || !sample_offset.is_finite()
        || interval.left.is_nan()
        || interval.right.is_nan()
        || interval.right < interval.left
    {
        return Err(invalid());
    }
    let first = if interval.left == f64::NEG_INFINITY {
        bounds.pixels.x_first as usize
    } else {
        lower_bound(
            composition,
            view,
            bounds,
            interval.left,
            sample_offset,
            map,
            control,
        )?
    };
    let end = if interval.right == f64::INFINITY {
        (bounds.pixels.x_last as usize)
            .checked_add(1)
            .ok_or_else(arithmetic)?
    } else {
        lower_bound(
            composition,
            view,
            bounds,
            interval.right,
            sample_offset,
            map,
            control,
        )?
    };
    if end < first {
        return Err(invalid());
    }
    Ok((first, end))
}
fn lower_bound(
    composition: &mut vexel_scene::RasterComposition<'_>,
    view: SceneView<'_>,
    bounds: ShapeBounds,
    target: f64,
    sample_offset: f64,
    map: ViewMapping,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    let mut low = bounds.pixels.x_first as usize;
    let mut high = (bounds.pixels.x_last as usize)
        .checked_add(1)
        .ok_or_else(arithmetic)?;
    while low < high {
        composition.charge_work(WorkKind::FillIntervalTest, 1, control)?;
        let middle = low + (high - low) / 2;
        let device_x = middle as f64 + sample_offset;
        let user_x = view.viewport.view_x + (device_x - map.offset_x) / map.scale_x;
        if user_x < target {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    Ok(low)
}

fn run_at(
    composition: &mut vexel_scene::RasterComposition<'_>,
    scratch: JobRasterScratch,
    index: usize,
    control: &mut dyn WorkControl,
) -> Result<PixelRun> {
    composition.with_buffers(
        scratch.intervals,
        scratch.pixel_runs,
        control,
        |_cache, _intervals, runs, _meter| runs.get(index).copied().ok_or_else(invalid),
    )
}
fn sample_at(
    view: SceneView<'_>,
    id: ShapeId,
    fill: [u8; 4],
    resources: &dyn ControlledKernelResources,
    cache: vexel_scene::ValidatedDashView<'_>,
    edges: &[vexel_geometry::Edge],
    seams: &[bool],
    px: f64,
    py: f64,
    paint_x: f64,
    paint_y: f64,
    control: &mut dyn WorkControl,
) -> Result<[u8; 4]> {
    let input = RasterSampleInput {
        resources,
        active_edges: &[],
        local_edges: edges,
        dash_seams: seams,
        dash_cache: cache,
        px,
        py,
        paint_x,
        paint_y,
    };
    paint_sample_inside(view, id, fill, &input, AlphaMode::SourceOver, control)
}
fn write_sample(
    composition: &mut vexel_scene::RasterComposition<'_>,
    scratch: JobRasterScratch,
    view: SceneView<'_>,
    id: ShapeId,
    fill: [u8; 4],
    resources: &dyn ControlledKernelResources,
    map: ViewMapping,
    x: usize,
    y: u32,
    dx: f64,
    dy: f64,
    center: bool,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if !sample_is_in_view(view.viewport, dx, dy, map) {
        return Ok(());
    }
    let (px, py) = user_point(view, dx, dy, map);
    let paint = user_point(view, x as f64 + 0.5, y as f64 + 0.5, map);
    let color = composition.with_buffers(
        scratch.local_edges,
        scratch.dash_seams,
        control,
        |cache, edges, seams, meter| {
            meter.charge(WorkKind::ShapeSample, 1).map_err(stopped)?;
            sample_at(
                view,
                id,
                fill,
                resources,
                cache,
                edges,
                seams,
                px,
                py,
                if center { px } else { paint.0 },
                if center { py } else { paint.1 },
                meter,
            )
        },
    )?;
    let at = if center {
        x.checked_mul(4).ok_or_else(arithmetic)?
    } else {
        x.checked_mul(FILL_SAMPLE_COUNT)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(arithmetic)?
    };
    if center {
        composition.with_buffers(
            scratch.center_samples,
            scratch.subpixel_samples,
            control,
            |_cache, center, _sub, _meter| {
                center
                    .get_mut(at..at + 4)
                    .ok_or_else(invalid)?
                    .copy_from_slice(&color);
                Ok(())
            },
        )
    } else {
        Err(invalid())
    }
}

/// Render one fill row from the exact interval/run index stored in same-job buffers.
/// The output handles are scratch and are publishable only after this returns `Ok`;
/// an error may leave a partial row for the owner to clear or discard.
pub fn fill_row(
    view: SceneView<'_>,
    id: ShapeId,
    y: u32,
    layout: FillRowLayout,
    job: &mut RasterJob,
    scratch: JobRasterScratch,
    map: ViewMapping,
    resources: &dyn ControlledKernelResources,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if view.shape(id).is_none()
        || y >= view.viewport.height
        || !map.scale_x.is_finite()
        || !map.scale_y.is_finite()
        || map.scale_x <= 0.0
        || map.scale_y <= 0.0
        || !map.offset_x.is_finite()
        || !map.offset_y.is_finite()
    {
        return Err(invalid());
    }
    job.validate_source(view.source_owner())?;
    scratch.validate(job, control)?;
    let eligible = job
        .slice(scratch.eligible_shapes)?
        .get(id.index())
        .copied()
        .ok_or_else(invalid)?;
    if !eligible {
        return Err(invalid());
    }
    let bounds = job
        .slice(scratch.bounds)?
        .get(id.index())
        .copied()
        .ok_or_else(invalid)?;
    checked_row(
        bounds,
        y,
        view.viewport.width as usize,
        job.slice(scratch.intervals)?.len(),
        job.slice(scratch.crossings)?.len(),
        job.slice(scratch.pixel_runs)?.len(),
        layout,
    )?;
    let width = view.viewport.width as usize;
    let center_len = width.checked_mul(4).ok_or_else(arithmetic)?;
    let sub_len = width
        .checked_mul(FILL_SAMPLE_COUNT)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(arithmetic)?;
    if job.capacity(scratch.center_samples)? < center_len {
        job.reserve(scratch.center_samples, center_len, control)?;
    }
    if job.capacity(scratch.subpixel_samples)? < sub_len {
        job.reserve(scratch.subpixel_samples, sub_len, control)?;
    }
    job.truncate(scratch.center_samples, 0)?;
    job.truncate(scratch.subpixel_samples, 0)?;
    let zeros = [0u8; 256];
    for (buffer, len) in [
        (scratch.center_samples, center_len),
        (scratch.subpixel_samples, sub_len),
    ] {
        let mut left = len;
        while left != 0 {
            let n = left.min(zeros.len());
            job.extend(buffer, &zeros[..n], control)?;
            left -= n;
        }
    }
    let shape = view.shape(id).ok_or_else(invalid)?;
    let fill = *view
        .edge_colors
        .get(shape.edges.first)
        .ok_or_else(invalid)?;
    let mut composition = job.compose(view.source_owner(), scratch.dash, control)?;
    // Frozen center-only phase: slot 2 and only center_only runs.
    for interval_index in 0..layout.interval_counts[2] {
        composition.charge_work(WorkKind::ShapeRowVisit, 1, control)?;
        let index = 2usize
            .checked_mul(layout.interval_stride)
            .and_then(|v| v.checked_add(interval_index))
            .ok_or_else(arithmetic)?;
        let interval = interval_at(&mut composition, scratch, index, control)?;
        let (first, end) = range_for(&mut composition, view, bounds, interval, 0.5, map, control)?;
        for run_index in 0..layout.run_count {
            composition.charge_work(WorkKind::ShapeRowVisit, 1, control)?;
            let run = run_at(&mut composition, scratch, run_index, control)?;
            if !run.center_only {
                continue;
            }
            let first = first.max(run.first_x as usize);
            let end = end.min(run.end_x as usize);
            for x in first..end {
                composition.charge_work(WorkKind::ShapeRowVisit, 1, control)?;
                write_sample(
                    &mut composition,
                    scratch,
                    view,
                    id,
                    fill,
                    resources,
                    map,
                    x,
                    y,
                    x as f64 + 0.5,
                    y as f64 + 0.5,
                    true,
                    control,
                )?;
            }
        }
    }
    const LOC: [f64; FILL_SAMPLES_PER_AXIS] = [0.125, 0.375, 0.625, 0.875];
    for sy in 0..FILL_SAMPLES_PER_AXIS {
        let sample_y = y as f64 + LOC[sy];
        let slot = if sy < 2 { sy } else { sy + 1 };
        for sx in 0..FILL_SAMPLES_PER_AXIS {
            for interval_index in 0..layout.interval_counts[slot] {
                composition.charge_work(WorkKind::ShapeRowVisit, 1, control)?;
                let index = slot
                    .checked_mul(layout.interval_stride)
                    .and_then(|v| v.checked_add(interval_index))
                    .ok_or_else(arithmetic)?;
                let interval = interval_at(&mut composition, scratch, index, control)?;
                let (first, end) = range_for(
                    &mut composition,
                    view,
                    bounds,
                    interval,
                    LOC[sx],
                    map,
                    control,
                )?;
                for run_index in 0..layout.run_count {
                    composition.charge_work(WorkKind::ShapeRowVisit, 1, control)?;
                    let run = run_at(&mut composition, scratch, run_index, control)?;
                    if run.center_only {
                        continue;
                    }
                    let first = first.max(run.first_x as usize);
                    let end = end.min(run.end_x as usize);
                    for x in first..end {
                        composition.charge_work(WorkKind::ShapeRowVisit, 1, control)?;
                        let sample_x = x as f64 + LOC[sx];
                        if !sample_is_in_view(view.viewport, sample_x, sample_y, map) {
                            continue;
                        }
                        let (px, py) = user_point(view, sample_x, sample_y, map);
                        let paint = user_point(view, x as f64 + 0.5, y as f64 + 0.5, map);
                        let at = (x * FILL_SAMPLE_COUNT + sy * FILL_SAMPLES_PER_AXIS + sx) * 4;
                        let color = composition.with_buffers(
                            scratch.local_edges,
                            scratch.dash_seams,
                            control,
                            |cache, edges, seams, meter| {
                                meter.charge(WorkKind::ShapeSample, 1).map_err(stopped)?;
                                sample_at(
                                    view, id, fill, resources, cache, edges, seams, px, py,
                                    paint.0, paint.1, meter,
                                )
                            },
                        )?;
                        composition.with_buffers(
                            scratch.center_samples,
                            scratch.subpixel_samples,
                            control,
                            |_cache, _center, sub, _meter| {
                                sub.get_mut(at..at + 4)
                                    .ok_or_else(invalid)?
                                    .copy_from_slice(&color);
                                Ok(())
                            },
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}
