mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;
#[test]
fn source_is_shared_once_with_animation_and_survives_worker_move() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<SceneGraph>();
    let source = owner();
    let ptr = source.as_ptr() as usize;
    let anim = animation(&source);
    let retained = anim.storage_bytes().unwrap();
    let builder =
        SceneBuilder::new(source.clone(), anim, 72, viewport(), Limits::default()).unwrap();
    assert_eq!(builder.memory_stats().source_bytes, source.capacity());
    assert!(builder.memory_stats().live_bytes >= retained);
    assert_eq!(Arc::strong_count(&source), 3);
    drop(source);
    let graph = builder.freeze().unwrap();
    assert_eq!(graph.generation(), 72);
    assert_eq!(graph.animation().source.as_ptr(), graph.source().as_ptr());
    assert_eq!(
        std::thread::spawn(move || graph.source().as_ptr() as usize)
            .join()
            .unwrap(),
        ptr
    );
}
#[test]
fn equal_bytes_from_a_different_owner_are_rejected() {
    let source = owner();
    let other = Arc::new(source.as_slice().to_vec());
    assert_eq!(
        SceneBuilder::new(other, animation(&source), 0, viewport(), Limits::default())
            .unwrap_err()
            .kind,
        ErrorKind::InvalidArgument
    );
}
#[test]
fn exact_colors_and_raw_path_metadata_are_rebased() {
    let mut b = builder();
    let root = b.root_id();
    let original = shape(owner().as_slice());
    let expected = original.path.clone();
    let colors = original.edge_colors.clone();
    let a = b.append_shape(root, original, &mut Run).unwrap();
    let second = shape(owner().as_slice());
    let id = b.append_shape(root, second, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let view = graph.root();
    let count = expected.edges.len();
    assert_eq!(view.shapes.len(), 2);
    assert_eq!(view.edges[..count], expected.edges);
    assert_eq!(view.edges[count..], expected.edges);
    assert_eq!(&view.edge_colors[..count], &colors);
    assert_eq!(&view.edge_colors[count..], &colors);
    for (i, segment) in expected.segments.iter().enumerate() {
        let actual = &view.path_segments[i + expected.segments.len()];
        assert_eq!(actual.shape, id);
        assert_eq!(actual.path.first_edge, segment.first_edge + count);
        assert_eq!(
            actual.path.first_number,
            segment.first_number + expected.numbers.len()
        );
        assert_eq!(actual.path.source, segment.source);
        assert_eq!(actual.path.command_offset, segment.command_offset);
        assert_eq!(actual.path.command, segment.command);
    }
    assert!(expected.segments.iter().any(|s| s.command_offset.is_none()));
    assert_eq!(view.edge_shape(0), Some(a));
    assert_eq!(view.edge_shape(count), Some(id));
    assert_eq!(
        view.shape(id).unwrap().subpaths.first,
        expected.subpaths.len()
    );
}
#[test]
fn rollback_rejects_reused_handles_and_stale_marks_preserving_prior_handles() {
    let mut b = builder();
    let root = b.root_id();
    let first = b.append_transform(root, transform(None), &mut Run).unwrap();
    let mark = b.mark(root).unwrap();
    let stale_mark = b.mark(root).unwrap();
    let removed = b
        .append_transform(root, transform(Some(first)), &mut Run)
        .unwrap();
    b.rollback(mark).unwrap();
    assert!(b.rollback(stale_mark).is_err());
    let replacement = b
        .append_transform(root, transform(Some(first)), &mut Run)
        .unwrap();
    assert_eq!(removed.index(), replacement.index());
    assert_ne!(removed, replacement);
    let view = b.scene(root).unwrap();
    assert!(view.transform(first).is_some());
    assert!(view.transform(removed).is_none());
    assert!(view.transform(replacement).is_some());
    assert!(
        b.append_transform(root, transform(Some(removed)), &mut Run)
            .is_err()
    );
}
#[test]
fn handles_and_marks_reject_wrong_graph_and_scene() {
    let mut a = builder();
    let root = a.root_id();
    let id = a.append_transform(root, transform(None), &mut Run).unwrap();
    let mark = a.mark(root).unwrap();
    let other = a.new_scene(viewport()).unwrap();
    assert!(
        a.append_transform(other, transform(Some(id)), &mut Run)
            .is_err()
    );
    assert!(a.scene(other).unwrap().transform(id).is_none());
    let mut b = builder();
    assert!(b.scene(root).is_err());
    assert!(b.rollback(mark).is_err());
    assert!(b.scene(b.root_id()).unwrap().transform(id).is_none());
}
#[test]
fn masks_require_sealed_children_and_cannot_create_cycles() {
    let mut b = builder();
    let root = b.root_id();
    let child = b.new_scene(viewport()).unwrap();
    assert!(b.append_mask(root, mask(child), &mut Run).is_err());
    assert!(b.append_mask(root, mask(root), &mut Run).is_err());
    b.seal(child, &mut Run).unwrap();
    assert!(
        b.append_transform(child, transform(None), &mut Run)
            .is_err()
    );
    let mid = b.append_mask(root, mask(child), &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let value = graph.root().mask(mid).unwrap();
    assert_eq!(graph.scene(value.content).unwrap().id, child);
}
#[test]
fn all_scene_tables_are_accessible_as_immutable_views() {
    let mut b = builder();
    let root = b.root_id();
    let child = b.new_scene(viewport()).unwrap();
    b.seal(child, &mut Run).unwrap();
    let t = b.append_transform(root, transform(None), &mut Run).unwrap();
    let c = b.append_clip(root, clip(), &mut Run).unwrap();
    let p = b.append_pattern(root, pattern(), &mut Run).unwrap();
    let m = b.append_mask(root, mask(child), &mut Run).unwrap();
    let f = b
        .append_filter(root, filter(), &[primitive()], &mut Run)
        .unwrap();
    let image = b.append_image(root, image(), &mut Run).unwrap();
    let dashes = b.append_dashes(root, &[2., 3.], &mut Run).unwrap();
    let stops = b
        .append_stops(
            root,
            &[
                PaintStop {
                    offset: 0.,
                    rgba: [0; 4],
                },
                PaintStop {
                    offset: 1.,
                    rgba: [255; 4],
                },
            ],
            &mut Run,
        )
        .unwrap();
    let mut fill = paint();
    fill.kind = PaintKind::Pattern;
    fill.pattern = Some(p);
    fill.stops = stops;
    let fp = b.append_fill_paint(root, fill, &mut Run).unwrap();
    let sp = b.append_stroke_paint(root, paint(), &mut Run).unwrap();
    let mut input = shape(owner().as_slice());
    input.shape.source.transform_node = Some(t);
    input.shape.clip = Some(c);
    input.shape.image = Some(image);
    input.shape.fill_paint = Some(fp);
    input.shape.stroke_paint = Some(sp);
    input.shape.stroke.dash_values = dashes;
    let shape = b.append_shape(root, input, &mut Run).unwrap();
    let mut g = group();
    g.shapes = TableRange { first: 0, count: 1 };
    g.mask = Some(m);
    g.filter = Some(f);
    let group = b.append_group(root, g, &mut Run).unwrap();
    b.record_ignored(IgnoredKind::Meta, 0).unwrap();
    let graph = b.freeze().unwrap();
    let v = graph.root();
    assert!(v.shape(shape).is_some());
    assert!(v.group(group).is_some());
    assert_eq!(v.transforms.len(), 1);
    assert_eq!(v.images.len(), 1);
    assert_eq!(v.clips.len(), 1);
    assert_eq!(v.patterns.len(), 1);
    assert_eq!(v.masks.len(), 1);
    assert_eq!(v.filters.len(), 1);
    assert_eq!(v.filter_primitives.len(), 1);
    assert_eq!(v.groups.len(), 1);
    assert_eq!(v.fill_paints.len(), 1);
    assert_eq!(v.stroke_paints.len(), 1);
    assert_eq!(v.dash_values, &[2., 3.]);
    assert_eq!(v.stops.len(), 2);
    assert!(!v.path_segments.is_empty());
    assert!(!v.path_subpaths.is_empty());
    assert!(!v.path_numbers.is_empty());
    assert_eq!(graph.ignored(IgnoredKind::Meta).count, 1);
}
#[test]
fn invalid_ranges_geometry_parents_and_media_do_not_publish() {
    let mut b = builder();
    let root = b.root_id();
    let before = b.scene(root).unwrap().shapes.len();
    let mut input = shape(owner().as_slice());
    input.path.edges[0].a.x = f64::NAN;
    assert!(b.append_shape(root, input, &mut Run).is_err());
    assert_eq!(b.scene(root).unwrap().shapes.len(), before);
    let mut bad = image();
    bad.pixels.row_stride = 3;
    assert!(b.append_image(root, bad, &mut Run).is_err());
    let mut bad = image();
    bad.pixels.rgba[0] = 255;
    bad.pixels.rgba[3] = 1;
    assert!(b.append_image(root, bad, &mut Run).is_err());
    let mut clip = clip();
    clip.geometry.shapes.push(ClipChild {
        edges: TableRange {
            first: usize::MAX,
            count: 2,
        },
        evenodd: false,
        filled: true,
    });
    assert!(b.append_clip(root, clip, &mut Run).is_err());
    let mut g = group();
    g.shapes = TableRange { first: 0, count: 1 };
    assert!(b.append_group(root, g, &mut Run).is_err());
    let mut p = primitive();
    p.input = FilterInput::Result(0);
    assert!(b.append_filter(root, filter(), &[p], &mut Run).is_err());
    assert_eq!(b.scene(root).unwrap().images.len(), 0);
    assert!(b.freeze().is_ok());
}
#[test]
fn cancellation_in_every_shape_copy_phase_restores_all_table_lengths() {
    let reference = shape(owner().as_slice());
    let total = reference.path.edges.len() * 2
        + reference.path.subpaths.len() * 2
        + reference.path.segments.len() * 2
        + reference.path.numbers.len() * 2
        + 1;
    let mut failed = 0;
    for stop in 0..total {
        let mut b = builder();
        let root = b.root_id();
        let result = b.append_shape(root, shape(owner().as_slice()), &mut StopAfter(stop));
        if let Err(error) = result {
            assert_eq!(error.kind, ErrorKind::Canceled);
            failed += 1;
            let v = b.scene(root).unwrap();
            assert!(v.edges.is_empty());
            assert!(v.edge_colors.is_empty());
            assert!(v.edge_shape_ids.is_empty());
            assert!(v.shapes.is_empty());
            assert!(v.path_segments.is_empty());
            assert!(v.path_subpaths.is_empty());
            assert!(v.path_numbers.is_empty());
            assert!(b.freeze().is_ok());
        }
    }
    assert_eq!(failed, total);
}
#[test]
fn shape_policy_and_memory_failures_keep_the_scene_logically_empty() {
    let source = owner();
    let mut limits = Limits::default();
    limits.edges = 0;
    let mut b =
        SceneBuilder::new(source.clone(), animation(&source), 0, viewport(), limits).unwrap();
    let root = b.root_id();
    assert_eq!(
        b.append_shape(root, shape(&source), &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::Edges)
    );
    assert!(b.scene(root).unwrap().edges.is_empty());
    let baseline = builder().memory_stats().live_bytes;
    limits = Limits::default();
    limits.reserved_bytes = baseline;
    let mut b =
        SceneBuilder::new(source.clone(), animation(&source), 0, viewport(), limits).unwrap();
    let root = b.root_id();
    assert_eq!(
        b.append_shape(root, shape(&source), &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert!(b.scene(root).unwrap().edges.is_empty());
    assert!(b.freeze().is_ok());
}
#[test]
fn nested_capacity_and_padding_are_charged_and_released_on_rollback() {
    let mut b = builder();
    let root = b.root_id();
    let initial = b.memory_stats();
    let mark = b.mark(root).unwrap();
    let im = image();
    let image_capacity = im.pixels.rgba.capacity();
    let cl = clip();
    let clip_capacity = cl.geometry.edges.capacity() * std::mem::size_of::<Edge>()
        + cl.geometry.shapes.capacity() * std::mem::size_of::<ClipChild>();
    let pat = pattern();
    let pattern_capacity = pat.tile.allocated_byte_count();
    b.append_image(root, im, &mut Run).unwrap();
    b.append_clip(root, cl, &mut Run).unwrap();
    b.append_pattern(root, pat, &mut Run).unwrap();
    let full = b.memory_stats();
    assert!(
        full.live_bytes >= initial.live_bytes + image_capacity + clip_capacity + pattern_capacity
    );
    b.rollback(mark).unwrap();
    let after = b.memory_stats();
    assert_eq!(
        full.live_bytes - after.live_bytes,
        image_capacity + clip_capacity + pattern_capacity
    );
    assert_eq!(full.peak_live_bytes, after.peak_live_bytes);
    assert_eq!(full.total_allocated_bytes, after.total_allocated_bytes);
    assert!(b.scene(root).unwrap().images.is_empty());
    assert!(b.freeze().is_ok());
}
#[test]
fn retained_growth_can_fit_while_transient_overlap_is_rejected() {
    let mut reference = builder();
    let root = reference.root_id();
    for _ in 0..8 {
        reference
            .append_transform(root, transform(None), &mut Run)
            .unwrap();
    }
    let live = reference.memory_stats().live_bytes;
    let one = reference.memory_stats().live_bytes - builder().memory_stats().live_bytes;
    let source = owner();
    let mut limits = Limits::default();
    limits.reserved_bytes = live + one;
    let mut b =
        SceneBuilder::new(source.clone(), animation(&source), 0, viewport(), limits).unwrap();
    let root = b.root_id();
    for _ in 0..8 {
        b.append_transform(root, transform(None), &mut Run).unwrap();
    }
    let stats = b.memory_stats();
    assert_eq!(
        b.append_transform(root, transform(None), &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert_eq!(b.scene(root).unwrap().transforms.len(), 8);
    assert!(b.memory_stats().peak_live_bytes <= limits.reserved_bytes);
    assert!(b.memory_stats().live_bytes >= stats.live_bytes);
}
#[test]
fn freeze_cancellation_never_returns_a_partial_graph() {
    let mut b = builder();
    let root = b.root_id();
    b.append_shape(root, shape(owner().as_slice()), &mut Run)
        .unwrap();
    assert_eq!(
        b.freeze_controlled(&mut StopAfter(1)).unwrap_err().kind,
        ErrorKind::Canceled
    );
}
#[test]
fn per_table_caps_and_invalid_paint_dashes_stops_are_enforced() {
    let source = owner();
    let mut limits = Limits::default();
    limits.transforms = 1;
    let mut b =
        SceneBuilder::new(source.clone(), animation(&source), 0, viewport(), limits).unwrap();
    let root = b.root_id();
    b.append_transform(root, transform(None), &mut Run).unwrap();
    assert_eq!(
        b.append_transform(root, transform(None), &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::Transforms)
    );
    assert!(b.append_dashes(root, &[-1.], &mut Run).is_err());
    assert!(
        b.append_stops(
            root,
            &[PaintStop {
                offset: f64::NAN,
                rgba: [0; 4]
            }],
            &mut Run
        )
        .is_err()
    );
    let mut p = paint();
    p.stops = TableRange {
        first: usize::MAX,
        count: 1,
    };
    assert!(b.append_fill_paint(root, p, &mut Run).is_err());
}
#[test]
fn text_inputs_and_returned_outlines_are_bounded_and_cancelable() {
    let request = TextRequest {
        text: b"hello",
        font_families: &[b"sans"],
        size_points: 16.,
        weight: 400.,
        style: TextStyle::Normal,
        letter_spacing_points: 0.,
    };
    assert!(request.validate(&mut Run).is_ok());
    assert_eq!(
        request.validate(&mut StopAfter(2)).unwrap_err().kind,
        ErrorKind::Canceled
    );
    let mut layout = TextLayout {
        glyphs: vec![TextGlyph {
            glyph_id: Some(1),
            x: 0.,
            y: 0.,
            advance: 3.,
            commands: TableRange { first: 0, count: 1 },
        }],
        commands: vec![TextCommand {
            verb: TextVerb::Move,
            values: [0.; 6],
        }],
        advance_width: 3.,
        ascent: 4.,
        descent: 1.,
        leading: 0.,
    };
    assert!(layout.validate(&mut Run).is_ok());
    assert!(layout.storage_bytes().unwrap() > 0);
    layout.glyphs[0].commands.count = 2;
    assert!(layout.validate(&mut Run).is_err());
}
#[test]
fn a_move_command_cannot_claim_a_geometry_edge() {
    let mut b = builder();
    let root = b.root_id();
    let mut input = shape(owner().as_slice());
    input.path.segments[0].edge_count = 1;
    assert_eq!(input.path.segments[0].command, b'M');
    assert_eq!(
        b.append_shape(root, input, &mut Run).unwrap_err().kind,
        ErrorKind::InvalidArgument
    );
    assert!(b.scene(root).unwrap().edges.is_empty());
}
#[test]
fn child_and_sparse_paint_accessors_are_scoped_and_source_coherent() {
    let mut b = builder();
    let root = b.root_id();
    let child = b.new_scene(viewport()).unwrap();
    assert!(b.scene(root).unwrap().scene(child).is_none());
    b.seal(child, &mut Run).unwrap();
    let p = b.append_fill_paint(root, paint(), &mut Run).unwrap();
    let d = b.append_dashes(root, &[1., 2.], &mut Run).unwrap();
    let mut input = shape(owner().as_slice());
    input.shape.fill_paint = Some(p);
    input.shape.stroke.dash_values = d;
    let id = b.append_shape(root, input, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let view = graph.root();
    assert_eq!(
        view.scene(child).unwrap().source().as_ptr(),
        view.source().as_ptr()
    );
    assert_eq!(view.animation().source.as_ptr(), view.source().as_ptr());
    assert_eq!(view.fill_paint(id), Some(&paint()));
    assert!(view.stroke_paint(id).is_none());
    assert_eq!(view.shape_dashes(id), Some([1., 2.].as_slice()));
    assert_eq!(view.tables().shapes.len(), 1);
}
#[test]
fn empty_collection_appends_still_observe_cancellation() {
    let mut b = builder();
    let root = b.root_id();
    assert_eq!(
        b.append_dashes(root, &[], &mut StopAfter(0))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(
        b.append_stops(root, &[], &mut StopAfter(0))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
}
#[test]
fn arithmetic_composite_mode_and_multiple_open_subpaths_keep_legacy_metadata() {
    let mut b = builder();
    let root = b.root_id();
    let mut p = primitive();
    p.kind = FilterKind::Composite;
    p.mode = 6;
    b.append_filter(root, filter(), &[p], &mut Run).unwrap();
    for data in [
        "m0 0 l1 1 m2 2 l3 3",
        "M0 0 C1 0 2 1 3 2 S4 4 5 5 Q6 5 7 7 T8 8 A2 2 0 0 1 10 10 Z",
        "M0 0 L0 0 Z",
    ] {
        let source = Arc::new(format!("<svg><path d=\"{data}\"/></svg>").into_bytes());
        let mut b = SceneBuilder::new(
            source.clone(),
            animation(&source),
            0,
            viewport(),
            Limits::default(),
        )
        .unwrap();
        let root = b.root_id();
        let start = 14;
        let path = vexel_svg_path::parse_path(
            &source,
            Span {
                start,
                end: start + data.len(),
            },
            vexel_geometry::affine_identity(),
            Default::default(),
        )
        .unwrap();
        let colors = vec![[0, 0, 0, 255]; path.edges.len()];
        b.append_shape(
            root,
            ShapeInput {
                shape: Shape::new(SourceMap {
                    element: Span {
                        start: 5,
                        end: source.len() - 6,
                    },
                    transform: vexel_geometry::affine_identity(),
                    transform_node: None,
                }),
                path,
                edge_colors: colors,
            },
            &mut Run,
        )
        .unwrap();
        assert!(b.freeze().is_ok());
    }
}
#[test]
fn opacity_group_siblings_are_disjoint_and_nested_ranges_follow_painter_order() {
    let mut b = builder();
    let root = b.root_id();
    for _ in 0..3 {
        b.append_shape(root, shape(owner().as_slice()), &mut Run)
            .unwrap();
    }
    let mut outer = group();
    outer.shapes = TableRange { first: 0, count: 2 };
    let parent = b.append_group(root, outer, &mut Run).unwrap();
    let mut child = group();
    child.parent = Some(parent);
    child.shapes = TableRange { first: 0, count: 1 };
    b.append_group(root, child, &mut Run).unwrap();
    child.shapes = TableRange { first: 1, count: 1 };
    b.append_group(root, child, &mut Run).unwrap();
    let mut overlap = group();
    overlap.shapes = TableRange { first: 1, count: 1 };
    assert!(b.append_group(root, overlap, &mut Run).is_err());
    let mut sibling = group();
    sibling.shapes = TableRange { first: 2, count: 1 };
    b.append_group(root, sibling, &mut Run).unwrap();
    assert!(b.append_group(root, child, &mut Run).is_err());
    assert_eq!(b.scene(root).unwrap().groups.len(), 4);
    assert!(b.freeze().is_ok());
}
#[test]
fn overlapping_root_groups_are_rejected_without_publication() {
    let mut b = builder();
    let root = b.root_id();
    for _ in 0..2 {
        b.append_shape(root, shape(owner().as_slice()), &mut Run)
            .unwrap();
    }
    let mut a = group();
    a.shapes = TableRange { first: 0, count: 2 };
    b.append_group(root, a, &mut Run).unwrap();
    a.shapes = TableRange { first: 1, count: 1 };
    assert_eq!(
        b.append_group(root, a, &mut Run).unwrap_err().kind,
        ErrorKind::InvalidArgument
    );
    assert_eq!(b.scene(root).unwrap().groups.len(), 1);
    assert!(b.freeze().is_ok());
}
