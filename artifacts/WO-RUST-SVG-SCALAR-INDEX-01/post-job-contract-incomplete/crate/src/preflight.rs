use crate::fill::{build_fill_row, build_fill_runs, eligible, interval_pixel_range, pixel_runs};
use crate::scanline::{build_row, prepare_rows, reset_scanlines};
use crate::util::*;
use vexel_scene::{
    ErrorKind, FillRowWork, KernelResources, LimitKind, RasterScratch, RenderWorkStats, Result,
    SceneView, ShapeId, ViewMapping, WorkControl, WorkKind,
};

const BASE: u64 = 200_000_000;
const RAMP_START: u64 = 1_048_576;
const PER_PIXEL: u64 = 24_576;
const CAP: u64 = 16_000_000_000;
const SAMPLES: [f64; 4] = [0.125, 0.375, 0.625, 0.875];

/// Return the frozen linear ramp with the 16-billion-unit cap.
pub fn work_budget(width: u32, height: u32) -> u64 {
    let pixels = u64::from(width) * u64::from(height);
    BASE.saturating_add(pixels.saturating_sub(RAMP_START).saturating_mul(PER_PIXEL))
        .min(CAP)
}
pub(super) fn work_add(total: &mut usize, amount: usize, budget: u64) -> Result<()> {
    let next = add(*total, amount)?;
    if next as u128 > budget as u128 {
        return Err(limit(LimitKind::Work));
    }
    *total = next;
    Ok(())
}
fn sample_in_view(view: SceneView<'_>, x: f64, y: f64, map: ViewMapping) -> bool {
    let w = view.viewport.view_width * map.scale_x;
    let h = view.viewport.view_height * map.scale_y;
    x >= map.offset_x && y >= map.offset_y && x < map.offset_x + w && y < map.offset_y + h
}
fn pixel_fully_in_view(view: SceneView<'_>, x: u32, y: u32, map: ViewMapping) -> bool {
    let w = view.viewport.view_width * map.scale_x;
    let h = view.viewport.view_height * map.scale_y;
    x as f64 >= map.offset_x
        && y as f64 >= map.offset_y
        && x as f64 + 1.0 <= map.offset_x + w
        && y as f64 + 1.0 <= map.offset_y + h
}
fn active_count(
    rows: &vexel_scene::RowBuffers,
    stride: usize,
    sample: usize,
    shape: usize,
) -> Result<usize> {
    let i = sample
        .checked_mul(stride)
        .and_then(|n| n.checked_add(shape))
        .ok_or_else(|| limit(LimitKind::Arithmetic))?;
    let a = *rows
        .shape_offsets
        .get(i)
        .ok_or_else(|| limit(LimitKind::Scratch))?;
    let b = *rows
        .shape_offsets
        .get(i + 1)
        .ok_or_else(|| limit(LimitKind::Scratch))?;
    b.checked_sub(a).ok_or_else(|| invalid(0))
}
fn aux_work(
    view: SceneView<'_>,
    shape: ShapeId,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    let mut work = 0usize;
    let s = view.shape(shape).ok_or_else(|| invalid(0))?;
    if let Some(clip) = s.clip {
        let c = resources.clip_work_counts(view, clip, control)?;
        work = add(work, add(c.chain_elements, add(c.child_shapes, c.edges)?)?)?;
    }
    if let Some(p) = view.fill_paint(shape) {
        if p.kind != vexel_scene::PaintKind::Solid {
            work = add(work, p.stops.count)?;
            if p.kind == vexel_scene::PaintKind::Pattern {
                work = add(work, 4)?;
            }
        }
    }
    if let Some(p) = view.stroke_paint(shape) {
        if p.kind != vexel_scene::PaintKind::Solid {
            work = add(work, p.stops.count)?;
            if p.kind == vexel_scene::PaintKind::Pattern {
                work = add(work, 4)?;
            }
        }
    }
    Ok(work)
}
fn stroke_work(
    view: SceneView<'_>,
    shape: ShapeId,
    resources: &dyn KernelResources,
    scratch: &RasterScratch,
    x: f64,
    y: f64,
    control: &mut dyn WorkControl,
) -> Result<(usize, bool, bool)> {
    resources.stroke_sample_work(view, shape, &scratch.dash, x, y, control)
}
fn charge(total: &mut usize, amount: usize, budget: u64) -> Result<()> {
    work_add(total, amount, budget)
}

