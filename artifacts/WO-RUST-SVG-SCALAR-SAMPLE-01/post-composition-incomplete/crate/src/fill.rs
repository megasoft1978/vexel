use crate::sample::{RasterSampleInput, paint_sample};
use crate::{FILL_SAMPLE_COUNT, FILL_SAMPLES_PER_AXIS, stopped};
use vexel_scene::{
    AlphaMode, ControlledKernelResources, Diagnostic, ErrorKind, JobBuffer, JobRasterScratch,
    LimitKind, RasterJob, Result, SceneView, ShapeId, ViewMapping, WorkControl, WorkKind,
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
fn in_view(view: SceneView<'_>, x: f64, y: f64, map: ViewMapping) -> bool {
    sample_is_in_view(view.viewport, x, y, map)
}
pub(super) fn sample_is_in_view(viewport: vexel_scene::Viewport, x: f64, y: f64, map: ViewMapping) -> bool {
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
fn zeroed(
    job: &mut RasterJob,
    handle: JobBuffer<u8>,
    len: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if job.capacity(handle)? < len {
        job.reserve(handle, len, control)?;
    }
    job.truncate(handle, 0)?;
    let zeros = [0u8; 256];
    let mut left = len;
    while left != 0 {
        let n = left.min(zeros.len());
        job.extend(handle, &zeros[..n], control)?;
        left -= n;
    }
    Ok(())
}
fn center_only(view: SceneView<'_>, boundary: &[u8], x: usize, y: u32, map: ViewMapping) -> bool {
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
    boundary
        .get(pixel >> 3)
        .is_some_and(|byte| byte & (1 << (pixel & 7)) == 0)
}
fn sample_at(
    view: SceneView<'_>,
    id: ShapeId,
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
    paint_sample(view, id, &input, AlphaMode::SourceOver, control)
}

/// Render one shape row using only buffers admitted to the same RasterJob.
/// Dash validation occurs once in `compose`; each callback then reads the
/// validated dash/grid view beside two disjoint retained buffers.
pub fn fill_row(
    view: SceneView<'_>,
    id: ShapeId,
    y: u32,
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
        || ![map.offset_x, map.offset_y].into_iter().all(f64::is_finite)
    {
        return Err(invalid());
    }
    job.validate_source(view.source_owner())?;
    scratch.validate(job, control)?;
    let width = view.viewport.width as usize;
    let center_len = width.checked_mul(4).ok_or_else(arithmetic)?;
    let sub_len = width
        .checked_mul(FILL_SAMPLE_COUNT)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(arithmetic)?;
    zeroed(job, scratch.center_samples, center_len, control)?;
    zeroed(job, scratch.subpixel_samples, sub_len, control)?;
    let mut composition = job.compose(view.source_owner(), scratch.dash, control)?;
    const LOC: [f64; FILL_SAMPLES_PER_AXIS] = [0.125, 0.375, 0.625, 0.875];
    for x in 0..width {
        control.charge(WorkKind::ShapeRowVisit, 1).map_err(stopped)?;
        let clean = composition.with_buffers(
            scratch.boundary_bits,
            scratch.bounds,
            control,
            |_cache, boundary, _bounds, _meter| Ok(center_only(view, boundary, x, y, map)),
        )?;
        if clean {
            let dx = x as f64 + 0.5;
            let dy = y as f64 + 0.5;
            if in_view(view, dx, dy, map) {
                let color = composition.with_buffers(
                    scratch.local_edges,
                    scratch.dash_seams,
                    control,
                    |cache, edges, seams, meter| {
                        meter.charge(WorkKind::ShapeSample, 1).map_err(stopped)?;
                        let (px, py) = user_point(view, dx, dy, map);
                        sample_at(
                            view, id, resources, cache, edges, seams, px, py, px, py, meter,
                        )
                    },
                )?;
                composition.with_buffers(
                    scratch.center_samples,
                    scratch.subpixel_samples,
                    control,
                    |_cache, center, _sub, _meter| {
                        center[x * 4..x * 4 + 4].copy_from_slice(&color);
                        Ok(())
                    },
                )?;
            }
            continue;
        }
        let paint = user_point(view, x as f64 + 0.5, y as f64 + 0.5, map);
        for sy in 0..FILL_SAMPLES_PER_AXIS {
            for sx in 0..FILL_SAMPLES_PER_AXIS {
                control.charge(WorkKind::ShapeRowVisit, 1).map_err(stopped)?;
                let dx = x as f64 + LOC[sx];
                let dy = y as f64 + LOC[sy];
                if !in_view(view, dx, dy, map) {
                    continue;
                }
                let color = composition.with_buffers(
                    scratch.local_edges,
                    scratch.dash_seams,
                    control,
                    |cache, edges, seams, meter| {
                        meter.charge(WorkKind::ShapeSample, 1).map_err(stopped)?;
                        let (px, py) = user_point(view, dx, dy, map);
                        sample_at(
                            view, id, resources, cache, edges, seams, px, py, paint.0, paint.1,
                            meter,
                        )
                    },
                )?;
                let at = (x * FILL_SAMPLE_COUNT + sy * FILL_SAMPLES_PER_AXIS + sx) * 4;
                composition.with_buffers(
                    scratch.center_samples,
                    scratch.subpixel_samples,
                    control,
                    |_cache, _center, sub, _meter| {
                        sub[at..at + 4].copy_from_slice(&color);
                        Ok(())
                    },
                )?;
            }
        }
    }
    Ok(())
}
