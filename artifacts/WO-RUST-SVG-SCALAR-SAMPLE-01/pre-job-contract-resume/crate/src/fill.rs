use crate::*;
use vexel_scene::{FillInterval, KernelResources, SceneView, ShapeId};

fn in_view(view: SceneView<'_>, x: f64, y: f64, map: ViewMapping) -> bool {
    x >= map.offset_x
        && y >= map.offset_y
        && x < map.offset_x + view.viewport.view_width * map.scale_x
        && y < map.offset_y + view.viewport.view_height * map.scale_y
}
fn user_x(view: SceneView<'_>, x: usize, sample: f64, map: ViewMapping) -> f64 {
    view.viewport.view_x + (x as f64 + sample - map.offset_x) / map.scale_x
}
fn first_at_least(
    view: SceneView<'_>,
    sample: f64,
    target: f64,
    width: usize,
    map: ViewMapping,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    let (mut low, mut high) = (0usize, width);
    while low < high {
        control
            .charge(vexel_scene::WorkKind::FillIntervalTest, 1)
            .map_err(crate::stopped)?;
        let middle = low + (high - low) / 2;
        if user_x(view, middle, sample, map) < target {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    Ok(low)
}
fn pixel_range(
    view: SceneView<'_>,
    interval: FillInterval,
    sample: f64,
    width: usize,
    map: ViewMapping,
    control: &mut dyn WorkControl,
) -> Result<Option<(usize, usize)>> {
    if interval.left.is_nan() || interval.right.is_nan() || interval.right < interval.left {
        return Ok(None);
    }
    let first = if interval.left == f64::NEG_INFINITY {
        0
    } else {
        first_at_least(view, sample, interval.left, width, map, control)?
    };
    let end = if interval.right == f64::INFINITY {
        width
    } else {
        first_at_least(view, sample, interval.right, width, map, control)?
    };
    Ok((end >= first).then_some((first, end)))
}
fn center_only(
    scratch: &RasterScratch,
    view: SceneView<'_>,
    x: usize,
    y: u32,
    map: ViewMapping,
) -> bool {
    let fully_visible = x as f64 >= map.offset_x
        && y as f64 >= map.offset_y
        && x as f64 + 1.0 <= map.offset_x + view.viewport.view_width * map.scale_x
        && y as f64 + 1.0 <= map.offset_y + view.viewport.view_height * map.scale_y;
    if !fully_visible {
        return false;
    }
    let Some(pixel) = (y as usize)
        .checked_mul(view.viewport.width as usize)
        .and_then(|n| n.checked_add(x))
    else {
        return false;
    };
    if pixel >= scratch.boundary.bit_count {
        return false;
    }
    scratch
        .boundary
        .bits
        .get(pixel >> 3)
        .is_some_and(|byte| byte & (1 << (pixel & 7)) == 0)
}
fn input<'a>(
    resources: &'a dyn KernelResources,
    scratch: &'a RasterScratch,
    px: f64,
    py: f64,
    paint_x: f64,
    paint_y: f64,
) -> SampleInput<'a> {
    SampleInput {
        resources,
        active_edges: &[],
        local_edges: &scratch.local_edges,
        dash_seams: &scratch.dash_seams,
        dash_cache: &scratch.dash,
        px,
        py,
        paint_x,
        paint_y,
    }
}
fn sample_center(
    view: SceneView<'_>,
    id: ShapeId,
    x: usize,
    y: u32,
    scratch: &mut RasterScratch,
    map: ViewMapping,
    alpha: AlphaMode,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let dx = x as f64 + 0.5;
    let dy = y as f64 + 0.5;
    if !in_view(view, dx, dy, map) {
        return Ok(());
    }
    control
        .charge(vexel_scene::WorkKind::ShapeSample, 1)
        .map_err(crate::stopped)?;
    let px = view.viewport.view_x + (dx - map.offset_x) / map.scale_x;
    let py = view.viewport.view_y + (dy - map.offset_y) / map.scale_y;
    let sample = input(resources, scratch, px, py, px, py);
    let color = paint_sample(view, id, &sample, alpha, control)?;
    scratch.center_samples[x * 4..x * 4 + 4].copy_from_slice(&color);
    Ok(())
}
fn sample_subpixels(
    view: SceneView<'_>,
    id: ShapeId,
    x: usize,
    y: u32,
    scratch: &mut RasterScratch,
    map: ViewMapping,
    alpha: AlphaMode,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<()> {
    const LOC: [f64; 4] = [0.125, 0.375, 0.625, 0.875];
    let px_center = view.viewport.view_x + (x as f64 + 0.5 - map.offset_x) / map.scale_x;
    let py_center = view.viewport.view_y + (y as f64 + 0.5 - map.offset_y) / map.scale_y;
    for sy in 0..4 {
        for sx in 0..4 {
            let dx = x as f64 + LOC[sx];
            let dy = y as f64 + LOC[sy];
            if !in_view(view, dx, dy, map) {
                continue;
            }
            control
                .charge(vexel_scene::WorkKind::ShapeSample, 1)
                .map_err(crate::stopped)?;
            let px = view.viewport.view_x + (dx - map.offset_x) / map.scale_x;
            let py = view.viewport.view_y + (dy - map.offset_y) / map.scale_y;
            let sample = input(resources, scratch, px, py, px_center, py_center);
            let color = paint_sample(view, id, &sample, alpha, control)?;
            let at = (x * FILL_SAMPLE_COUNT + sy * 4 + sx) * 4;
            scratch.subpixel_samples[at..at + 4].copy_from_slice(&color)
        }
    }
    Ok(())
}
fn scanline_cached(
    view: SceneView<'_>,
    id: ShapeId,
    y: u32,
    scratch: &mut RasterScratch,
    map: ViewMapping,
    alpha: AlphaMode,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(true) = scratch.fill.eligible_shapes.get(id.index()).copied() else {
        return Ok(false);
    };
    let cache = &scratch.fill;
    let runs = &cache.pixel_runs;
    if runs.is_empty() || cache.interval_stride == 0 {
        return Ok(false);
    }
    let needed = cache
        .interval_stride
        .checked_mul(5)
        .ok_or(vexel_scene::Diagnostic {
            kind: vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic),
            offset: 0,
        })?;
    if cache.intervals.len() < needed
        || cache
            .interval_counts
            .iter()
            .any(|n| *n > cache.interval_stride)
        || runs
            .iter()
            .any(|r| r.first_x > r.end_x || r.end_x > view.viewport.width)
    {
        return Ok(false);
    }
    let shape = view.shape(id).ok_or(vexel_scene::Diagnostic {
        kind: vexel_scene::ErrorKind::InvalidArgument,
        offset: 0,
    })?;
    let fill = shape
        .edges
        .slice(view.edge_colors)
        .and_then(|v| v.first())
        .copied()
        .unwrap_or([0; 4]);
    for i in 0..cache.interval_counts[2] {
        control
            .charge(vexel_scene::WorkKind::FillIntervalTest, 1)
            .map_err(crate::stopped)?;
        let interval = cache.intervals[2 * cache.interval_stride + i];
        let Some((a, b)) = pixel_range(
            view,
            interval,
            0.5,
            view.viewport.width as usize,
            map,
            control,
        )?
        else {
            continue;
        };
        for run in runs.iter().filter(|r| r.center_only) {
            let first = a.max(run.first_x as usize);
            let end = b.min(run.end_x as usize);
            for x in first..end {
                if in_view(view, x as f64 + 0.5, y as f64 + 0.5, map) {
                    control
                        .charge(vexel_scene::WorkKind::ShapeSample, 1)
                        .map_err(crate::stopped)?;
                    let px = view.viewport.view_x + (x as f64 + 0.5 - map.offset_x) / map.scale_x;
                    let py = view.viewport.view_y + (y as f64 + 0.5 - map.offset_y) / map.scale_y;
                    let sample = input(resources, scratch, px, py, px, py);
                    let color = paint_sample_inside(view, id, fill, &sample, alpha, control)?;
                    scratch.center_samples[x * 4..x * 4 + 4].copy_from_slice(&color)
                }
            }
        }
    }
    const LOC: [f64; 4] = [0.125, 0.375, 0.625, 0.875];
    for sy in 0..4 {
        let slot = if sy < 2 { sy } else { sy + 1 };
        for sx in 0..4 {
            for i in 0..cache.interval_counts[slot] {
                control
                    .charge(vexel_scene::WorkKind::FillIntervalTest, 1)
                    .map_err(crate::stopped)?;
                let interval = cache.intervals[slot * cache.interval_stride + i];
                let Some((a, b)) = pixel_range(
                    view,
                    interval,
                    LOC[sx],
                    view.viewport.width as usize,
                    map,
                    control,
                )?
                else {
                    continue;
                };
                for run in runs.iter().filter(|r| !r.center_only) {
                    let first = a.max(run.first_x as usize);
                    let end = b.min(run.end_x as usize);
                    for x in first..end {
                        let dx = x as f64 + LOC[sx];
                        let dy = y as f64 + LOC[sy];
                        if !in_view(view, dx, dy, map) {
                            continue;
                        }
                        control
                            .charge(vexel_scene::WorkKind::ShapeSample, 1)
                            .map_err(crate::stopped)?;
                        let px = view.viewport.view_x + (dx - map.offset_x) / map.scale_x;
                        let py = view.viewport.view_y + (dy - map.offset_y) / map.scale_y;
                        let paint_x =
                            view.viewport.view_x + (x as f64 + 0.5 - map.offset_x) / map.scale_x;
                        let paint_y =
                            view.viewport.view_y + (y as f64 + 0.5 - map.offset_y) / map.scale_y;
                        let sample = input(resources, scratch, px, py, paint_x, paint_y);
                        let color = paint_sample_inside(view, id, fill, &sample, alpha, control)?;
                        let at = (x * FILL_SAMPLE_COUNT + sy * 4 + sx) * 4;
                        scratch.subpixel_samples[at..at + 4].copy_from_slice(&color)
                    }
                }
            }
        }
    }
    Ok(true)
}
pub fn fill_row(
    view: SceneView<'_>,
    id: ShapeId,
    y: u32,
    scratch: &mut RasterScratch,
    map: ViewMapping,
    alpha: AlphaMode,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let shape = view.shape(id).ok_or(vexel_scene::Diagnostic {
        kind: vexel_scene::ErrorKind::InvalidArgument,
        offset: 0,
    })?;
    if y >= view.viewport.height
        || !map.scale_x.is_finite()
        || !map.scale_y.is_finite()
        || map.scale_x <= 0.0
        || map.scale_y <= 0.0
        || ![map.offset_x, map.offset_y].into_iter().all(f64::is_finite)
    {
        return Err(vexel_scene::Diagnostic {
            kind: vexel_scene::ErrorKind::InvalidArgument,
            offset: 0,
        });
    }
    let width = view.viewport.width as usize;
    let center_len = width.checked_mul(4).ok_or(vexel_scene::Diagnostic {
        kind: vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic),
        offset: 0,
    })?;
    let sub_len = width
        .checked_mul(FILL_SAMPLE_COUNT)
        .and_then(|n| n.checked_mul(4))
        .ok_or(vexel_scene::Diagnostic {
            kind: vexel_scene::ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic),
            offset: 0,
        })?;
    if scratch.center_samples.len() < center_len {
        scratch
            .center_samples
            .try_reserve_exact(center_len - scratch.center_samples.len())
            .map_err(|_| vexel_scene::Diagnostic {
                kind: vexel_scene::ErrorKind::OutOfMemory,
                offset: 0,
            })?;
        scratch.center_samples.resize(center_len, 0)
    } else {
        scratch.center_samples[..center_len].fill(0)
    }
    if scratch.subpixel_samples.len() < sub_len {
        scratch
            .subpixel_samples
            .try_reserve_exact(sub_len - scratch.subpixel_samples.len())
            .map_err(|_| vexel_scene::Diagnostic {
                kind: vexel_scene::ErrorKind::OutOfMemory,
                offset: 0,
            })?;
        scratch.subpixel_samples.resize(sub_len, 0)
    } else {
        scratch.subpixel_samples[..sub_len].fill(0)
    }
    shape
        .edges
        .slice(&scratch.local_edges)
        .ok_or(vexel_scene::Diagnostic {
            kind: vexel_scene::ErrorKind::InvalidArgument,
            offset: 0,
        })?;
    if scanline_cached(view, id, y, scratch, map, alpha, resources, control)? {
        return Ok(());
    }
    for x in 0..width {
        if center_only(scratch, view, x, y, map) {
            sample_center(view, id, x, y, scratch, map, alpha, resources, control)?
        } else {
            sample_subpixels(view, id, x, y, scratch, map, alpha, resources, control)?
        }
    }
    Ok(())
}
