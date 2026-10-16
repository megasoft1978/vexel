//! View-box to tile mapping (C `raster_shape_render_flat` meet/slice/none).

use crate::RenderError;

/// SVG `viewBox` in user units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// `preserveAspectRatio`; `align_*` are 0.0 (min), 0.5 (mid) or 1.0 (max).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreserveAspect {
    pub align_x: f64,
    pub align_y: f64,
    pub none: bool,
    pub slice: bool,
}

impl PreserveAspect {
    /// `xMidYMid meet`, the C default when the attribute is unset.
    pub const MID_MEET: Self = Self { align_x: 0.5, align_y: 0.5, none: false, slice: false };
}

/// Resolved mapping from user space to a `width` x `height` tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewMap {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) view: ViewBox,
    pub(crate) scale_x: f64,
    pub(crate) scale_y: f64,
    pub(crate) offset_x: f64,
    pub(crate) offset_y: f64,
}

impl ViewMap {
    /// Matches the C mapping. `None` means the default `xMidYMid meet`.
    pub fn from_viewbox(
        vb: ViewBox,
        width: u32,
        height: u32,
        preserve_aspect: Option<PreserveAspect>,
    ) -> Result<Self, RenderError> {
        if width == 0
            || height == 0
            || !vb.x.is_finite()
            || !vb.y.is_finite()
            || !vb.width.is_finite()
            || !vb.height.is_finite()
            || vb.width <= 0.0
            || vb.height <= 0.0
        {
            return Err(RenderError::InvalidGeometry);
        }
        let aspect = preserve_aspect.unwrap_or(PreserveAspect::MID_MEET);
        let mut scale_x = f64::from(width) / vb.width;
        let mut scale_y = f64::from(height) / vb.height;
        let (mut offset_x, mut offset_y) = (0.0, 0.0);
        if !aspect.none {
            let scale = if aspect.slice { scale_x.max(scale_y) } else { scale_x.min(scale_y) };
            scale_x = scale;
            scale_y = scale;
            offset_x = (f64::from(width) - vb.width * scale_x) * aspect.align_x;
            offset_y = (f64::from(height) - vb.height * scale_y) * aspect.align_y;
        }
        if !scale_x.is_finite()
            || scale_x <= 0.0
            || !scale_y.is_finite()
            || scale_y <= 0.0
            || !offset_x.is_finite()
            || !offset_y.is_finite()
        {
            return Err(RenderError::InvalidGeometry);
        }
        Ok(Self { width, height, view: vb, scale_x, scale_y, offset_x, offset_y })
    }

    /// Target tile width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Target tile height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// C `sample_is_in_view`: the content rectangle excludes meet letterboxing.
    pub(crate) fn sample_in_view(&self, sample_x: f64, sample_y: f64) -> bool {
        let content_width = self.view.width * self.scale_x;
        let content_height = self.view.height * self.scale_y;
        sample_x >= self.offset_x
            && sample_y >= self.offset_y
            && sample_x < self.offset_x + content_width
            && sample_y < self.offset_y + content_height
    }

    /// Device sample position to user space.
    pub(crate) fn to_user(&self, sample_x: f64, sample_y: f64) -> (f64, f64) {
        (
            self.view.x + (sample_x - self.offset_x) / self.scale_x,
            self.view.y + (sample_y - self.offset_y) / self.scale_y,
        )
    }

    /// User-space box to device space (C `map_user_box`).
    pub(crate) fn to_device(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.offset_x + (x - self.view.x) * self.scale_x,
            self.offset_y + (y - self.view.y) * self.scale_y,
        )
    }
}