/// Exact legacy conservative preflight, including event, row, fast-fill and scalar costs.
pub fn preflight(
    view: SceneView<'_>,
    scratch: &mut RasterScratch,
    map: ViewMapping,
    stats: &mut RenderWorkStats,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<()> {
    if !map.scale_x.is_finite()
        || map.scale_x <= 0.0
        || !map.scale_y.is_finite()
        || map.scale_y <= 0.0
        || !map.offset_x.is_finite()
        || !map.offset_y.is_finite()
        || scratch.bounds.shapes.len() != view.shapes.len()
        || scratch.stroke_cost.len() < view.shapes.len()
        || scratch.fill.eligible_shapes.len() != view.shapes.len()
        || view.viewport.width == 0
        || view.viewport.height == 0
        || scratch.scanlines.scanline_count
            != mul(view.viewport.height as usize, SCANLINE_VALUES_PER_ROW)?
        || scratch.boundary.bit_count
            != mul(view.viewport.width as usize, view.viewport.height as usize)?
        || scratch.boundary.bits.len() < add(scratch.boundary.bit_count, 7)? / 8
    {
        return Err(invalid(0));
    }
    for (shape, bounds) in scratch.bounds.shapes.iter().enumerate() {
        let p = bounds.pixels;
        if p.visible
            && (p.x_first > p.x_last
                || p.x_last >= view.viewport.width
                || p.y_first > p.y_last
                || p.y_last >= view.viewport.height)
        {
            return Err(invalid(source_offset(view, shape)));
        }
    }
    let width = view.viewport.width as usize;
    let height = view.viewport.height as usize;
    let pixels = mul(width, height)?;
    let budget = work_budget(view.viewport.width, view.viewport.height);
    let mut total = 0usize;
    charge(&mut total, pixels, budget)?;
    charge(&mut total, scratch.bounds.pixel_count, budget)?;
    charge(&mut total, scratch.boundary.mark_work, budget)?;
    if scratch.boundary.mark_work > BOUNDARY_WORK_LIMIT {
        return Err(limit(LimitKind::Work));
    }
    charge(&mut total, view.edges.len(), budget)?;
    charge(&mut total, view.shapes.len(), budget)?;
    let tree_size = view.edges.len().max(view.shapes.len());
    let mut levels = 1usize;
    let mut n = tree_size;
    while n > 1 {
        n = (n + 1) / 2;
        levels = add(levels, 1)?;
    }
    let mut scanline_levels = 1usize;
    n = scratch.scanlines.scanline_count;
    while n > 1 {
        n = (n + 1) / 2;
        scanline_levels = add(scanline_levels, 1)?;
    }
    let mut stroke_setup = 0usize;
    for &c in scratch.stroke_cost.iter().take(view.shapes.len()) {
        poll(control, WorkKind::ShapeRowVisit)?;
        stroke_setup = add(stroke_setup, c)?;
    }
    stroke_setup = add(stroke_setup, scratch.dash.build_steps)?;
    stroke_setup = add(stroke_setup, scratch.dash.grid_cell_count)?;
    stroke_setup = add(stroke_setup, scratch.dash.grid_ref_count)?;
    let event_count = add(
        scratch.scanlines.events.len(),
        scratch.scanlines.shape_events.len(),
    )?;
    let event_work = mul(event_count, add(mul(levels, 6)?, 8)?)?;
    let lookup_work = mul(view.edges.len(), add(mul(scanline_levels, 2)?, 1)?)?;
    charge(&mut total, event_work, budget)?;
    charge(&mut total, lookup_work, budget)?;
    charge(&mut total, stroke_setup, budget)?;
    prepare_rows(&scratch.scanlines, &mut scratch.rows)?;
    reset_scanlines(&mut scratch.scanlines);
    let stride = add(view.shapes.len(), 1)?;
    let mut shape_pixels = 0usize;
    let mut shape_samples = 0usize;
    let mut fill_edge_tests = 0usize;
    let mut fill_interval_tests = 0usize;
    let mut stroke_tests = 0usize;
    let mut shape_row_visits = 0usize;
    for y in 0..view.viewport.height {
        poll(control, WorkKind::ShapePixel)?;
        let listed_before = scratch.scanlines.listed_edge_count;
        let active_shape_count = build_row(
            &mut scratch.scanlines,
            view,
            y,
            &mut scratch.rows,
            resources,
            control,
        )?;
        charge(
            &mut total,
            scratch.scanlines.listed_edge_count - listed_before,
            budget,
        )?;
        shape_row_visits = add(shape_row_visits, active_shape_count)?;
        charge(&mut total, mul(active_shape_count, 6)?, budget)?;
        for ai in 0..active_shape_count {
            poll(control, WorkKind::ShapeRowVisit)?;
            let shape = scratch.rows.active_shapes[ai] as usize;
            let bounds = scratch
                .bounds
                .shapes
                .get(shape)
                .ok_or_else(|| invalid(0))?
                .pixels;
            if !bounds.visible || y < bounds.y_first || y > bounds.y_last {
                return Err(invalid(source_offset(view, shape)));
            }
            let first_edge = view.shapes[shape].edges.first;
            let shape_id = view
                .edge_shape(first_edge)
                .ok_or_else(|| invalid(source_offset(view, shape)))?;
            let aux = aux_work(view, shape_id, resources, control)?;
            let row_work = if eligible(&scratch.fill, shape_id) {
                match build_fill_row(
                    &mut scratch.fill,
                    view,
                    shape_id,
                    y,
                    map,
                    &scratch.rows,
                    resources,
                    control,
                ) {
                    Ok(w) => Some(w),
                    Err(e)
                        if matches!(
                            e.kind,
                            ErrorKind::Canceled
                                | ErrorKind::OutOfMemory
                                | ErrorKind::Limit(
                                    LimitKind::Work | LimitKind::Arithmetic | LimitKind::Scratch
                                )
                        ) =>
                    {
                        return Err(e);
                    }
                    Err(_) => None,
                }
            } else {
                None
            };
            let run_work = if row_work.is_some() {
                match build_fill_runs(
                    &mut scratch.fill,
                    view,
                    bounds,
                    y,
                    &scratch.boundary,
                    map,
                    resources,
                    control,
                ) {
                    Ok(w) => Some(w),
                    Err(e)
                        if matches!(
                            e.kind,
                            ErrorKind::Canceled
                                | ErrorKind::OutOfMemory
                                | ErrorKind::Limit(
                                    LimitKind::Work | LimitKind::Arithmetic | LimitKind::Scratch
                                )
                        ) =>
                    {
                        return Err(e);
                    }
                    Err(_) => None,
                }
            } else {
                None
            };
            let fast = row_work.is_some() && run_work.is_some();
            if fast {
                let FillRowWork {
                    edge_visits,
                    work_units,
                } = row_work.ok_or_else(|| invalid(source_offset(view, shape)))?;
                let run_units = run_work.ok_or_else(|| invalid(source_offset(view, shape)))?;
                fill_edge_tests = add(fill_edge_tests, edge_visits)?;
                charge(&mut total, work_units, budget)?;
                charge(&mut total, run_units, budget)?;
                let runs = pixel_runs(&scratch.fill);
                if runs.is_empty() {
                    return Err(invalid(source_offset(view, shape)));
                }
                let width_box = (bounds.x_last - bounds.x_first) as usize + 1;
                shape_pixels = add(shape_pixels, width_box)?;
                if shape_pixels > scratch.bounds.pixel_count {
                    return Err(invalid(source_offset(view, shape)));
                }
                let sample_row = 2usize;
                let intervals = &scratch.fill.intervals[sample_row * scratch.fill.interval_stride
                    ..sample_row * scratch.fill.interval_stride
                        + scratch.fill.interval_counts[sample_row]];
                for interval in intervals {
                    poll(control, WorkKind::FillIntervalTest)?;
                    let (range, comparisons) =
                        interval_pixel_range(view, bounds, map, 0.5, *interval)?;
                    fill_interval_tests = add(fill_interval_tests, comparisons)?;
                    charge(&mut total, comparisons, budget)?;
                    for run in runs {
                        poll(control, WorkKind::FillIntervalTest)?;
                        fill_interval_tests = add(fill_interval_tests, 1)?;
                        charge(&mut total, 1, budget)?;
                        if !run.center_only {
                            continue;
                        }
                        let first = range.first.max(run.first_x as usize);
                        let end = (range.first + range.count).min(run.end_x as usize);
                        for x in first..end {
                            poll(control, WorkKind::FillIntervalTest)?;
                            fill_interval_tests = add(fill_interval_tests, 1)?;
                            charge(&mut total, 1, budget)?;
                            let sx = x as f64 + 0.5;
                            let sy = y as f64 + 0.5;
                            if !sample_in_view(view, sx, sy, map) {
                                continue;
                            }
                            shape_samples = add(shape_samples, 1)?;
                            charge(&mut total, add(1, aux)?, budget)?;
                        }
                    }
                }
                for (syi, &sample_y_offset) in SAMPLES.iter().enumerate() {
                    let sample_y = y as f64 + sample_y_offset;
                    let slot = if syi < 2 { syi } else { syi + 1 };
                    for &sample_x_offset in &SAMPLES {
                        poll(control, WorkKind::FillIntervalTest)?;
                        let row = slot;
                        let intervals = &scratch.fill.intervals[row * scratch.fill.interval_stride
                            ..row * scratch.fill.interval_stride
                                + scratch.fill.interval_counts[row]];
                        for interval in intervals {
                            poll(control, WorkKind::FillIntervalTest)?;
                            let (range, comparisons) = interval_pixel_range(
                                view,
                                bounds,
                                map,
                                sample_x_offset,
                                *interval,
                            )?;
                            fill_interval_tests = add(fill_interval_tests, comparisons)?;
                            charge(&mut total, comparisons, budget)?;
                            for run in runs {
                                poll(control, WorkKind::FillIntervalTest)?;
                                fill_interval_tests = add(fill_interval_tests, 1)?;
                                charge(&mut total, 1, budget)?;
                                if run.center_only {
                                    continue;
                                }
                                let first = range.first.max(run.first_x as usize);
                                let end = (range.first + range.count).min(run.end_x as usize);
                                for x in first..end {
                                    poll(control, WorkKind::FillIntervalTest)?;
                                    fill_interval_tests = add(fill_interval_tests, 1)?;
                                    charge(&mut total, 1, budget)?;
                                    let sample_x = x as f64 + sample_x_offset;
                                    if !sample_in_view(view, sample_x, sample_y, map) {
                                        continue;
                                    }
                                    shape_samples = add(shape_samples, 1)?;
                                    charge(&mut total, add(1, aux)?, budget)?;
                                }
                            }
                        }
                    }
                }
                continue;
            }
            for x in bounds.x_first..=bounds.x_last {
                poll(control, WorkKind::ShapePixel)?;
                shape_pixels = add(shape_pixels, 1)?;
                if shape_pixels > scratch.bounds.pixel_count {
                    return Err(invalid(source_offset(view, shape)));
                }
                let pixel = (y as usize) * width + x as usize;
                let crisp = view.shapes[shape].stroke.crisp_edges;
                let center_only = pixel_fully_in_view(view, x, y, map)
                    && (crisp || !crate::boundary_test(&scratch.boundary, pixel));
                if center_only {
                    let sample_x = x as f64 + 0.5;
                    let sample_y = y as f64 + 0.5;
                    let edges = active_count(&scratch.rows, stride, 2, shape)?;
                    let (mut sw, indexed, candidates) = stroke_work(
                        view,
                        shape_id,
                        resources,
                        scratch,
                        view.viewport.view_x + (sample_x - map.offset_x) / map.scale_x,
                        view.viewport.view_y + (sample_y - map.offset_y) / map.scale_y,
                        control,
                    )?;
                    if !view.shapes[shape].filled && indexed && !candidates {
                        stroke_tests = add(stroke_tests, sw)?;
                        charge(&mut total, sw, budget)?;
                        continue;
                    }
                    if !view.shapes[shape].filled && indexed {
                        sw = add(sw, 1)?;
                    }
                    let sample_work = add(add(add(1, edges)?, sw)?, aux)?;
                    shape_samples = add(shape_samples, 1)?;
                    fill_edge_tests = add(fill_edge_tests, edges)?;
                    stroke_tests = add(stroke_tests, sw)?;
                    charge(&mut total, sample_work, budget)?;
                    continue;
                }
                for (syi, &sample_y_offset) in SAMPLES.iter().enumerate() {
                    poll(control, WorkKind::ShapeSample)?;
                    let sample_y = y as f64 + sample_y_offset;
                    let slot = if syi < 2 { syi } else { syi + 1 };
                    let edges = active_count(&scratch.rows, stride, slot, shape)?;
                    for &sample_x_offset in &SAMPLES {
                        let sample_x = x as f64 + sample_x_offset;
                        if !sample_in_view(view, sample_x, sample_y, map) {
                            continue;
                        }
                        let (mut sw, indexed, candidates) = stroke_work(
                            view,
                            shape_id,
                            resources,
                            scratch,
                            view.viewport.view_x + (sample_x - map.offset_x) / map.scale_x,
                            view.viewport.view_y + (sample_y - map.offset_y) / map.scale_y,
                            control,
                        )?;
                        if !view.shapes[shape].filled && indexed && !candidates {
                            stroke_tests = add(stroke_tests, sw)?;
                            charge(&mut total, sw, budget)?;
                            continue;
                        }
                        if !view.shapes[shape].filled && indexed {
                            sw = add(sw, 1)?;
                        }
                        let sample_work = add(add(add(1, edges)?, sw)?, aux)?;
                        shape_samples = add(shape_samples, 1)?;
                        fill_edge_tests = add(fill_edge_tests, edges)?;
                        stroke_tests = add(stroke_tests, sw)?;
                        charge(&mut total, sample_work, budget)?;
                    }
                }
            }
        }
    }
    if shape_pixels != scratch.bounds.pixel_count {
        return Err(invalid(0));
    }
    let mut result = *stats;
    result.shape_pixels = u64::try_from(shape_pixels).map_err(|_| limit(LimitKind::Arithmetic))?;
    result.shape_samples =
        u64::try_from(shape_samples).map_err(|_| limit(LimitKind::Arithmetic))?;
    result.fill_edge_tests =
        u64::try_from(fill_edge_tests).map_err(|_| limit(LimitKind::Arithmetic))?;
    result.fill_interval_tests =
        u64::try_from(fill_interval_tests).map_err(|_| limit(LimitKind::Arithmetic))?;
    result.stroke_tests = u64::try_from(stroke_tests).map_err(|_| limit(LimitKind::Arithmetic))?;
    result.scanline_edge_entries = u64::try_from(scratch.scanlines.listed_edge_count)
        .map_err(|_| limit(LimitKind::Arithmetic))?;
    result.shape_row_visits =
        u64::try_from(shape_row_visits).map_err(|_| limit(LimitKind::Arithmetic))?;
    result.boundary_mark_work =
        u64::try_from(scratch.boundary.mark_work).map_err(|_| limit(LimitKind::Arithmetic))?;
    result.total_work = u64::try_from(total).map_err(|_| limit(LimitKind::Arithmetic))?;
    *stats = result;
    Ok(())
}
