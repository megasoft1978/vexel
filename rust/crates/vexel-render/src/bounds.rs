//! Per-shape pixel bounds (C `shape_bounds.c`: fill box plus padded stroke box).

use crate::stroke::{self, bounds_padding};
use crate::{RenderError, RenderShape, ViewMap};
use vexel_geometry::Affine;

/// Inclusive pixel rectangle clipped to the tile.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PixelBounds {
    pub x_first: u32,
    pub x_last: u32,
    pub y_first: u32,
    pub y_last: u32,
}

struct Extent {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl Extent {
    fn include(slot: &mut Option<Self>, x: f64, y: f64) {
        match slot {
            None => *slot = Some(Self { left: x, top: y, right: x, bottom: y }),
            Some(e) => {
                e.left = e.left.min(x);
                e.right = e.right.max(x);
                e.top = e.top.min(y);
                e.bottom = e.bottom.max(y);
            }
        }
    }
}

/// C `pixel_range`: every 4x4 sample footprint plus one pixel of margin.
fn pixel_range(low: f64, high: f64, extent: u32) -> Option<(u32, u32)> {
    if extent == 0 || !low.is_finite() || !high.is_finite() || high < low {
        return None;
    }
    let first_value = (low - 0.875).ceil() - 1.0;
    let last_value = (high - 0.125).floor() + 1.0;
    if last_value < 0.0 || first_value >= f64::from(extent) {
        return None;
    }
    let first = if first_value <= 0.0 { 0 } else { first_value as u32 };
    let last = if last_value >= f64::from(extent) { extent - 1 } else { last_value as u32 };
    (first <= last).then_some((first, last))
}

/// Pixel bounds of one shape, or `None` when nothing of it can be visible.
pub(crate) fn shape_bounds(
    shape: &RenderShape<'_>,
    view: &ViewMap,
    inverse: Option<Affine>,
) -> Result<Option<PixelBounds>, RenderError> {
    let mut fill: Option<Extent> = None;
    if shape.filled {
        for edge in shape.edges {
            Extent::include(&mut fill, edge.x0, edge.y0);
            Extent::include(&mut fill, edge.x1, edge.y1);
        }
    }
    let mut stroke_box: Option<Extent> = None;
    if let (Some(style), Some(inverse)) = (shape.stroke.as_ref(), inverse)
        && stroke::is_visible(style)
        && !shape.edges.is_empty()
    {
        for edge in shape.edges {
            Extent::include(&mut stroke_box, edge.x0, edge.y0);
            Extent::include(&mut stroke_box, edge.x1, edge.y1);
        }
        let (pad_x, pad_y) =
            bounds_padding(style, inverse, shape.stroke_closed).ok_or(RenderError::InvalidGeometry)?;
        if let Some(e) = stroke_box.as_mut() {
            e.left -= pad_x;
            e.right += pad_x;
            e.top -= pad_y;
            e.bottom += pad_y;
        }
    }
    let combined = match (fill, stroke_box) {
        (None, None) => return Ok(None),
        (Some(e), None) | (None, Some(e)) => e,
        (Some(a), Some(b)) => Extent {
            left: a.left.min(b.left),
            top: a.top.min(b.top),
            right: a.right.max(b.right),
            bottom: a.bottom.max(b.bottom),
        },
    };
    let (left, top) = view.to_device(combined.left, combined.top);
    let (right, bottom) = view.to_device(combined.right, combined.bottom);
    let margin_x = 1.0f64.max(left.abs().max(right.abs())) * f64::EPSILON * 8.0;
    let margin_y = 1.0f64.max(top.abs().max(bottom.abs())) * f64::EPSILON * 8.0;
    let (Some((x_first, x_last)), Some((y_first, y_last))) = (
        pixel_range(left - margin_x, right + margin_x, view.width()),
        pixel_range(top - margin_y, bottom + margin_y, view.height()),
    ) else {
        return Ok(None);
    };
    Ok(Some(PixelBounds { x_first, x_last, y_first, y_last }))
}
