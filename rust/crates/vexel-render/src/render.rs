//! Shape orchestration: per pixel 4x4 samples, per shape source-over
//! (C `raster_shape_render_flat` with solid paint only).

use crate::bounds::{PixelBounds, shape_bounds};
use crate::clip::ClipList;
use crate::composite::{SAMPLE_COUNT, resolve_samples, source_over};
use crate::dash::Pattern;
use crate::fill::{Crossings, inside};
use crate::pattern::{PatternPaint, sample as sample_pattern};
use crate::pattern_boundary::{mark_boundaries, pixel_fully_in_view};
use crate::scan::{AxisTables, SAMPLE_LOCATIONS, Sweep};
use crate::stroke::{self, StrokePrims};
use crate::validate::validate;
use crate::{GradientRef, RenderError, RenderShape, SAMPLES_PER_AXIS, ViewMap};
use vexel_geometry::{Affine, Point, affine_map_point};
use vexel_raster::Tile;
use vexel_svg_paint::PreparedPaint;

/// Per-shape state shared by every sample.
struct ShapePass<'a, 'b> {
    shape: &'a RenderShape<'b>,
    prims: Option<StrokePrims>,
    inverse: Option<Affine>,
    fill_visible: bool,
    clips: ClipList<'a>,
    fill_pattern: Option<PatternPaint<'a>>,
    stroke_pattern: Option<PatternPaint<'a>>,
    /// Prepared gradient of each channel; the outer `None` means solid paint, an
    /// inner `None` a gradient that can never paint.
    fill_gradient: Option<Option<PreparedPaint<'a>>>,
    stroke_gradient: Option<Option<PreparedPaint<'a>>>,
}

/// Pattern paints of one shape (fill, stroke); `None` means solid paint.
pub(crate) type ShapePaints<'a> = (Option<PatternPaint<'a>>, Option<PatternPaint<'a>>);

/// Rasterizes `shapes` in document order into the empty `tile`.
///
/// `tile` must be fully transparent black and match `view`'s size. `control`
/// is polled once per shape and per pixel row; returning true aborts with
/// [`RenderError::Cancelled`], leaving the tile contents unspecified.
pub fn render_shapes(
    shapes: &[RenderShape<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    render_core(shapes, &[], &[], &[], view, tile, control)
}

/// A shape plus its `stroke-dasharray` values (document units, before the
/// shape transform as for the stroke width). `StrokeStyle::dash_offset` is
/// the `stroke-dashoffset`. An empty slice, a negative or non-finite entry or
/// an all-zero pattern renders solid; an odd count repeats; more than
/// [`crate::MAX_DASH_VALUES`] entries is [`RenderError::LimitExceeded`].
#[derive(Debug, Clone, Copy)]
pub struct DashedShape<'a> {
    pub shape: RenderShape<'a>,
    pub dashes: &'a [f64],
}

