//! Edits behind the extended inspector controls. Each press is one session
//! edit (so one undo step) built from the document's own operations:
//! `set_attribute` for opacity and dash, `set_transform` for rotate and scale,
//! `add_*_gradient` + `set_fill_ref` + `set_gradient_stop` for gradients.

use crate::chrome::{
    Dash, FillKind, ShapeExt, TransformParts, TransformView, attr, hex_text, normalize_degrees,
    number, parse_hex, round_to,
};
use crate::file_ui::FileUi;
use crate::layout::ExtControl;
use crate::model::{INSPECTOR_SWATCHES, Model};
use crate::tools::ToolController;
use vexel_platform_apple::PanelBackend;
use vexel_v1_document::{
    GradientStop, GradientUnits, Spread, TransformSpec, V1Document, V1DocumentError,
};

pub const OPACITY_STEP: f64 = 0.1;
pub const ANGLE_STEP: f64 = 15.0;
pub const SCALE_STEP: f64 = 0.1;
pub const PIVOT_STEP: f64 = 1.0;
const SCALE_MAX: f64 = 10.0;
const FALLBACK_STOP: [u8; 3] = [0x1f, 0x29, 0x37];
const WHITE: [u8; 3] = [0xff, 0xff, 0xff];

type Edit = Result<Option<&'static str>, V1DocumentError>;

fn label(control: ExtControl) -> &'static str {
    match control {
        ExtControl::OpacityDown | ExtControl::OpacityUp => "Changed opacity",
        ExtControl::FillKind(0) => "Set solid fill",
        ExtControl::FillKind(1) => "Set linear gradient",
        ExtControl::FillKind(_) => "Set radial gradient",
        ExtControl::GradStop { .. } => "Changed gradient stop colour",
        ExtControl::AngleDown | ExtControl::AngleUp => "Changed gradient angle",
        ExtControl::Dash(_) => "Changed stroke style",
        ExtControl::RotateDown | ExtControl::RotateUp => "Rotated shape",
        ExtControl::PivotXDown
        | ExtControl::PivotXUp
        | ExtControl::PivotYDown
        | ExtControl::PivotYUp => "Moved pivot",
        _ => "Scaled shape",
    }
}

/// A press on an extended control: edits the tool selection through the
/// session and reports on the status line. Always consumes the press.
pub(crate) fn run<B: PanelBackend>(
    control: ExtControl,
    model: &mut Model,
    files: &mut FileUi<B>,
    tools: &ToolController,
) -> bool {
    model.dirty = true;
    let Some(id) = tools.selection().map(str::to_owned) else {
        model.status = "Select a shape first";
        return true;
    };
    model.status = match files.document_mut().edit(|doc| apply(doc, &id, control)) {
        Ok(Some(done)) => done,
        Ok(None) => "No change",
        Err(_) => "Could not apply that change",
    };
    true
}

