//! Paint helpers that draw a control from its interpolated [`ControlVisual`].
use crate::render::label;
use vexel_ui::motion::ControlVisual;
use vexel_ui::{FontWeight, Rect, WidgetCanvas};

/// 2 px accent ring at 2 px offset, faded by the visual's alpha (keyboard focus).
pub fn ring(c: &mut WidgetCanvas<'_>, bounds: Rect, v: &ControlVisual) {
    if v.ring & 255 != 0 {
        let r = Rect {
            x: bounds.x.saturating_sub(3),
            y: bounds.y.saturating_sub(3),
            width: bounds.width + 6,
            height: bounds.height + 6,
        };
        c.paint_round(r, 9, 0, v.ring, 2);
    }
}

/// Mode tab: filled round rect plus its label, same geometry as the static look.
pub fn segment(c: &mut WidgetCanvas<'_>, text: &str, bounds: Rect, v: &ControlVisual) {
    c.paint_round(bounds, 6, v.fill, 0, 0);
    let inner = Rect {
        x: bounds.x + 8,
        y: bounds.y,
        width: bounds.width.saturating_sub(16),
        height: bounds.height,
    };
    label(c, text, inner, v.glyph, 13, FontWeight::Regular);
    ring(c, bounds, v);
}