/// Same contract as [`render_shapes`], with dashed strokes. With every
/// `dashes` slice empty the output is byte-identical to [`render_shapes`].
pub fn render_dashed_shapes(
    shapes: &[DashedShape<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let mut plain: Vec<RenderShape<'_>> = Vec::new();
    plain.try_reserve_exact(shapes.len()).map_err(|_| RenderError::LimitExceeded)?;
    let mut patterns: Vec<Option<Pattern>> = Vec::new();
    patterns.try_reserve_exact(shapes.len()).map_err(|_| RenderError::LimitExceeded)?;
    for item in shapes {
        let offset = item.shape.stroke.map_or(0.0, |s| s.dash_offset);
        patterns.push(Pattern::new(item.dashes, offset)?);
        plain.push(item.shape);
    }
    render_core(&plain, &patterns, &[], &[], view, tile, control)
}

/// [`render_shapes`] with a per-shape clip chain: `clips` is empty or parallel
/// to `shapes`, and a sample paints a shape only inside every clip of its list.
pub(crate) fn render_shapes_clipped(
    shapes: &[RenderShape<'_>],
    clips: &[ClipList<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    render_core(shapes, &[], clips, &[], view, tile, control)
}

/// [`render_dashed_shapes`] limited to the pixel rectangle `scissor`
/// (`[x0, y0, x1, y1]`, end exclusive). Pixels inside it equal a full render;
/// pixels outside are left untouched. The tile is not checked to be empty, so
/// disjoint scissors may be painted into one tile one after another.
pub fn render_dashed_shapes_in(
    shapes: &[DashedShape<'_>],
    view: ViewMap,
    tile: &mut Tile,
    scissor: [u32; 4],
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let mut plain: Vec<RenderShape<'_>> = Vec::new();
    plain.try_reserve_exact(shapes.len()).map_err(|_| RenderError::LimitExceeded)?;
    let mut patterns: Vec<Option<Pattern>> = Vec::new();
    patterns.try_reserve_exact(shapes.len()).map_err(|_| RenderError::LimitExceeded)?;
    for item in shapes {
        let offset = item.shape.stroke.map_or(0.0, |s| s.dash_offset);
        patterns.push(Pattern::new(item.dashes, offset)?);
        plain.push(item.shape);
    }
    render_core_scissored(&plain, &patterns, &[], &[], view, tile, control, false, Some(scissor))
}

/// [`render_shapes_clipped`] into a freshly allocated transparent `tile`.
pub(crate) fn render_shapes_fresh(
    shapes: &[RenderShape<'_>],
    clips: &[ClipList<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    render_core_checked(shapes, &[], clips, &[], view, tile, control, false)
}

/// A shape that survived validation, with everything its rows share.
struct Prep<'a, 'b> {
    pass: ShapePass<'a, 'b>,
    bounds: PixelBounds,
    fill_only: bool,
    /// Resolved colour of a pixel whose 16 samples all paint the fill.
    full_pixel: Option<[u8; 4]>,
}

/// Read-only state every band of one render shares.
struct Shared<'a, 'b, 'c> {
    view: &'c ViewMap,
    tables: &'c AxisTables,
    boundary: Option<&'c crate::pattern_boundary::BoundaryMask>,
    preps: &'c [Prep<'a, 'b>],
    width: usize,
}

/// Fewer shaded pixels than this render on the calling thread.
const PARALLEL_MIN_PIXELS: u64 = 60_000;
/// Rows per work item when a render is split across threads.
const BAND_ROWS: usize = 16;
const MAX_THREADS: usize = 8;

/// [`render_shapes`] with optional per-shape pattern paints; `paints` is empty
/// (all solid) or parallel to `shapes`.
///
/// Large renders split the tile into row bands drawn by several threads. Every
/// pixel is computed by the same operations on the same inputs in the same
/// shape order, so the bytes equal a single-threaded render.
pub(crate) fn render_core(
    shapes: &[RenderShape<'_>],
    patterns: &[Option<Pattern>],
    clips: &[ClipList<'_>],
    paints: &[ShapePaints<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    render_core_checked(shapes, patterns, clips, paints, view, tile, control, true)
}

/// [`render_core`]; `check_clear` false skips the transparent-tile scan for a
/// tile the caller just allocated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_core_checked(
    shapes: &[RenderShape<'_>],
    patterns: &[Option<Pattern>],
    clips: &[ClipList<'_>],
    paints: &[ShapePaints<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
    check_clear: bool,
) -> Result<(), RenderError> {
    render_core_scissored(shapes, patterns, clips, paints, view, tile, control, check_clear, None)
}

/// [`render_core_checked`] restricted to the pixel rectangle `scissor`.
#[allow(clippy::too_many_arguments)]
fn render_core_scissored(
    shapes: &[RenderShape<'_>],
    patterns: &[Option<Pattern>],
    clips: &[ClipList<'_>],
    paints: &[ShapePaints<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
    check_clear: bool,
    scissor: Option<[u32; 4]>,
) -> Result<(), RenderError> {
    if !clips.is_empty() && clips.len() != shapes.len() {
        return Err(RenderError::InvalidGeometry);
    }
    let mut budget = validate(shapes, patterns, &view, tile)?;
    if check_clear && !crate::composite::all_zero(tile.rgba()) {
        return Err(RenderError::TileNotEmpty);
    }
    let width = tile.width() as usize;
    let height = tile.height() as usize;
    let tables = AxisTables::new(&view)?;
    let boundary = if paints.iter().any(|(fill, stroke)| fill.is_some() || stroke.is_some()) {
        Some(mark_boundaries(shapes, &view)?)
    } else {
        None
    };
    let mut preps = Vec::new();
    let mut pixels = 0u64;
    for (shape_index, shape) in shapes.iter().enumerate() {
        let (fill_pattern, stroke_pattern) =
            paints.get(shape_index).copied().unwrap_or((None, None));
        if control() {
            return Err(RenderError::Cancelled);
        }
        let stroke_gradient = shape.stroke_gradient.is_some();
        let style =
            shape.stroke.filter(|s| stroke::is_painted(s, stroke_gradient) && !shape.edges.is_empty());
        let patterned = fill_pattern.is_some() || stroke_pattern.is_some();
        let inverse =
            if style.is_some() || patterned { stroke::invert(shape.transform) } else { None };
        let Some(bounds) = shape_bounds(shape, &view, inverse)? else {
            continue;
        };
        let Some(bounds) = scissor.map_or(Some(bounds), |s| bounds.within(s)) else {
            continue;
        };
        let prims = match (style.as_ref(), inverse) {
            (Some(style), Some(inverse)) => {
                let pattern = patterns.get(shape_index).and_then(Option::as_ref);
                Some(StrokePrims::build(shape, style, inverse, pattern, &mut budget)?)
            }
            _ => None,
        };
        let pass = ShapePass {
            shape,
            prims,
            inverse,
            fill_visible: shape.filled
                && (fill_pattern.is_some()
                    || shape.fill_gradient.is_some()
                    || shape.fill_rgba[3] != 0),
            clips: clips.get(shape_index).copied().unwrap_or(&[]),
            fill_pattern,
            stroke_pattern,
            fill_gradient: shape.fill_gradient.map(prepare_gradient),
            stroke_gradient: shape.stroke_gradient.map(prepare_gradient),
        };
        // Without stroke or pattern paint only the fill can touch a pixel, so
        // rows and spans with no crossing can be skipped outright.
        let fill_only = pass.prims.is_none()
            && fill_pattern.is_none()
            && stroke_pattern.is_none()
            && shape.fill_gradient.is_none();
        if fill_only && !pass.fill_visible {
            continue;
        }
        let full_pixel = (fill_only && pass.clips.is_empty()).then(|| {
            let mut sample = [0u8; 4];
            source_over(&mut sample, shape.fill_rgba);
            resolve_samples(&[sample; SAMPLE_COUNT])
        });
        pixels += u64::from(bounds.x_last - bounds.x_first + 1)
            * u64::from(bounds.y_last - bounds.y_first + 1);
        preps.push(Prep { pass, bounds, fill_only, full_pixel });
    }
    let shared = Shared { view: &view, tables: &tables, boundary: boundary.as_ref(), preps: &preps, width };
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(MAX_THREADS);
    if threads < 2 || pixels < PARALLEL_MIN_PIXELS || height < 2 * BAND_ROWS {
        return render_band(&shared, 0..height as u32, tile.rgba_mut(), control);
    }
    render_parallel(&shared, tile.rgba_mut(), control, threads)
}

/// Draws the row bands of `rgba` on `threads` threads; the calling thread
/// also polls `control` and tells the others to stop.
fn render_parallel(
    shared: &Shared<'_, '_, '_>,
    rgba: &mut [u8],
    control: &dyn Fn() -> bool,
    threads: usize,
) -> Result<(), RenderError> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    let stop = AtomicBool::new(false);
    let bands: Vec<(u32, &mut [u8])> = rgba
        .chunks_mut(BAND_ROWS * shared.width * 4)
        .enumerate()
        .map(|(i, chunk)| ((i * BAND_ROWS) as u32, chunk))
        .collect();
    let queue = Mutex::new(bands.into_iter().rev().collect::<Vec<_>>());
    let first_error = Mutex::new(None);
    let work = |poll: &dyn Fn() -> bool| loop {
        let Some((first, chunk)) = queue.lock().ok().and_then(|mut q| q.pop()) else {
            return;
        };
        let rows = (chunk.len() / (shared.width * 4)) as u32;
        let poll_or_stop = || stop.load(Ordering::Relaxed) || poll();
        if let Err(error) = render_band(shared, first..first + rows, chunk, &poll_or_stop) {
            stop.store(true, Ordering::Relaxed);
            if let Ok(mut slot) = first_error.lock() {
                slot.get_or_insert(error);
            }
            return;
        }
    };
    std::thread::scope(|scope| {
        for _ in 1..threads {
            scope.spawn(|| work(&|| false));
        }
        work(control);
    });
    match first_error.into_inner() {
        Ok(Some(error)) => Err(error),
        _ => Ok(()),
    }
}

/// Renders the pixel rows `rows` of every shape into `rgba`, whose first byte
/// is the first pixel of row `rows.start`.
fn render_band(
    shared: &Shared<'_, '_, '_>,
    rows: std::ops::Range<u32>,
    rgba: &mut [u8],
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let Shared { view, tables, boundary, preps, width } = *shared;
    let view = view.clone();
    let mut sweep = Sweep::default();
    let mut crossings: [Crossings; SAMPLES_PER_AXIS] = Default::default();
    let mut candidates: Vec<u32> = Vec::new();
    for prep in preps {
        let Prep { pass, bounds, fill_only, full_pixel } = prep;
        let (fill_only, full_pixel) = (*fill_only, *full_pixel);
        let shape = pass.shape;
        let y_first = bounds.y_first.max(rows.start);
        let y_last = bounds.y_last.min(rows.end.saturating_sub(1));
        if rows.is_empty() || y_first > y_last {
            continue;
        }
        if control() {
            return Err(RenderError::Cancelled);
        }
        sweep.reset(shape.edges, tables, y_first, y_last)?;
        for y in y_first..=y_last {
            if control() {
                return Err(RenderError::Cancelled);
            }
            let row_base = (y - rows.start) as usize * width;
            // Edges that can span some sample row of this pixel row.
            sweep.advance(y);
            let mut sample_y = [0.0f64; SAMPLES_PER_AXIS];
            let mut row_in_view = [false; SAMPLES_PER_AXIS];
            for (sy, location_y) in SAMPLE_LOCATIONS.iter().enumerate() {
                let device_y = f64::from(y) + location_y;
                row_in_view[sy] = view.y_in_view(device_y);
                sample_y[sy] = view.to_user(0.0, device_y).1;
                if pass.fill_visible {
                    crossings[sy].collect(shape.edges, &sweep.active, sample_y[sy]);
                }
            }
            let (mut x_first, mut x_last) = (bounds.x_first, bounds.x_last);
            if fill_only {
                let mut span: Option<(f64, f64)> = None;
                let mut left_empty = true;
                for row in &crossings {
                    if let (Some(first), Some(last)) = (row.first_x(), row.last_x()) {
                        span =
                            Some(span.map_or((first, last), |(a, b)| (a.min(first), b.max(last))));
                        left_empty &= row.outside_on_left(shape.evenodd);
                    }
                }
                let Some((low, high)) = span else {
                    continue;
                };
                // One pixel of slack beyond the exact crossing columns.
                let high_pixel = view.to_device(high, 0.0).0.floor() as i64 + 2;
                if high_pixel < i64::from(x_first) {
                    continue;
                }
                x_last = x_last.min(high_pixel.min(i64::from(u32::MAX)) as u32);
                if left_empty {
                    let low_pixel = view.to_device(low, 0.0).0.floor() as i64 - 2;
                    if low_pixel > i64::from(x_last) {
                        continue;
                    }
                    x_first = x_first.max(low_pixel.max(0) as u32);
                }
            }
            let mut cursors = [[0usize; 2]; SAMPLES_PER_AXIS];
            for x in x_first..=x_last {
                if let Some(full) = full_pixel {
                    // Samples 0 and 3 of a row bracket the other two (positions
                    // only grow), so equal crossing counts left of both mean one
                    // state for the whole row of samples.
                    let (left, left_in_view) = tables.column(x, 0);
                    let (right, right_in_view) = tables.column(x, SAMPLES_PER_AXIS - 1);
                    let mut uniform = left_in_view && right_in_view;
                    let (mut all_in, mut all_out) = (true, true);
                    for sy in 0..SAMPLES_PER_AXIS {
                        if !uniform {
                            break;
                        }
                        let row = &crossings[sy];
                        let at_left = row.advance(cursors[sy][0], left);
                        let at_right = row.advance(cursors[sy][1], right);
                        cursors[sy] = [at_left, at_right];
                        uniform = row_in_view[sy] && at_left == at_right;
                        let covered = row.inside_after(shape.evenodd, at_left);
                        all_in &= covered;
                        all_out &= !covered;
                    }
                    if uniform && all_out {
                        continue;
                    }
                    if uniform && all_in {
                        let offset = (row_base + x as usize) * 4;
                        let pixel: &mut [u8; 4] = (&mut rgba[offset..offset + 4])
                            .try_into()
                            .map_err(|_| RenderError::InvalidGeometry)?;
                        source_over(pixel, full);
                        continue;
                    }
                }
                // C samples pattern paint once at the centre of pixels that no
                // edge touches (see `pattern_boundary`).
                let center_only = (pass.fill_pattern.is_some() || pass.stroke_pattern.is_some())
                    && boundary.is_some_and(|mask| {
                        pixel_fully_in_view(&view, x, y) && !mask.is_marked(x, y)
                    });
                let mut samples = [[0u8; 4]; SAMPLE_COUNT];
                if center_only {
                    let (sample_x, sample_y) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                    if view.sample_in_view(sample_x, sample_y) {
                        let (px, py) = view.to_user(sample_x, sample_y);
                        let mut center = [0u8; 4];
                        let fill_inside =
                            || inside(shape.edges, &sweep.active, shape.evenodd, px, py);
                        let local = map_local(pass, px, py);
                        paint_sample(pass, fill_inside, px, py, local, None, &mut center);
                        let offset = (row_base + x as usize) * 4;
                        let pixel: &mut [u8; 4] = (&mut rgba[offset..offset + 4])
                            .try_into()
                            .map_err(|_| RenderError::InvalidGeometry)?;
                        source_over(pixel, center);
                    }
                    continue;
                }
                // Stroke hit tests only need the primitives near this pixel's
                // samples: their local bounding box comes from the four corner samples.
                let mut culled = false;
                if let Some(prims) = pass.prims.as_ref().filter(|p| p.has_boxes()) {
                    let mut rect = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
                    let mut known = true;
                    for (sy, sx) in [(0, 0), (0, 3), (3, 0), (3, 3)] {
                        let corner = map_local(pass, tables.column(x, sx).0, sample_y[sy]);
                        if let Some(p) = corner {
                            rect = [rect[0].min(p.x), rect[1].min(p.y), rect[2].max(p.x), rect[3].max(p.y)];
                        } else {
                            known = false;
                        }
                    }
                    if known {
                        prims.candidates_in(rect, &mut candidates);
                        culled = true;
                        if candidates.is_empty() && !pass.fill_visible {
                            continue;
                        }
                    }
                }
                for sy in 0..SAMPLES_PER_AXIS {
                    if !row_in_view[sy] {
                        continue;
                    }
                    for sx in 0..SAMPLES_PER_AXIS {
                        let (px, column_in_view) = tables.column(x, sx);
                        if !column_in_view {
                            continue;
                        }
                        let fill_inside = || crossings[sy].inside(shape.evenodd, px);
                        let index = sy * SAMPLES_PER_AXIS + sx;
                        paint_sample(
                            pass,
                            fill_inside,
                            px,
                            sample_y[sy],
                            map_local(pass, px, sample_y[sy]),
                            culled.then_some(candidates.as_slice()),
                            &mut samples[index],
                        );
                    }
                }
                let source = resolve_samples(&samples);
                let offset = (row_base + x as usize) * 4;
                let pixel: &mut [u8; 4] = (&mut rgba[offset..offset + 4])
                    .try_into()
                    .map_err(|_| RenderError::InvalidGeometry)?;
                source_over(pixel, source);
            }
        }
    }
    Ok(())
}

/// Document point mapped into the stroke's local space, when the shape needs it.
fn map_local(pass: &ShapePass<'_, '_>, px: f64, py: f64) -> Option<Point> {
    pass.inverse.and_then(|inverse| affine_map_point(inverse, Point { x: px, y: py }))
}

/// C `paint_sample_result`: fill first, then stroke on top; a pattern paint
/// replaces the solid colour of its channel.
fn paint_sample(
    pass: &ShapePass<'_, '_>,
    fill_inside: impl FnOnce() -> bool,
    px: f64,
    py: f64,
    local: Option<Point>,
    candidates: Option<&[u32]>,
    out: &mut [u8; 4],
) {
    let shape = pass.shape;
    if !pass.clips.iter().all(|clip| clip.contains(px, py)) {
        return;
    }
    if pass.fill_visible && fill_inside() {
        match (pass.fill_pattern, shape.fill_gradient) {
            (Some(paint), _) => {
                if let Some(sampled) = local.and_then(|p| sample_pattern(&paint, p))
                    && sampled[3] != 0
                {
                    source_over(out, sampled);
                }
            }
            (None, Some(_)) => {
                let sampled = sample_gradient(pass.fill_gradient.flatten(), px, py);
                if sampled[3] != 0 {
                    source_over(out, sampled);
                }
            }
            (None, None) => source_over(out, shape.fill_rgba),
        }
    }
    if let (Some(prims), Some(style), Some(local)) =
        (pass.prims.as_ref(), shape.stroke.as_ref(), local)
        && candidates.map_or_else(
            || prims.contains(local.x, local.y),
            |tags| prims.contains_among(tags, local.x, local.y),
        )
    {
        match (pass.stroke_pattern, shape.stroke_gradient) {
            (Some(paint), _) => {
                if let Some(sampled) = sample_pattern(&paint, local)
                    && sampled[3] != 0
                {
                    source_over(out, sampled);
                }
            }
            (None, Some(_)) => {
                let sampled = sample_gradient(pass.stroke_gradient.flatten(), px, py);
                if sampled[3] != 0 {
                    source_over(out, sampled);
                }
            }
            (None, None) => source_over(out, style.rgba),
        }
    }
}

/// Validates a gradient channel once for all its samples.
fn prepare_gradient<'a>(gradient: GradientRef<'a>) -> Option<PreparedPaint<'a>> {
    PreparedPaint::new(gradient.paint, gradient.stops)
}

/// Colour of a gradient channel at document point `(x, y)`, through the frozen
/// sampler. A point the sampler cannot place keeps alpha 0 and paints nothing.
fn sample_gradient(gradient: Option<PreparedPaint<'_>>, x: f64, y: f64) -> [u8; 4] {
    let mut rgba = [0u8; 4];
    if let Some(prepared) = gradient {
        prepared.sample(x, y, &mut rgba);
    }
    rgba
}