/// Applies `control` to element `id`; `Ok(None)` when it changes nothing.
pub fn apply(doc: &mut V1Document, id: &str, control: ExtControl) -> Edit {
    let cur = ShapeExt::read(doc, id).ok_or(V1DocumentError::MissingStableId)?;
    let done = label(control);
    let changed = match control {
        ExtControl::OpacityDown => opacity(doc, id, &cur, -OPACITY_STEP)?,
        ExtControl::OpacityUp => opacity(doc, id, &cur, OPACITY_STEP)?,
        ExtControl::FillKind(k) => fill_kind(doc, id, &cur, k)?,
        ExtControl::GradStop { stop, swatch } => stop_colour(doc, id, &cur, stop, swatch)?,
        ExtControl::AngleDown => angle(doc, id, &cur, -ANGLE_STEP)?,
        ExtControl::AngleUp => angle(doc, id, &cur, ANGLE_STEP)?,
        ExtControl::Dash(k) => dash(doc, id, &cur, k)?,
        ExtControl::RotateDown => transform(doc, id, &cur, |p| p.rotation -= ANGLE_STEP)?,
        ExtControl::RotateUp => transform(doc, id, &cur, |p| p.rotation += ANGLE_STEP)?,
        ExtControl::ScaleXDown => transform(doc, id, &cur, |p| p.scale.0 = step(p.scale.0, -1.0))?,
        ExtControl::ScaleXUp => transform(doc, id, &cur, |p| p.scale.0 = step(p.scale.0, 1.0))?,
        ExtControl::ScaleYDown => transform(doc, id, &cur, |p| p.scale.1 = step(p.scale.1, -1.0))?,
        ExtControl::ScaleYUp => transform(doc, id, &cur, |p| p.scale.1 = step(p.scale.1, 1.0))?,
        ExtControl::PivotXDown => transform(doc, id, &cur, |p| {
            p.pivot.0 = round_to(p.pivot.0 - PIVOT_STEP, 3)
        })?,
        ExtControl::PivotXUp => transform(doc, id, &cur, |p| {
            p.pivot.0 = round_to(p.pivot.0 + PIVOT_STEP, 3)
        })?,
        ExtControl::PivotYDown => transform(doc, id, &cur, |p| {
            p.pivot.1 = round_to(p.pivot.1 - PIVOT_STEP, 3)
        })?,
        ExtControl::PivotYUp => transform(doc, id, &cur, |p| {
            p.pivot.1 = round_to(p.pivot.1 + PIVOT_STEP, 3)
        })?,
    };
    Ok(changed.then_some(done))
}

fn opacity(
    doc: &mut V1Document,
    id: &str,
    cur: &ShapeExt,
    delta: f64,
) -> Result<bool, V1DocumentError> {
    let next = round_to((cur.opacity + delta).clamp(0.0, 1.0), 2);
    if next == round_to(cur.opacity, 2) {
        return Ok(false);
    }
    doc.set_attribute(id, "opacity", &number(next))?;
    Ok(true)
}

/// One scale step. The sign is kept and the magnitude stays within
/// 0.1..=10, so a step never lands on zero or mirrors the shape.
fn step(value: f64, direction: f64) -> f64 {
    let next = round_to(value + direction * SCALE_STEP, 2);
    if value < 0.0 {
        next.clamp(-SCALE_MAX, -SCALE_STEP)
    } else {
        next.clamp(SCALE_STEP, SCALE_MAX)
    }
}

fn transform(
    doc: &mut V1Document,
    id: &str,
    cur: &ShapeExt,
    change: impl FnOnce(&mut TransformParts),
) -> Result<bool, V1DocumentError> {
    let TransformView::Parts(before) = cur.transform else {
        return Err(V1DocumentError::UnsupportedTransform);
    };
    let mut parts = before;
    change(&mut parts);
    parts.rotation = normalize_degrees(parts.rotation);
    if parts == before {
        return Ok(false);
    }
    doc.set_transform(
        id,
        &TransformSpec {
            translate: parts.translate,
            rotate_deg: parts.rotation,
            pivot: parts.pivot,
            scale: parts.scale,
            skew_x_deg: 0.0,
        },
    )?;
    Ok(true)
}

fn dash(
    doc: &mut V1Document,
    id: &str,
    cur: &ShapeExt,
    preset: usize,
) -> Result<bool, V1DocumentError> {
    let want = [Dash::None, Dash::Dashed, Dash::Dotted][preset.min(2)];
    if want == cur.dash {
        return Ok(false);
    }
    let info = doc.element_get(id)?;
    let width = attr(&info, "stroke-width")
        .and_then(|w| w.trim().parse::<f64>().ok())
        .filter(|w| w.is_finite())
        .unwrap_or(1.0)
        .clamp(1.0, 16.0);
    match want {
        Dash::Dashed => {
            let array = format!("{} {}", number(4.0 * width), number(3.0 * width));
            doc.set_attribute(id, "stroke-dasharray", &array)?;
            doc.set_attribute(id, "stroke-linecap", "round")?;
        }
        Dash::Dotted => {
            let array = format!("0 {}", number(2.0 * width));
            doc.set_attribute(id, "stroke-dasharray", &array)?;
            doc.set_attribute(id, "stroke-linecap", "round")?;
        }
        _ => {
            if attr(&info, "stroke-dasharray").is_some() {
                doc.remove_attribute(id, "stroke-dasharray")?;
            }
        }
    }
    Ok(true)
}

