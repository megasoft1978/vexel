use crate::bounds::{
    GeomBounds, checked_area, edge_box, edge_radius, map_bounds, point_box, stroke_padding,
    validate_map,
};
use crate::util::*;
use vexel_scene::{
    JobRasterScratch, LimitKind, Linejoin, PixelBounds, RasterJob, Result, SceneView, ViewMapping,
    WorkControl, WorkKind,
};

/// Job-owned production bounds builder. The destination must be empty; every
/// element is admitted by the caller's RasterJob before it is published.
pub fn build_bounds_job(
    view: SceneView<'_>,
    map: ViewMapping,
    job: &mut RasterJob,
    scratch: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    validate_map(map)?;
    job.validate_source(view.source_owner())?;
    scratch.validate(job, control)?;
    if view.viewport.width == 0
        || view.viewport.height == 0
        || !job.slice(scratch.bounds)?.is_empty()
    {
        return Err(invalid(0));
    }
    job.reserve(scratch.bounds, view.shapes.len(), control)?;
    let mut pixels = 0usize;
    let result = (|| {
        for (i, shape) in view.shapes.iter().enumerate() {
            poll(control, WorkKind::ShapeRowVisit)?;
            let edge_slice = shape
                .edges
                .slice(view.edges)
                .ok_or_else(|| invalid(shape.source.element.start))?;
            let mut geom = GeomBounds::default();
            if shape.filled {
                for edge in edge_slice {
                    poll(control, WorkKind::Edge)?;
                    geom.include(edge.a);
                    geom.include(edge.b);
                }
            }
            if shape.stroke.rgba[3] != 0 && shape.stroke.width > 0.0 && !edge_slice.is_empty() {
                let mut stroke = GeomBounds::default();
                for edge in edge_slice {
                    poll(control, WorkKind::Edge)?;
                    stroke.include(edge.a);
                    stroke.include(edge.b);
                }
                match stroke_padding(view, i) {
                    Some((px, py)) if stroke.present && !stroke.invalid => {
                        stroke.left -= px;
                        stroke.right += px;
                        stroke.top -= py;
                        stroke.bottom += py;
                    }
                    _ => stroke.invalid = true,
                }
                geom.merge(stroke);
            }
            let bound = map_bounds(view, map, geom);
            if bound.pixels.visible {
                let area = checked_area(bound.pixels)?;
                pixels = add(pixels, area)?;
            }
            job.extend(scratch.bounds, &[bound], control)?;
        }
        Ok(())
    })();
    if result.is_err() {
        job.truncate(scratch.bounds, 0)?;
        return result.map(|_| 0);
    }
    Ok(pixels)
}

