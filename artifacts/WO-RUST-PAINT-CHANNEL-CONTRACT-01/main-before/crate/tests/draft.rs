mod support;
use support::*;
use vexel_scene::*;
#[test]
fn deferred_attributes_colors_and_structural_metadata_finalize_without_base_mutation() {
    let mut b = builder();
    let root = b.root_id();
    let before = b
        .append_shape(root, shape(owner().as_slice()), &mut Run)
        .unwrap();
    let draft = b.begin_draft(root).unwrap();
    let id = b
        .append_shape(root, shape(owner().as_slice()), &mut Run)
        .unwrap();
    let mut s = *b.scene(root).unwrap().shape(id).unwrap();
    s.stroke.width = 7.;
    b.finalize_shape(&draft, id, s, &mut Run).unwrap();
    let n = s.edges.count;
    b.finalize_shape_colors(&draft, id, &vec![[9, 8, 7, 255]; n], &mut Run)
        .unwrap();
    let old = *b.scene(root).unwrap().shape(before).unwrap();
    assert!(b.finalize_shape(&draft, before, old, &mut Run).is_err());
    b.commit_draft(&draft, &mut Run).unwrap();
    assert!(b.commit_draft(&draft, &mut Run).is_err());
    let graph = b.freeze().unwrap();
    assert_eq!(graph.root().shape(id).unwrap().stroke.width, 7.);
    assert_eq!(graph.root().shape(before).unwrap().stroke.width, 1.);
    assert_eq!(graph.root().edge_colors[s.edges.first], [9, 8, 7, 255]);
}
#[test]
fn shape_finalization_rejects_range_source_foreign_and_canceled_updates_atomically() {
    let mut b = builder();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let id = b
        .append_shape(root, shape(owner().as_slice()), &mut Run)
        .unwrap();
    let old = *b.scene(root).unwrap().shape(id).unwrap();
    let mut wrong = old;
    wrong.edges.count += 1;
    assert!(b.finalize_shape(&draft, id, wrong, &mut Run).is_err());
    wrong = old;
    wrong.source.element.end += 1;
    assert!(b.finalize_shape(&draft, id, wrong, &mut Run).is_err());
    wrong = old;
    wrong.stroke.width = 2.;
    assert_eq!(
        b.finalize_shape(&draft, id, wrong, &mut StopAfter(0))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    let colors = b.scene(root).unwrap().edge_colors.to_vec();
    assert!(b
        .finalize_shape_colors(
            &draft,
            id,
            &vec![[1; 4]; old.edges.count],
            &mut StopAfter(1)
        )
        .is_err());
    assert_eq!(b.scene(root).unwrap().shape(id), Some(&old));
    assert_eq!(b.scene(root).unwrap().edge_colors, colors);
    let mut other = builder();
    let foreign = other
        .append_shape(other.root_id(), shape(owner().as_slice()), &mut Run)
        .unwrap();
    assert!(b.finalize_shape(&draft, foreign, wrong, &mut Run).is_err());
    b.rollback_draft(draft).unwrap();
    assert!(b.scene(root).unwrap().shapes.is_empty());
}
#[test]
fn ordinary_checkpoints_cannot_bypass_or_overlap_a_pending_draft() {
    let mut b = builder();
    let root = b.root_id();
    let earlier = b.mark(root).unwrap();
    let draft = b.begin_draft(root).unwrap();
    assert!(b.mark(root).is_err());
    assert!(b.rollback(earlier).is_err());
    assert!(b.begin_draft(root).is_err());
    assert!(b.seal(root, &mut Run).is_err());
    b.rollback_draft(draft).unwrap();
    assert!(b.mark(root).is_ok());
}
#[test]
fn freeze_rejects_lost_pending_draft_instead_of_publishing_it() {
    let mut b = builder();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    b.append_shape(root, shape(owner().as_slice()), &mut Run)
        .unwrap();
    drop(draft);
    assert_eq!(b.freeze().unwrap_err().kind, ErrorKind::InvalidArgument);
}
#[test]
fn tokens_reject_wrong_builder_wrong_top_and_stale_successful_commits() {
    let mut b = builder();
    let root = b.root_id();
    let parent = b.begin_draft(root).unwrap();
    let mut other = builder();
    assert!(other.commit_draft(&parent, &mut Run).is_err());
    let child = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(child).unwrap();
    assert!(b.commit_draft(&parent, &mut Run).is_err());
    assert!(b.append_transform(root, transform(None), &mut Run).is_err());
    b.commit_draft(&nested, &mut Run).unwrap();
    assert!(b.commit_draft(&nested, &mut Run).is_err());
    b.seal(child, &mut Run).unwrap();
    b.commit_draft(&parent, &mut Run).unwrap();
    assert!(b.freeze().is_ok());
}
#[test]
fn preexisting_unsealed_peer_cannot_be_modified_by_a_root_draft() {
    let mut b = builder();
    let root = b.root_id();
    let peer = b.new_scene(viewport()).unwrap();
    let draft = b.begin_draft(root).unwrap();
    assert!(b.begin_draft(peer).is_err());
    assert!(b.append_transform(peer, transform(None), &mut Run).is_err());
    assert!(b.seal(peer, &mut Run).is_err());
    b.rollback_draft(draft).unwrap();
    assert!(b.append_transform(peer, transform(None), &mut Run).is_ok());
}
#[test]
fn canceled_commit_and_failed_internal_append_leave_full_scope_rollbackable() {
    let mut b = builder();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    assert!(b
        .append_shape(root, shape(owner().as_slice()), &mut StopAfter(15))
        .is_err());
    let id = b
        .append_shape(root, shape(owner().as_slice()), &mut Run)
        .unwrap();
    let mut s = *b.scene(root).unwrap().shape(id).unwrap();
    s.stroke.width = 4.;
    b.finalize_shape(&draft, id, s, &mut Run).unwrap();
    assert_eq!(
        b.commit_draft(&draft, &mut StopAfter(0)).unwrap_err().kind,
        ErrorKind::Canceled
    );
    b.rollback_draft(draft).unwrap();
    assert!(b.scene(root).unwrap().shapes.is_empty());
    assert!(b.freeze().is_ok());
}
#[test]
fn parent_abort_drops_descendant_payloads_and_invalidates_child_handles() {
    let mut b = builder();
    let root = b.root_id();
    let before = b.memory_stats();
    let parent = b.begin_draft(root).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(child).unwrap();
    b.append_image(child, image(), &mut Run).unwrap();
    b.append_pattern(child, pattern(), &mut Run).unwrap();
    let removed = b
        .append_transform(child, transform(None), &mut Run)
        .unwrap();
    b.commit_draft(&nested, &mut Run).unwrap();
    b.seal(child, &mut Run).unwrap();
    b.append_mask(root, mask(child), &mut Run).unwrap();
    let full = b.memory_stats();
    b.rollback_draft(parent).unwrap();
    let after = b.memory_stats();
    assert!(after.live_bytes < full.live_bytes);
    assert_eq!(after.peak_live_bytes, full.peak_live_bytes);
    assert_eq!(after.total_allocated_bytes, full.total_allocated_bytes);
    assert!(after.live_bytes >= before.live_bytes);
    assert!(b.scene(child).is_err());
    let replacement = b.new_scene(viewport()).unwrap();
    assert_eq!(child.index(), replacement.index());
    assert_ne!(child, replacement);
    assert!(b.scene(replacement).unwrap().transform(removed).is_none());
    assert!(b.freeze().is_ok());
}
#[test]
fn all_deferred_definition_paint_filter_mask_clip_and_transform_records_finalize() {
    let mut b = builder();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(child).unwrap();
    b.commit_draft(&nested, &mut Run).unwrap();
    b.seal(child, &mut Run).unwrap();
    let t = b.append_transform(root, transform(None), &mut Run).unwrap();
    let mut tr = transform(None);
    tr.motion.e = 3.;
    b.finalize_transform(&draft, t, tr, &mut Run).unwrap();
    let c = b.append_clip(root, clip(), &mut Run).unwrap();
    let old_size = b.memory_stats().live_bytes;
    let mut cl = clip();
    cl.geometry.edges = Vec::with_capacity(17);
    cl.bounds[0] = 3.;
    b.finalize_clip(&draft, c, cl, &mut Run).unwrap();
    assert!(b.memory_stats().live_bytes > old_size);
    let m = b.append_mask(root, mask(child), &mut Run).unwrap();
    let mut mk = mask(child);
    mk.luminance = true;
    b.finalize_mask(&draft, m, mk, &mut Run).unwrap();
    let f = b.append_filter(root, filter(), &[], &mut Run).unwrap();
    b.finalize_filter(&draft, f, filter(), &[primitive()], &mut Run)
        .unwrap();
    let fill = b.append_fill_paint(root, paint(), &mut Run).unwrap();
    let stroke = b.append_stroke_paint(root, paint(), &mut Run).unwrap();
    let mut p = paint();
    p.coordinates[0] = 2.;
    b.finalize_fill_paint(&draft, fill, p, &mut Run).unwrap();
    b.finalize_stroke_paint(&draft, stroke, p, &mut Run)
        .unwrap();
    let g = b.append_group(root, group(), &mut Run).unwrap();
    let shape = b
        .append_shape(root, shape(owner().as_slice()), &mut Run)
        .unwrap();
    let mut gr = group();
    gr.shapes = TableRange { first: 0, count: 1 };
    gr.mask = Some(m);
    gr.filter = Some(f);
    b.finalize_group(&draft, g, gr, &mut Run).unwrap();
    let mut s = *b.scene(root).unwrap().shape(shape).unwrap();
    s.clip = Some(c);
    s.fill_paint = Some(fill);
    s.stroke_paint = Some(stroke);
    b.finalize_shape(&draft, shape, s, &mut Run).unwrap();
    b.commit_draft(&draft, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    let v = graph.root();
    assert_eq!(v.transform(t).unwrap().motion.e, 3.);
    assert_eq!(v.clip(c).unwrap().bounds[0], 3.);
    assert!(v.mask(m).unwrap().luminance);
    assert_eq!(v.filter(f).unwrap().primitives.count, 1);
    assert_eq!(v.group(g).unwrap().shapes.count, 1);
    assert_eq!(v.fill_paint(shape), Some(&p));
    assert_eq!(v.stroke_paint(shape), Some(&p));
}
#[test]
fn pre_scope_definition_records_cannot_be_changed_then_mistakenly_survive_rollback() {
    let mut b = builder();
    let root = b.root_id();
    let t = b.append_transform(root, transform(None), &mut Run).unwrap();
    let c = b.append_clip(root, clip(), &mut Run).unwrap();
    let f = b.append_filter(root, filter(), &[], &mut Run).unwrap();
    let g = b.append_group(root, group(), &mut Run).unwrap();
    let p = b.append_fill_paint(root, paint(), &mut Run).unwrap();
    let draft = b.begin_draft(root).unwrap();
    assert!(b
        .finalize_transform(&draft, t, transform(None), &mut Run)
        .is_err());
    assert!(b.finalize_clip(&draft, c, clip(), &mut Run).is_err());
    assert!(b
        .finalize_filter(&draft, f, filter(), &[primitive()], &mut Run)
        .is_err());
    assert!(b.finalize_group(&draft, g, group(), &mut Run).is_err());
    assert!(b.finalize_fill_paint(&draft, p, paint(), &mut Run).is_err());
    b.rollback_draft(draft).unwrap();
    assert!(b.scene(root).unwrap().transform(t).is_some());
    assert!(b.scene(root).unwrap().clip(c).is_some());
    assert!(b.freeze().is_ok());
}
#[test]
fn invalid_filter_and_clip_replacements_keep_records_and_payload_capacity() {
    let mut b = builder();
    let root = b.root_id();
    let draft = b.begin_draft(root).unwrap();
    let c = b.append_clip(root, clip(), &mut Run).unwrap();
    let f = b
        .append_filter(root, filter(), &[primitive()], &mut Run)
        .unwrap();
    let before = b.memory_stats();
    let mut bad = clip();
    bad.bounds[0] = f64::NAN;
    assert!(b.finalize_clip(&draft, c, bad, &mut Run).is_err());
    let mut bad = primitive();
    bad.input = FilterInput::Result(0);
    assert!(b
        .finalize_filter(&draft, f, filter(), &[bad], &mut Run)
        .is_err());
    assert_eq!(b.memory_stats().live_bytes, before.live_bytes);
    assert_eq!(
        b.scene(root).unwrap().filter(f).unwrap().primitives.count,
        1
    );
    assert!(b
        .finalize_filter(&draft, f, filter(), &[primitive()], &mut StopAfter(0))
        .is_err());
    b.rollback_draft(draft).unwrap();
    assert!(b.freeze().is_ok());
}
#[test]
fn nested_draft_depth_is_bounded_and_unwinds_in_lifo_order() {
    let mut b = builder();
    let root = b.root_id();
    let mut tokens = vec![b.begin_draft(root).unwrap()];
    for _ in 1..64 {
        let child = b.new_scene(viewport()).unwrap();
        tokens.push(b.begin_draft(child).unwrap());
    }
    let extra = b.new_scene(viewport()).unwrap();
    assert!(b.begin_draft(extra).is_err());
    while let Some(token) = tokens.pop() {
        b.rollback_draft(token).unwrap();
    }
    assert_eq!(b.freeze().unwrap().scene_count(), 1);
}

#[test]
fn draft_rollback_restores_ignored_counts_and_first_offsets() {
    let mut b = builder();
    b.record_ignored(IgnoredKind::Meta, 0).unwrap();
    let token = b.begin_draft(b.root_id()).unwrap();
    b.record_ignored(IgnoredKind::Meta, 1).unwrap();
    b.record_ignored(IgnoredKind::UnknownAttribute, 2).unwrap();
    b.rollback_draft(token).unwrap();
    let graph = b.freeze().unwrap();
    assert_eq!(
        graph.ignored(IgnoredKind::Meta),
        Ignored {
            count: 1,
            first_offset: Some(0)
        }
    );
    assert_eq!(
        graph.ignored(IgnoredKind::UnknownAttribute),
        Ignored::default()
    );
}

#[test]
fn nested_ignored_notices_follow_child_and_parent_commit_or_rollback() {
    let mut b = builder();
    let parent = b.begin_draft(b.root_id()).unwrap();
    b.record_ignored(IgnoredKind::Meta, 1).unwrap();
    let child = b.new_scene(viewport()).unwrap();
    let discarded = b.begin_draft(child).unwrap();
    b.record_ignored(IgnoredKind::Meta, 2).unwrap();
    b.record_ignored(IgnoredKind::WritingMode, 3).unwrap();
    assert!(b.commit_draft(&parent, &mut Run).is_err());
    b.rollback_draft(discarded).unwrap();
    assert_eq!(b.scene(child).unwrap().ignored(IgnoredKind::Meta).count, 1);
    assert_eq!(
        b.scene(child).unwrap().ignored(IgnoredKind::WritingMode),
        Ignored::default()
    );
    let retained = b.begin_draft(child).unwrap();
    b.record_ignored(IgnoredKind::Meta, 2).unwrap();
    b.record_ignored(IgnoredKind::WritingMode, 3).unwrap();
    b.commit_draft(&retained, &mut Run).unwrap();
    assert_eq!(b.scene(child).unwrap().ignored(IgnoredKind::Meta).count, 2);
    b.rollback_draft(parent).unwrap();
    assert_eq!(
        b.scene(b.root_id()).unwrap().ignored(IgnoredKind::Meta),
        Ignored::default()
    );
    assert_eq!(
        b.scene(b.root_id())
            .unwrap()
            .ignored(IgnoredKind::WritingMode),
        Ignored::default()
    );
    let token = b.begin_draft(b.root_id()).unwrap();
    b.record_ignored(IgnoredKind::Meta, 2).unwrap();
    b.commit_draft(&token, &mut Run).unwrap();
    assert_eq!(
        b.freeze().unwrap().ignored(IgnoredKind::Meta),
        Ignored {
            count: 1,
            first_offset: Some(2)
        }
    );
}
