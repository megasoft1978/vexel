//! Easing curve drawing and the custom cubic-bezier editor popover.

use super::bezier;
use super::interaction::InteractionState;
use super::layout::{Hit, TimelineLayout};
use super::model::TimelineModel;
use super::paint::rect;
use crate::layout::paint::{mono, text};
use crate::ui_kit::{field, glyph_button};
use vexel_ui::{
    ControlState, FontWeight, Rect, WidgetCanvas, WidgetDrawSpec, WidgetKind, widget_paint,
};

/// Control points that approximate a preset easing; `None` for `step`.
fn preset_points(ease: &str) -> Option<[f64; 4]> {
    bezier::parse(ease).or(match ease {
        "ease_in" => Some([0.42, 0.0, 1.0, 1.0]),
        "ease_out" => Some([0.0, 0.0, 0.58, 1.0]),
        "ease_in_out" => Some([0.42, 0.0, 0.58, 1.0]),
        "step" => None,
        _ => Some([0.0, 0.0, 1.0, 1.0]),
    })
}

fn unit_box(r: Rect, pad: u32) -> Rect {
    rect(
        r.x + pad,
        r.y + pad,
        r.width.saturating_sub(2 * pad),
        r.height.saturating_sub(2 * pad),
    )
}

/// Point of the unit curve (x, y in 0..1, y up) inside `b`, kept inside `bounds`.
fn plot(b: Rect, bounds: Rect, x: f64, y: f64) -> (u32, u32) {
    let px = f64::from(b.x) + x * f64::from(b.width);
    let py = f64::from(b.y + b.height) - y * f64::from(b.height);
    (
        px.round().clamp(
            f64::from(bounds.x + 1),
            f64::from(bounds.x + bounds.width - 3),
        ) as u32,
        py.round().clamp(
            f64::from(bounds.y + 1),
            f64::from(bounds.y + bounds.height - 3),
        ) as u32,
    )
}

/// The curve of `ease` drawn as accent dots inside the box inset by `pad`.
pub(super) fn paint_curve(c: &mut WidgetCanvas<'_>, r: Rect, ease: &str, pad: u32) {
    let t = c.tokens;
    let b = unit_box(r, pad);
    if b.width < 4 || b.height < 4 {
        return;
    }
    match preset_points(ease) {
        Some(p) => paint_points(c, r, b, p),
        None => {
            let (x0, y0) = plot(b, r, 0.0, 0.0);
            let (xm, ym) = plot(b, r, 0.5, 1.0);
            let (x1, _) = plot(b, r, 1.0, 1.0);
            c.paint_rect(rect(x0, y0, xm.saturating_sub(x0) + 2, 2), t.accent);
            c.paint_rect(rect(xm, ym, x1.saturating_sub(xm) + 2, 2), t.accent);
            c.paint_rect(rect(xm, ym, 2, y0.saturating_sub(ym) + 2), t.accent);
        }
    }
}

fn paint_points(c: &mut WidgetCanvas<'_>, r: Rect, b: Rect, p: [f64; 4]) {
    let accent = c.tokens.accent;
    for (x, y) in bezier::sample(p, (b.width as usize * 2).clamp(24, 160)) {
        let (px, py) = plot(b, r, x, y);
        c.paint_rect(rect(px, py, 2, 2), accent);
    }
}

fn button(
    c: &mut WidgetCanvas<'_>,
    r: Rect,
    kind: WidgetKind,
    label: &str,
    s: &InteractionState,
    hit: Hit,
) {
    let mut spec = WidgetDrawSpec::new(kind, r);
    spec.label = Some(label);
    spec.state = if s.pressed() == Some(hit) && s.hover == Some(hit) {
        ControlState::Pressed
    } else if s.hover == Some(hit) {
        ControlState::Hover
    } else {
        ControlState::Rest
    };
    let _ = widget_paint(c, &spec);
}

