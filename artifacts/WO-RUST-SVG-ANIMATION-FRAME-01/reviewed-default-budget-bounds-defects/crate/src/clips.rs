//! Owned clip updates for animated transform owners.
use vexel_geometry::{affine_compose, affine_identity, affine_map_point};
use vexel_scene::{
    Clip, ClipGeometry, Diagnostic, Edge, ErrorKind, EvaluatedFrame, LimitKind, Result, SceneView,
    Shape, ShapeId, TransformFrame, TransformId, WorkControl, WorkKind,
};

const CLIP_COPY_MAX_BYTES: usize = super::DEFAULT_FRAME_BYTE_BUDGET;
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

fn charge(
    control: &mut dyn WorkControl,
    kind: WorkKind,
    count: usize,
    work: &mut usize,
) -> Result<()> {
    let next = work.checked_add(count).ok_or_else(arithmetic)?;
    if next > TRANSFORM_WORK_MAX {
        *work = TRANSFORM_WORK_MAX;
        return Err(limit(LimitKind::Work));
    }
    let units = u64::try_from(count).map_err(|_| arithmetic())?;
    control.charge(kind, units).map_err(|stop| Diagnostic {
        kind: match stop {
            vexel_scene::Stop::Canceled => ErrorKind::Canceled,
            vexel_scene::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            vexel_scene::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    })?;
    *work = next;
    Ok(())
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
        charge(control, WorkKind::ClipChain, 1, work)?;
        let mut current = shape.clip;
        let mut depth = 0;
        while let Some(id) = current {
            if depth >= CLIP_OWNER_DEPTH_MAX || id.index() >= clips.len() {
                return Err(invalid(shape.source.element.start));
            }
            charge(control, WorkKind::ClipChain, 1, work)?;
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
    work: &mut usize,
) -> Result<Vec<Clip>> {
    let mut estimate = bytes::<Clip>(view.clips.len())?;
    for clip in view.clips {
        charge(control, WorkKind::ClipChild, 1, work)?;
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
        charge(control, WorkKind::ClipChild, 1, work)?;
        let mut edges = Vec::new();
        edges
            .try_reserve_exact(source.geometry.edges.len())
            .map_err(|_| oom())?;
        for edge in &source.geometry.edges {
            charge(control, WorkKind::ClipEdge, 1, work)?;
            edges.push(*edge);
        }
        let mut shapes = Vec::new();
        shapes
            .try_reserve_exact(source.geometry.shapes.len())
            .map_err(|_| oom())?;
        for child in &source.geometry.shapes {
            charge(control, WorkKind::ClipChild, 1, work)?;
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
    let actual = actual_clip_capacity(&output, control, work)?;
    if actual > CLIP_COPY_MAX_BYTES {
        return Err(limit(LimitKind::Animation));
    }
    Ok(output)
}

fn actual_clip_capacity(
    clips: &Vec<Clip>,
    control: &mut dyn WorkControl,
    work: &mut usize,
) -> Result<usize> {
    let mut total = clips
        .capacity()
        .checked_mul(std::mem::size_of::<Clip>())
        .ok_or_else(arithmetic)?;
    for clip in clips {
        charge(control, WorkKind::ClipChild, 1, work)?;
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
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use vexel_geometry::{Affine, Edge, Point, affine_identity};
    use vexel_scene::{
        ClipChild, FrameBuilder, Limits, SceneBuilder, Shape, SourceMap, Span, TableRange, Viewport,
    };
    use vexel_svg_animation::parse_owned;

    struct NoStop;
    impl WorkControl for NoStop {
        fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), vexel_scene::Stop> {
            Ok(())
        }
        fn checkpoint(&mut self) -> std::result::Result<(), vexel_scene::Stop> {
            Ok(())
        }
    }
    struct CancelAfterKind {
        kind: WorkKind,
        after: u64,
        seen: u64,
        canceled: AtomicBool,
    }
    impl CancelAfterKind {
        fn new(kind: WorkKind, after: u64) -> Self {
            Self {
                kind,
                after,
                seen: 0,
                canceled: AtomicBool::new(false),
            }
        }
    }
    impl WorkControl for CancelAfterKind {
        fn charge(&mut self, kind: WorkKind, _: u64) -> std::result::Result<(), vexel_scene::Stop> {
            if self.canceled.load(Ordering::Acquire) {
                return Err(vexel_scene::Stop::Canceled);
            }
            if kind == self.kind {
                self.seen += 1;
                if self.seen == self.after {
                    self.canceled.store(true, Ordering::Release);
                }
            }
            Ok(())
        }
        fn checkpoint(&mut self) -> std::result::Result<(), vexel_scene::Stop> {
            if self.canceled.load(Ordering::Acquire) {
                Err(vexel_scene::Stop::Canceled)
            } else {
                Ok(())
            }
        }
    }
    fn tx(x: f64) -> vexel_scene::Affine {
        Affine {
            e: x,
            ..affine_identity()
        }
    }
    fn make_clip(
        owner: Option<TransformId>,
        before: bool,
        parent: Option<vexel_scene::ClipId>,
    ) -> Clip {
        Clip {
            geometry: ClipGeometry {
                edges: vec![Edge {
                    a: Point { x: 0., y: 0. },
                    b: Point { x: 2., y: 2. },
                }],
                shapes: vec![ClipChild {
                    edges: TableRange { first: 0, count: 1 },
                    evenodd: false,
                    filled: true,
                }],
            },
            parent,
            owner_node: owner,
            owner_before_suffix: before,
            object_bounding_box: true,
            bounds: [0., 0., 2., 2.],
            user_inverse: affine_identity(),
        }
    }
    fn scene() -> (
        vexel_scene::SceneGraph,
        TransformId,
        TransformId,
        Vec<Shape>,
        Vec<Edge>,
    ) {
        let source = Arc::new(b"<svg/>".to_vec());
        let animation = parse_owned(&source, &[]).unwrap();
        let viewport = Viewport {
            width: 8,
            height: 8,
            view_x: 0.,
            view_y: 0.,
            view_width: 8.,
            view_height: 8.,
            aspect: Default::default(),
        };
        let mut builder =
            SceneBuilder::new(source.clone(), animation, 1, viewport, Limits::default()).unwrap();
        let scene = builder.root_id();
        let mut control = NoStop;
        let parent = builder
            .append_transform(
                scene,
                TransformFrame {
                    source_offset: 0,
                    parent: None,
                    attribute: tx(5.),
                    suffix: tx(7.),
                    motion: tx(3.),
                },
                &mut control,
            )
            .unwrap();
        let child = builder
            .append_transform(
                scene,
                TransformFrame {
                    source_offset: 1,
                    parent: Some(parent),
                    attribute: affine_identity(),
                    suffix: affine_identity(),
                    motion: affine_identity(),
                },
                &mut control,
            )
            .unwrap();
        let parent_clip = builder
            .append_clip(scene, make_clip(Some(parent), true, None), &mut control)
            .unwrap();
        let root_clip = builder
            .append_clip(
                scene,
                make_clip(Some(child), false, Some(parent_clip)),
                &mut control,
            )
            .unwrap();
        let graph = builder.freeze().unwrap();
        let mut shape = Shape::new(SourceMap {
            element: Span { start: 0, end: 6 },
            transform: tx(15.),
            transform_node: Some(child),
        });
        shape.edges = TableRange { first: 0, count: 1 };
        shape.clip = Some(root_clip);
        // Geometry is in the evaluated world frame for the transform chain above.
        let edges = vec![Edge {
            a: Point { x: 15., y: 0. },
            b: Point { x: 17., y: 2. },
        }];
        (graph, parent, child, vec![shape], edges)
    }

    #[test]
    fn clip_owner_before_suffix_and_bounds_follow_complete_ancestor_chain() {
        let (graph, _parent, child, shapes, edges) = scene();
        let view = graph.root();
        let mut control = NoStop;
        let mut work = 0;
        let mut clips = clone_clip_table(view, &mut control, &mut work).unwrap();
        update_clip_shape(
            &shapes,
            &edges,
            view.transforms,
            &mut clips,
            &shapes[0],
            true,
            &mut control,
            &mut work,
        )
        .unwrap();
        assert_eq!(clips[0].user_inverse.e, -8.0);
        assert_eq!(clips[1].user_inverse.e, -15.0);
        assert_eq!(clips[0].bounds, [0.0, 0.0, 2.0, 2.0]);
        assert_eq!(clips[1].bounds, [0.0, 0.0, 2.0, 2.0]);
        assert_eq!(view.transform_id(child.index()), Some(child));
    }

    #[test]
    fn update_without_bounds_preserves_object_box_bounds() {
        let (graph, _, _, shapes, edges) = scene();
        let view = graph.root();
        let mut control = NoStop;
        let mut work = 0;
        let original = [1.0, 2.0, 3.0, 4.0];
        let mut clips = clone_clip_table(view, &mut control, &mut work).unwrap();
        clips[0].bounds = original;
        update_clip_shape(
            &shapes,
            &edges,
            view.transforms,
            &mut clips,
            &shapes[0],
            false,
            &mut control,
            &mut work,
        )
        .unwrap();
        assert_eq!(clips[0].bounds, original);
        assert_eq!(clips[0].user_inverse.e, -8.0);
    }

    #[test]
    fn clip_estimation_capacity_and_owner_walk_poll_cancellation_in_loop() {
        let (graph, parent, child, shapes, _) = scene();
        let view = graph.root();
        let mut no_stop = NoStop;
        let frame = FrameBuilder::new(view, super::super::DEFAULT_FRAME_BYTE_BUDGET)
            .unwrap()
            .finish(&mut no_stop)
            .unwrap();
        let original = frame
            .view()
            .clips
            .iter()
            .map(|clip| (clip.bounds, clip.user_inverse))
            .collect::<Vec<_>>();

        let mut work = 0;
        let mut estimator_control = CancelAfterKind::new(WorkKind::ClipChild, 1);
        assert_eq!(
            clone_clip_table(frame.view(), &mut estimator_control, &mut work)
                .unwrap_err()
                .kind,
            ErrorKind::Canceled
        );
        let mut no_stop = NoStop;
        let mut cloned = clone_clip_table(frame.view(), &mut no_stop, &mut 0).unwrap();
        let mut capacity_control = CancelAfterKind::new(WorkKind::ClipChild, 1);
        assert_eq!(
            actual_clip_capacity(&cloned, &mut capacity_control, &mut 0)
                .unwrap_err()
                .kind,
            ErrorKind::Canceled
        );
        let mut no_stop = NoStop;
        assert!(actual_clip_capacity(&cloned, &mut no_stop, &mut 0).unwrap() > 0);

        let mut owner_control = CancelAfterKind::new(WorkKind::ClipChain, 2);
        assert_eq!(
            update_clip_owner(
                &shapes,
                &mut cloned,
                child,
                affine_identity(),
                affine_identity(),
                Some([0.0, 0.0, 2.0, 2.0]),
                &mut owner_control,
                &mut 0,
            )
            .unwrap_err()
            .kind,
            ErrorKind::Canceled
        );
        let mut no_stop = NoStop;
        let mut success = clone_clip_table(frame.view(), &mut no_stop, &mut 0).unwrap();
        let updated_inverse = Affine {
            e: -3.0,
            ..affine_identity()
        };
        update_clip_owner(
            &shapes,
            &mut success,
            child,
            updated_inverse,
            updated_inverse,
            Some([0.0, 0.0, 2.0, 2.0]),
            &mut no_stop,
            &mut 0,
        )
        .unwrap();
        assert_eq!(success[1].user_inverse, updated_inverse);
        assert_eq!(parent.index(), 0);
        assert_eq!(
            frame
                .view()
                .clips
                .iter()
                .map(|clip| (clip.bounds, clip.user_inverse))
                .collect::<Vec<_>>(),
            original
        );
        assert_eq!(
            graph.root().clip(view.clip_id(1).unwrap()).unwrap().bounds,
            original[1].0
        );
    }
}
