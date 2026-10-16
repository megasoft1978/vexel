mod support;
use std::sync::Arc;
use support::*;
use vexel_scene::*;

fn clip_bytes(values: &Vec<Clip>) -> usize {
    values.capacity() * std::mem::size_of::<Clip>()
        + values
            .iter()
            .map(|v| {
                v.geometry.edges.capacity() * std::mem::size_of::<Edge>()
                    + v.geometry.shapes.capacity() * std::mem::size_of::<ClipChild>()
            })
            .sum::<usize>()
}
fn stats(frame: &EvaluatedFrame<'_>) -> (usize, usize, usize, usize) {
    let s = frame.memory_stats();
    (
        s.live_bytes,
        s.peak_live_bytes,
        s.total_allocated_bytes,
        s.allocation_count,
    )
}
fn cache_entry(source: &SourceOwner) -> GradientCacheEntry {
    let mut stops = Vec::with_capacity(17);
    stops.extend([
        PaintStop {
            offset: 0.,
            rgba: [0, 1, 2, 255],
        },
        PaintStop {
            offset: 1.,
            rgba: [3, 4, 5, 255],
        },
    ]);
    let range = TableRange { first: 0, count: 2 };
    let mut paint = paint();
    paint.stops = range;
    GradientCacheEntry {
        definition: ElementId(0),
        context: GradientCacheContext {
            viewport: [100., 200.],
            user_inverse: vexel_geometry::affine_identity(),
            bounds: [0., 0., 1., 1.],
        },
        paint,
        stop_range: range,
        stops,
        source: source.clone(),
    }
}

#[test]
fn all_existing_record_handles_are_enumerable_and_range_checked() {
    let mut b = builder();
    let root = b.root_id();
    let t = b.append_transform(root, transform(None), &mut Run).unwrap();
    let c = b.append_clip(root, clip(), &mut Run).unwrap();
    let p = b.append_pattern(root, pattern(), &mut Run).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    b.seal(child, &mut Run).unwrap();
    let m = b
        .append_mask(
            root,
            Mask {
                content: child,
                luminance: false,
                region_object_bounding_box: false,
                custom_region: false,
                region: [0., 0., 1., 1.],
                region_transform: vexel_geometry::affine_identity(),
                source_offset: 0,
            },
            &mut Run,
        )
        .unwrap();
    let f = b
        .append_filter(root, filter(), &[primitive()], &mut Run)
        .unwrap();
    let s = b
        .append_shape(root, shape(owner().as_slice()), &mut Run)
        .unwrap();
    let g = b
        .append_group(
            root,
            OpacityGroup {
                shapes: TableRange { first: 0, count: 1 },
                parent: None,
                opacity: 1.,
                filter: None,
                mask: None,
                blend_mode: 0,
                isolation: false,
            },
            &mut Run,
        )
        .unwrap();
    let graph = b.freeze().unwrap();
    let v = graph.root();
    assert_eq!(v.shape_id(0), Some(s));
    assert_eq!(v.transform_id(0), Some(t));
    assert_eq!(v.clip_id(0), Some(c));
    assert_eq!(v.pattern_id(0), Some(p));
    assert_eq!(v.mask_id(0), Some(m));
    assert_eq!(v.filter_id(0), Some(f));
    assert_eq!(v.group_id(0), Some(g));
    assert_eq!(graph.scene_id(0), Some(root));
    assert_eq!(graph.scene_id(1), Some(child));
    for i in [1, usize::MAX] {
        assert!(v.shape_id(i).is_none());
        assert!(v.transform_id(i).is_none());
        assert!(v.clip_id(i).is_none());
        assert!(v.pattern_id(i).is_none());
        assert!(v.mask_id(i).is_none());
        assert!(v.filter_id(i).is_none());
        assert!(v.group_id(i).is_none());
    }
    assert!(graph.scene_id(2).is_none());
    assert!(graph.scene_id(usize::MAX).is_none());
}

