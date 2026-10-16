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
    let frame = FrameBuilder::new(view, 128 * 1024 * 1024)
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

#[test]
fn suffix_polling_preserves_legacy_clip_local_work_and_caps() {
    let (graph, _, child, mut shapes, edges) = scene();
    let view = graph.root();
    let mut control = NoStop;
    let mut baseline_work = 0;
    let baseline = bounds_for_node(
        &shapes,
        &edges,
        view.transforms,
        child,
        affine_identity(),
        &mut control,
        &mut baseline_work,
    )
    .unwrap();
    let empty = Shape::new(SourceMap {
        element: Span { start: 0, end: 0 },
        transform: affine_identity(),
        transform_node: None,
    });
    shapes.extend(std::iter::repeat_n(empty, 1024));
    let mut suffix_work = 0;
    assert_eq!(
        bounds_for_node(
            &shapes,
            &edges,
            view.transforms,
            child,
            affine_identity(),
            &mut control,
            &mut suffix_work
        )
        .unwrap(),
        baseline
    );
    assert_eq!(suffix_work, baseline_work);
    let mut cancel = CancelAfterKind::new(WorkKind::ReferenceProbe, 16);
    assert_eq!(
        bounds_for_node(
            &shapes,
            &edges,
            view.transforms,
            child,
            affine_identity(),
            &mut cancel,
            &mut 0
        )
        .unwrap_err()
        .kind,
        ErrorKind::Canceled
    );
    assert_eq!(cancel.seen, 16);
    let mut local = 0;
    let mut clips = clone_clip_table(view, &mut control, &mut local).unwrap();
    assert_eq!(local, 0);
    update_clip_owner(
        &shapes[..1],
        &mut clips,
        child,
        affine_identity(),
        affine_identity(),
        None,
        &mut control,
        &mut local,
    )
    .unwrap();
    assert_eq!(local, CLIP_OWNER_DEPTH_MAX);
    let mut cap = TRANSFORM_WORK_MAX - CLIP_OWNER_DEPTH_MAX;
    update_clip_owner(
        &shapes[..1],
        &mut clips,
        child,
        affine_identity(),
        affine_identity(),
        None,
        &mut control,
        &mut cap,
    )
    .unwrap();
    assert_eq!(cap, TRANSFORM_WORK_MAX);
    assert_eq!(
        add_work(&mut cap, 0).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Work)
    );
    assert_eq!(cap, TRANSFORM_WORK_MAX);
}
