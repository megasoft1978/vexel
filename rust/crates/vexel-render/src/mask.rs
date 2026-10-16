//! Mask application, a port of C `apply_mask` (`shape_render_group.c`).
//!
//! The mask content is rendered with the document's own view mapping into a
//! transparent tile. Layer pixels whose centre lies outside the mapped mask
//! region are cleared; the others keep `alpha * mask_alpha` (times the linear
//! luminance of the mask pixel for `mask-type: luminance`).

use crate::group::{Ctx, RenderNode};
use crate::{MAX_COORDINATE, RenderError};
use vexel_geometry::{Affine, Point, affine_map_point, affine_valid};
use vexel_raster::Tile;

/// A mask definition resolved to renderable content (C `svg_shape_mask`).
///
/// `content` is in document user space (mask content units already applied).
/// `region` is `[x, y, width, height]` in the mask's region space and
/// `region_transform` maps that space to document user space. A region of zero
/// or negative area is accepted and clears the whole layer.
#[derive(Debug, Clone, Copy)]
pub struct RenderMask<'a> {
    pub luminance: bool,
    pub region: [f64; 4],
    pub region_transform: Affine,
    pub content: &'a [RenderNode<'a>],
}

impl RenderMask<'_> {
    pub(crate) fn validate(&self) -> Result<(), RenderError> {
        let finite = self
            .region
            .iter()
            .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE);
        if !finite || !affine_valid(self.region_transform) {
            return Err(RenderError::InvalidGeometry);
        }
        Ok(())
    }
}

/// C `linear_luminance`: sRGB to linear light, Rec. 709 weights.
fn linear_luminance(rgba: &[u8]) -> f64 {
    let linear = |byte: u8| {
        let value = f64::from(byte) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgba[0]) + 0.7152 * linear(rgba[1]) + 0.0722 * linear(rgba[2])
}

/// C `point_in_quad`: the point is on one side of every edge (within 1e-9).
fn point_in_quad(quad: &[(f64, f64); 4], x: f64, y: f64) -> bool {
    let (mut positive, mut negative) = (0u8, 0u8);
    for i in 0..4 {
        let (a, b) = (quad[i], quad[(i + 1) % 4]);
        let cross = (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
        if cross > 1e-9 {
            positive += 1;
        } else if cross < -1e-9 {
            negative += 1;
        }
    }
    positive == 0 || negative == 0
}

/// Mask region corners in device space, or `None` when the region covers no area.
fn device_quad(
    ctx: &Ctx<'_>,
    mask: &RenderMask<'_>,
) -> Result<Option<[(f64, f64); 4]>, RenderError> {
    let [x, y, width, height] = mask.region;
    let corners = [
        (x, y),
        (x + width, y),
        (x + width, y + height),
        (x, y + height),
    ];
    let mut quad = [(0.0, 0.0); 4];
    for (slot, (cx, cy)) in quad.iter_mut().zip(corners) {
        let point = affine_map_point(mask.region_transform, Point { x: cx, y: cy })
            .ok_or(RenderError::InvalidGeometry)?;
        *slot = ctx.view.to_device(point.x, point.y);
    }
    let area: f64 = (0..4)
        .map(|i| quad[i].0 * quad[(i + 1) % 4].1 - quad[(i + 1) % 4].0 * quad[i].1)
        .sum();
    Ok((width > 0.0 && height > 0.0 && area != 0.0).then_some(quad))
}

/// Applies `mask` to `layer` in place. The caller owns the layer's reservation.
pub(crate) fn apply(
    ctx: &mut Ctx<'_>,
    mask: &RenderMask<'_>,
    layer: &mut Tile,
) -> Result<(), RenderError> {
    if ctx.mask_depth >= crate::MAX_MASK_DEPTH {
        return Err(RenderError::LimitExceeded);
    }
    ctx.reserve()?;
    let result = apply_reserved(ctx, mask, layer);
    ctx.release();
    result
}

fn apply_reserved(
    ctx: &mut Ctx<'_>,
    mask: &RenderMask<'_>,
    layer: &mut Tile,
) -> Result<(), RenderError> {
    let mut content = ctx.new_tile()?;
    ctx.mask_depth += 1;
    let rendered = ctx.render_tree(mask.content, &mut content);
    ctx.mask_depth -= 1;
    rendered?;
    let quad = device_quad(ctx, mask)?;
    let width = layer.width() as usize;
    let row_bytes = width * 4;
    for (row, (dst_row, src_row)) in layer
        .rgba_mut()
        .chunks_exact_mut(row_bytes)
        .zip(content.rgba().chunks_exact(row_bytes))
        .enumerate()
    {
        if ctx.cancelled() {
            return Err(RenderError::Cancelled);
        }
        for (column, (dst, src)) in dst_row
            .chunks_exact_mut(4)
            .zip(src_row.chunks_exact(4))
            .enumerate()
        {
            let inside = quad
                .as_ref()
                .is_some_and(|q| point_in_quad(q, column as f64 + 0.5, row as f64 + 0.5));
            if !inside {
                dst.fill(0);
                continue;
            }
            let mut alpha = f64::from(src[3]) / 255.0;
            if mask.luminance {
                alpha *= linear_luminance(src);
            }
            dst[3] = (f64::from(dst[3]) * alpha).round() as u8;
            if dst[3] == 0 {
                dst.fill(0);
            }
        }
    }
    Ok(())
}
