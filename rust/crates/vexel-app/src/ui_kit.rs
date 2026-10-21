//! Small shared controls in the inspector's visual language: a rounded
//! level-3 value field, a bordered glyph button, a segmented row with an
//! accent ring on the active cell and an icon button with an active state.
//! The inspector and the native timeline both paint through these, so the
//! two panels cannot drift apart.

use crate::layout::paint::centred;
use vexel_ui::{FontWeight, IconKind, Rect, WidgetCanvas};

/// Rounded level-3 field behind a caption and value.
pub(crate) fn field(c: &mut WidgetCanvas<'_>, r: Rect) {
    let fill = c.tokens.surface[3];
    c.paint_round(r, 6, fill, 0, 0);
}

/// A small square glyph button (`-` or `+`).
pub(crate) fn glyph_button(c: &mut WidgetCanvas<'_>, r: Rect, glyph: &str, enabled: bool) {
    let t = c.tokens;
    c.paint_round(r, 5, t.surface[4], t.control_edge, 1);
    let colour = if enabled {
        t.text_primary
    } else {
        t.text_tertiary
    };
    centred(c, glyph, r, colour, 14, FontWeight::Semibold);
}

fn ring(r: Rect) -> Rect {
    Rect {
        x: r.x.saturating_sub(1),
        y: r.y.saturating_sub(1),
        width: r.width + 2,
        height: r.height + 2,
    }
}

/// One cell per label; the active cell gets the accent ring. `hover` lifts a
/// cell to the hover level and `enabled == false` dims every label.
pub(crate) fn segmented_cells(
    c: &mut WidgetCanvas<'_>,
    cells: &[Rect],
    labels: &[&str],
    active: Option<usize>,
    hover: Option<usize>,
    enabled: bool,
) {
    let t = c.tokens;
    for (i, (rect, label)) in cells.iter().zip(labels).enumerate() {
        let fill = if enabled && hover == Some(i) && active != Some(i) {
            t.surface[4]
        } else {
            t.surface[3]
        };
        c.paint_round(*rect, 5, fill, t.control_edge, 1);
        if enabled && active == Some(i) {
            c.paint_round(ring(*rect), 6, 0, t.accent, 1);
            c.paint_round(*rect, 5, t.surface[3], t.control_edge, 1);
        }
        let colour = if enabled {
            t.text_primary
        } else {
            t.text_tertiary
        };
        centred(c, label, *rect, colour, 12, FontWeight::Semibold);
    }
}

/// Bordered square icon button: hover and pressed lift the fill, `active`
/// draws the accent ring and an accent glyph, `enabled == false` dims it.
pub(crate) fn icon_button(
    c: &mut WidgetCanvas<'_>,
    r: Rect,
    icon: IconKind,
    enabled: bool,
    active: bool,
    hovered: bool,
    pressed: bool,
) {
    let t = c.tokens;
    let fill = if pressed && enabled {
        t.pressed_fill
    } else if hovered && enabled {
        t.panel_hover
    } else {
        t.surface[3]
    };
    if active && enabled {
        c.paint_round(ring(r), 6, 0, t.accent, 1);
    }
    c.paint_round(r, 5, fill, t.control_edge, 1);
    let colour = if !enabled {
        t.disabled_text
    } else if active {
        t.accent
    } else if hovered {
        t.text_primary
    } else {
        t.text_secondary
    };
    let side = r.height.min(r.width).saturating_sub(12).clamp(10, 16);
    c.paint_icon_color(
        icon,
        Rect {
            x: r.x + r.width.saturating_sub(side) / 2,
            y: r.y + r.height.saturating_sub(side) / 2,
            width: side,
            height: side,
        },
        colour,
    );
}