fn gradient_id(doc: &V1Document, id: &str) -> Result<String, V1DocumentError> {
    let info = doc.element_get(id)?;
    attr(&info, "fill")
        .and_then(|v| v.trim().strip_prefix("url(#"))
        .and_then(|v| v.strip_suffix(')'))
        .map(str::to_owned)
        .ok_or(V1DocumentError::Unsupported)
}

/// Adds a gradient over `stops` and points the fill at it.
fn paint_gradient(
    doc: &mut V1Document,
    id: &str,
    kind: FillKind,
    stops: [[u8; 3]; 2],
    angle: f64,
) -> Result<(), V1DocumentError> {
    let stops = [
        GradientStop {
            offset: 0.0,
            color: hex_text(stops[0]),
            opacity: 1.0,
        },
        GradientStop {
            offset: 1.0,
            color: hex_text(stops[1]),
            opacity: 1.0,
        },
    ];
    let units = GradientUnits::ObjectBoundingBox;
    let gradient = if kind == FillKind::Radial {
        doc.add_radial_gradient(0.5, 0.5, 0.5, 0.5, 0.5, units, Spread::Pad, &stops)?
    } else {
        let (sin, cos) = angle.to_radians().sin_cos();
        let at = |centre: f64, v: f64| round_to(centre + v * 0.5, 4);
        doc.add_linear_gradient(
            at(0.5, -cos),
            at(0.5, -sin),
            at(0.5, cos),
            at(0.5, sin),
            units,
            Spread::Pad,
            &stops,
        )?
    };
    doc.set_fill_ref(id, &gradient)
}

fn fill_kind(
    doc: &mut V1Document,
    id: &str,
    cur: &ShapeExt,
    want: usize,
) -> Result<bool, V1DocumentError> {
    let want = [FillKind::Solid, FillKind::Linear, FillKind::Radial][want.min(2)];
    if want == cur.fill {
        return Ok(false);
    }
    if want == FillKind::Solid {
        let first = cur.stops.map_or(FALLBACK_STOP, |s| s[0]);
        doc.set_fill(id, &hex_text(first))?;
        return Ok(true);
    }
    let stops = match cur.stops {
        Some(stops) => stops,
        None => {
            let info = doc.element_get(id)?;
            let first = attr(&info, "fill")
                .and_then(parse_hex)
                .unwrap_or(FALLBACK_STOP);
            [first, if first == WHITE { FALLBACK_STOP } else { WHITE }]
        }
    };
    paint_gradient(doc, id, want, stops, cur.angle.unwrap_or(0.0))?;
    Ok(true)
}

fn stop_colour(
    doc: &mut V1Document,
    id: &str,
    cur: &ShapeExt,
    stop: usize,
    swatch: usize,
) -> Result<bool, V1DocumentError> {
    let (Some(stops), true) = (cur.stops, cur.fill != FillKind::Solid) else {
        return Err(V1DocumentError::Unsupported);
    };
    let colour = INSPECTOR_SWATCHES
        .get(swatch)
        .and_then(|hex| parse_hex(hex))
        .ok_or(V1DocumentError::InvalidColor)?;
    let slot = usize::from(stop != 0);
    if stops[slot] == colour {
        return Ok(false);
    }
    let gradient = gradient_id(doc, id)?;
    let index = if slot == 0 {
        0
    } else {
        doc.element_get(&gradient)?.children.len().saturating_sub(1)
    };
    doc.set_gradient_stop(&gradient, index, None, Some(&hex_text(colour)), None)?;
    Ok(true)
}

fn angle(
    doc: &mut V1Document,
    id: &str,
    cur: &ShapeExt,
    delta: f64,
) -> Result<bool, V1DocumentError> {
    let (FillKind::Linear, Some(stops), Some(now)) = (cur.fill, cur.stops, cur.angle) else {
        return Err(V1DocumentError::Unsupported);
    };
    paint_gradient(
        doc,
        id,
        FillKind::Linear,
        stops,
        normalize_degrees(now + delta),
    )?;
    Ok(true)
}
