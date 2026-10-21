//! Stage view points <-> compose frame pixels <-> document (viewBox) units.

use super::{size::frame_matches, view::View};
use crate::compose::{OwnedShape, PointMap};
use crate::tools::CanvasMap;

/// `(x0, y0, x1, y1)`.
pub type Bounds = (f64, f64, f64, f64);

/// The single `view_to_doc` / `doc_to_view` pair: one cached compose frame
/// presented through a [`View`], with the compose's own [`PointMap`] for the
/// last step to document units. Points are stage-local logical points.
#[derive(Clone, Copy, Debug)]
pub struct ViewMap {
    view: View,
    artboard: (f64, f64),
    frame: (u32, u32),
    point: PointMap,
}

impl ViewMap {
    /// `None` when `frame` is not the artboard at a uniform resolution.
    pub fn new(
        view: View,
        artboard: (f64, f64),
        frame: (u32, u32),
        point: PointMap,
    ) -> Option<Self> {
        frame_matches(frame, artboard).then_some(Self {
            view,
            artboard,
            frame,
            point,
        })
    }

    /// View point to compose frame pixel.
    pub fn view_to_frame(&self, p: (f64, f64)) -> (f64, f64) {
        let d = self.view.view_to_doc(p);
        (
            d.0 * f64::from(self.frame.0) / self.artboard.0,
            d.1 * f64::from(self.frame.1) / self.artboard.1,
        )
    }

    /// Compose frame pixel to view point.
    pub fn frame_to_view(&self, p: (f64, f64)) -> (f64, f64) {
        self.view.doc_to_view((
            p.0 * self.artboard.0 / f64::from(self.frame.0),
            p.1 * self.artboard.1 / f64::from(self.frame.1),
        ))
    }

    /// Document (viewBox) unit point to view point.
    pub fn doc_to_view(&self, x: f64, y: f64) -> (f64, f64) {
        self.frame_to_view(self.point.doc_to_canvas(x, y))
    }

    /// View-space bounds of a shape's fill and stroke.
    pub fn shape_bounds(&self, shape: &OwnedShape) -> Option<Bounds> {
        let (x0, y0, x1, y1) = doc_bounds(shape)?;
        let (a, b) = (self.doc_to_view(x0, y0), self.doc_to_view(x1, y1));
        Some((a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)))
    }
}

impl CanvasMap for ViewMap {
    fn to_doc(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let (fx, fy) = self.view_to_frame((x, y));
        self.point.canvas_to_doc(fx, fy)
    }
}

/// Document-unit bounds of a shape, grown by half its transformed stroke.
fn doc_bounds(shape: &OwnedShape) -> Option<Bounds> {
    let mut edges = shape.edges.iter();
    let first = edges.next()?;
    let mut b = (
        first.x0.min(first.x1),
        first.y0.min(first.y1),
        first.x0.max(first.x1),
        first.y0.max(first.y1),
    );
    for e in edges {
        b = (
            b.0.min(e.x0).min(e.x1),
            b.1.min(e.y0).min(e.y1),
            b.2.max(e.x0).max(e.x1),
            b.3.max(e.y0).max(e.y1),
        );
    }
    let t = shape.transform;
    let reach = shape
        .stroke
        .filter(|s| s.rgba[3] > 0 || shape.stroke_pattern || shape.stroke_gradient.is_some())
        .map_or(0.0, |s| {
            s.width * (t.a * t.d - t.b * t.c).abs().sqrt() / 2.0
        });
    let b = (b.0 - reach, b.1 - reach, b.2 + reach, b.3 + reach);
    [b.0, b.1, b.2, b.3]
        .iter()
        .all(|v| v.is_finite())
        .then_some(b)
}

#[cfg(test)]
mod tests {
    use super::{Bounds, doc_bounds};
    use crate::compose::compose_document;
    use vexel_geometry::affine_identity;
    use vexel_render::GradientRef;
    use vexel_scene::{Paint, PaintKind, PaintStop, Spread, TableRange};

    /// Bounds of a 4-wide stroke along y = 10 whose colour is fully transparent,
    /// with a transparent linear gradient in place of the solid colour if asked.
    fn stroke_bounds(gradient: bool) -> Bounds {
        let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='40' height='40'>\
             <path d='M5 10 L35 10' fill='none' stroke-width='4' stroke='rgba(0,0,0,0)'/></svg>";
        let composed = compose_document(svg.as_bytes(), 40, 40, &|| false).unwrap();
        let mut shape = composed.shapes.into_iter().next().expect("one path");
        assert_eq!(shape.stroke.map(|s| s.rgba[3]), Some(0));
        if gradient {
            let paint = Paint {
                kind: PaintKind::Linear,
                spread: Spread::Pad,
                object_bounding_box: false,
                coordinates: [0.0, 0.0, 40.0, 0.0, 0.0, 0.0],
                bounds: [0.0; 4],
                user_inverse: affine_identity(),
                gradient_inverse: affine_identity(),
                stops: TableRange { first: 0, count: 2 },
                pattern: None,
                pattern_inverse: affine_identity(),
                pattern_opacity: 1.0,
            };
            let stops = [
                PaintStop {
                    offset: 0.0,
                    rgba: [255, 0, 0, 0],
                },
                PaintStop {
                    offset: 1.0,
                    rgba: [0, 0, 255, 0],
                },
            ];
            let gradient = GradientRef {
                paint: &paint,
                stops: &stops,
            };
            shape.set_stroke_gradient_for_test(gradient).unwrap();
        }
        doc_bounds(&shape).expect("bounds")
    }

    #[test]
    fn stroke_reach_counts_a_gradient_even_at_alpha_zero() {
        // Solid colour at alpha 0: no reach, the box is the centre line.
        let solid = stroke_bounds(false);
        assert_eq!((solid.1, solid.3), (10.0, 10.0), "{solid:?}");
        // Gradient stroke at alpha 0: half the width of reach on each side.
        let gradient = stroke_bounds(true);
        assert_eq!((gradient.1, gradient.3), (8.0, 12.0), "{gradient:?}");
    }
}
