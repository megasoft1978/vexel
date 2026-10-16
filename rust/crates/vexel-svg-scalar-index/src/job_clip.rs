//! Conservative controlled clip boundaries. All staging buffers belong to the job.
use crate::bounds::{checked_area, validate_map};
use crate::job_storage::{initialize, restore, save};
use crate::util::*;
use vexel_scene::*;

fn box_for(view: SceneView<'_>, map: ViewMapping, edge: Edge) -> Option<PixelBounds> {
    let x1 = map.offset_x + (edge.a.x - view.viewport.view_x) * map.scale_x;
    let x2 = map.offset_x + (edge.b.x - view.viewport.view_x) * map.scale_x;
    let y1 = map.offset_y + (edge.a.y - view.viewport.view_y) * map.scale_y;
    let y2 = map.offset_y + (edge.b.y - view.viewport.view_y) * map.scale_y;
    if ![x1, x2, y1, y2].iter().all(|x| x.is_finite()) {
        return None;
    }
    let eps = x1.abs().max(x2.abs()).max(y1.abs()).max(y2.abs()).max(1.0) * f64::EPSILON * 8.0;
    match (
        boundary_pixel_range(x1.min(x2) - eps, x1.max(x2) + eps, view.viewport.width),
        boundary_pixel_range(y1.min(y2) - eps, y1.max(y2) + eps, view.viewport.height),
    ) {
        (Some((x_first, x_last)), Some((y_first, y_last))) => Some(PixelBounds {
            x_first,
            x_last,
            y_first,
            y_last,
            visible: true,
        }),
        _ => Some(PixelBounds::default()),
    }
}
fn capped(total: usize, amount: usize) -> Result<usize> {
    let n = add(total, amount)?;
    if n > BOUNDARY_WORK_LIMIT {
        return Err(limit(LimitKind::Work));
    }
    Ok(n)
}
fn collect(
    view: SceneView<'_>,
    clip: ClipId,
    cap: usize,
    job: &mut RasterJob,
    edges: JobBuffer<Edge>,
    resources: &dyn ControlledKernelResources,
    control: &mut dyn WorkControl,
) -> Result<(usize, bool)> {
    let mut count = 0usize;
    let mut error = None;
    let mut visitor = |edge: Edge| {
        if count >= cap {
            error = Some(invalid(0));
            return false;
        }
        match job.slice_mut(edges) {
            Ok(data) => {
                data[count] = edge;
                count += 1;
                true
            }
            Err(e) => {
                error = Some(e);
                false
            }
        }
    };
    // The actual provider owns traversal control; the visitor performs one
    // fixed-size write. Pixel loops run later with this same control.
    let valid = resources.clip_boundary_edges(view, clip, &mut visitor, control)?;
    if let Some(e) = error {
        return Err(e);
    }
    Ok((count, valid))
}
fn mark_box(
    job: &mut RasterJob,
    bits: JobBuffer<u8>,
    box_: PixelBounds,
    width: u32,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if !box_.visible {
        return Ok(());
    }
    for y in box_.y_first..=box_.y_last {
        for x in box_.x_first..=box_.x_last {
            let pixel = add(mul(y as usize, width as usize)?, x as usize)?;
            job.charge_boundary(bits, 1, control)?;
            let byte = job
                .slice_mut(bits)?
                .get_mut(pixel >> 3)
                .ok_or_else(|| invalid(0))?;
            *byte |= 1 << (pixel & 7);
        }
    }
    Ok(())
}
fn full(
    job: &mut RasterJob,
    bits: JobBuffer<u8>,
    pixels: usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    for pixel in 0..pixels {
        job.charge_boundary(bits, 1, control)?;
        job.slice_mut(bits)?[pixel >> 3] |= 1 << (pixel & 7);
    }
    Ok(())
}

