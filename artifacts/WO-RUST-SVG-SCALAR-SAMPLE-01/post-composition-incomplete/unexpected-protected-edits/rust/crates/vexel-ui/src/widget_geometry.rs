//! Geometry validation shares the painter's branch and layout calculations.
//! The validation pass uses a zero origin so bounded local offsets cannot overflow.
//! Every emitted rectangle is translated with checked arithmetic before any paint.
use super::{Rect, WidgetCanvas, WidgetDrawSpec, WidgetKind};
use std::cell::Cell;

pub(super) struct GeometryPreflight {
    x: u32,
    y: u32,
    valid: Cell<bool>,
}

impl GeometryPreflight {
    pub(super) fn reject(&self) {
        self.valid.set(false);
    }

    pub(super) fn check(&self, local: Rect, scale: f64, active: bool) -> bool {
        let translated = local.x.checked_add(self.x).zip(local.y.checked_add(self.y));
        let valid = translated
            .is_some_and(|(x, y)| !active || scaled_rect(Rect { x, y, ..local }, scale).is_some());
        if !valid {
            self.reject();
        }
        valid
    }
}

/// C scaled_rect rules: round each component, then check both scaled endpoints.
/// A valid off-canvas rectangle stays valid; clipping belongs to the raster/callback.
pub(super) fn scaled_rect(r: Rect, scale: f64) -> Option<Rect> {
    if r.width == 0
        || r.height == 0
        || r.x.checked_add(r.width).is_none()
        || r.y.checked_add(r.height).is_none()
    {
        return None;
    }
    let values = [r.x, r.y, r.width, r.height].map(|v| (v as f64 * scale).round());
    if values
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0 || *v > u32::MAX as f64)
        || values[2] < 1.0
        || values[3] < 1.0
    {
        return None;
    }
    let p = Rect {
        x: values[0] as u32,
        y: values[1] as u32,
        width: values[2] as u32,
        height: values[3] as u32,
    };
    p.x.checked_add(p.width)?;
    p.y.checked_add(p.height)?;
    Some(p)
}

/// All composite public entries validate before the first pixel or text callback.
/// No fixed margin: the same component dispatch emits that kind's actual geometry.
pub(super) fn preflight(c: &mut WidgetCanvas<'_>, s: &WidgetDrawSpec<'_>) -> bool {
    if !super::valid_spec(c, s) || scaled_rect(s.bounds, c.scale()).is_none() {
        return false;
    }
    let mut local = *s;
    local.bounds.x = 0;
    local.bounds.y = 0;
    c.geometry = Some(GeometryPreflight {
        x: s.bounds.x,
        y: s.bounds.y,
        valid: Cell::new(true),
    });
    let result = super::paint_component(c, &local);
    let valid = c
        .geometry
        .take()
        .is_some_and(|geometry| geometry.valid.get());
    result && valid
}

pub(super) fn focus_bounds(s: &WidgetDrawSpec<'_>) -> Option<Rect> {
    let mut r = s.bounds;
    if s.kind == WidgetKind::Dropdown && s.expanded {
        r.height = 30;
    }
    // C explicitly clamps the left/top of the focus ring to zero.
    let x = r.x.saturating_sub(3);
    let y = r.y.saturating_sub(3);
    let right = r.x.checked_add(r.width)?.checked_add(3)?;
    let bottom = r.y.checked_add(r.height)?.checked_add(3)?;
    Some(Rect {
        x,
        y,
        width: right.checked_sub(x)?,
        height: bottom.checked_sub(y)?,
    })
}