#[test]
fn enumeration_preserves_stale_foreign_and_scene_identity_guards() {
    let mut b = builder();
    let root = b.root_id();
    let mark = b.mark(root).unwrap();
    let old = b.append_clip(root, clip(), &mut Run).unwrap();
    b.rollback(mark).unwrap();
    let fresh = b.append_clip(root, clip(), &mut Run).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let child_clip = b.append_clip(child, clip(), &mut Run).unwrap();
    b.seal(child, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let v = graph.root();
    assert_eq!(v.clip_id(0), Some(fresh));
    assert_ne!(fresh, old);
    assert!(v.clip(old).is_none());
    assert!(v.clip(child_clip).is_none());
    assert_eq!(graph.scene(child).unwrap().clip_id(0), Some(child_clip));
    let other = builder().freeze().unwrap();
    assert!(other.scene(root).is_none());
    assert!(other.root().clip(fresh).is_none());
}

#[test]
fn finished_clip_edit_preserves_effective_handles_transforms_and_base() {
    let mut b = builder();
    let root = b.root_id();
    let t = b.append_transform(root, transform(None), &mut Run).unwrap();
    let mut original = clip();
    original.owner_node = Some(t);
    let c = b.append_clip(root, original, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let mut build = FrameBuilder::new(graph.root(), 1 << 20).unwrap();
    let mut tr = transform(None);
    tr.attribute.e = 7.;
    build.set_transform(t, tr, &mut Run).unwrap();
    let mut frame = build.finish(&mut Run).unwrap();
    let mut replacement = clip();
    replacement.owner_node = Some(t);
    replacement.user_inverse.e = -7.;
    replacement.bounds[2] = 9.;
    let values = vec![replacement];
    let bytes = clip_bytes(&values);
    let before = stats(&frame);
    frame.replace_clips(values, &mut Run).unwrap();
    let v = frame.view();
    assert_eq!(v.clip_id(0), Some(c));
    assert_eq!(v.transform_id(0), Some(t));
    assert_eq!(v.transform(t).unwrap().attribute.e, 7.);
    assert_eq!(v.clip(c).unwrap().user_inverse.e, -7.);
    assert_eq!(v.scene(v.id).unwrap().clip(c).unwrap().bounds[2], 9.);
    assert_eq!(graph.root().clip(c).unwrap().bounds[2], 1.);
    assert_eq!(graph.root().transform(t).unwrap().attribute.e, 0.);
    assert_eq!(frame.memory_stats().live_bytes, before.0 + bytes);
    assert_eq!(frame.memory_stats().total_allocated_bytes, before.2 + bytes);
}

#[test]
fn finished_clip_replacement_respects_original_budget_and_live_overlap() {
    let mut b = builder();
    let root = b.root_id();
    let c = b.append_clip(root, clip(), &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let values = vec![clip()];
    let limit = clip_bytes(&values);
    let mut frame = FrameBuilder::new(graph.root(), limit)
        .unwrap()
        .finish(&mut Run)
        .unwrap();
    frame.replace_clips(values, &mut Run).unwrap();
    let before = stats(&frame);
    let mut changed = clip();
    changed.bounds[2] = 3.;
    assert_eq!(
        frame
            .replace_clips(vec![changed], &mut Run)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::ReservedBytes)
    );
    assert_eq!(stats(&frame), before);
    assert_eq!(frame.view().clip(c).unwrap().bounds[2], 1.);
    let mut tiny = FrameBuilder::new(graph.root(), 0)
        .unwrap()
        .finish(&mut Run)
        .unwrap();
    assert!(tiny.replace_clips(vec![clip()], &mut Run).is_err());
    assert_eq!(stats(&tiny), (0, 0, 0, 0));
}

#[test]
fn finished_clip_invalid_parent_range_and_cancellation_publish_nothing() {
    let mut b = builder();
    let root = b.root_id();
    let a = b.append_clip(root, clip(), &mut Run).unwrap();
    let mut child = clip();
    child.parent = Some(a);
    let c = b.append_clip(root, child, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let mut frame = FrameBuilder::new(graph.root(), 1 << 20)
        .unwrap()
        .finish(&mut Run)
        .unwrap();
    let before = stats(&frame);
    assert!(frame.replace_clips(vec![clip()], &mut Run).is_err());
    assert!(frame.replace_clips(vec![clip(), clip()], &mut Run).is_err());
    let mut bad = clip();
    bad.parent = Some(a);
    bad.geometry.shapes.push(ClipChild {
        edges: TableRange {
            first: usize::MAX,
            count: 1,
        },
        evenodd: false,
        filled: true,
    });
    assert!(frame.replace_clips(vec![clip(), bad], &mut Run).is_err());
    let mut changed = clip();
    changed.bounds[2] = 8.;
    let mut child = clip();
    child.parent = Some(a);
    assert_eq!(
        frame
            .replace_clips(vec![changed, child], &mut StopAfter(1))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(stats(&frame), before);
    assert_eq!(frame.view().clip(a).unwrap().bounds[2], 1.);
    assert_eq!(frame.view().clip(c).unwrap().parent, Some(a));
}

#[test]
fn cache_context_includes_viewport_inverse_bounds_and_owned_stop_storage() {
    let source = Arc::new(b"<svg/>".to_vec());
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&lexical, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 16).unwrap();
    let mut cache = PaintCache::default();
    let mut other = cache_entry(&source);
    other.context.viewport[0] = 200.;
    assert_ne!(cache_entry(&source).context, other.context);
    let mut inverse = cache_entry(&source);
    inverse.context.user_inverse.e = 2.;
    assert_ne!(inverse.context, other.context);
    let mut bounds = cache_entry(&source);
    bounds.context.bounds[2] = 2.;
    assert_ne!(bounds.context, other.context);
    cache.entries.push(cache_entry(&source));
    cache.entries.push(other);
    cache.slots = vec![Some(0), Some(1)];
    let expected = cache.entries.capacity() * std::mem::size_of::<GradientCacheEntry>()
        + cache.slots.capacity() * std::mem::size_of::<Option<usize>>()
        + cache
            .entries
            .iter()
            .map(|e| e.stops.capacity() * std::mem::size_of::<PaintStop>())
            .sum::<usize>();
    assert_eq!(cache.storage_bytes().unwrap(), expected);
    assert_eq!(cache.storage_bytes_controlled(&mut Run).unwrap(), expected);
    assert_eq!(ledger.stats().source_bytes, source.capacity());
    ResolveContext {
        shared: &mut shared,
        cache: &mut cache,
        ledger: &mut ledger,
    }
    .validate_cache(&mut Run)
    .unwrap();
}

#[test]
fn cached_source_identity_rejects_equal_bytes_in_another_allocation() {
    let source = Arc::new(b"<svg/>".to_vec());
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&lexical, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 16).unwrap();
    let foreign = Arc::new(source.as_ref().clone());
    let mut cache = PaintCache::default();
    cache.entries.push(cache_entry(&foreign));
    assert_eq!(
        ResolveContext {
            shared: &mut shared,
            cache: &mut cache,
            ledger: &mut ledger
        }
        .validate_cache(&mut Run)
        .unwrap_err()
        .kind,
        ErrorKind::InvalidArgument
    );
}

#[test]
fn malformed_cache_context_ranges_slots_and_stops_fail_with_cancellation() {
    let source = Arc::new(b"<svg/>".to_vec());
    let lexical = LexicalDocument::parse(source.clone(), 1 << 20, &mut Run).unwrap();
    let mut shared = DecodeSource::new(&lexical, &mut Run).unwrap();
    let mut ledger = ChargeLedger::new(&source, 1 << 20, 16).unwrap();
    for case in 0..7 {
        let mut entry = cache_entry(&source);
        let mut cache = PaintCache::default();
        match case {
            0 => entry.context.viewport[0] = f64::NAN,
            1 => entry.context.user_inverse.a = f64::INFINITY,
            2 => entry.context.bounds[0] = 2.,
            3 => entry.stop_range.first = usize::MAX,
            4 => entry.stops[1].offset = -1.,
            5 => entry.definition = ElementId(usize::MAX),
            _ => cache.slots.push(Some(1)),
        }
        cache.entries.push(entry);
        assert!(
            ResolveContext {
                shared: &mut shared,
                cache: &mut cache,
                ledger: &mut ledger
            }
            .validate_cache(&mut Run)
            .is_err()
        );
    }
    let mut cache = PaintCache::default();
    cache.entries.push(cache_entry(&source));
    assert_eq!(
        ResolveContext {
            shared: &mut shared,
            cache: &mut cache,
            ledger: &mut ledger
        }
        .validate_cache(&mut StopAfter(1))
        .unwrap_err()
        .kind,
        ErrorKind::Canceled
    );
    assert_eq!(
        cache
            .storage_bytes_controlled(&mut StopAfter(0))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    cache.entries[0].stops.resize(
        257,
        PaintStop {
            offset: 1.,
            rgba: [0; 4],
        },
    );
    assert_eq!(
        cache.storage_bytes_controlled(&mut Run).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::PaintStops)
    );
}

#[test]
fn zero_edge_shapes_are_enumerable_without_an_edge_handle() {
    let mut b = builder();
    let root = b.root_id();
    let mut input = shape(owner().as_slice());
    input.path = vexel_svg_path::parse_path(
        owner().as_slice(),
        Span { start: 0, end: 0 },
        vexel_geometry::affine_identity(),
        Default::default(),
    )
    .unwrap();
    input.edge_colors.clear();
    let id = b.append_shape(root, input, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    assert!(graph.root().edges.is_empty());
    assert_eq!(graph.root().shape_id(0), Some(id));
    assert!(graph.root().edge_shape(0).is_none());
    let frame = FrameBuilder::new(graph.root(), 0)
        .unwrap()
        .finish(&mut Run)
        .unwrap();
    assert_eq!(frame.view().shape_id(0), Some(id));
}
