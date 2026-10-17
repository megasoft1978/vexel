//! Differential tests for the incremental commit-time validation
//! (WO-RUST-PIPELINE-PERF-01). At base_sha `commit_request` ran
//! `validate::scene` over the whole scene; these tests corrupt records that
//! belong to the committing request and require the commit to fail with the
//! exact `ErrorKind` and offset the whole-scene validator reports. The
//! expected values were captured from the whole-scene validator at base_sha.
use crate::builder::NoStop;
use crate::*;

const SOURCE: &[u8] = br#"<svg><path d="M0 0 L1 1 2 3 Z"/><path d="M0 0 L1 1 M2 2 L3 3"/></svg>"#;

fn builder() -> SceneBuilder {
    let source = std::sync::Arc::new(SOURCE.to_vec());
    let animation = vexel_svg_animation::parse_owned(&source, &[]).unwrap();
    let viewport = Viewport {
        width: 100,
        height: 100,
        view_x: 0.,
        view_y: 0.,
        view_width: 100.,
        view_height: 100.,
        aspect: Default::default(),
    };
    SceneBuilder::new(source, animation, 0, viewport, Limits::default()).unwrap()
}

fn input(second_path: bool) -> ShapeInput {
    let first = SOURCE.windows(2).position(|w| w == b"M0").unwrap();
    let last = SOURCE.windows(2).rposition(|w| w == b"M0").unwrap();
    let (start, end) = if second_path {
        (last, last + "M0 0 L1 1 M2 2 L3 3".len())
    } else {
        (first, first + "M0 0 L1 1 2 3 Z".len())
    };
    let path = vexel_svg_path::parse_path(
        SOURCE,
        Span { start, end },
        vexel_geometry::affine_identity(),
        Default::default(),
    )
    .unwrap();
    let colors = vec![[1, 2, 3, 255]; path.edges.len()];
    ShapeInput {
        shape: Shape::new(SourceMap {
            element: Span { start: 5, end: 31 },
            transform: vexel_geometry::affine_identity(),
            transform_node: None,
        }),
        path,
        edge_colors: colors,
    }
}

fn frame() -> TransformFrame {
    let a = vexel_geometry::affine_identity();
    TransformFrame {
        source_offset: 5,
        parent: None,
        attribute: a,
        suffix: a,
        motion: a,
    }
}

/// One committed request, then a second request holding `during` shapes and
/// two transforms, corrupted by `corrupt`. Returns the commit result and the
/// whole-scene validator result on the same state (the base behaviour).
fn outcome(
    before: usize,
    during: usize,
    corrupt: impl FnOnce(&mut SceneBuilder),
) -> (Result<()>, Result<()>) {
    let mut b = builder();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let first = b.begin_request(&draft, &mut NoStop).unwrap();
    for _ in 0..before {
        b.append_shape(root, input(false), &mut NoStop).unwrap();
    }
    b.commit_request(&draft, first, &mut NoStop).unwrap();
    let second = b.begin_request(&draft, &mut NoStop).unwrap();
    for _ in 0..during {
        b.append_shape(root, input(false), &mut NoStop).unwrap();
    }
    b.append_shape(root, input(true), &mut NoStop).unwrap();
    let parent = b.append_transform(root, frame(), &mut NoStop).unwrap();
    let mut child = frame();
    child.parent = Some(parent);
    b.append_transform(root, child, &mut NoStop).unwrap();
    corrupt(&mut b);
    let whole = crate::validate::scene(&b, root, &mut NoStop);
    let commit = b.commit_request(&draft, second, &mut NoStop);
    (commit, whole)
}

fn expect(
    before: usize,
    during: usize,
    corrupt: impl FnOnce(&mut SceneBuilder),
    kind: ErrorKind,
    offset: usize,
) {
    let (commit, whole) = outcome(before, during, corrupt);
    let commit = commit.unwrap_err();
    let whole = whole.unwrap_err();
    assert_eq!((commit.kind, commit.offset), (whole.kind, whole.offset));
    assert_eq!((commit.kind, commit.offset), (kind, offset));
}

#[test]
fn clean_requests_commit() {
    let (commit, whole) = outcome(5, 5, |_| {});
    assert!(commit.is_ok() && whole.is_ok());
}

#[test]
fn bad_stroke_width_is_rejected_identically() {
    expect(
        3,
        3,
        |b| b.scenes[0].shapes[4].stroke.width = f64::NAN,
        ErrorKind::InvalidArgument,
        0,
    );
}

#[test]
fn clip_index_out_of_range_is_rejected_identically() {
    expect(
        3,
        3,
        |b| {
            let owner = b.owner;
            b.scenes[0].shapes[5].clip = Some(ClipId {
                owner,
                scene: 0,
                slot: 9,
                serial: 1,
            });
        },
        ErrorKind::InvalidArgument,
        0,
    );
}

#[test]
fn edge_outside_its_shape_range_is_rejected_identically() {
    expect(
        3,
        3,
        |b| {
            let last = b.scenes[0].edge_shape_ids.len() - 1;
            b.scenes[0].edge_shape_ids[last] = 0;
        },
        ErrorKind::InvalidArgument,
        0,
    );
}

