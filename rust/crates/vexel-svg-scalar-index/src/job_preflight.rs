//! Exact frozen render work preflight over the actual admitted engine.
use crate::job_fill::fully_in_view;
use crate::util::*;
use crate::*;
use vexel_scene::*;
const SAMPLES: [f64; 4] = [0.125, 0.375, 0.625, 0.875];

fn charge(total: &mut usize, units: usize, budget: u64) -> Result<()> {
    crate::preflight::work_add(total, units, budget)
}
fn in_view(view: SceneView<'_>, x: f64, y: f64, m: ViewMapping) -> bool {
    x >= m.offset_x
        && y >= m.offset_y
        && x < m.offset_x + view.viewport.view_width * m.scale_x
        && y < m.offset_y + view.viewport.view_height * m.scale_y
}
fn active(index: &ScalarIndex, job: &RasterJob, sample: usize, shape: usize) -> Result<usize> {
    let base = add(mul(sample, add(index.scanlines.shape_count, 1)?)?, shape)?;
    let data = job.slice(index.scratch.shape_offsets)?;
    let a = *data.get(base).ok_or_else(|| limit(LimitKind::Scratch))?;
    let b = *data
        .get(base + 1)
        .ok_or_else(|| limit(LimitKind::Scratch))?;
    if b > index.rows.edge_count {
        return Err(invalid(0));
    }
    b.checked_sub(a).ok_or_else(|| invalid(0))
}
fn aux(
    view: SceneView<'_>,
    shape: ShapeId,
    resources: &dyn ControlledKernelResources,
    control: &mut dyn WorkControl,
) -> Result<usize> {
    let mut units = 0usize;
    let s = view.shape(shape).ok_or_else(|| invalid(0))?;
    if let Some(clip) = s.clip {
        let c = resources.clip_work_counts(view, clip, control)?;
        units = add(units, add(c.chain_elements, add(c.child_shapes, c.edges)?)?)?;
    }
    for paint in [view.fill_paint(shape), view.stroke_paint(shape)]
        .into_iter()
        .flatten()
    {
        poll(control, WorkKind::PaintVisit)?;
        if paint.kind != PaintKind::Solid {
            units = add(units, paint.stops.count)?;
            if paint.kind == PaintKind::Pattern {
                units = add(units, 4)?;
            }
        }
    }
    Ok(units)
}
#[derive(Default)]
struct Counts {
    pixels: usize,
    samples: usize,
    edges: usize,
    intervals: usize,
    stroke: usize,
    rows: usize,
}
fn fast_intervals(
    index: &ScalarIndex,
    view: SceneView<'_>,
    bounds: PixelBounds,
    y: u32,
    row: usize,
    sx: f64,
    sy: f64,
    center: bool,
    m: ViewMapping,
    aux: usize,
    job: &RasterJob,
    total: &mut usize,
    counts: &mut Counts,
    budget: u64,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let offset = mul(row, index.fill.interval_stride)?;
    let data = job
        .slice(index.scratch.intervals)?
        .get(offset..add(offset, index.fill.interval_counts[row])?)
        .ok_or_else(|| invalid(0))?;
    let runs = pixel_runs_job(&index.fill, job, &index.scratch)?;
    for &interval in data {
        poll(control, WorkKind::FillIntervalTest)?;
        let (range, comparisons) =
            interval_pixel_range_controlled(view, bounds, m, sx, interval, control)?;
        counts.intervals = add(counts.intervals, comparisons)?;
        charge(total, comparisons, budget)?;
        for run in runs {
            poll(control, WorkKind::FillIntervalTest)?;
            counts.intervals = add(counts.intervals, 1)?;
            charge(total, 1, budget)?;
            if run.center_only != center {
                continue;
            }
            let first = range.first.max(run.first_x as usize);
            let end = add(range.first, range.count)?.min(run.end_x as usize);
            for x in first..end {
                poll(control, WorkKind::FillIntervalTest)?;
                counts.intervals = add(counts.intervals, 1)?;
                charge(total, 1, budget)?;
                if !in_view(view, x as f64 + sx, sy, m) {
                    continue;
                }
                counts.samples = add(counts.samples, 1)?;
                charge(total, add(1, aux)?, budget)?;
            }
        }
    }
    let _ = y;
    Ok(())
}
fn scalar_sample(
    index: &ScalarIndex,
    view: SceneView<'_>,
    shape: ShapeId,
    sample: usize,
    sx: f64,
    sy: f64,
    m: ViewMapping,
    aux: usize,
    job: &RasterJob,
    dash: JobDashView<'_>,
    resources: &dyn ControlledKernelResources,
    total: &mut usize,
    counts: &mut Counts,
    budget: u64,
    control: &mut dyn WorkControl,
) -> Result<()> {
    poll(control, WorkKind::ShapeSample)?;
    let edges = active(index, job, sample, shape.index())?;
    let (mut sw, indexed, candidates) = resources.stroke_sample_work(
        view,
        shape,
        dash,
        view.viewport.view_x + (sx - m.offset_x) / m.scale_x,
        view.viewport.view_y + (sy - m.offset_y) / m.scale_y,
        control,
    )?;
    let filled = view.shape(shape).ok_or_else(|| invalid(0))?.filled;
    if !filled && indexed && !candidates {
        counts.stroke = add(counts.stroke, sw)?;
        return charge(total, sw, budget);
    }
    if !filled && indexed {
        sw = add(sw, 1)?;
    }
    counts.samples = add(counts.samples, 1)?;
    counts.edges = add(counts.edges, edges)?;
    counts.stroke = add(counts.stroke, sw)?;
    charge(total, add(add(add(1, edges)?, sw)?, aux)?, budget)
}

