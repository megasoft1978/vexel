//! Shape orchestration: per pixel 4x4 samples, per shape source-over
//! (C `raster_shape_render_flat` with solid paint only).

use crate::bounds::shape_bounds;
use crate::clip::ClipList;
use crate::composite::{SAMPLE_COUNT, resolve_samples, source_over};
use crate::dash::Pattern;
use crate::fill::{Crossings, inside};
use crate::pattern::{PatternPaint, sample as sample_pattern};
use crate::pattern_boundary::{mark_boundaries, pixel_fully_in_view};
use crate::scan::{AxisTables, SAMPLE_LOCATIONS, Sweep};
use crate::stroke::{self, StrokePrims};
use crate::validate::validate;
use crate::{RenderError, RenderShape, SAMPLES_PER_AXIS, ViewMap};
use vexel_geometry::{Affine, Point, affine_map_point};
use vexel_raster::Tile;

/// Per-shape state shared by every sample.
struct ShapePass<'a, 'b> {
    shape: &'a RenderShape<'b>,
    prims: Option<StrokePrims>,
    inverse: Option<Affine>,
    fill_visible: bool,
    clips: ClipList<'a>,
    fill_pattern: Option<PatternPaint<'a>>,
    stroke_pattern: Option<PatternPaint<'a>>,
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

/// [`render_shapes`] with optional per-shape pattern paints; `paints` is empty
/// (all solid) or parallel to `shapes`.
pub(crate) fn render_core(
    shapes: &[RenderShape<'_>],
    patterns: &[Option<Pattern>],
    clips: &[ClipList<'_>],
    paints: &[ShapePaints<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    if !clips.is_empty() && clips.len() != shapes.len() {
        return Err(RenderError::InvalidGeometry);
    }
    let mut budget = validate(shapes, patterns, &view, tile)?;
    let width = tile.width() as usize;
    let tables = AxisTables::new(&view)?;
    let mut sweep = Sweep::default();
    let mut crossings: [Crossings; SAMPLES_PER_AXIS] = Default::default();
    let boundary = if paints.iter().any(|(fill, stroke)| fill.is_some() || stroke.is_some()) {
        Some(mark_boundaries(shapes, &view)?)
    } else {
        None
    };
    for (shape_index, shape) in shapes.iter().enumerate() {
        let (fill_pattern, stroke_pattern) =
            paints.get(shape_index).copied().unwrap_or((None, None));
        if control() {
            return Err(RenderError::Cancelled);
        }
        let style = shape.stroke.filter(|s| stroke::is_visible(s) && !shape.edges.is_empty());
        let patterned = fill_pattern.is_some() || stroke_pattern.is_some();
        let inverse =
            if style.is_some() || patterned { stroke::invert(shape.transform) } else { None };
        let Some(bounds) = shape_bounds(shape, &view, inverse)? else {
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
            fill_visible: shape.filled && (fill_pattern.is_some() || shape.fill_rgba[3] != 0),
            clips: clips.get(shape_index).copied().unwrap_or(&[]),
            fill_pattern,
            stroke_pattern,
        };
        sweep.reset(shape.edges, &tables, bounds.y_first, bounds.y_last)?;
        // Without stroke or pattern paint only the fill can touch a pixel, so
        // rows and spans with no crossing can be skipped outright.
        let fill_only = pass.prims.is_none() && fill_pattern.is_none() && stroke_pattern.is_none();
        if fill_only && !pass.fill_visible {
            continue;
        }
        // Resolved colour of a pixel whose 16 samples all paint the fill.
        let full_pixel = (fill_only && pass.clips.is_empty()).then(|| {
            let mut sample = [0u8; 4];
            source_over(&mut sample, shape.fill_rgba);
            resolve_samples(&[sample; SAMPLE_COUNT])
        });
        for y in bounds.y_first..=bounds.y_last {
            if control() {
                return Err(RenderError::Cancelled);
            }
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
                        let offset = (y as usize * width + x as usize) * 4;
                        let pixel: &mut [u8; 4] = (&mut tile.rgba_mut()[offset..offset + 4])
                            .try_into()
                            .map_err(|_| RenderError::InvalidGeometry)?;
                        source_over(pixel, full);
                        continue;
                    }
                }
                // C samples pattern paint once at the centre of pixels that no
                // edge touches (see `pattern_boundary`).
                let center_only = (pass.fill_pattern.is_some() || pass.stroke_pattern.is_some())
                    && boundary.as_ref().is_some_and(|mask| {
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
                        paint_sample(&pass, fill_inside, px, py, &mut center);
                        let offset = (y as usize * width + x as usize) * 4;
                        let pixel: &mut [u8; 4] = (&mut tile.rgba_mut()[offset..offset + 4])
                            .try_into()
                            .map_err(|_| RenderError::InvalidGeometry)?;
                        source_over(pixel, center);
                    }
                    continue;
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
                        paint_sample(
                            &pass,
                            fill_inside,
                            px,
                            sample_y[sy],
                            &mut samples[sy * SAMPLES_PER_AXIS + sx],
                        );
                    }
                }
                let source = resolve_samples(&samples);
                let offset = (y as usize * width + x as usize) * 4;
                let pixel: &mut [u8; 4] = (&mut tile.rgba_mut()[offset..offset + 4])
                    .try_into()
                    .map_err(|_| RenderError::InvalidGeometry)?;
                source_over(pixel, source);
            }
        }
    }
    Ok(())
}

/// C `paint_sample_result`: fill first, then stroke on top; a pattern paint
/// replaces the solid colour of its channel.
fn paint_sample(
    pass: &ShapePass<'_, '_>,
    fill_inside: impl FnOnce() -> bool,
    px: f64,
    py: f64,
    out: &mut [u8; 4],
) {
    let shape = pass.shape;
    if !pass.clips.iter().all(|clip| clip.contains(px, py)) {
        return;
    }
    let local = pass.inverse.and_then(|inverse| affine_map_point(inverse, Point { x: px, y: py }));
    if pass.fill_visible && fill_inside() {
        match pass.fill_pattern {
            Some(paint) => {
                if let Some(sampled) = local.and_then(|p| sample_pattern(&paint, p))
                    && sampled[3] != 0
                {
                    source_over(out, sampled);
                }
            }
            None => source_over(out, shape.fill_rgba),
        }
    }
    if let (Some(prims), Some(style), Some(local)) =
        (pass.prims.as_ref(), shape.stroke.as_ref(), local)
        && prims.contains(local.x, local.y)
    {
        match pass.stroke_pattern {
            Some(paint) => {
                if let Some(sampled) = sample_pattern(&paint, local)
                    && sampled[3] != 0
                {
                    source_over(out, sampled);
                }
            }
            None => source_over(out, style.rgba),
        }
    }
}
