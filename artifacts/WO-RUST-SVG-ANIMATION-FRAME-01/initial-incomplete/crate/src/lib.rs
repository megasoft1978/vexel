//! Copy-on-change SVG animation frame evaluation over an immutable scene.
#![forbid(unsafe_code)]

use vexel_geometry::{affine_compose, affine_identity, affine_map_point};
use vexel_scene::{
    Affine, Diagnostic, Edge, ErrorKind, EvaluatedFrame, FrameBuilder, LimitKind, Paint, PaintKind,
    Result, SceneView, Shape, ShapeId, TransformFrame, TransformId, WorkControl, WorkKind,
};
use vexel_svg_animation::{
    Attribute, EvaluationError, Kind, OverlayItem, Track, TransformType, Value, ValueKind,
    evaluate_view_controlled,
};

const CHAIN_MAX: usize = 128;
const WORK_MAX: usize = 50_000_000;
#[allow(dead_code)]
const CLIP_BYTES_MAX: usize = 64 * 1024 * 1024;
#[allow(dead_code)]
const CLIP_OWNER_DEPTH_MAX: usize = 64;

fn diag(kind: ErrorKind, offset: usize) -> Diagnostic {
    Diagnostic { kind, offset }
}
fn invalid(offset: usize) -> Diagnostic {
    diag(ErrorKind::InvalidArgument, offset)
}
fn limit(kind: LimitKind) -> Diagnostic {
    diag(ErrorKind::Limit(kind), 0)
}
fn arithmetic() -> Diagnostic {
    limit(LimitKind::Arithmetic)
}
fn oom() -> Diagnostic {
    diag(ErrorKind::OutOfMemory, 0)
}
fn charge(control: &mut dyn WorkControl, amount: usize) -> Result<()> {
    let units = u64::try_from(amount).map_err(|_| arithmetic())?;
    control
        .charge(WorkKind::AnimationTransform, units)
        .map_err(|s| match s {
            vexel_scene::Stop::Canceled => diag(ErrorKind::Canceled, 0),
            vexel_scene::Stop::Limit => limit(LimitKind::Work),
            vexel_scene::Stop::Overflow => arithmetic(),
        })
}
fn copy_table<T: Copy>(values: &[T], control: &mut dyn WorkControl) -> Result<Vec<T>> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(values.len()).map_err(|_| oom())?;
    for value in values {
        control
            .charge(WorkKind::AnimationTransform, 1)
            .map_err(|s| match s {
                vexel_scene::Stop::Canceled => diag(ErrorKind::Canceled, 0),
                vexel_scene::Stop::Limit => limit(LimitKind::Work),
                vexel_scene::Stop::Overflow => arithmetic(),
            })?;
        copy.push(*value);
    }
    Ok(copy)
}

