//! Offscreen (export, golden) tile sizing.
//!
//! Offscreen output is always 1x: the pixel size is the point size rounded up,
//! exactly as C `render-frame` sizes its region tiles (`ceil(size * zoom)`
//! with zoom 1). No host scale is read or accepted here, which resolves
//! BLOCKERS.md defect 3 for the Rust path only; the live window path keeps the
//! host backing scale from `vexel-platform-apple`, and `native_ui_render.m`
//! and its goldens are untouched.

use crate::{PreserveAspect, RenderError, RenderShape, ViewBox, ViewMap, render_shapes};
use vexel_raster::{MAX_AXIS, Tile};

/// Backing scale of every offscreen render.
pub const OFFSCREEN_BACKING_SCALE: f64 = 1.0;

fn axis(points: f64) -> Result<u32, RenderError> {
    let pixels = (points * OFFSCREEN_BACKING_SCALE).ceil();
    if !pixels.is_finite() || pixels < 1.0 || pixels > f64::from(MAX_AXIS) {
        return Err(RenderError::InvalidGeometry);
    }
    Ok(pixels as u32)
}

/// Pixel size of an offscreen tile for a size in points.
pub fn offscreen_tile_size(points_w: f64, points_h: f64) -> Result<(u32, u32), RenderError> {
    Ok((axis(points_w)?, axis(points_h)?))
}

/// Renders into a fresh transparent tile sized by [`offscreen_tile_size`].
pub fn render_offscreen(
    shapes: &[RenderShape<'_>],
    view_box: ViewBox,
    preserve_aspect: Option<PreserveAspect>,
    points_w: f64,
    points_h: f64,
    control: &dyn Fn() -> bool,
) -> Result<Tile, RenderError> {
    let (width, height) = offscreen_tile_size(points_w, points_h)?;
    let bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(RenderError::LimitExceeded)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(bytes).map_err(|_| RenderError::LimitExceeded)?;
    rgba.resize(bytes, 0);
    let mut tile = Tile::from_owned_rgba(width, height, rgba)
        .map_err(|_| RenderError::LimitExceeded)?;
    let view = ViewMap::from_viewbox(view_box, width, height, preserve_aspect)?;
    render_shapes(shapes, view, &mut tile, control)?;
    Ok(tile)
}
