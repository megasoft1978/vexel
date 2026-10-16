use super::*;

pub(super) fn apply_value(
    view: SceneView<'_>,
    t: &Track,
    item: OverlayItem,
    edges: &mut [Edge],
    colors: &mut [[u8; 4]],
    shapes: &mut [Shape],
    fills: &mut [Paint],
    _strokes: &mut [Paint],
    world: &[Affine],
    control: &mut dyn WorkControl,
    work: &mut usize,
    ce: &mut bool,
    cc: &mut bool,
    cs: &mut bool,
    cf: &mut bool,
    _csp: &mut bool,
    clip_shape_flags: &mut [bool],
) -> Result<()> {
    let mut descendant = false;
    let mut direct = false;
    let target_end = t
        .target
        .offset
        .checked_add(t.target.length)
        .ok_or_else(arithmetic)?;
    for (i, shape) in shapes.iter_mut().enumerate() {
        charge(control, 1)?;
        *work = work.checked_add(1).ok_or_else(arithmetic)?;
        if *work > WORK_MAX {
            return Err(limit(LimitKind::Work));
        }
        let mut node = shape.source.transform_node;
        let mut depth = 0;
        let mut in_target = shape.source.element.start >= t.target.offset
            && shape.source.element.start < target_end;
        while let Some(id) = node {
            if depth >= CHAIN_MAX {
                return Err(limit(LimitKind::Animation));
            }
            let n = view
                .transform(id)
                .ok_or_else(|| invalid(t.element.offset))?;
            charge(control, 1)?;
            count_work(work, 1)?;
            if n.source_offset >= t.target.offset && n.source_offset < target_end {
                in_target = true;
            }
            node = n.parent;
            depth += 1;
        }
        if !in_target {
            continue;
        }
        descendant = true;
        let is_direct = shape.source.element.start == t.target.offset
            || shape.source.transform_node.is_some_and(|id| {
                view.transform(id)
                    .is_some_and(|n| n.source_offset == t.target.offset)
            });
        if !is_direct {
            continue;
        }
        direct = true;
        match t.attribute {
            Attribute::Fill if item.value.kind == ValueKind::Color => {
                let r = shape.edges;
                if r.end().is_none_or(|e| e > colors.len()) {
                    return Err(invalid(t.element.offset));
                }
                charge(control, r.count)?;
                for c in &mut colors[r.first..r.first + r.count] {
                    *c = item.value.rgba;
                }
                *cc = true;
            }
            Attribute::Stroke if item.value.kind == ValueKind::Color => {
                shape.stroke.rgba = item.value.rgba;
                *cs = true;
            }
            Attribute::Opacity | Attribute::FillOpacity if item.value.kind == ValueKind::Number => {
                let op = item.value.number[0].clamp(0., 1.);
                let r = shape.edges;
                if r.end().is_none_or(|e| e > colors.len()) {
                    return Err(invalid(t.element.offset));
                }
                charge(control, r.count)?;
                for c in &mut colors[r.first..r.first + r.count] {
                    c[3] = (f64::from(c[3]) * op + 0.5).floor() as u8;
                }
                *cc = true;
                if t.attribute == Attribute::Opacity {
                    shape.stroke.rgba[3] =
                        (f64::from(shape.stroke.rgba[3]) * op + 0.5).floor() as u8;
                    *cs = true;
                }
            }
            Attribute::StrokeOpacity if item.value.kind == ValueKind::Number => {
                let op = item.value.number[0].clamp(0., 1.);
                shape.stroke.rgba[3] = (f64::from(shape.stroke.rgba[3]) * op + 0.5).floor() as u8;
                *cs = true;
            }
            Attribute::StrokeWidth | Attribute::StrokeDashoffset
                if item.value.kind == ValueKind::Number =>
            {
                let v = item.value.number[0];
                if v.abs() > 1e6 || (t.attribute == Attribute::StrokeWidth && v < 0.) {
                    return Err(diag(ErrorKind::Unsupported, t.element.offset));
                }
                if t.attribute == Attribute::StrokeWidth {
                    shape.stroke.width = v
                } else {
                    shape.stroke.dash_offset = v
                }
                *cs = true;
            }
            Attribute::D => return Err(diag(ErrorKind::Unsupported, t.element.offset)),
            Attribute::X
            | Attribute::Y
            | Attribute::Cx
            | Attribute::Cy
            | Attribute::Width
            | Attribute::Height
            | Attribute::Rx
            | Attribute::Ry
            | Attribute::R
                if item.value.kind == ValueKind::Number =>
            {
                if has_effect(view, i, control, work)? {
                    return Err(diag(ErrorKind::Unsupported, t.element.offset));
                }
                let delta = item.value.number[0] - t.base_value.number[0];
                if !delta.is_finite() || delta.abs() > 1e6 {
                    return Err(diag(ErrorKind::Unsupported, t.element.offset));
                }
                let mut local = affine_identity();
                match t.attribute {
                    Attribute::X | Attribute::Cx => local.e = delta,
                    Attribute::Y | Attribute::Cy => local.f = delta,
                    _ => {
                        let base = t.base_value.number[0];
                        if base <= 0. {
                            return Err(diag(ErrorKind::Unsupported, t.element.offset));
                        }
                        let scale = item.value.number[0] / base;
                        if !scale.is_finite() || scale < 0. || scale > 1e6 {
                            return Err(diag(ErrorKind::Unsupported, t.element.offset));
                        }
                        let inv = inverse(world[i], 1e-15)
                            .ok_or_else(|| diag(ErrorKind::Unsupported, t.element.offset))?;
                        let b = shape_bounds(shape, edges, inv, control, work)?;
                        if t.attribute == Attribute::Height || t.attribute == Attribute::Ry {
                            local.d = scale;
                            local.f = b[1] * (1. - scale);
                        } else {
                            let center = if t.attribute == Attribute::R {
                                (b[0] + b[2]) * 0.5
                            } else {
                                b[0]
                            };
                            local.a = scale;
                            local.e = center * (1. - scale);
                            if t.attribute == Attribute::R {
                                local.d = scale;
                                local.f = ((b[1] + b[3]) * 0.5) * (1. - scale);
                            }
                        }
                    }
                }
                if shape
                    .fill_paint
                    .is_some_and(|p| fills.get(p).is_some_and(|p| p.kind == PaintKind::Pattern))
                {
                    return Err(diag(ErrorKind::Unsupported, t.element.offset));
                }
                let matrix = affine_compose(
                    affine_compose(world[i], local).ok_or_else(|| invalid(0))?,
                    inverse(world[i], 1e-15).ok_or_else(|| invalid(0))?,
                )
                .ok_or_else(|| invalid(0))?;
                map_shape_edges(view, shape, matrix, edges, control, work)?;
                *ce = true;
                if !clip_shape_flags.is_empty() {
                    clip_shape_flags[i] = true;
                }
                if let Some(p) = shape.fill_paint.and_then(|n| fills.get_mut(n)) {
                    if p.object_bounding_box
                        && (p.kind == PaintKind::Linear || p.kind == PaintKind::Radial)
                    {
                        p.bounds = shape_bounds(
                            shape,
                            edges,
                            inverse(world[i], 1e-15).ok_or_else(|| invalid(0))?,
                            control,
                            work,
                        )?;
                        *cf = true;
                    }
                }
            }
            _ => return Err(diag(ErrorKind::Unsupported, t.element.offset)),
        }
    }
    if descendant && !direct {
        return Err(diag(ErrorKind::Unsupported, t.element.offset));
    }
    Ok(())
}