/// Popover with a live curve preview, labelled X1 Y1 X2 Y2 steppers and
/// Cancel / Apply, in the inspector's popover and field styles.
pub(super) fn paint_bezier_editor(
    c: &mut WidgetCanvas<'_>,
    l: &TimelineLayout,
    m: &TimelineModel,
    s: &InteractionState,
) {
    if !s.bezier.open || m.selected_item().is_none() {
        return;
    }
    let (Some(editor), Some(preview)) = (l.bezier_editor(), l.bezier_preview()) else {
        return;
    };
    let t = c.tokens;
    c.paint_round(
        Rect {
            y: editor.y + 2,
            ..editor
        },
        8,
        t.popover_shadow,
        0,
        0,
    );
    c.paint_round(editor, 8, t.pressed_fill, t.control_edge, 1);
    text(
        c,
        "Custom easing",
        rect(editor.x + 12, editor.y + 8, 140, 24),
        t.text_primary,
        13,
        FontWeight::Semibold,
    );
    if let (Some(cancel), Some(apply)) = (l.bezier_cancel_rect(), l.bezier_apply_rect()) {
        button(
            c,
            cancel,
            WidgetKind::ButtonSecondary,
            "Cancel",
            s,
            Hit::BezierCancel,
        );
        button(
            c,
            apply,
            WidgetKind::ButtonPrimary,
            "Apply",
            s,
            Hit::BezierApply,
        );
    }
    field(c, preview);
    let b = unit_box(preview, 12);
    let v = s.bezier.values;
    // Unit square, then the two handles from the end points to the controls.
    for (x, y, w, h) in [
        (b.x, b.y, b.width, 1),
        (b.x, b.y + b.height, b.width, 1),
        (b.x, b.y, 1, b.height),
        (b.x + b.width, b.y, 1, b.height + 1),
    ] {
        c.paint_rect(rect(x, y, w, h), t.control_edge);
    }
    for ((ax, ay), (bx, by)) in [((0.0, 0.0), (v[0], v[1])), ((1.0, 1.0), (v[2], v[3]))] {
        let (x0, y0) = plot(b, preview, ax, ay);
        let (x1, y1) = plot(b, preview, bx, by);
        let steps = x0.abs_diff(x1).max(y0.abs_diff(y1)).max(1) / 3;
        for i in 0..=steps {
            let f = f64::from(i) / f64::from(steps.max(1));
            let px = (f64::from(x0) + (f64::from(x1) - f64::from(x0)) * f).round() as u32;
            let py = (f64::from(y0) + (f64::from(y1) - f64::from(y0)) * f).round() as u32;
            c.paint_rect(rect(px, py, 1, 1), t.text_tertiary);
        }
        c.paint_round(
            rect(x1.saturating_sub(2), y1.saturating_sub(2), 5, 5),
            2,
            t.text_secondary,
            0,
            0,
        );
    }
    paint_points(c, preview, b, v);
    for (field_index, name) in ["X1", "Y1", "X2", "Y2"].into_iter().enumerate() {
        let f = field_index as u8;
        let (Some(cell), Some(minus), Some(plus), Some(value)) = (
            l.bezier_cell_rect(f),
            l.bezier_step_rect(f, false),
            l.bezier_step_rect(f, true),
            l.bezier_value_rect(f),
        ) else {
            continue;
        };
        field(c, cell);
        text(
            c,
            name,
            rect(cell.x + 8, cell.y, 20, cell.height),
            t.text_secondary,
            12,
            FontWeight::Regular,
        );
        mono(
            c,
            &format!("{:.2}", s.bezier.values[field_index]),
            value,
            t.text_primary,
            12,
        );
        for (r, up) in [(minus, false), (plus, true)] {
            glyph_button(c, r, if up { "+" } else { "-" }, true);
            let hit = Hit::BezierStep { field: f, up };
            if s.hover == Some(hit) {
                let tint = if s.pressed() == Some(hit) {
                    t.pressed_fill
                } else {
                    t.panel_hover
                };
                c.paint_round(r, 5, tint, t.control_edge, 1);
                crate::layout::paint::centred(
                    c,
                    if up { "+" } else { "-" },
                    r,
                    t.text_primary,
                    14,
                    FontWeight::Semibold,
                );
            }
        }
    }
    if let Some(cell) = l.bezier_cell_rect(2) {
        text(
            c,
            "x 0 to 1, y -4 to 4, step 0.05",
            rect(cell.x, cell.y + cell.height + 6, 2 * cell.width + 6, 16),
            t.text_tertiary,
            11,
            FontWeight::Regular,
        );
    }
}
