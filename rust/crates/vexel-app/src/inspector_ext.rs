//! Painting of the extended inspector rows: opacity, fill kind and gradient
//! stops, dash presets and the transform steppers. Rects come from the layout
//! (`Layout::ext_row` / `ExtControl`), the same ones the hit test uses.

use crate::chrome::{Dash, FillKind, ShapeExt, TransformView, number, parse_hex};
use crate::layout::paint::{mono, text, text_width};
use crate::layout::{ExtRow, Layout, STOP_SWATCHES, segments, stepper_buttons, stop_swatch};
use crate::model::INSPECTOR_SWATCHES;
use crate::ui_kit::{field, glyph_button, segmented_cells};
use vexel_ui::{FontWeight, Rect, WidgetCanvas};

fn caption(c: &mut WidgetCanvas<'_>, r: Rect, label: &str) {
    let t = c.tokens;
    text(
        c,
        label,
        Rect {
            x: r.x + 8,
            width: 64,
            ..r
        },
        t.text_secondary,
        12,
        FontWeight::Regular,
    );
}

fn value(c: &mut WidgetCanvas<'_>, r: Rect, v: &str, enabled: bool) {
    let t = c.tokens;
    let x = r.x + 8 + text_width("Scale X", 12).max(14) + 12;
    let (_, plus) = stepper_buttons(r);
    mono(
        c,
        v,
        Rect {
            x,
            width: plus.x.saturating_sub(x + 36),
            ..r
        },
        if enabled {
            t.text_primary
        } else {
            t.text_secondary
        },
        12,
    );
}

/// Caption, value and `-` / `+` buttons in one field.
fn stepper(
    c: &mut WidgetCanvas<'_>,
    row: Rect,
    label: &str,
    shown: Option<&str>,
    can_down: bool,
    can_up: bool,
) {
    field(c, row);
    caption(c, row, label);
    value(c, row, shown.unwrap_or("\u{2014}"), shown.is_some());
    let (down, up) = stepper_buttons(row);
    glyph_button(c, down, "-", can_down);
    glyph_button(c, up, "+", can_up);
}

/// Half a row: one-letter caption, value and `-` / `+`, for pivot X and Y.
fn half_stepper(c: &mut WidgetCanvas<'_>, row: Rect, label: &str, shown: Option<&str>, on: bool) {
    let t = c.tokens;
    field(c, row);
    text(
        c,
        label,
        Rect {
            x: row.x + 8,
            width: 12,
            ..row
        },
        t.text_secondary,
        12,
        FontWeight::Regular,
    );
    let (down, up) = stepper_buttons(row);
    let x = row.x + 24;
    let colour = if on { t.text_primary } else { t.text_secondary };
    mono(
        c,
        shown.unwrap_or("\u{2014}"),
        Rect {
            x,
            width: down.x.saturating_sub(x + 4),
            ..row
        },
        colour,
        12,
    );
    glyph_button(c, down, "-", on);
    glyph_button(c, up, "+", on);
}

fn segmented(c: &mut WidgetCanvas<'_>, row: Rect, labels: [&str; 3], active: Option<usize>) {
    segmented_cells(c, &segments(row), &labels, active, None, true);
}

fn swatch_rgb(index: usize) -> Option<([u8; 3], u32)> {
    let rgb = parse_hex(INSPECTOR_SWATCHES.get(index)?)?;
    Some((rgb, u32::from_be_bytes([rgb[0], rgb[1], rgb[2], 255])))
}

/// Selection section: opacity.
pub(super) fn draw_selection(c: &mut WidgetCanvas<'_>, l: &Layout, ext: Option<&ShapeExt>) {
    let Some(row) = l.ext_row(ExtRow::Opacity) else {
        return;
    };
    let shown = ext.map(|e| number(e.opacity));
    let o = ext.map_or(1.0, |e| e.opacity);
    stepper(
        c,
        row,
        "Opacity",
        shown.as_deref(),
        ext.is_some() && o > 0.0,
        ext.is_some() && o < 1.0,
    );
}

