//! Owned clip updates for animated transform owners.
use vexel_geometry::{affine_compose, affine_identity, affine_map_point};
use vexel_scene::{
    Clip, ClipGeometry, Diagnostic, Edge, ErrorKind, EvaluatedFrame, LimitKind, Result, SceneView,
    Shape, ShapeId, TransformFrame, TransformId, WorkControl, WorkKind,
};

const CLIP_COPY_MAX_BYTES: usize = 64 * 1024 * 1024;
const CLIP_CHAIN_MAX: usize = 128;
const CLIP_OWNER_DEPTH_MAX: usize = 64;
const TRANSFORM_WORK_MAX: usize = 50_000_000;

fn invalid(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset,
    }
}
fn limit(kind: LimitKind) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(kind),
        offset: 0,
    }
}
fn arithmetic() -> Diagnostic {
    limit(LimitKind::Arithmetic)
}
fn oom() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: 0,
    }
}
fn add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b).ok_or_else(arithmetic)
}
fn bytes<T>(count: usize) -> Result<usize> {
    count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or_else(arithmetic)
}

fn poll(control: &mut dyn WorkControl, kind: WorkKind, count: usize) -> Result<()> {
    let units = u64::try_from(count).map_err(|_| arithmetic())?;
    control.charge(kind, units).map_err(|stop| Diagnostic {
        kind: match stop {
            vexel_scene::Stop::Canceled => ErrorKind::Canceled,
            vexel_scene::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            vexel_scene::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    })
}
fn add_work(work: &mut usize, count: usize) -> Result<()> {
    if *work >= TRANSFORM_WORK_MAX || count > TRANSFORM_WORK_MAX - *work {
        *work = TRANSFORM_WORK_MAX;
        return Err(limit(LimitKind::Work));
    }
    *work += count;
    Ok(())
}
fn charge(
    control: &mut dyn WorkControl,
    kind: WorkKind,
    count: usize,
    work: &mut usize,
) -> Result<()> {
    add_work(work, count)?;
    poll(control, kind, count)
}
fn clip_inverse(m: vexel_scene::Affine) -> Option<vexel_scene::Affine> {
    let det = m.a * m.d - m.b * m.c;
    if !det.is_finite() || det == 0.0 {
        return None;
    }
    let k = 1.0 / det;
    let out = vexel_scene::Affine {
        a: m.d * k,
        b: -m.b * k,
        c: -m.c * k,
        d: m.a * k,
        e: (m.c * m.f - m.d * m.e) * k,
        f: (m.b * m.e - m.a * m.f) * k,
    };
    vexel_geometry::affine_valid(out).then_some(out)
}

fn node_product(
    transforms: &[TransformFrame],
    target: TransformId,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<(vexel_scene::Affine, vexel_scene::Affine)> {
    let mut chain = [None; CLIP_CHAIN_MAX];
    let mut count = 0;
    let mut node = Some(target);
    while let Some(id) = node {
        if count == CLIP_CHAIN_MAX || id.index() >= transforms.len() {
            return Err(invalid(0));
        }
        charge(control, WorkKind::ClipChain, 1, work)?;
        chain[count] = Some(id);
        count += 1;
        node = transforms[id.index()].parent;
    }
    let mut full = affine_identity();
    let mut before_suffix = affine_identity();
    while count != 0 {
        poll(control, WorkKind::ClipChain, 1)?;
        count -= 1;
        let id = chain[count].ok_or_else(|| invalid(0))?;
        let transform = transforms[id.index()];
        let local = affine_compose(transform.motion, transform.attribute)
            .ok_or_else(|| invalid(transform.source_offset))?;
        if id == target {
            before_suffix =
                affine_compose(full, local).ok_or_else(|| invalid(transform.source_offset))?;
        }
        let with_suffix = affine_compose(local, transform.suffix)
            .ok_or_else(|| invalid(transform.source_offset))?;
        full = affine_compose(full, with_suffix).ok_or_else(|| invalid(transform.source_offset))?;
    }
    Ok((full, before_suffix))
}

fn shape_has_node(
    shape: &Shape,
    transforms: &[TransformFrame],
    target: TransformId,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<bool> {
    let mut node = shape.source.transform_node;
    let mut depth = 0;
    while let Some(id) = node {
        if depth >= CLIP_CHAIN_MAX || id.index() >= transforms.len() {
            return Err(invalid(shape.source.element.start));
        }
        charge(control, WorkKind::ClipChain, 1, work)?;
        if id == target {
            return Ok(true);
        }
        node = transforms[id.index()].parent;
        depth += 1;
    }
    Ok(false)
}

fn bounds_for_node(
    shapes: &[Shape],
    edges: &[Edge],
    transforms: &[TransformFrame],
    target: TransformId,
    inverse: vexel_scene::Affine,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<[f64; 4]> {
    let mut bounds: Option<[f64; 4]> = None;
    for shape in shapes {
        poll(control, WorkKind::ReferenceProbe, 1)?;
        if !shape_has_node(shape, transforms, target, control, work)? {
            continue;
        }
        let segment = shape
            .edges
            .slice(edges)
            .ok_or_else(|| invalid(shape.source.element.start))?;
        for edge in segment {
            charge(control, WorkKind::ClipEdge, 1, work)?;
            for point in [edge.a, edge.b] {
                let local = affine_map_point(inverse, point)
                    .ok_or_else(|| invalid(shape.source.element.start))?;
                match &mut bounds {
                    Some(b) => {
                        b[0] = b[0].min(local.x);
                        b[1] = b[1].min(local.y);
                        b[2] = b[2].max(local.x);
                        b[3] = b[3].max(local.y);
                    }
                    None => bounds = Some([local.x, local.y, local.x, local.y]),
                }
            }
        }
    }
    bounds.ok_or_else(|| invalid(0))
}

fn update_clip_owner(
    shapes: &[Shape],
    clips: &mut [Clip],
    node: TransformId,
    full_inverse: vexel_scene::Affine,
    before_suffix_inverse: vexel_scene::Affine,
    bounds: Option<[f64; 4]>,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<()> {
    for shape in shapes {
        add_work(work, CLIP_OWNER_DEPTH_MAX)?;
        poll(control, WorkKind::ClipChain, 1)?;
        let mut current = shape.clip;
        let mut depth = 0;
        while let Some(id) = current {
            if depth >= CLIP_OWNER_DEPTH_MAX || id.index() >= clips.len() {
                return Err(invalid(shape.source.element.start));
            }
            poll(control, WorkKind::ClipChain, 1)?;
            let clip = &mut clips[id.index()];
            if clip.owner_node == Some(node) {
                clip.user_inverse = if clip.owner_before_suffix {
                    before_suffix_inverse
                } else {
                    full_inverse
                };
                if let Some(b) = bounds {
                    if clip.object_bounding_box
                        && b.into_iter().all(f64::is_finite)
                        && b[2] >= b[0]
                        && b[3] >= b[1]
                    {
                        clip.bounds = b;
                    }
                }
            }
            current = clip.parent;
            depth += 1;
        }
    }
    Ok(())
}

fn update_clip_shape(
    shapes: &[Shape],
    edges: &[Edge],
    transforms: &[TransformFrame],
    clips: &mut [Clip],
    shape: &Shape,
    update_bounds: bool,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<()> {
    let mut node = shape.source.transform_node;
    let mut depth = 0;
    while let Some(id) = node {
        poll(control, WorkKind::ClipChain, 1)?;
        if depth >= CLIP_CHAIN_MAX || id.index() >= transforms.len() {
            return Err(invalid(shape.source.element.start));
        }
        let (full, before_suffix) = node_product(transforms, id, control, work)?;
        let full_inverse = clip_inverse(full).ok_or_else(|| invalid(shape.source.element.start))?;
        let attribute_inverse =
            clip_inverse(before_suffix).ok_or_else(|| invalid(shape.source.element.start))?;
        let bounds = if update_bounds {
            Some(bounds_for_node(
                shapes,
                edges,
                transforms,
                id,
                full_inverse,
                control,
                work,
            )?)
        } else {
            None
        };
        update_clip_owner(
            shapes,
            clips,
            id,
            full_inverse,
            attribute_inverse,
            bounds,
            control,
            work,
        )?;
        node = transforms[id.index()].parent;
        depth += 1;
    }
    Ok(())
}

fn clone_clip_table(
    view: SceneView<'_>,
    control: &mut dyn WorkControl,
    _work: &mut usize,
) -> Result<Vec<Clip>> {
    let mut estimate = bytes::<Clip>(view.clips.len())?;
    for clip in view.clips {
        poll(control, WorkKind::ClipChild, 1)?;
        estimate = add(estimate, bytes::<Edge>(clip.geometry.edges.len())?)?;
        estimate = add(
            estimate,
            bytes::<vexel_scene::ClipChild>(clip.geometry.shapes.len())?,
        )?;
    }
    if estimate > CLIP_COPY_MAX_BYTES {
        return Err(limit(LimitKind::Animation));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(view.clips.len())
        .map_err(|_| oom())?;
    for source in view.clips {
        poll(control, WorkKind::ClipChild, 1)?;
        let mut edges = Vec::new();
        edges
            .try_reserve_exact(source.geometry.edges.len())
            .map_err(|_| oom())?;
        for edge in &source.geometry.edges {
            poll(control, WorkKind::ClipEdge, 1)?;
            edges.push(*edge);
        }
        let mut shapes = Vec::new();
        shapes
            .try_reserve_exact(source.geometry.shapes.len())
            .map_err(|_| oom())?;
        for child in &source.geometry.shapes {
            poll(control, WorkKind::ClipChild, 1)?;
            shapes.push(*child);
        }
        output.push(Clip {
            geometry: ClipGeometry { edges, shapes },
            parent: source.parent,
            owner_node: source.owner_node,
            owner_before_suffix: source.owner_before_suffix,
            object_bounding_box: source.object_bounding_box,
            bounds: source.bounds,
            user_inverse: source.user_inverse,
        });
    }
    let actual = actual_clip_capacity(&output, control, _work)?;
    if actual > CLIP_COPY_MAX_BYTES {
        return Err(limit(LimitKind::Animation));
    }
    Ok(output)
}

fn actual_clip_capacity(
    clips: &Vec<Clip>,
    control: &mut dyn WorkControl,
    _work: &mut usize,
) -> Result<usize> {
    let mut total = clips
        .capacity()
        .checked_mul(std::mem::size_of::<Clip>())
        .ok_or_else(arithmetic)?;
    for clip in clips {
        poll(control, WorkKind::ClipChild, 1)?;
        let edges = clip
            .geometry
            .edges
            .capacity()
            .checked_mul(std::mem::size_of::<Edge>())
            .ok_or_else(arithmetic)?;
        let shapes = clip
            .geometry
            .shapes
            .capacity()
            .checked_mul(std::mem::size_of::<vexel_scene::ClipChild>())
            .ok_or_else(arithmetic)?;
        total = add(total, add(edges, shapes)?)?;
    }
    Ok(total)
}

pub(crate) fn clone_clips_for_overlay(
    view: SceneView<'_>,
    control: &mut dyn WorkControl,
) -> Result<Vec<Clip>> {
    let mut work = 0;
    clone_clip_table(view, control, &mut work)
}

pub(crate) fn update_clip_table_for_shape(
    transforms: &[TransformFrame],
    shapes: &[Shape],
    edges: &[Edge],
    clips: &mut [Clip],
    index: usize,
    update_bounds: bool,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<()> {
    let shape = shapes.get(index).ok_or_else(|| invalid(0))?;
    update_clip_shape(
        shapes,
        edges,
        transforms,
        clips,
        shape,
        update_bounds,
        control,
        work,
    )
}

/// Update clip owner inverses and object-bounding-box bounds for one shape's
/// complete instance-specific transform ancestry. Publication is atomic.
pub fn update_animation_clips(
    frame: &mut EvaluatedFrame<'_>,
    shape: ShapeId,
    update_bounds: bool,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let view = frame.view();
    if view.clips.is_empty() {
        return Err(invalid(0));
    }
    control.checkpoint().map_err(|s| match s {
        vexel_scene::Stop::Canceled => Diagnostic {
            kind: ErrorKind::Canceled,
            offset: 0,
        },
        vexel_scene::Stop::Limit => limit(LimitKind::Work),
        vexel_scene::Stop::Overflow => arithmetic(),
    })?;
    let mut work = 0;
    let mut clips = clone_clip_table(view, control, &mut work)?;
    let shapes = view.shapes;
    let edges = view.edges;
    let transforms = view.transforms;
    let selected = view.shape(shape).ok_or_else(|| invalid(0))?;
    update_clip_shape(
        shapes,
        edges,
        transforms,
        &mut clips,
        selected,
        update_bounds,
        control,
        &mut work,
    )?;
    frame.replace_clips(clips, control)
}

#[cfg(test)]
#[path = "clip_tests.rs"]
mod tests;
