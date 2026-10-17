use crate::builder::NoStop;
use crate::*;
fn builder() -> SceneBuilder {
    let source = std::sync::Arc::new(b"<svg/>".to_vec());
    let animation = vexel_svg_animation::parse_owned(&source, &[]).unwrap();
    SceneBuilder::new(
        source,
        animation,
        0,
        Viewport {
            width: 1,
            height: 1,
            view_x: 0.,
            view_y: 0.,
            view_width: 1.,
            view_height: 1.,
            aspect: Default::default(),
        },
        Limits::default(),
    )
    .unwrap()
}
#[test]
fn publication_rechecks_source_spans_and_parallel_tables() {
    let mut b = builder();
    b.scenes[0].edges.push(Edge {
        a: Point { x: 0., y: 0. },
        b: Point { x: 1., y: 1. },
    });
    assert!(b.freeze().is_err());
    let mut b = builder();
    b.scenes[0].path_numbers.push(vexel_svg_path::PathNumber {
        source: Span {
            start: 1,
            end: usize::MAX,
        },
        value: 1.,
    });
    assert!(b.freeze().is_err());
}
#[test]
fn publication_rechecks_parent_order_and_stale_serials() {
    let mut b = builder();
    let root = b.root_id();
    let a = vexel_geometry::affine_identity();
    let id = b
        .append_transform(
            root,
            TransformFrame {
                source_offset: 0,
                parent: None,
                attribute: a,
                suffix: a,
                motion: a,
            },
            &mut NoStop,
        )
        .unwrap();
    b.scenes[0].transforms[0].parent = Some(id);
    assert!(b.freeze().is_err());
    let mut b = builder();
    let root = b.root_id();
    b.append_transform(
        root,
        TransformFrame {
            source_offset: 0,
            parent: None,
            attribute: a,
            suffix: a,
            motion: a,
        },
        &mut NoStop,
    )
    .unwrap();
    b.scenes[0].transforms_serials.clear();
    assert!(b.freeze().is_err());
}
#[test]
fn publication_rechecks_ranges_and_actual_memory_statistics() {
    let mut b = builder();
    b.scenes[0].groups.push(OpacityGroup {
        shapes: TableRange {
            first: usize::MAX,
            count: 1,
        },
        parent: None,
        opacity: 1.,
        filter: None,
        mask: None,
        blend_mode: 0,
        isolation: false,
    });
    b.scenes[0].groups_serials.push(1);
    assert!(b.freeze().is_err());
    let mut b = builder();
    b.stats.live_bytes += 1;
    assert!(b.freeze().is_err());
}
#[test]
fn default_policy_preserves_reference_caps_and_clamps_larger_overrides() {
    let d = Limits::default();
    assert_eq!(d.source_bytes, 1 << 30);
    assert_eq!(d.shapes, MAX_SHAPES);
    assert_eq!(d.edges, 4194304);
    assert_eq!(d.commands, 4194304);
    assert_eq!(d.numbers, 29360128);
    assert_eq!(d.reserved_bytes, usize::MAX);
    let mut larger = d;
    larger.edges = usize::MAX;
    larger.scenes = usize::MAX;
    larger.reserved_bytes = 123;
    let actual = larger.effective();
    assert_eq!(actual.edges, d.edges);
    assert_eq!(actual.scenes, d.scenes);
    assert_eq!(actual.reserved_bytes, 123);
}
#[test]
fn limit03_shape_override_clamps_to_the_524288_default() {
    // effective() clamps to the default: a raised default is what admits the cap.
    let mut larger = Limits::default();
    larger.shapes = 524_289;
    assert_eq!(larger.effective().shapes, 524_288);
    larger.shapes = 524_288;
    assert_eq!(larger.effective().shapes, 524_288);
}
#[test]
fn source_capacity_is_included_exactly_once_in_admission_and_freeze() {
    let b = builder();
    let expected = b.animation.storage_bytes().unwrap()
        + b.scenes.capacity() * std::mem::size_of::<crate::model::SceneData>();
    assert_eq!(b.stats.live_bytes, expected);
    assert_eq!(b.stats.source_bytes, b.source.capacity());
    assert!(b.animation.storage_bytes().unwrap() >= b.source.capacity());
    let source = std::sync::Arc::new(Vec::with_capacity(8192));
    let animation = vexel_svg_animation::parse_owned(&source, &[]).unwrap();
    let mut limits = Limits::default();
    limits.reserved_bytes = source.capacity() - 1;
    assert_eq!(
        SceneBuilder::new(
            source,
            animation,
            0,
            b.scene(b.root_id()).unwrap().viewport,
            limits
        )
        .unwrap_err()
        .kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert_eq!(b.freeze().unwrap().memory_stats().live_bytes, expected);
}
#[test]
fn publication_rechecks_sibling_group_overlap() {
    let mut b = builder();
    let root = b.root_id();
    for _ in 0..2 {
        let input = ShapeInput {
            shape: Shape::new(SourceMap {
                element: Span { start: 0, end: 6 },
                transform: vexel_geometry::affine_identity(),
                transform_node: None,
            }),
            path: vexel_svg_path::PathDocument {
                edges: vec![Edge {
                    a: Point { x: 0., y: 0. },
                    b: Point { x: 1., y: 1. },
                }],
                subpaths: vec![vexel_svg_path::PathSubpath {
                    first_edge: 0,
                    edge_count: 1,
                    closed: false,
                }],
                segments: vec![],
                numbers: vec![],
            },
            edge_colors: vec![[0, 0, 0, 255]],
        };
        b.append_shape(root, input, &mut NoStop).unwrap();
    }
    let mut g = OpacityGroup {
        shapes: TableRange { first: 0, count: 1 },
        parent: None,
        opacity: 1.,
        filter: None,
        mask: None,
        blend_mode: 0,
        isolation: false,
    };
    b.append_group(root, g, &mut NoStop).unwrap();
    g.shapes = TableRange { first: 1, count: 1 };
    b.append_group(root, g, &mut NoStop).unwrap();
    b.scenes[0].groups[1].shapes = TableRange { first: 0, count: 2 };
    assert_eq!(b.freeze().unwrap_err().kind, ErrorKind::InvalidArgument);
}
