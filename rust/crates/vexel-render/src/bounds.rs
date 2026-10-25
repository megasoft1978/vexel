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

impl PixelBounds {
    /// These bounds limited to the pixel rectangle `[x0, y0, x1, y1)`.
    pub(crate) fn within(self, [x0, y0, x1, y1]: [u32; 4]) -> Option<Self> {
        let x_first = self.x_first.max(x0);
        let y_first = self.y_first.max(y0);
        let x_last = self.x_last.min(x1.checked_sub(1)?);
        let y_last = self.y_last.min(y1.checked_sub(1)?);
        (x_first <= x_last && y_first <= y_last).then_some(Self { x_first, x_last, y_first, y_last })
    }
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
    dashed: bool,
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
        && stroke::is_painted(style, shape.stroke_gradient.is_some())
        && !shape.edges.is_empty()
    {
        for edge in shape.edges {
            Extent::include(&mut stroke_box, edge.x0, edge.y0);
            Extent::include(&mut stroke_box, edge.x1, edge.y1);
        }
        let (pad_x, pad_y) =
            bounds_padding(style, inverse, shape.stroke_closed, dashed).ok_or(RenderError::InvalidGeometry)?;
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

#[cfg(test)]
mod tests {
    use super::shape_bounds;
    use crate::{
        FlatEdge, GradientRef, Linecap, Linejoin, RenderShape, StrokeStyle, ViewBox, ViewMap,
    };
    use vexel_geometry::affine_identity;
    use vexel_scene::{Paint, PaintKind, PaintStop, Spread, TableRange};

    /// Transparent at both stops, so the gradient stroke has alpha 0 everywhere.
    const CLEAR_STOPS: [PaintStop; 2] = [
        PaintStop {
            offset: 0.0,
            rgba: [0, 0, 0, 0],
        },
        PaintStop {
            offset: 1.0,
            rgba: [255, 255, 255, 0],
        },
    ];

    fn view() -> ViewMap {
        let vb = ViewBox {
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 8.0,
        };
        ViewMap::from_viewbox(vb, 8, 8, None).unwrap()
    }

    fn gradient() -> Paint {
        Paint {
            kind: PaintKind::Linear,
            spread: Spread::Pad,
            object_bounding_box: false,
            coordinates: [0.0, 0.0, 8.0, 0.0, 0.0, 0.0],
            bounds: [0.0; 4],
            user_inverse: affine_identity(),
            gradient_inverse: affine_identity(),
            stops: TableRange { first: 0, count: 2 },
            pattern: None,
            pattern_inverse: affine_identity(),
            pattern_opacity: 1.0,
        }
    }

    /// Open stroke (0,4)-(8,4), width 2, butt caps.
    fn stroked<'a>(
        edges: &'a [FlatEdge],
        rgba: [u8; 4],
        stroke_gradient: Option<GradientRef<'a>>,
    ) -> RenderShape<'a> {
        RenderShape {
            edges,
            subpaths: &[],
            subpath_closed: &[],
            stroke_closed: false,
            evenodd: false,
            filled: false,
            fill_rgba: [0; 4],
            stroke: Some(StrokeStyle {
                rgba,
                width: 2.0,
                miter_limit: 4.0,
                dash_offset: 0.0,
                cap: Linecap::Butt,
                join: Linejoin::Miter,
                crisp_edges: false,
            }),
            transform: affine_identity(),
            fill_gradient: None,
            stroke_gradient,
        }
    }

    #[test]
    fn gradient_stroke_at_zero_alpha_keeps_its_box() {
        let edges = [FlatEdge {
            x0: 0.0,
            y0: 4.0,
            x1: 8.0,
            y1: 4.0,
        }];
        let paint = gradient();
        let gradient = GradientRef {
            paint: &paint,
            stops: &CLEAR_STOPS,
        };
        let shape = stroked(&edges, [0; 4], Some(gradient));
        let bounds = shape_bounds(&shape, &view(), Some(affine_identity()), false)
            .unwrap()
            .expect("gradient stroke has bounds");
        // The stroke covers rows 3 and 4 (y in [3, 5]) across x in [0, 8).
        assert!(bounds.y_first <= 3 && bounds.y_last >= 4, "{bounds:?}");
        assert_eq!((bounds.x_first, bounds.x_last), (0, 7));
    }

    #[test]
    fn solid_stroke_at_zero_alpha_is_still_skipped() {
        let edges = [FlatEdge {
            x0: 0.0,
            y0: 4.0,
            x1: 8.0,
            y1: 4.0,
        }];
        let shape = stroked(&edges, [0; 4], None);
        let bounds = shape_bounds(&shape, &view(), Some(affine_identity()), false).unwrap();
        assert!(bounds.is_none(), "{bounds:?}");
    }
}