/// Publishes stats only on success. Any failure restores all index/row/fill
/// payload and scalar state using capacity admitted before row processing.
pub fn preflight_job(
    view: SceneView<'_>,
    index: &mut ScalarIndex,
    m: ViewMapping,
    job: &mut RasterJob,
    stats: &mut RenderWorkStats,
    resources: &dyn ControlledKernelResources,
    control: &mut dyn WorkControl,
) -> Result<()> {
    crate::bounds::validate_map(m)?;
    job.validate_source(view.source_owner())?;
    index.validate_view(view, m)?;
    index.scratch.validate(job, control)?;
    if view.viewport.width == 0
        || view.viewport.height == 0
        || job.slice(index.scratch.bounds)?.len() != view.shapes.len()
        || job.slice(index.scratch.stroke_cost)?.len() != view.shapes.len()
        || job.slice(index.scratch.eligible_shapes)?.len() != view.shapes.len()
    {
        return Err(invalid(0));
    }
    let pixels = mul(view.viewport.width as usize, view.viewport.height as usize)?;
    if job.slice(index.scratch.boundary_bits)?.len() != add(pixels, 7)? / 8 {
        return Err(invalid(0));
    }
    validate_scanlines_job(&index.scanlines, job, &index.scratch, control)?;
    for (shape, b) in job.slice(index.scratch.bounds)?.iter().enumerate() {
        poll(control, WorkKind::ShapeRowVisit)?;
        let p = b.pixels;
        if p.visible
            && (p.x_first > p.x_last
                || p.x_last >= view.viewport.width
                || p.y_first > p.y_last
                || p.y_last >= view.viewport.height)
        {
            return Err(invalid(source_offset(view, shape)));
        }
    }
    index.save_preflight(job, control)?;
    let old_scanlines = index.scanlines;
    let old_rows = index.rows;
    let old_fill = index.fill;
    let result = (|| {
        let budget = work_budget(view.viewport.width, view.viewport.height);
        let mut total = 0usize;
        let boundary_work = job.boundary_work(index.scratch.boundary_bits)?;
        for amount in [
            pixels,
            index.pixel_count,
            boundary_work,
            view.edges.len(),
            view.shapes.len(),
        ] {
            charge(&mut total, amount, budget)?;
        }
        let mut levels = 1usize;
        let mut n = view.edges.len().max(view.shapes.len());
        while n > 1 {
            poll(control, WorkKind::ScanlineEdgeEntry)?;
            n = add(n, 1)? / 2;
            levels = add(levels, 1)?;
        }
        let mut scan_levels = 1usize;
        n = index.scanlines.scanline_count;
        while n > 1 {
            poll(control, WorkKind::ScanlineEdgeEntry)?;
            n = add(n, 1)? / 2;
            scan_levels = add(scan_levels, 1)?;
        }
        let mut setup = 0usize;
        for &cost in job.slice(index.scratch.stroke_cost)? {
            poll(control, WorkKind::StrokeTest)?;
            setup = add(setup, cost)?;
        }
        setup = add(setup, index.scratch.dash.build_steps)?;
        let dash = job.dash_view(index.scratch.dash, control)?;
        for range in dash.shapes {
            poll(control, WorkKind::StrokeTest)?;
            let (cells, refs) = dash.grid(range)?;
            setup = add(setup, add(cells.len(), refs.len())?)?;
        }
        let events = add(
            job.slice(index.scratch.scanline_events)?.len(),
            job.slice(index.scratch.shape_events)?.len(),
        )?;
        charge(&mut total, mul(events, add(mul(levels, 6)?, 8)?)?, budget)?;
        charge(
            &mut total,
            mul(view.edges.len(), add(mul(scan_levels, 2)?, 1)?)?,
            budget,
        )?;
        charge(&mut total, setup, budget)?;
        index.reset(job, control)?;
        let mut counts = Counts::default();
        for y in 0..view.viewport.height {
            poll(control, WorkKind::ShapePixel)?;
            let listed = index.scanlines.listed_edge_count;
            let active_shapes = index.build_row(view, y, job, control)?;
            charge(
                &mut total,
                index
                    .scanlines
                    .listed_edge_count
                    .checked_sub(listed)
                    .ok_or_else(|| invalid(0))?,
                budget,
            )?;
            counts.rows = add(counts.rows, active_shapes)?;
            charge(&mut total, mul(active_shapes, 6)?, budget)?;
            for ai in 0..active_shapes {
                poll(control, WorkKind::ShapeRowVisit)?;
                let shape = job.slice(index.scratch.active_shapes)?[ai] as usize;
                let bounds = job
                    .slice(index.scratch.bounds)?
                    .get(shape)
                    .ok_or_else(|| invalid(0))?
                    .pixels;
                if !bounds.visible || y < bounds.y_first || y > bounds.y_last {
                    return Err(invalid(source_offset(view, shape)));
                }
                let id = view
                    .edge_shape(view.shapes[shape].edges.first)
                    .ok_or_else(|| invalid(source_offset(view, shape)))?;
                let aux = aux(view, id, resources, control)?;
                let fast = eligible_job(job, &index.scratch, id)?;
                let row_work = if fast {
                    match index.build_fill_row(view, id, y, m, job, control) {
                        Ok(w) => Some(w),
                        Err(e) if e.kind == ErrorKind::InvalidArgument => None,
                        Err(e) => return Err(e),
                    }
                } else {
                    None
                };
                let run_work = if row_work.is_some() {
                    match index.build_fill_runs(view, bounds, y, m, job, control) {
                        Ok(w) => Some(w),
                        Err(e) if e.kind == ErrorKind::InvalidArgument => None,
                        Err(e) => return Err(e),
                    }
                } else {
                    None
                };
                if let (Some(w), Some(r)) = (row_work, run_work) {
                    if index.fill.run_count == 0 {
                        return Err(invalid(0));
                    }
                    counts.edges = add(counts.edges, w.edge_visits)?;
                    charge(&mut total, w.work_units, budget)?;
                    charge(&mut total, r, budget)?;
                    counts.pixels = add(
                        counts.pixels,
                        bounds.x_last as usize - bounds.x_first as usize + 1,
                    )?;
                    if counts.pixels > index.pixel_count {
                        return Err(invalid(0));
                    }
                    fast_intervals(
                        index,
                        view,
                        bounds,
                        y,
                        2,
                        0.5,
                        y as f64 + 0.5,
                        true,
                        m,
                        aux,
                        job,
                        &mut total,
                        &mut counts,
                        budget,
                        control,
                    )?;
                    for (i, &sy) in SAMPLES.iter().enumerate() {
                        let row = if i < 2 { i } else { i + 1 };
                        for &sx in &SAMPLES {
                            poll(control, WorkKind::FillIntervalTest)?;
                            fast_intervals(
                                index,
                                view,
                                bounds,
                                y,
                                row,
                                sx,
                                y as f64 + sy,
                                false,
                                m,
                                aux,
                                job,
                                &mut total,
                                &mut counts,
                                budget,
                                control,
                            )?;
                        }
                    }
                    continue;
                }
                // Reuse one actual borrowed dash view throughout this shape
                // row's read-only samples; never revalidate the whole cache for
                // each individual subpixel call.
                let dash = job.dash_view(index.scratch.dash, control)?;
                for x in bounds.x_first..=bounds.x_last {
                    poll(control, WorkKind::ShapePixel)?;
                    counts.pixels = add(counts.pixels, 1)?;
                    if counts.pixels > index.pixel_count {
                        return Err(invalid(source_offset(view, shape)));
                    }
                    let pixel = add(mul(y as usize, view.viewport.width as usize)?, x as usize)?;
                    let center = fully_in_view(view, x, y, m)
                        && (view.shapes[shape].stroke.crisp_edges
                            || !boundary_test_job(job, &index.scratch, pixels, pixel)?);
                    if center {
                        scalar_sample(
                            index,
                            view,
                            id,
                            2,
                            x as f64 + 0.5,
                            y as f64 + 0.5,
                            m,
                            aux,
                            job,
                            dash,
                            resources,
                            &mut total,
                            &mut counts,
                            budget,
                            control,
                        )?;
                    } else {
                        for (i, &sy) in SAMPLES.iter().enumerate() {
                            let row = if i < 2 { i } else { i + 1 };
                            for &sx in &SAMPLES {
                                poll(control, WorkKind::ShapeSample)?;
                                let (px, py) = (x as f64 + sx, y as f64 + sy);
                                if in_view(view, px, py, m) {
                                    scalar_sample(
                                        index,
                                        view,
                                        id,
                                        row,
                                        px,
                                        py,
                                        m,
                                        aux,
                                        job,
                                        dash,
                                        resources,
                                        &mut total,
                                        &mut counts,
                                        budget,
                                        control,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
        if counts.pixels != index.pixel_count {
            return Err(invalid(0));
        }
        let mut next = *stats;
        next.shape_pixels = counts.pixels as u64;
        next.shape_samples = counts.samples as u64;
        next.fill_edge_tests = counts.edges as u64;
        next.fill_interval_tests = counts.intervals as u64;
        next.stroke_tests = counts.stroke as u64;
        next.scanline_edge_entries = index.scanlines.listed_edge_count as u64;
        next.shape_row_visits = counts.rows as u64;
        next.boundary_mark_work = boundary_work as u64;
        next.total_work = total as u64;
        *stats = next;
        Ok(())
    })();
    if result.is_err() {
        index.restore_preflight(job)?;
        index.scanlines = old_scanlines;
        index.rows = old_rows;
        index.fill = old_fill;
    }
    result
}
