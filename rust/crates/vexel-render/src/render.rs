//! Shape orchestration: per pixel 4x4 samples, per shape source-over
//! (C `raster_shape_render_flat` with solid paint only).

use crate::bounds::shape_bounds;
use crate::composite::{SAMPLE_COUNT, resolve_samples, source_over};
use crate::fill::inside;
use crate::stroke::{self, StrokePrims};
use crate::validate::validate;
use crate::{RenderError, RenderShape, SAMPLES_PER_AXIS, ViewMap};
use vexel_geometry::{Affine, Point, affine_map_point};
use vexel_raster::Tile;

const SAMPLE_LOCATIONS: [f64; SAMPLES_PER_AXIS] = [0.125, 0.375, 0.625, 0.875];

/// Per-shape state shared by every sample.
struct ShapePass<'a, 'b> {
    shape: &'a RenderShape<'b>,
    prims: Option<StrokePrims>,
    inverse: Option<Affine>,
    fill_visible: bool,
}

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
    validate(shapes, &view, tile)?;
    let width = tile.width() as usize;
    let mut active: Vec<usize> = Vec::new();
    for shape in shapes {
        if control() {
            return Err(RenderError::Cancelled);
        }
        let style = shape.stroke.filter(|s| stroke::is_visible(s) && !shape.edges.is_empty());
        let inverse = style.and_then(|_| stroke::invert(shape.transform));
        let Some(bounds) = shape_bounds(shape, &view, inverse)? else {
            continue;
        };
        let prims = match (style.as_ref(), inverse) {
            (Some(style), Some(inverse)) => Some(StrokePrims::build(shape, style, inverse)?),
            _ => None,
        };
        let pass = ShapePass {
            shape,
            prims,
            inverse,
            fill_visible: shape.filled && shape.fill_rgba[3] != 0,
        };
        active.clear();
        active.try_reserve(shape.edges.len()).map_err(|_| RenderError::LimitExceeded)?;
        for y in bounds.y_first..=bounds.y_last {
            if control() {
                return Err(RenderError::Cancelled);
            }
            // Edges that can span some sample row of this pixel row.
            // The user-space y mapping is monotonic, so every sample py lies in this range.
            let row_low = view.to_user(0.0, f64::from(y)).1;
            let row_high = view.to_user(0.0, f64::from(y) + 1.0).1;
            active.clear();
            for (index, edge) in shape.edges.iter().enumerate() {
                if edge.y0.min(edge.y1) <= row_high && edge.y0.max(edge.y1) >= row_low {
                    active.push(index);
                }
            }
            for x in bounds.x_first..=bounds.x_last {
                let mut samples = [[0u8; 4]; SAMPLE_COUNT];
                for (sy, location_y) in SAMPLE_LOCATIONS.iter().enumerate() {
                    let sample_y = f64::from(y) + location_y;
                    for (sx, location_x) in SAMPLE_LOCATIONS.iter().enumerate() {
                        let sample_x = f64::from(x) + location_x;
                        if !view.sample_in_view(sample_x, sample_y) {
                            continue;
                        }
                        let (px, py) = view.to_user(sample_x, sample_y);
                        paint_sample(&pass, &active, px, py, &mut samples[sy * SAMPLES_PER_AXIS + sx]);
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

/// C `paint_sample_result` for solid paint: fill first, then stroke on top.
fn paint_sample(pass: &ShapePass<'_, '_>, active: &[usize], px: f64, py: f64, out: &mut [u8; 4]) {
    let shape = pass.shape;
    if pass.fill_visible && inside(shape.edges, active, shape.evenodd, px, py) {
        source_over(out, shape.fill_rgba);
    }
    if let (Some(prims), Some(inverse), Some(style)) =
        (pass.prims.as_ref(), pass.inverse, shape.stroke.as_ref())
        && let Some(local) = affine_map_point(inverse, Point { x: px, y: py })
        && prims.contains(local.x, local.y)
    {
        source_over(out, style.rgba);
    }
}
