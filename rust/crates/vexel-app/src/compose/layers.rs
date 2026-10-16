//! Layer glue for the scene renderer: source-over compositing of a layer into
//! its parent and mask application. Ports of C `composite_tile` (blend mode
//! normal) and `apply_mask` (`shape_render_group.c`), the same arithmetic
//! `vexel-render` applies inside `render_groups`.

use vexel_geometry::{Point, affine_map_point, affine_valid};
use vexel_raster::Tile;
use vexel_render::{MAX_COORDINATE, RenderError, ViewBox};
use vexel_scene::Mask;

/// User space to device mapping (C `map_viewport`), used by the mask region.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Geo {
    pub view: ViewBox,
    pub scale_x: f64,
    pub scale_y: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Geo {
    pub(crate) fn to_device(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.offset_x + (x - self.view.x) * self.scale_x,
            self.offset_y + (y - self.view.y) * self.scale_y,
        )
    }
}

/// C `composite_source_over`: straight-alpha RGBA8, integer arithmetic.
fn source_over(destination: &mut [u8], source: [u8; 4]) {
    let source_alpha = u32::from(source[3]);
    if source_alpha == 0 {
        return;
    }
    let destination_alpha = u32::from(destination[3]);
    let inverse = 255 - source_alpha;
    let output = source_alpha * 255 + destination_alpha * inverse;
    for channel in 0..3 {
        let numerator = u32::from(source[channel]) * source_alpha * 255
            + u32::from(destination[channel]) * destination_alpha * inverse;
        destination[channel] = ((numerator + output / 2) / output) as u8;
    }
    destination[3] = ((output + 127) / 255) as u8;
}

/// C `composite_tile` with blend mode normal: layer alpha times `opacity`,
/// source-over. `control` is polled per row.
pub(crate) fn composite_tile(
    destination: &mut Tile,
    source: &Tile,
    opacity: f64,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let row_bytes = source.width() as usize * 4;
    for (dst_row, src_row) in destination
        .rgba_mut()
        .chunks_exact_mut(row_bytes)
        .zip(source.rgba().chunks_exact(row_bytes))
    {
        if control() {
            return Err(RenderError::Cancelled);
        }
        for (dst, src) in dst_row.chunks_exact_mut(4).zip(src_row.chunks_exact(4)) {
            let alpha = (f64::from(src[3]) / 255.0 * opacity).min(1.0);
            if alpha <= 0.0 {
                continue;
            }
            source_over(dst, [src[0], src[1], src[2], (alpha * 255.0).round() as u8]);
        }
    }
    Ok(())
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

/// Mask region corners in device space, or `None` when the region has no area.
fn device_quad(geo: &Geo, mask: &Mask) -> Result<Option<[(f64, f64); 4]>, RenderError> {
    let finite = mask
        .region
        .iter()
        .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE);
    if !finite || !affine_valid(mask.region_transform) {
        return Err(RenderError::InvalidGeometry);
    }
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
        *slot = geo.to_device(point.x, point.y);
    }
    let area: f64 = (0..4)
        .map(|i| quad[i].0 * quad[(i + 1) % 4].1 - quad[(i + 1) % 4].0 * quad[i].1)
        .sum();
    Ok((width > 0.0 && height > 0.0 && area != 0.0).then_some(quad))
}

/// Applies the rendered mask `content` to `layer` in place: pixels whose
/// centre lies outside the mapped region are cleared; the others keep
/// `alpha * mask_alpha` (times linear luminance for `mask-type: luminance`).
pub(crate) fn mask_layer(
    layer: &mut Tile,
    content: &Tile,
    mask: &Mask,
    geo: &Geo,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let quad = device_quad(geo, mask)?;
    let row_bytes = layer.width() as usize * 4;
    for (row, (dst_row, src_row)) in layer
        .rgba_mut()
        .chunks_exact_mut(row_bytes)
        .zip(content.rgba().chunks_exact(row_bytes))
        .enumerate()
    {
        if control() {
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