/// Fill section: Solid | Linear | Radial, stop swatches and the angle.
pub(super) fn draw_fill(c: &mut WidgetCanvas<'_>, l: &Layout, ext: Option<&ShapeExt>) {
    let t = c.tokens;
    let Some(kind_row) = l.ext_row(ExtRow::FillKind) else {
        return;
    };
    let active = ext.map(|e| match e.fill {
        FillKind::Solid => 0,
        FillKind::Linear => 1,
        FillKind::Radial => 2,
    });
    segmented(c, kind_row, ["Solid", "Linear", "Radial"], active);
    let gradient = ext.filter(|e| e.fill != FillKind::Solid);
    let (Some(first), Some(last), Some(angle)) = (
        l.ext_row(ExtRow::StopFirst),
        l.ext_row(ExtRow::StopLast),
        l.ext_row(ExtRow::Angle),
    ) else {
        return;
    };
    let Some(e) = gradient else {
        text(
            c,
            "Choose Linear or Radial for a gradient",
            Rect {
                x: first.x,
                width: first.width,
                height: 20,
                ..first
            },
            t.text_tertiary,
            12,
            FontWeight::Regular,
        );
        return;
    };
    for (row, label, slot) in [(first, "Start", 0), (last, "End", 1)] {
        text(
            c,
            label,
            Rect { width: 44, ..row },
            t.text_secondary,
            12,
            FontWeight::Regular,
        );
        for index in 0..STOP_SWATCHES {
            let Some((rgb, packed)) = swatch_rgb(index) else {
                continue;
            };
            let r = stop_swatch(row, index);
            c.paint_round(r, 5, packed, t.control_edge, 1);
            if e.stops.is_some_and(|s| s[slot] == rgb) {
                let ring = Rect {
                    x: r.x.saturating_sub(2),
                    y: r.y.saturating_sub(2),
                    width: r.width + 4,
                    height: r.height + 4,
                };
                c.paint_round(ring, 6, 0, t.accent, 1);
                c.paint_round(r, 5, packed, t.control_edge, 1);
            }
        }
    }
    let linear = e.fill == FillKind::Linear;
    let shown = e
        .angle
        .filter(|_| linear)
        .map(|a| format!("{}\u{b0}", number(a)));
    stepper(
        c,
        angle,
        "Angle",
        shown.as_deref().or(Some("n/a")),
        linear,
        linear,
    );
}

/// Stroke section: dash presets.
pub(super) fn draw_stroke(c: &mut WidgetCanvas<'_>, l: &Layout, ext: Option<&ShapeExt>) {
    let Some(row) = l.ext_row(ExtRow::Dash) else {
        return;
    };
    let active = ext.and_then(|e| match e.dash {
        Dash::None => Some(0),
        Dash::Dashed => Some(1),
        Dash::Dotted => Some(2),
        Dash::Custom => None,
    });
    segmented(c, row, ["Solid", "Dashed", "Dotted"], active);
}

/// Transform section: rotation, pivot X/Y and scale (or "custom").
pub(super) fn draw_transform(c: &mut WidgetCanvas<'_>, l: &Layout, ext: Option<&ShapeExt>) {
    let parts = ext.and_then(|e| match e.transform {
        TransformView::Parts(p) => Some(p),
        TransformView::Custom => None,
    });
    let custom = ext.is_some() && parts.is_none();
    let shown = |v: Option<String>| v.or_else(|| custom.then(|| "custom".to_owned()));
    let rows = [
        (
            ExtRow::Rotate,
            "Rotate",
            parts.map(|p| format!("{}\u{b0}", number(p.rotation))),
        ),
        (ExtRow::ScaleX, "Scale X", parts.map(|p| number(p.scale.0))),
        (ExtRow::ScaleY, "Scale Y", parts.map(|p| number(p.scale.1))),
    ];
    for (which, label, v) in rows {
        if let Some(row) = l.ext_row(which) {
            let v = shown(v);
            let on = parts.is_some();
            stepper(c, row, label, v.as_deref(), on, on);
        }
    }
    let pivots = [
        (ExtRow::PivotX, "X", parts.map(|p| number(p.pivot.0))),
        (ExtRow::PivotY, "Y", parts.map(|p| number(p.pivot.1))),
    ];
    for (which, label, v) in pivots {
        if let Some(row) = l.ext_row(which) {
            let v = shown(v);
            half_stepper(c, row, label, v.as_deref(), parts.is_some());
        }
    }
}