fn inverse(m: Affine, threshold: f64) -> Option<Affine> {
    let determinant = m.a * m.d - m.b * m.c;
    if !determinant.is_finite() || determinant.abs() < threshold {
        return None;
    }
    let k = 1.0 / determinant;
    let out = Affine {
        a: m.d * k,
        b: -m.b * k,
        c: -m.c * k,
        d: m.a * k,
        e: (m.c * m.f - m.d * m.e) * k,
        f: (m.b * m.e - m.a * m.f) * k,
    };
    vexel_geometry::affine_valid(out).then_some(out)
}
fn transform_value(track: &Track, value: Value) -> Option<Affine> {
    let x = value.number[0];
    let y = value.number[1];
    if !x.is_finite() || !y.is_finite() || x.abs() > 1e6 || y.abs() > 1e6 {
        return None;
    }
    let mut m = affine_identity();
    match track.transform_type {
        TransformType::Translate => {
            m.e = x;
            m.f = y;
        }
        TransformType::Scale => {
            m.a = x;
            m.d = if value.number[3] >= 2. { y } else { x };
        }
        TransformType::Rotate => {
            let angle = x * 0.017453292519943295769;
            let (s, c) = angle.sin_cos();
            if !s.is_finite() || !c.is_finite() {
                return None;
            }
            m = Affine {
                a: c,
                b: s,
                c: -s,
                d: c,
                e: 0.,
                f: 0.,
            };
            if value.number[3] >= 3. {
                let to = Affine {
                    e: value.number[1],
                    f: value.number[2],
                    ..affine_identity()
                };
                let from = Affine {
                    e: -value.number[1],
                    f: -value.number[2],
                    ..affine_identity()
                };
                m = affine_compose(affine_compose(to, m)?, from)?;
            }
        }
        TransformType::SkewX | TransformType::SkewY => {
            let t = (x * 0.017453292519943295769).tan();
            if !t.is_finite() || t.abs() > 1e6 {
                return None;
            }
            if track.transform_type == TransformType::SkewX {
                m.c = t
            } else {
                m.b = t
            }
        }
    }
    vexel_geometry::affine_valid(m).then_some(m)
}
fn product(
    transforms: &[TransformFrame],
    shape: &Shape,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<Affine> {
    let mut chain = [None; CHAIN_MAX];
    let mut count = 0;
    let mut node = shape.source.transform_node;
    while let Some(id) = node {
        if count == CHAIN_MAX || id.index() >= transforms.len() {
            return Err(invalid(shape.source.element.start));
        }
        charge(control, 1)?;
        *work = work.checked_add(1).ok_or_else(arithmetic)?;
        if *work > WORK_MAX {
            return Err(limit(LimitKind::Work));
        }
        chain[count] = Some(id);
        count += 1;
        node = transforms[id.index()].parent;
    }
    let mut out = affine_identity();
    while count > 0 {
        count -= 1;
        let n = transforms[chain[count].ok_or_else(|| invalid(0))?.index()];
        let local = affine_compose(n.attribute, n.suffix)
            .and_then(|m| affine_compose(n.motion, m))
            .ok_or_else(|| invalid(n.source_offset))?;
        out = affine_compose(out, local).ok_or_else(|| invalid(n.source_offset))?;
    }
    Ok(out)
}
fn shape_bounds(
    shape: &Shape,
    current_edges: &[Edge],
    inverse: Affine,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<[f64; 4]> {
    let edges = shape
        .edges
        .slice(current_edges)
        .ok_or_else(|| invalid(shape.source.element.start))?;
    if edges.is_empty() {
        return Err(invalid(shape.source.element.start));
    }
    charge(control, edges.len())?;
    *work = work.checked_add(edges.len()).ok_or_else(arithmetic)?;
    if *work > WORK_MAX {
        return Err(limit(LimitKind::Work));
    }
    let p = affine_map_point(inverse, edges[0].a).ok_or_else(|| invalid(0))?;
    let mut b = [p.x, p.y, p.x, p.y];
    for edge in edges {
        for point in [edge.a, edge.b] {
            let q = affine_map_point(inverse, point).ok_or_else(|| invalid(0))?;
            b[0] = b[0].min(q.x);
            b[1] = b[1].min(q.y);
            b[2] = b[2].max(q.x);
            b[3] = b[3].max(q.y);
        }
    }
    Ok(b)
}
fn map_shape_edges(
    view: SceneView<'_>,
    shape: &Shape,
    matrix: Affine,
    edges: &mut [Edge],
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<()> {
    let range = shape.edges;
    let end = range.end().ok_or_else(arithmetic)?;
    if end > edges.len() {
        return Err(invalid(shape.source.element.start));
    }
    charge(control, range.count)?;
    *work = work.checked_add(range.count).ok_or_else(arithmetic)?;
    if *work > WORK_MAX {
        return Err(limit(LimitKind::Work));
    }
    for edge in &mut edges[range.first..end] {
        edge.a = affine_map_point(matrix, edge.a).ok_or_else(|| invalid(0))?;
        edge.b = affine_map_point(matrix, edge.b).ok_or_else(|| invalid(0))?;
    }
    let _ = view;
    Ok(())
}

/// Evaluate parsed animation tracks and apply their overlay to a temporary frame.
/// The returned frame borrows its source scene and owns only changed tables.
pub fn evaluate_frame<'a>(
    view: SceneView<'a>,
    time_seconds: f64,
    control: &mut dyn WorkControl,
) -> Result<EvaluatedFrame<'a>> {
    let overlay =
        evaluate_view_controlled(view.animation(), time_seconds, control).map_err(|e| match e {
            EvaluationError::InvalidTime => invalid(0),
            EvaluationError::InvalidTrack => diag(ErrorKind::Unsupported, 0),
            EvaluationError::OutOfMemory => oom(),
            EvaluationError::Canceled => diag(ErrorKind::Canceled, 0),
            EvaluationError::Limit => limit(LimitKind::Work),
            EvaluationError::Overflow => arithmetic(),
        })?;
    let mut builder = FrameBuilder::new(view, usize::MAX)?;
    let mut work = 0usize;
    let mut transforms = copy_table(view.transforms, control)?;
    let mut transform_seen = Vec::new();
    transform_seen
        .try_reserve_exact(view.transforms.len())
        .map_err(|_| oom())?;
    transform_seen.resize(view.transforms.len(), false);
    let mut world = Vec::new();
    world
        .try_reserve_exact(view.shapes.len())
        .map_err(|_| oom())?;
    for shape in view.shapes {
        world.push(product(view.transforms, shape, control, &mut work)?);
    }
    let mut edges = copy_table(view.edges, control)?;
    let mut colors = copy_table(view.edge_colors, control)?;
    let mut shapes = copy_table(view.shapes, control)?;
    let mut fills = copy_table(view.fill_paints, control)?;
    let mut strokes = copy_table(view.stroke_paints, control)?;
    let mut changed_edges = false;
    let mut changed_colors = false;
    let mut changed_shapes = false;
    let mut changed_fills = false;
    let mut changed_strokes = false;
    for item in &overlay.items {
        if !item.active {
            continue;
        }
        charge(control, 1)?;
        work = work.checked_add(1).ok_or_else(arithmetic)?;
        if work > WORK_MAX {
            return Err(limit(LimitKind::Work));
        }
        let track = view
            .animation()
            .tracks
            .get(item.track_index)
            .ok_or_else(|| invalid(0))?;
        match track.kind {
            Kind::Transform | Kind::Motion => {
                let anim = if track.kind == Kind::Transform {
                    transform_value(track, item.value)
                        .ok_or_else(|| diag(ErrorKind::Unsupported, track.element.offset))?
                } else if item.value.kind == ValueKind::Motion {
                    let (s, c) = (item.value.number[2] * 0.017453292519943295769).sin_cos();
                    Affine {
                        a: c,
                        b: s,
                        c: -s,
                        d: c,
                        e: item.value.number[0],
                        f: item.value.number[1],
                    }
                } else {
                    return Err(diag(ErrorKind::Unsupported, track.element.offset));
                };
                let mut found = false;
                for (i, node) in transforms.iter_mut().enumerate() {
                    charge(control, 1)?;
                    work += 1;
                    if work > WORK_MAX {
                        return Err(limit(LimitKind::Work));
                    }
                    if node.source_offset != track.target.offset {
                        continue;
                    }
                    found = true;
                    if track.kind == Kind::Motion {
                        node.motion = affine_compose(node.motion, anim)
                            .ok_or_else(|| invalid(track.element.offset))?;
                    } else if track.additive_sum || transform_seen[i] {
                        node.attribute = affine_compose(node.attribute, anim)
                            .ok_or_else(|| invalid(track.element.offset))?;
                    } else {
                        node.attribute = anim;
                    }
                    transform_seen[i] = true;
                }
                if found {
                    for si in 0..shapes.len() {
                        let shape = shapes[si];
                        charge(control, 1)?;
                        work += 1;
                        if work > WORK_MAX {
                            return Err(limit(LimitKind::Work));
                        }
                        let mut n = shape.source.transform_node;
                        let mut has = false;
                        let mut depth = 0;
                        while let Some(id) = n {
                            if depth >= CHAIN_MAX || id.index() >= transforms.len() {
                                return Err(invalid(track.element.offset));
                            }
                            charge(control, 1)?;
                            work += 1;
                            if work > WORK_MAX {
                                return Err(limit(LimitKind::Work));
                            }
                            if transforms[id.index()].source_offset == track.target.offset {
                                has = true;
                                break;
                            }
                            n = transforms[id.index()].parent;
                            depth += 1;
                        }
                        if !has {
                            continue;
                        }
                        if has_effect(view, si, control, &mut work)? {
                            return Err(diag(ErrorKind::Unsupported, track.element.offset));
                        }
                        let next = product(&transforms, &shape, control, &mut work)?;
                        let delta = affine_compose(
                            next,
                            inverse(world[si], 1e-15)
                                .ok_or_else(|| invalid(track.element.offset))?,
                        )
                        .ok_or_else(|| invalid(track.element.offset))?;
                        map_shape_edges(view, &shape, delta, &mut edges, control, &mut work)?;
                        changed_edges = true;
                        world[si] = next;
                        let mut updated = shape;
                        updated.stroke_inverse = affine_compose(
                            updated.stroke_inverse,
                            inverse(delta, 1e-15).ok_or_else(|| invalid(track.element.offset))?,
                        )
                        .ok_or_else(|| invalid(track.element.offset))?;
                        if let Some(index) = updated.fill_paint {
                            if let Some(p) = fills.get_mut(index) {
                                if p.kind != PaintKind::Solid {
                                    p.user_inverse = affine_compose(
                                        p.user_inverse,
                                        inverse(delta, 1e-15).ok_or_else(|| invalid(0))?,
                                    )
                                    .ok_or_else(|| invalid(0))?;
                                    changed_fills = true;
                                }
                            }
                        }
                        if let Some(index) = updated.stroke_paint {
                            if let Some(p) = strokes.get_mut(index) {
                                if p.kind != PaintKind::Solid {
                                    p.user_inverse = affine_compose(
                                        p.user_inverse,
                                        inverse(delta, 1e-15).ok_or_else(|| invalid(0))?,
                                    )
                                    .ok_or_else(|| invalid(0))?;
                                    changed_strokes = true;
                                }
                            }
                        }
                        shapes[si] = updated;
                        changed_shapes = true;
                    }
                }
            }
            _ => apply_value(
                view,
                track,
                *item,
                &mut edges,
                &mut colors,
                &mut shapes,
                &mut fills,
                &mut strokes,
                &world,
                control,
                &mut work,
                &mut changed_edges,
                &mut changed_colors,
                &mut changed_shapes,
                &mut changed_fills,
                &mut changed_strokes,
            )?,
        }
    }
    if changed_edges {
        builder.replace_edges(edges, control)?;
    }
    if changed_colors {
        builder.replace_colors(colors, control)?;
    }
    if changed_shapes {
        for edge in 0..view.edges.len() {
            if let Some(id) = view.edge_shape(edge) {
                let idx = id.index();
                if let Some(value) = shapes.get(idx) {
                    if view.shapes.get(idx) != Some(value) {
                        builder.set_shape(id, *value, control)?;
                    }
                }
            }
        }
    }
    if changed_fills {
        for (i, p) in fills.into_iter().enumerate() {
            if view.fill_paints.get(i) != Some(&p) {
                builder.set_fill_paint(i, p, control)?;
            }
        }
    }
    if changed_strokes {
        for (i, p) in strokes.into_iter().enumerate() {
            if view.stroke_paints.get(i) != Some(&p) {
                builder.set_stroke_paint(i, p, control)?;
            }
        }
    }
    for (i, t) in transforms.into_iter().enumerate() {
        if view.transforms.get(i) != Some(&t) {
            let id = find_transform_id(view, i).ok_or_else(|| diag(ErrorKind::Unsupported, 0))?;
            builder.set_transform(id, t, control)?;
        }
    }
    builder.finish(control)
}

fn find_transform_id(view: SceneView<'_>, index: usize) -> Option<TransformId> {
    for s in view.shapes {
        let mut n = s.source.transform_node;
        let mut depth = 0;
        while let Some(id) = n {
            if id.index() == index {
                return Some(id);
            }
            if depth >= CHAIN_MAX {
                return None;
            }
            n = view.transform(id)?.parent;
            depth += 1;
        }
    }
    None
}

fn apply_value(
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
) -> Result<()> {
    let mut descendant = false;
    let mut direct = false;
    for (i, shape) in shapes.iter_mut().enumerate() {
        charge(control, 1)?;
        *work = work.checked_add(1).ok_or_else(arithmetic)?;
        if *work > WORK_MAX {
            return Err(limit(LimitKind::Work));
        }
        let mut node = shape.source.transform_node;
        let mut depth = 0;
        let mut in_target = shape.source.element.start >= t.target.offset
            && shape.source.element.start < t.target.offset.saturating_add(t.target.length);
        while let Some(id) = node {
            if depth >= CHAIN_MAX {
                return Err(limit(LimitKind::Animation));
            }
            let n = view
                .transform(id)
                .ok_or_else(|| invalid(t.element.offset))?;
            charge(control, 1)?;
            *work += 1;
            if *work > WORK_MAX {
                return Err(limit(LimitKind::Work));
            }
            if n.source_offset >= t.target.offset
                && n.source_offset < t.target.offset.saturating_add(t.target.length)
            {
                in_target = true;
            }
            if n.source_offset == t.target.offset {
                direct = true;
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
fn has_effect(
    view: SceneView<'_>,
    shape: usize,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<bool> {
    for g in view.groups {
        charge(control, 1)?;
        *work = work.checked_add(1).ok_or_else(arithmetic)?;
        if *work > WORK_MAX {
            return Err(limit(LimitKind::Work));
        }
        if shape >= g.shapes.first
            && shape < g.shapes.end().ok_or_else(arithmetic)?
            && (g.filter.is_some() || g.mask.is_some())
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Refresh animated clip owners from a frame's current transform state.
pub fn update_animation_clips(
    _frame: &mut EvaluatedFrame<'_>,
    _shape: ShapeId,
    _update_bounds: bool,
    _control: &mut dyn WorkControl,
) -> Result<()> {
    Err(diag(ErrorKind::Unsupported, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vexel_svg_animation::{Kind, Track, TransformType, Value};

    fn track(transform_type: TransformType) -> Track {
        Track {
            kind: Kind::Transform,
            transform_type,
            ..Track::default()
        }
    }

    #[test]
    fn transform_matrix_keeps_svg_order_and_rotation_center() {
        let mut value = Value::default();
        value.number = [12.0, 3.0, 4.0, 3.0];
        let m = transform_value(&track(TransformType::Rotate), value).unwrap();
        assert!((m.a - 12f64.to_radians().cos()).abs() < 1e-12);
        assert!((m.b - 12f64.to_radians().sin()).abs() < 1e-12);
        assert!((m.e - (3.0 - m.a * 3.0 - m.c * 4.0)).abs() < 1e-12);
        assert!((m.f - (4.0 - m.b * 3.0 - m.d * 4.0)).abs() < 1e-12);
    }

    #[test]
    fn inverse_uses_animation_singularity_threshold() {
        let singular = Affine {
            a: 1e-8,
            b: 0.0,
            c: 0.0,
            d: 1e-8,
            e: 0.0,
            f: 0.0,
        };
        assert!(inverse(singular, 1e-15).is_none());
        assert!(inverse(singular, 0.0).is_some());
        let translated = Affine {
            e: 8.0,
            f: -3.0,
            ..affine_identity()
        };
        let back = inverse(translated, 1e-15).unwrap();
        assert_eq!(back.e, -8.0);
        assert_eq!(back.f, 3.0);
    }

    #[test]
    fn invalid_transform_input_does_not_produce_a_matrix() {
        let mut value = Value::default();
        value.number = [f64::INFINITY, 0.0, 0.0, 1.0];
        assert!(transform_value(&track(TransformType::Translate), value).is_none());
        value.number = [1_000_001.0, 0.0, 0.0, 1.0];
        assert!(transform_value(&track(TransformType::Translate), value).is_none());
    }
}
