use crate::types::{invalid, stopped};
use crate::*;
impl ViewMapping {
    pub fn validate(self) -> Result<()> {
        if ![self.scale_x, self.scale_y, self.offset_x, self.offset_y]
            .into_iter()
            .all(f64::is_finite)
            || self.scale_x <= 0.0
            || self.scale_y <= 0.0
        {
            Err(invalid(0))
        } else {
            Ok(())
        }
    }
    pub fn point(self, point: Point) -> Result<Point> {
        self.validate()?;
        let result = Point {
            x: point.x * self.scale_x + self.offset_x,
            y: point.y * self.scale_y + self.offset_y,
        };
        if !vexel_geometry::finite_point(result) {
            Err(invalid(0))
        } else {
            Ok(result)
        }
    }
}
impl PixelBounds {
    pub fn validate(self, width: u32, height: u32) -> Result<()> {
        if self.x_first > self.x_last
            || self.y_first > self.y_last
            || self.x_last > width
            || self.y_last > height
        {
            Err(invalid(0))
        } else {
            Ok(())
        }
    }
}
impl BoundaryMask {
    pub fn validate(&self) -> Result<()> {
        let required = self
            .bit_count
            .checked_add(7)
            .ok_or_else(crate::memory::arithmetic)?
            / 8;
        if self.bits.len() < required || self.marked_pixel_count > self.bit_count {
            return Err(invalid(0));
        }
        Ok(())
    }
    pub fn test(&self, pixel: usize) -> Option<bool> {
        if pixel >= self.bit_count {
            return None;
        }
        let byte = *self.bits.get(pixel / 8)?;
        Some(byte & (1 << (pixel % 8)) != 0)
    }
}
impl SampleInput<'_> {
    pub fn validate(&self, view: SceneView<'_>, control: &mut dyn WorkControl) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        if ![self.px, self.py, self.paint_x, self.paint_y]
            .into_iter()
            .all(f64::is_finite)
            || self.dash_seams.len() != self.local_edges.len()
        {
            return Err(invalid(0));
        }
        self.dash_cache.validate(control)?;
        for index in self.active_edges {
            control.charge(WorkKind::Edge, 1).map_err(stopped)?;
            if *index as usize >= view.edges.len() {
                return Err(invalid(0));
            }
        }
        Ok(())
    }
}
impl Rect {
    pub fn validate(self) -> Result<()> {
        if ![self.x, self.y, self.width, self.height]
            .into_iter()
            .all(f64::is_finite)
            || self.width < 0.0
            || self.height < 0.0
        {
            Err(invalid(0))
        } else {
            Ok(())
        }
    }
}