/// Errors restore bits; incurred work and actual allocation history remain
/// recorded. A failed whole-index build releases this mask's lifetime.
pub fn mark_clip_boundaries_job(
    view: SceneView<'_>,
    map: ViewMapping,
    job: &mut RasterJob,
    s: &JobRasterScratch,
    backup: &JobRasterScratch,
    resources: &dyn ControlledKernelResources,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    validate_map(map)?;
    job.validate_source(view.source_owner())?;
    s.validate(job, control)?;
    backup.validate(job, control)?;
    let pixels = mul(view.viewport.width as usize, view.viewport.height as usize)?;
    let bytes = add(pixels, 7)? / 8;
    if pixels == 0
        || job.slice(s.boundary_bits)?.len() != bytes
        || s.boundary_bits == backup.boundary_bits
    {
        return Err(invalid(0));
    }
    let old_work = job.boundary_work(s.boundary_bits)?;
    let mut has_clip = false;
    for shape in view.shapes {
        poll(control, WorkKind::ShapeRowVisit)?;
        has_clip |= shape.clip.is_some();
    }
    if !has_clip {
        return Ok(old_work);
    }
    initialize(job, backup.boundary_bits, bytes, 0, control)?;
    initialize(job, backup.shape_offsets, view.shapes.len(), 0, control)?;
    initialize(job, backup.active_shapes, view.shapes.len(), 0, control)?;
    let mut header = mul(view.shapes.len(), 3)?;
    let mut max_edges = 0usize;
    for (i, shape) in view.shapes.iter().enumerate() {
        poll(control, WorkKind::ShapeRowVisit)?;
        if let Some(clip) = shape.clip {
            let counts = resources.clip_work_counts(view, clip, control)?;
            let visits = add(counts.chain_elements, counts.child_shapes)?;
            header = add(header, add(counts.chain_elements, mul(visits, 2)?)?)?;
            max_edges = max_edges.max(counts.edges);
            job.slice_mut(backup.shape_offsets)?[i] = counts.edges;
        }
    }
    let mut total = capped(old_work, header)?;
    initialize(
        job,
        backup.local_edges,
        max_edges,
        Edge {
            a: Point { x: 0.0, y: 0.0 },
            b: Point { x: 0.0, y: 0.0 },
        },
        control,
    )?;
    let mut force_full = false;
    let mut edge_work = 0usize;
    for (i, shape) in view.shapes.iter().enumerate() {
        poll(control, WorkKind::ShapeRowVisit)?;
        let Some(clip) = shape.clip else {
            continue;
        };
        let cap = job.slice(backup.shape_offsets)?[i];
        let (count, valid) = collect(view, clip, cap, job, backup.local_edges, resources, control)?;
        job.slice_mut(backup.active_shapes)?[i] =
            u32::try_from(count).map_err(|_| limit(LimitKind::Arithmetic))?;
        for j in 0..count {
            poll(control, WorkKind::BoundaryMark)?;
            let box_ = box_for(view, map, job.slice(backup.local_edges)?[j]);
            let work = match box_ {
                Some(b) => add(2, checked_area(b)?)?,
                None => {
                    force_full = true;
                    2
                }
            };
            total = capped(total, work)?;
            edge_work = add(edge_work, work)?;
        }
        if !valid {
            force_full = true;
        }
        if force_full {
            break;
        }
    }
    if force_full {
        total = capped(total, pixels)?;
    }
    save(job, s.boundary_bits, backup.boundary_bits, control)?;
    let result = (|| {
        job.charge_boundary(s.boundary_bits, header, control)?;
        if force_full {
            // Original preflight includes predicted boxes from the visited
            // prefix even when invalid mapping later forces a full mask.
            job.charge_boundary(s.boundary_bits, edge_work, control)?;
            full(job, s.boundary_bits, pixels, control)?;
            return Ok(total);
        }
        let mut charged_edges = 0usize;
        for (i, shape) in view.shapes.iter().enumerate() {
            poll(control, WorkKind::ShapeRowVisit)?;
            let Some(clip) = shape.clip else {
                continue;
            };
            let cap = job.slice(backup.shape_offsets)?[i];
            let (count, valid) =
                collect(view, clip, cap, job, backup.local_edges, resources, control)?;
            if valid && count != job.slice(backup.active_shapes)?[i] as usize {
                return Err(invalid(shape.source.element.start));
            }
            let mut invalid_mapping = !valid;
            for j in 0..count {
                poll(control, WorkKind::BoundaryMark)?;
                let box_ = box_for(view, map, job.slice(backup.local_edges)?[j]);
                if box_.is_none() {
                    invalid_mapping = true;
                }
                if invalid_mapping {
                    continue;
                }
                let b = box_.ok_or_else(|| invalid(0))?;
                job.charge_boundary(s.boundary_bits, 2, control)?;
                mark_box(job, s.boundary_bits, b, view.viewport.width, control)?;
                charged_edges = add(charged_edges, add(2, checked_area(b)?)?)?;
            }
            if invalid_mapping {
                // The second mapping can also be invalid; retain the complete
                // first-pass cost and add exactly one full-mask scan.
                let remaining = edge_work
                    .checked_sub(charged_edges)
                    .ok_or_else(|| invalid(0))?;
                job.charge_boundary(s.boundary_bits, remaining, control)?;
                total = capped(total, pixels)?;
                full(job, s.boundary_bits, pixels, control)?;
                return Ok(total);
            }
        }
        if charged_edges != edge_work {
            return Err(invalid(0));
        }
        Ok(total)
    })();
    if result.is_err() {
        restore(job, backup.boundary_bits, s.boundary_bits)?;
    }
    result
}