#[test]
fn unordered_segment_is_rejected_identically() {
    expect(
        3,
        3,
        |b| {
            let first = b.scenes[0].path_segments[0].shape;
            b.scenes[0].path_segments.last_mut().unwrap().shape = first;
        },
        ErrorKind::InvalidArgument,
        0,
    );
}

#[test]
fn non_finite_path_number_is_rejected_identically() {
    expect(
        3,
        3,
        |b| {
            let number = b.scenes[0].path_numbers.last_mut().unwrap();
            number.value = f64::INFINITY;
        },
        ErrorKind::InvalidArgument,
        59,
    );
}

#[test]
fn overlapping_subpath_is_rejected_identically() {
    expect(
        3,
        3,
        |b| {
            let shape = *b.scenes[0].shapes.iter().rev().nth(0).unwrap();
            assert_eq!(shape.subpaths.count, 2);
            let second = shape.subpaths.first + 1;
            b.scenes[0].path_subpaths[second].first_edge = shape.edges.first;
        },
        ErrorKind::InvalidArgument,
        0,
    );
}

#[test]
fn non_finite_edge_is_rejected_identically() {
    expect(
        3,
        3,
        |b| b.scenes[0].edges.last_mut().unwrap().b.x = f64::NAN,
        ErrorKind::InvalidArgument,
        0,
    );
}

#[test]
fn transform_parent_order_is_rejected_identically() {
    expect(
        3,
        3,
        |b| {
            let last = b.scenes[0].transforms.len() - 1;
            let id = TransformId {
                owner: b.owner,
                scene: 0,
                slot: last as u32,
                serial: b.scenes[0].transforms_serials[last],
            };
            b.scenes[0].transforms[last].parent = Some(id);
        },
        ErrorKind::InvalidArgument,
        5,
    );
}

#[test]
fn unresolved_admission_is_rejected_identically() {
    let mut b = builder();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let request = b.begin_request(&draft, &mut NoStop).unwrap();
    b.append_shape(root, input(false), &mut NoStop).unwrap();
    let _admission = b.admit_shape(&draft, request, 9, &mut NoStop).unwrap();
    let whole = crate::validate::scene(&b, root, &mut NoStop).unwrap_err();
    let commit = b.commit_request(&draft, request, &mut NoStop).unwrap_err();
    assert_eq!((commit.kind, commit.offset), (whole.kind, whole.offset));
    assert_eq!(
        (commit.kind, commit.offset),
        (ErrorKind::InvalidArgument, 9)
    );
}

#[test]
fn prefix_corruption_is_still_rejected_at_freeze() {
    let mut b = builder();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let first = b.begin_request(&draft, &mut NoStop).unwrap();
    for _ in 0..3 {
        b.append_shape(root, input(false), &mut NoStop).unwrap();
    }
    b.commit_request(&draft, first, &mut NoStop).unwrap();
    let second = b.begin_request(&draft, &mut NoStop).unwrap();
    b.append_shape(root, input(false), &mut NoStop).unwrap();
    // Only the record next to the request boundary is re-checked at commit.
    b.scenes[0].shapes[0].stroke.width = f64::NAN;
    b.commit_request(&draft, second, &mut NoStop).unwrap();
    // Strict acceptance is unchanged: draft commit and freeze check the whole scene.
    assert_eq!(
        b.commit_draft(&draft, &mut NoStop).unwrap_err().kind,
        ErrorKind::InvalidArgument
    );
    assert!(b.freeze().is_err());
}

fn tables(graph: &SceneGraph) -> String {
    let view = graph.root();
    let segments: Vec<_> = view.path_segments.iter().map(|s| s.path).collect();
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        view.edges,
        view.edge_colors,
        view.edge_shape_ids,
        view.shapes,
        view.path_subpaths,
        segments,
        view.path_numbers
    )
}

#[test]
fn scene_built_in_several_requests_equals_one_shot_scene() {
    // One fixed shape sequence, split into different request boundaries.
    let sequence = |split: &[usize]| {
        let mut b = builder();
        let root = b.root_id();
        let draft = b.begin_draft(root).unwrap();
        let mut index = 0;
        for &count in split {
            let request = b.begin_request(&draft, &mut NoStop).unwrap();
            for _ in 0..count {
                b.append_shape(root, input(index % 3 == 0), &mut NoStop)
                    .unwrap();
                index += 1;
            }
            b.commit_request(&draft, request, &mut NoStop).unwrap();
        }
        b.commit_draft(&draft, &mut NoStop).unwrap();
        b.freeze().unwrap()
    };
    let one = tables(&sequence(&[60]));
    assert_eq!(one, tables(&sequence(&[20, 20, 20])));
    assert_eq!(one, tables(&sequence(&[1, 58, 1])));
    assert_eq!(one, tables(&sequence(&[7, 7, 7, 7, 7, 7, 7, 7, 4])));
}