/// Build a job-owned boundary mask with two allocation-free geometry passes.
/// The caller supplies a fresh boundary handle; the 200M limit is retained in
/// the private RasterJob slot and every marked pixel is charged in-loop.
pub fn mark_boundaries_job(
    view: SceneView<'_>,
    map: ViewMapping,
    job: &mut RasterJob,
    scratch: &JobRasterScratch,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    validate_map(map)?;
    job.validate_source(view.source_owner())?;
    scratch.validate(job, control)?;
    if view.viewport.width == 0
        || view.viewport.height == 0
        || !job.slice(scratch.boundary_bits)?.is_empty()
    {
        return Err(invalid(0));
    }
    let pixels = mul(view.viewport.width as usize, view.viewport.height as usize)?;
    let bytes = add(pixels, 7)? / 8;
    let result = (|| {
        let mut total = 0usize;
        for (i, edge) in view.edges.iter().enumerate() {
            poll(control, WorkKind::Edge)?;
            let shape = view.edge_shape(i).ok_or_else(|| invalid(0))?.index();
            let radius = edge_radius(view, shape, false).unwrap_or(f64::NAN);
            let bounds = if radius.is_finite() {
                edge_box(view, map, *edge, radius)
            } else {
                full_pixels(view)
            };
            total = add_work_checked(total, checked_area(bounds)?)?;
            let s = &view.shapes[shape];
            if s.stroke.rgba[3] != 0 && s.stroke.width > 0.0 && s.stroke.join == Linejoin::Miter {
                let radius =
                    edge_radius(view, shape, true).ok_or_else(|| limit(LimitKind::Work))?;
                for point in [edge.a, edge.b] {
                    poll(control, WorkKind::BoundaryMark)?;
                    let bounds = point_box(view, map, point, radius)
                        .ok_or_else(|| limit(LimitKind::Work))?;
                    total = add_work_checked(total, checked_area(bounds)?)?;
                }
            }
        }
        if total > BOUNDARY_WORK_LIMIT {
            return Err(limit(LimitKind::Work));
        }
        job.reserve(scratch.boundary_bits, bytes, control)?;
        for _ in 0..bytes {
            job.extend(scratch.boundary_bits, &[0], control)?;
        }
        for (i, edge) in view.edges.iter().enumerate() {
            poll(control, WorkKind::Edge)?;
            let shape = view.edge_shape(i).ok_or_else(|| invalid(0))?.index();
            let radius = edge_radius(view, shape, false).unwrap_or(f64::NAN);
            let bounds = if radius.is_finite() {
                edge_box(view, map, *edge, radius)
            } else {
                full_pixels(view)
            };
            mark_job_box(job, scratch, bounds, view.viewport.width, control)?;
            let s = &view.shapes[shape];
            if s.stroke.rgba[3] != 0 && s.stroke.width > 0.0 && s.stroke.join == Linejoin::Miter {
                let radius =
                    edge_radius(view, shape, true).ok_or_else(|| limit(LimitKind::Work))?;
                for point in [edge.a, edge.b] {
                    poll(control, WorkKind::BoundaryMark)?;
                    let bounds = point_box(view, map, point, radius)
                        .ok_or_else(|| limit(LimitKind::Work))?;
                    mark_job_box(job, scratch, bounds, view.viewport.width, control)?;
                }
            }
        }
        Ok(total)
    })();
    if result.is_err() {
        job.truncate(scratch.boundary_bits, 0)?;
    }
    result
}

fn full_pixels(view: SceneView<'_>) -> PixelBounds {
    PixelBounds {
        x_first: 0,
        x_last: view.viewport.width - 1,
        y_first: 0,
        y_last: view.viewport.height - 1,
        visible: true,
    }
}
fn add_work_checked(work: usize, amount: usize) -> Result<usize> {
    let next = add(work, amount)?;
    if next > BOUNDARY_WORK_LIMIT {
        return Err(limit(LimitKind::Work));
    }
    Ok(next)
}
fn mark_job_box(
    job: &mut RasterJob,
    scratch: &JobRasterScratch,
    b: PixelBounds,
    width: u32,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if !b.visible {
        return Ok(());
    }
    let len = job.slice(scratch.boundary_bits)?.len();
    for y in b.y_first..=b.y_last {
        for x in b.x_first..=b.x_last {
            let index = (y as usize)
                .checked_mul(width as usize)
                .and_then(|v| v.checked_add(x as usize))
                .ok_or_else(|| limit(LimitKind::Arithmetic))?;
            if index / 8 >= len {
                return Err(invalid(0));
            }
            job.charge_boundary(scratch.boundary_bits, 1, control)?;
            let byte = &mut job.slice_mut(scratch.boundary_bits)?[index >> 3];
            *byte |= 1 << (index & 7);
        }
    }
    Ok(())
}

/// Unique marked pixels, including correct masking of the final partial byte.
pub fn marked_pixel_count_job(
    job: &RasterJob,
    scratch: &JobRasterScratch,
    bit_count: usize,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    let bytes = add(bit_count, 7)? / 8;
    let data = job.slice(scratch.boundary_bits)?;
    if data.len() != bytes {
        return Err(invalid(0));
    }
    let mut count = 0usize;
    for (i, &byte) in data.iter().enumerate() {
        poll(control, WorkKind::BoundaryMark)?;
        let mask = if i + 1 == bytes && bit_count % 8 != 0 {
            (1u8 << (bit_count % 8)) - 1
        } else {
            u8::MAX
        };
        count = add(count, (byte & mask).count_ones() as usize)?;
    }
    Ok(count)
}
