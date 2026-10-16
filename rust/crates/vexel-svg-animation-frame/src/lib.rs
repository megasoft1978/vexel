//! Copy-on-change SVG animation frame evaluation over an immutable scene.
#![forbid(unsafe_code)]
mod apply;
mod clips;
use apply::apply_value;
pub use clips::update_animation_clips;

use vexel_geometry::{affine_compose, affine_identity, affine_map_point};
use vexel_scene::{
    Affine, Diagnostic, Edge, ErrorKind, EvaluatedFrame, FrameBuilder, LimitKind, Paint, PaintKind,
    Result, SceneView, Shape, TransformFrame, WorkControl, WorkKind,
};
use vexel_svg_animation::{
    Attribute, EvaluationError, Kind, OverlayItem, Track, TransformType, Value, ValueKind,
    evaluate_view_controlled,
};

const CHAIN_MAX: usize = 128;
const WORK_MAX: usize = 50_000_000;

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
fn count_work(work: &mut usize, amount: usize) -> Result<()> {
    if *work >= WORK_MAX || amount > WORK_MAX - *work {
        *work = WORK_MAX;
        return Err(limit(LimitKind::Work));
    }
    *work += amount;
    Ok(())
}
fn stopped(stop: vexel_scene::Stop) -> Diagnostic {
    match stop {
        vexel_scene::Stop::Canceled => diag(ErrorKind::Canceled, 0),
        vexel_scene::Stop::Limit => limit(LimitKind::Work),
        vexel_scene::Stop::Overflow => arithmetic(),
    }
}
fn changes_clips(track: &Track, item: OverlayItem) -> bool {
    matches!(track.kind, Kind::Transform | Kind::Motion)
        || (matches!(track.kind, Kind::Animate | Kind::Set)
            && item.value.kind == ValueKind::Number
            && matches!(
                track.attribute,
                Attribute::X
                    | Attribute::Y
                    | Attribute::Cx
                    | Attribute::Cy
                    | Attribute::Width
                    | Attribute::Height
                    | Attribute::Rx
                    | Attribute::Ry
                    | Attribute::R
            ))
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
        count_work(work, 1)?;
        chain[count] = Some(id);
        count += 1;
        node = transforms[id.index()].parent;
    }
    let mut out = affine_identity();
    while count > 0 {
        charge(control, 1)?;
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
    count_work(work, edges.len())?;
    let p = affine_map_point(inverse, edges[0].a).ok_or_else(|| invalid(0))?;
    let mut b = [p.x, p.y, p.x, p.y];
    for edge in edges {
        charge(control, 1)?;
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
    count_work(work, range.count)?;
    for edge in &mut edges[range.first..end] {
        charge(control, 1)?;
        edge.a = affine_map_point(matrix, edge.a).ok_or_else(|| invalid(0))?;
        edge.b = affine_map_point(matrix, edge.b).ok_or_else(|| invalid(0))?;
    }
    Ok(())
}

/// Evaluate a frame using the caller's configured budget for all published
/// frame-owned tables, including old-plus-new clip replacement overlap.
pub fn evaluate_frame<'a>(
    view: SceneView<'a>,
    time_seconds: f64,
    byte_budget: usize,
    control: &mut dyn WorkControl,
) -> Result<EvaluatedFrame<'a>> {
    if byte_budget == usize::MAX {
        return Err(limit(LimitKind::ReservedBytes));
    }
    let overlay =
        evaluate_view_controlled(view.animation(), time_seconds, control).map_err(|e| match e {
            EvaluationError::InvalidTime => invalid(0),
            EvaluationError::InvalidTrack => diag(ErrorKind::Unsupported, 0),
            EvaluationError::OutOfMemory => oom(),
            EvaluationError::Canceled => diag(ErrorKind::Canceled, 0),
            EvaluationError::Limit => limit(LimitKind::Work),
            EvaluationError::Overflow => arithmetic(),
        })?;
    let mut builder = FrameBuilder::new(view, byte_budget)?;
    let mut any_active = false;
    for item in &overlay.items {
        charge(control, 1)?;
        any_active |= item.active;
    }
    if !any_active {
        return builder.finish(control);
    }
    let mut work = 0usize;
    let mut transforms = copy_table(view.transforms, control)?;
    let mut transform_seen = Vec::new();
    transform_seen
        .try_reserve_exact(view.transforms.len())
        .map_err(|_| oom())?;
    for _ in view.transforms {
        charge(control, 1)?;
        transform_seen.push(false);
    }
    let mut world = Vec::new();
    world
        .try_reserve_exact(view.shapes.len())
        .map_err(|_| oom())?;
    for shape in view.shapes {
        charge(control, 1)?;
        world.push(shape.source.transform);
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
    let mut has_clip_updates = false;
    if !view.clips.is_empty() {
        for item in &overlay.items {
            charge(control, 1)?;
            has_clip_updates |= item.active
                && view
                    .animation()
                    .tracks
                    .get(item.track_index)
                    .is_some_and(|track| changes_clips(track, *item));
        }
    }
    let mut clip_shape_flags = Vec::new();
    let mut clips = if !has_clip_updates {
        None
    } else {
        Some(clips::clone_clips_for_overlay(view, control)?)
    };
    if has_clip_updates {
        clip_shape_flags
            .try_reserve_exact(view.shapes.len())
            .map_err(|_| oom())?;
        for _ in view.shapes {
            charge(control, 1)?;
            clip_shape_flags.push(false);
        }
    }
    for item in &overlay.items {
        control.checkpoint().map_err(stopped)?;
        if !item.active {
            continue;
        }
        charge(control, 1)?;
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
                    count_work(&mut work, 1)?;
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
                        count_work(&mut work, 1)?;
                        let mut n = shape.source.transform_node;
                        let mut has = false;
                        let mut depth = 0;
                        while let Some(id) = n {
                            if depth >= CHAIN_MAX || id.index() >= transforms.len() {
                                return Err(invalid(track.element.offset));
                            }
                            charge(control, 1)?;
                            count_work(&mut work, 1)?;
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
                        map_shape_edges(&shape, delta, &mut edges, control, &mut work)?;
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
                        if !clip_shape_flags.is_empty() {
                            clip_shape_flags[si] = true;
                        }
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
                &mut clip_shape_flags,
            )?,
        }
        if let Some(clip_table) = clips.as_mut() {
            for si in 0..clip_shape_flags.len() {
                charge(control, 1)?;
                if clip_shape_flags[si] {
                    clips::update_clip_table_for_shape(
                        &transforms,
                        &shapes,
                        &edges,
                        clip_table,
                        si,
                        true,
                        control,
                        &mut work,
                    )?;
                    clip_shape_flags[si] = false;
                }
            }
        }
    }
    if changed_edges {
        builder.replace_edges(edges, control)?;
    }
    if changed_colors {
        builder.replace_colors(colors, control)?;
    }
    if changed_shapes {
        for (idx, value) in shapes.iter().enumerate() {
            charge(control, 1)?;
            let id = view.shape_id(idx).ok_or_else(|| invalid(0))?;
            if view.shapes.get(idx) != Some(value) {
                builder.set_shape(id, *value, control)?;
            }
        }
    }
    if changed_fills {
        for (i, p) in fills.into_iter().enumerate() {
            charge(control, 1)?;
            if view.fill_paints.get(i) != Some(&p) {
                builder.set_fill_paint(i, p, control)?;
            }
        }
    }
    if changed_strokes {
        for (i, p) in strokes.into_iter().enumerate() {
            charge(control, 1)?;
            if view.stroke_paints.get(i) != Some(&p) {
                builder.set_stroke_paint(i, p, control)?;
            }
        }
    }
    for (i, t) in transforms.into_iter().enumerate() {
        charge(control, 1)?;
        if view.transforms.get(i) != Some(&t) {
            let id = view.transform_id(i).ok_or_else(|| invalid(0))?;
            builder.set_transform(id, t, control)?;
        }
    }
    let mut frame = builder.finish(control)?;
    if let Some(clip_table) = clips {
        frame.replace_clips(clip_table, control)?;
    }
    Ok(frame)
}

