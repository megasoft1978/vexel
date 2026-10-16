//! Canvas pixel <-> document coordinate mapping at 1 pixel per point.
//!
//! `vexel_render::ViewMap` keeps its scale and offsets crate-private, so this
//! repeats its exact `from_viewbox` arithmetic (meet, slice, none); the
//! `render_agrees_with_pointer_map` test renders through `ViewMap` and checks
//! that both agree on where a document point lands.

use vexel_render::{PreserveAspect, ViewBox};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointMap {
    width: u32,
    height: u32,
    view: ViewBox,
    scale_x: f64,
    scale_y: f64,
    offset_x: f64,
    offset_y: f64,
}

impl PointMap {
    /// `None` aspect means the default `xMidYMid meet`, as in `ViewMap`.
    pub fn new(
        view: ViewBox,
        width: u32,
        height: u32,
        aspect: Option<PreserveAspect>,
    ) -> Option<Self> {
        if width == 0
            || height == 0
            || !(view.x.is_finite() && view.y.is_finite())
            || !(view.width.is_finite() && view.height.is_finite())
            || view.width <= 0.0
            || view.height <= 0.0
        {
            return None;
        }
        let aspect = aspect.unwrap_or(PreserveAspect::MID_MEET);
        let mut scale_x = f64::from(width) / view.width;
        let mut scale_y = f64::from(height) / view.height;
        let (mut offset_x, mut offset_y) = (0.0, 0.0);
        if !aspect.none {
            let scale = if aspect.slice {
                scale_x.max(scale_y)
            } else {
                scale_x.min(scale_y)
            };
            scale_x = scale;
            scale_y = scale;
            offset_x = (f64::from(width) - view.width * scale_x) * aspect.align_x;
            offset_y = (f64::from(height) - view.height * scale_y) * aspect.align_y;
        }
        let finite = scale_x.is_finite()
            && scale_y.is_finite()
            && scale_x > 0.0
            && scale_y > 0.0
            && offset_x.is_finite()
            && offset_y.is_finite();
        finite.then_some(Self {
            width,
            height,
            view,
            scale_x,
            scale_y,
            offset_x,
            offset_y,
        })
    }

    /// Document (user-space) point to canvas pixel coordinates.
    pub fn doc_to_canvas(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.offset_x + (x - self.view.x) * self.scale_x,
            self.offset_y + (y - self.view.y) * self.scale_y,
        )
    }

    /// Canvas pixel coordinates to a document point. `None` when the point is
    /// outside the canvas or in the meet letterbox, i.e. outside the viewport.
    pub fn canvas_to_doc(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let content_w = self.view.width * self.scale_x;
        let content_h = self.view.height * self.scale_y;
        let inside = x.is_finite()
            && y.is_finite()
            && x >= 0.0
            && y >= 0.0
            && x < f64::from(self.width)
            && y < f64::from(self.height)
            && x >= self.offset_x
            && y >= self.offset_y
            && x < self.offset_x + content_w
            && y < self.offset_y + content_h;
        inside.then(|| {
            (
                self.view.x + (x - self.offset_x) / self.scale_x,
                self.view.y + (y - self.offset_y) / self.scale_y,
            )
        })
    }
}
