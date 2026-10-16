//! Boundary mask of the C renderer (`raster_shape_bounds_mark_boundaries`).
//!
//! C paints a pixel that is fully inside the view and not touched by any edge
//! (padded by the stroke radius) with a single sample at the pixel centre
//! instead of the 4x4 grid. For solid paint the two are identical, so only
//! pattern-painted pixels need this distinction: a pattern sampled once at
//! the centre differs from the average of sixteen samples.

use crate::{RenderError, RenderShape, ViewMap};
use vexel_geometry::Affine;

const EPSILON_SCALE: f64 = f64::EPSILON * 8.0;

pub(crate) struct BoundaryMask {
    width: usize,
    bits: Vec<bool>,
}

impl BoundaryMask {
    pub(crate) fn is_marked(&self, x: u32, y: u32) -> bool {
        self.bits[y as usize * self.width + x as usize]
    }

    fn mark(&mut self, x: (u32, u32), y: (u32, u32)) {
        for row in y.0..=y.1 {
            let base = row as usize * self.width;
            for slot in &mut self.bits[base + x.0 as usize..=base + x.1 as usize] {
                *slot = true;
            }
        }
    }
}

/// C `boundary_pixel_range`.
fn range(low: f64, high: f64, extent: u32) -> Option<(u32, u32)> {
    if extent == 0 || !low.is_finite() || !high.is_finite() || high < low {
        return None;
    }
    let first_value = (low - 1.0).ceil() - 1.0;
    let last_value = high.floor() + 1.0;
    if last_value < 0.0 || first_value >= f64::from(extent) {
        return None;
    }
    let first = if first_value <= 0.0 {
        0
    } else {
        first_value as u32
    };
    let last = if last_value >= f64::from(extent) {
        extent - 1
    } else {
        last_value as u32
    };
    (first <= last).then_some((first, last))
}

/// C `edge_stroke_radius`: half stroke width scaled to document space.
fn stroke_radius(shape: &RenderShape<'_>, include_miter: bool) -> f64 {
    let Some(style) = shape.stroke.filter(|s| s.rgba[3] != 0 && s.width > 0.0) else {
        return 0.0;
    };
    let Affine { a, b, c, d, .. } = shape.transform;
    let scale = (a * a + b * b + c * c + d * d).sqrt();
    let join = if include_miter && matches!(style.join, crate::Linejoin::Miter) {
        style.miter_limit
    } else {
        1.0
    };
    let cap = if !shape.stroke_closed && matches!(style.cap, crate::Linecap::Square) {
        std::f64::consts::SQRT_2
    } else {
        1.0
    };
    style.width * 0.5 * scale * join.max(cap)
}

/// Marks every pixel box any edge (or miter join) of `shapes` can touch.
pub(crate) fn mark_boundaries(
    shapes: &[RenderShape<'_>],
    view: &ViewMap,
) -> Result<BoundaryMask, RenderError> {
    let (width, height) = (view.width(), view.height());
    let mut mask = BoundaryMask {
        width: width as usize,
        bits: Vec::new(),
    };
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or(RenderError::LimitExceeded)?;
    mask.bits
        .try_reserve_exact(pixels)
        .map_err(|_| RenderError::LimitExceeded)?;
    mask.bits.resize(pixels, false);
    for shape in shapes {
        let radius = stroke_radius(shape, false);
        let miter = shape.stroke.is_some_and(|s| {
            s.rgba[3] != 0 && s.width > 0.0 && matches!(s.join, crate::Linejoin::Miter)
        });
        let join_radius = if miter {
            stroke_radius(shape, true)
        } else {
            0.0
        };
        for edge in shape.edges {
            let (x1, y1) = view.to_device(edge.x0, edge.y0);
            let (x2, y2) = view.to_device(edge.x1, edge.y1);
            let (pad_x, pad_y) = (radius * view.scale_x, radius * view.scale_y);
            let epsilon =
                1.0f64.max(x1.abs().max(x2.abs()).max(y1.abs()).max(y2.abs())) * EPSILON_SCALE;
            let x_range = range(
                x1.min(x2) - pad_x - epsilon,
                x1.max(x2) + pad_x + epsilon,
                width,
            );
            let y_range = range(
                y1.min(y2) - pad_y - epsilon,
                y1.max(y2) + pad_y + epsilon,
                height,
            );
            if let (Some(x), Some(y)) = (x_range, y_range) {
                mask.mark(x, y);
            }
            if miter {
                for (px, py) in [(edge.x0, edge.y0), (edge.x1, edge.y1)] {
                    let (x, y) = view.to_device(px, py);
                    let (pad_x, pad_y) = (join_radius * view.scale_x, join_radius * view.scale_y);
                    let epsilon = 1.0f64.max(x.abs().max(y.abs())) * EPSILON_SCALE;
                    let x_range = range(x - pad_x - epsilon, x + pad_x + epsilon, width);
                    let y_range = range(y - pad_y - epsilon, y + pad_y + epsilon, height);
                    if let (Some(x), Some(y)) = (x_range, y_range) {
                        mask.mark(x, y);
                    }
                }
            }
        }
    }
    Ok(mask)
}

/// C `pixel_is_fully_in_view`.
pub(crate) fn pixel_fully_in_view(view: &ViewMap, x: u32, y: u32) -> bool {
    let content_width = view.view.width * view.scale_x;
    let content_height = view.view.height * view.scale_y;
    f64::from(x) >= view.offset_x
        && f64::from(y) >= view.offset_y
        && f64::from(x) + 1.0 <= view.offset_x + content_width
        && f64::from(y) + 1.0 <= view.offset_y + content_height
}