fn has_effect(
    view: SceneView<'_>,
    shape: usize,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<bool> {
    for g in view.groups {
        charge(control, 1)?;
        count_work(work, 1)?;
        if shape >= g.shapes.first
            && shape < g.shapes.end().ok_or_else(arithmetic)?
            && (g.filter.is_some() || g.mask.is_some())
        {
            return Ok(true);
        }
    }
    Ok(false)
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
    fn transform_matrix_preserves_translate_scale_and_skew_components() {
        let mut value = Value::default();
        value.number = [3.0, -4.0, 0.0, 2.0];
        let translated = transform_value(&track(TransformType::Translate), value).unwrap();
        assert_eq!((translated.e, translated.f), (3.0, -4.0));

        value.number = [2.0, 3.0, 0.0, 2.0];
        let scaled = transform_value(&track(TransformType::Scale), value).unwrap();
        assert_eq!((scaled.a, scaled.d), (2.0, 3.0));

        value.number = [45.0, 0.0, 0.0, 1.0];
        let skew_x = transform_value(&track(TransformType::SkewX), value).unwrap();
        let skew_y = transform_value(&track(TransformType::SkewY), value).unwrap();
        assert!((skew_x.c - 1.0).abs() < 1e-12);
        assert!((skew_y.b - 1.0).abs() < 1e-12);
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

#[cfg(test)]
mod loop_tests;
