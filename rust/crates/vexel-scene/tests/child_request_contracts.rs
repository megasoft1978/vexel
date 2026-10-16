mod support;
use std::sync::Arc;
use support::*;
use vexel_geometry::affine_identity as identity;
use vexel_scene::*;

fn mapped_doc() -> vexel_scene::LexicalDocument {
    vexel_scene::LexicalDocument::parse(
        Arc::new(
            br#"<svg width="100" height="100"><pattern/><path d="M0 0 L1 1 2 3 Z"/></svg>"#
                .to_vec(),
        ),
        1 << 20,
        &mut Run,
    )
    .unwrap()
}

fn add_shape(b: &mut SceneBuilder, scene: SceneId) -> ShapeId {
    let input = shape(b.source_owner());
    b.append_shape(scene, input, &mut Run).unwrap()
}

#[test]
fn populated_three_level_inner_and_outer_success_survives_freeze() {
    let mut b = builder();
    let root = b.root_id();
    let outer = b.begin_draft(b.root_id()).unwrap();
    let outer_request = b.begin_request(&outer, &mut Run).unwrap();
    let outer_shape = add_shape(&mut b, root);

    let scene1 = b.new_scene(viewport()).unwrap();
    let draft1 = b.begin_draft(scene1).unwrap();
    let request1 = b
        .begin_nested_request(&outer, outer_request, &draft1, &mut Run)
        .unwrap();
    let shape1 = add_shape(&mut b, scene1);
    let scene2 = b.new_scene(viewport()).unwrap();
    let draft2 = b.begin_draft(scene2).unwrap();
    let request2 = b
        .begin_nested_request(&draft1, request1, &draft2, &mut Run)
        .unwrap();
    let shape2 = add_shape(&mut b, scene2);
    let scene3 = b.new_scene(viewport()).unwrap();
    let draft3 = b.begin_draft(scene3).unwrap();
    let request3 = b
        .begin_nested_request(&draft2, request2, &draft3, &mut Run)
        .unwrap();
    let shape3 = add_shape(&mut b, scene3);

    b.commit_request(&draft3, request3, &mut Run).unwrap();
    b.commit_draft(&draft3, &mut Run).unwrap();
    b.commit_request(&draft2, request2, &mut Run).unwrap();
    b.commit_draft(&draft2, &mut Run).unwrap();
    b.commit_request(&draft1, request1, &mut Run).unwrap();
    b.commit_draft(&draft1, &mut Run).unwrap();
    b.commit_request(&outer, outer_request, &mut Run).unwrap();
    b.commit_draft(&outer, &mut Run).unwrap();

    let graph = b.freeze().unwrap();
    for (scene, id) in [
        (root, outer_shape),
        (scene1, shape1),
        (scene2, shape2),
        (scene3, shape3),
    ] {
        assert!(graph.scene(scene).unwrap().shape(id).is_some());
    }
}

#[test]
fn populated_inner_rollback_preserves_parent_prefix_and_allocation_history() {
    let mut b = builder();
    let root = b.root_id();
    let outer = b.begin_draft(root).unwrap();
    let parent_request = b.begin_request(&outer, &mut Run).unwrap();
    let parent_prefix = add_shape(&mut b, root);
    let parent_shape = *b.scene(root).unwrap().shape(parent_prefix).unwrap();
    let child_scene = b.new_scene(viewport()).unwrap();
    let child = b.begin_draft(child_scene).unwrap();
    let child_prefix = add_shape(&mut b, child_scene);
    let child_value = *b.scene(child_scene).unwrap().shape(child_prefix).unwrap();
    let child_request = b
        .begin_nested_request(&outer, parent_request, &child, &mut Run)
        .unwrap();
    let before_growth = b.memory_stats();
    assert!(b
        .finalize_shape(&outer, parent_prefix, parent_shape, &mut Run)
        .is_err());
    assert!(b
        .append_shape(root, shape(b.source_owner()), &mut Run)
        .is_err());
    let child_suffix = add_shape(&mut b, child_scene);
    let descendant_scene = b.new_scene(viewport()).unwrap();
    let descendant = b.begin_draft(descendant_scene).unwrap();
    add_shape(&mut b, descendant_scene);
    b.commit_draft(&descendant, &mut Run).unwrap();
    let during = b.memory_stats();
    b.rollback_request(&child, child_request).unwrap();
    let after = b.memory_stats();
    assert_eq!(
        b.scene(root).unwrap().shape(parent_prefix),
        Some(&parent_shape)
    );
    assert_eq!(
        b.scene(child_scene).unwrap().shape(child_prefix),
        Some(&child_value)
    );
    assert!(b.scene(child_scene).unwrap().shape(child_suffix).is_none());
    assert!(b.scene(descendant_scene).is_err());
    assert!(after.live_bytes < during.live_bytes);
    assert!(after.live_bytes >= before_growth.live_bytes); // retained spare capacity
    assert!(after.total_allocated_bytes >= during.total_allocated_bytes);
    assert!(after.allocation_count >= during.allocation_count);
    assert!(after.total_allocated_bytes > before_growth.total_allocated_bytes);
    b.rollback_draft(child).unwrap();
    let continued = add_shape(&mut b, root);
    assert!(b.scene(root).unwrap().shape(continued).is_some());
    b.commit_request(&outer, parent_request, &mut Run).unwrap();
    b.commit_draft(&outer, &mut Run).unwrap();
    let graph = b.freeze().unwrap();
    assert!(graph.scene(root).unwrap().shape(parent_prefix).is_some());
    assert!(graph.scene(root).unwrap().shape(continued).is_some());
}

#[test]
fn nested_request_mints_real_mapped_static_pattern_capability() {
    let doc = mapped_doc();
    let root_element = ElementId(0);
    let pattern_element = ElementId(
        doc.tokens()
            .iter()
            .position(|t| t.local_name.slice(doc.source()) == Some(b"pattern"))
            .unwrap(),
    );
    let path_start = doc.source().windows(5).position(|w| w == b"<path").unwrap();
    let path_end = doc.source()[path_start..]
        .windows(2)
        .position(|w| w == b"/>")
        .map(|n| path_start + n + 2)
        .unwrap();
    let mut b =
        SceneBuilder::provisional(doc.source_owner().clone(), 7, Limits::default()).unwrap();
    let mut source = DecodeSource::new(&doc, &mut Run).unwrap();
    b.initialize_root(&mut source, root_element, &mut Run)
        .unwrap();
    let root = b.root_id();
    let outer = b.begin_draft(root).unwrap();
    let outer_request = b.begin_request(&outer, &mut Run).unwrap();
    let nested_scene = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(nested_scene).unwrap();
    let nested_request = b
        .begin_nested_request(&outer, outer_request, &nested, &mut Run)
        .unwrap();
    let mut input = shape(doc.source());
    input.shape.source.element = Span {
        start: path_start,
        end: path_end,
    };
    let shape_id = b.append_shape(nested_scene, input, &mut Run).unwrap();
    let source_map = b
        .traversal_scene(nested_scene)
        .unwrap()
        .shape(shape_id)
        .unwrap()
        .source;
    let mut static_viewport = viewport();
    static_viewport.width = 1;
    static_viewport.height = 1;
    static_viewport.view_width = 1.;
    static_viewport.view_height = 1.;
    static_viewport.aspect.none = true;
    let static_scene = b.new_scene(static_viewport).unwrap();
    b.seal(static_scene, &mut Run).unwrap();
    let descriptor = PatternDescriptor {
        definition: pattern_element,
        width: 1,
        height: 1,
        tile_width: 1.,
        tile_height: 1.,
        content_transform: identity(),
        error_offset: 0,
    };
    let mapping = PatternMapping {
        tile_origin: [0.; 2],
        user_to_base: identity(),
        pattern_transform: identity(),
        user_inverse: identity(),
        viewport: [100.; 2],
        bounds: [0., 0., 1., 1.],
        source_map,
        opacity: 1.,
    };
    let job =
        RasterJob::new(ChargeLedger::new(doc.source_owner(), 1 << 20, 4096).unwrap()).unwrap();
    let mut tickets = PatternTickets::new().unwrap();
    let request = tickets
        .request_mapped_static(
            &b,
            &nested,
            &source,
            static_scene,
            descriptor,
            mapping,
            shape_id,
            PaintChannel::Fill,
            &job,
            &mut Run,
        )
        .unwrap();
    assert_eq!(request.descriptor(), descriptor);
    assert_eq!(
        request.child().input().static_request(),
        Some(nested_request)
    );
    drop(request);
    b.rollback_request(&nested, nested_request).unwrap();
    b.rollback_draft(nested).unwrap();
    b.rollback_request(&outer, outer_request).unwrap();
    b.rollback_draft(outer).unwrap();
}

#[test]
fn explicit_child_scope_preserves_parent_and_rolls_back_descendants() {
    let mut b = builder();
    let root = b.root_id();
    let outer = b.begin_draft(root).unwrap();
    let parent_prefix = b.new_scene(viewport()).unwrap();
    let parent = b.begin_request(&outer, &mut Run).unwrap();
    let child_scene = b.new_scene(viewport()).unwrap();
    let child = b.begin_draft(child_scene).unwrap();
    let child_request = b
        .begin_nested_request(&outer, parent, &child, &mut Run)
        .unwrap();
    assert!(b.begin_request(&child, &mut Run).is_err());
    assert!(b.commit_draft(&child, &mut Run).is_err());
    assert!(b.rollback_request(&outer, parent).is_err());
    b.commit_request(&child, child_request, &mut Run).unwrap();
    b.append_shape(child_scene, shape(owner().as_slice()), &mut Run)
        .unwrap();
    b.commit_draft(&child, &mut Run).unwrap();
    let nested_scene = b.new_scene(viewport()).unwrap();
    let nested = b.begin_draft(nested_scene).unwrap();
    let nested_request = b
        .begin_nested_request(&outer, parent, &nested, &mut Run)
        .unwrap();
    b.rollback_request(&nested, nested_request).unwrap();
    assert!(b.scene(nested_scene).is_ok());
    b.rollback_draft(nested).unwrap();
    assert!(b.scene(nested_scene).is_ok());
    assert!(b.scene(parent_prefix).is_ok());
    b.rollback_request(&outer, parent).unwrap();
    assert!(b.scene(child_scene).is_err());
    assert!(b.scene(parent_prefix).is_ok());
    b.rollback_draft(outer).unwrap();
}

#[test]
fn child_request_depth_is_bounded_and_cancellation_cleanup_is_unconditional() {
    let mut b = builder();
    let root = b.root_id();
    let outer = b.begin_draft(root).unwrap();
    let mut current_request = b.begin_request(&outer, &mut Run).unwrap();
    let mut drafts = Vec::new();
    let mut requests = Vec::new();
    for _ in 1..64 {
        let scene = b.new_scene(viewport()).unwrap();
        let draft = b.begin_draft(scene).unwrap();
        let parent = drafts.last().unwrap_or(&outer);
        let request = b
            .begin_nested_request(parent, current_request, &draft, &mut Run)
            .unwrap();
        current_request = request;
        drafts.push(draft);
        requests.push(request);
    }
    let overflow_scene = b.new_scene(viewport()).unwrap();
    assert!(b.begin_draft(overflow_scene).is_err());
    for (draft, request) in drafts.into_iter().zip(requests.into_iter()).rev() {
        b.rollback_request(&draft, request).unwrap();
        b.rollback_draft(draft).unwrap();
    }
    b.abort_request(&outer).unwrap();
    b.rollback_draft(outer).unwrap();
}

#[test]
fn canceled_top_scope_remains_unconditionally_rollbackable() {
    let mut b = builder();
    let root = b.root_id();
    let outer = b.begin_draft(root).unwrap();
    let parent = b.begin_request(&outer, &mut Run).unwrap();
    let child_scene = b.new_scene(viewport()).unwrap();
    let child = b.begin_draft(child_scene).unwrap();
    let child_request = b
        .begin_nested_request(&outer, parent, &child, &mut Run)
        .unwrap();
    assert_eq!(
        b.commit_request(&child, child_request, &mut StopAfter(0))
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    b.abort_request(&child).unwrap();
    b.rollback_draft(child).unwrap();
    b.abort_request(&outer).unwrap();
    b.rollback_draft(outer).unwrap();
}

#[test]
fn cancellation_abort_cleanup_succeeds_at_every_nested_scope_depth() {
    for canceled_depth in 0..4 {
        let mut b = builder();
        let outer = b.begin_draft(b.root_id()).unwrap();
        let outer_request = b.begin_request(&outer, &mut Run).unwrap();
        if canceled_depth == 0 {
            assert_eq!(
                b.commit_request(&outer, outer_request, &mut StopAfter(0))
                    .unwrap_err()
                    .kind,
                ErrorKind::Canceled
            );
            b.abort_request(&outer).unwrap();
            b.rollback_draft(outer).unwrap();
            continue;
        }
        let mut drafts = vec![outer];
        let mut requests = vec![outer_request];
        for depth in 1..4 {
            let scene = b.new_scene(viewport()).unwrap();
            let draft = b.begin_draft(scene).unwrap();
            let request = b
                .begin_nested_request(
                    drafts.last().unwrap(),
                    *requests.last().unwrap(),
                    &draft,
                    &mut Run,
                )
                .unwrap();
            if depth == canceled_depth {
                assert_eq!(
                    b.commit_request(&draft, request, &mut StopAfter(0))
                        .unwrap_err()
                        .kind,
                    ErrorKind::Canceled
                );
                b.abort_request(&draft).unwrap();
                b.rollback_draft(draft).unwrap();
                break;
            }
            drafts.push(draft);
            requests.push(request);
        }
        while let Some(draft) = drafts.pop() {
            b.abort_request(&draft).unwrap();
            b.rollback_draft(draft).unwrap();
            requests.pop();
        }
    }
}

#[test]
fn nested_tokens_reject_foreign_stale_replayed_and_out_of_order_use() {
    let mut b = builder();
    let mut foreign = builder();
    let foreign_draft = foreign.begin_draft(foreign.root_id()).unwrap();
    let outer = b.begin_draft(b.root_id()).unwrap();
    let parent = b.begin_request(&outer, &mut Run).unwrap();
    let scene = b.new_scene(viewport()).unwrap();
    let child = b.begin_draft(scene).unwrap();
    let foreign_parent = foreign.begin_request(&foreign_draft, &mut Run).unwrap();
    assert!(b
        .begin_nested_request(&child, parent, &child, &mut Run)
        .is_err()); // wrong parent draft
    assert!(b
        .begin_nested_request(&foreign_draft, foreign_parent, &child, &mut Run)
        .is_err()); // foreign parent
    assert!(b
        .begin_nested_request(&outer, foreign_parent, &child, &mut Run)
        .is_err()); // foreign request
    let sibling_scene = b.new_scene(viewport()).unwrap();
    let sibling = b.begin_draft(sibling_scene).unwrap();
    assert!(b
        .begin_nested_request(&outer, parent, &outer, &mut Run)
        .is_err()); // noncurrent child
    b.rollback_draft(sibling).unwrap();
    let inner = b
        .begin_nested_request(&outer, parent, &child, &mut Run)
        .unwrap();
    let grand_scene = b.new_scene(viewport()).unwrap();
    let grand = b.begin_draft(grand_scene).unwrap();
    assert!(b
        .begin_nested_request(&outer, parent, &grand, &mut Run)
        .is_err()); // out-of-order parent
    let grand_token = b
        .begin_nested_request(&child, inner, &grand, &mut Run)
        .unwrap();
    b.rollback_request(&grand, grand_token).unwrap();
    b.rollback_draft(grand).unwrap();
    assert!(b.rollback_request(&outer, parent).is_err());
    assert!(b.rollback_request(&child, foreign_parent).is_err());
    b.rollback_request(&child, inner).unwrap();
    assert!(b.rollback_request(&child, inner).is_err());
    b.rollback_draft(child).unwrap();
    // The consumed token is stale/replayed; rejection leaves the live parent usable.
    let replacement_scene = b.new_scene(viewport()).unwrap();
    let replacement = b.begin_draft(replacement_scene).unwrap();
    assert!(b
        .begin_nested_request(&outer, inner, &replacement, &mut Run)
        .is_err());
    let replacement_token = b
        .begin_nested_request(&outer, parent, &replacement, &mut Run)
        .unwrap();
    b.rollback_request(&replacement, replacement_token).unwrap();
    b.rollback_draft(replacement).unwrap();
    b.rollback_request(&outer, parent).unwrap();
    assert!(b.rollback_request(&outer, parent).is_err());
    b.rollback_draft(outer).unwrap();
    foreign.abort_request(&foreign_draft).unwrap();
    foreign.rollback_draft(foreign_draft).unwrap();
}

#[test]
fn matching_consumed_parent_pair_is_stale_but_live_parent_can_continue() {
    let mut b = builder();
    let root = b.root_id();
    let parent = b.begin_draft(root).unwrap();
    let parent_request = b.begin_request(&parent, &mut Run).unwrap();

    let consumed_scene = b.new_scene(viewport()).unwrap();
    let consumed_draft = b.begin_draft(consumed_scene).unwrap();
    let consumed_request = b
        .begin_nested_request(&parent, parent_request, &consumed_draft, &mut Run)
        .unwrap();
    b.commit_request(&consumed_draft, consumed_request, &mut Run)
        .unwrap();
    b.commit_draft(&consumed_draft, &mut Run).unwrap();

    let descendant_scene = b.new_scene(viewport()).unwrap();
    let descendant = b.begin_draft(descendant_scene).unwrap();
    assert!(b
        .begin_nested_request(&consumed_draft, consumed_request, &descendant, &mut Run)
        .is_err()); // matching pair, both consumed
    let live_request = b
        .begin_nested_request(&parent, parent_request, &descendant, &mut Run)
        .unwrap();
    b.commit_request(&descendant, live_request, &mut Run)
        .unwrap();
    b.commit_draft(&descendant, &mut Run).unwrap();

    b.commit_request(&parent, parent_request, &mut Run).unwrap();
    b.commit_draft(&parent, &mut Run).unwrap();
    b.freeze().unwrap();
}

#[test]
fn shape_admissions_follow_top_request_across_nested_rollback() {
    let mut b = builder();
    let root = b.root_id();
    let parent = b.begin_draft(root).unwrap();
    let parent_request = b.begin_request(&parent, &mut Run).unwrap();
    let parent_admission = b.admit_shape(&parent, parent_request, 3, &mut Run).unwrap();
    let child_scene = b.new_scene(viewport()).unwrap();
    let child = b.begin_draft(child_scene).unwrap();
    let child_request = b
        .begin_nested_request(&parent, parent_request, &child, &mut Run)
        .unwrap();

    assert!(b.admit_shape(&parent, parent_request, 4, &mut Run).is_err());
    let source = b.source_owner().clone();
    assert!(b
        .append_admitted_shape(&parent, &parent_admission, shape(&source), &mut Run)
        .is_err());
    assert!(b
        .cancel_shape_admission(&parent, &parent_admission)
        .is_err());

    let child_admission = b.admit_shape(&child, child_request, 5, &mut Run).unwrap();
    b.append_admitted_shape(&child, &child_admission, shape(&source), &mut Run)
        .unwrap();
    let stale_child = b.admit_shape(&child, child_request, 6, &mut Run).unwrap();
    b.rollback_request(&child, child_request).unwrap();
    assert!(b
        .append_admitted_shape(&child, &stale_child, shape(&source), &mut Run)
        .is_err());
    b.rollback_draft(child).unwrap();

    b.append_admitted_shape(&parent, &parent_admission, shape(&source), &mut Run)
        .unwrap();
    let parent_after = b.admit_shape(&parent, parent_request, 7, &mut Run).unwrap();
    b.cancel_shape_admission(&parent, &parent_after).unwrap();
    b.commit_request(&parent, parent_request, &mut Run).unwrap();
    b.commit_draft(&parent, &mut Run).unwrap();
    assert_eq!(b.freeze().unwrap().root().shapes.len(), 1);
}

#[test]
fn nested_text_charges_restore_on_rollback_and_follow_commit_lineage() {
    let mut b = builder();
    let root = b.root_id();
    let parent = b.begin_draft(root).unwrap();
    let parent_request = b.begin_request(&parent, &mut Run).unwrap();
    b.admit_text_commands(&parent, 7, 1, &mut Run).unwrap();
    let child_scene = b.new_scene(viewport()).unwrap();
    let child = b.begin_draft(child_scene).unwrap();
    let child_request = b
        .begin_nested_request(&parent, parent_request, &child, &mut Run)
        .unwrap();
    assert!(b.admit_text_commands(&parent, 1, 2, &mut Run).is_err());
    b.admit_text_commands(&child, 11, 4, &mut Run).unwrap();
    assert_eq!(b.document_text_commands(), 18);
    b.rollback_request(&child, child_request).unwrap();
    assert_eq!(b.document_text_commands(), 7);
    b.rollback_draft(child).unwrap();
    b.admit_text_commands(&parent, 5, 5, &mut Run).unwrap();
    assert_eq!(b.document_text_commands(), 12);

    let committed_scene = b.new_scene(viewport()).unwrap();
    let committed = b.begin_draft(committed_scene).unwrap();
    let committed_request = b
        .begin_nested_request(&parent, parent_request, &committed, &mut Run)
        .unwrap();
    b.admit_text_commands(&committed, 13, 6, &mut Run).unwrap();
    b.commit_request(&committed, committed_request, &mut Run)
        .unwrap();
    assert_eq!(b.document_text_commands(), 25);
    b.commit_draft(&committed, &mut Run).unwrap();
    b.admit_text_commands(&parent, 2, 7, &mut Run).unwrap();
    assert_eq!(b.document_text_commands(), 27);
    b.rollback_request(&parent, parent_request).unwrap();
    assert_eq!(b.document_text_commands(), 0);
    b.rollback_draft(parent).unwrap();
}

#[test]
fn mapped_static_ticket_is_bound_to_nested_top_request_and_parent_recovers() {
    let doc = mapped_doc();
    let pattern_element = ElementId(
        doc.tokens()
            .iter()
            .position(|t| t.local_name.slice(doc.source()) == Some(b"pattern"))
            .unwrap(),
    );
    let path_start = doc.source().windows(5).position(|w| w == b"<path").unwrap();
    let path_end = doc.source()[path_start..]
        .windows(2)
        .position(|w| w == b"/>")
        .map(|n| path_start + n + 2)
        .unwrap();
    let mut b =
        SceneBuilder::provisional(doc.source_owner().clone(), 7, Limits::default()).unwrap();
    let mut source = DecodeSource::new(&doc, &mut Run).unwrap();
    b.initialize_root(&mut source, ElementId(0), &mut Run)
        .unwrap();
    let root = b.root_id();
    let parent = b.begin_draft(root).unwrap();
    let parent_request = b.begin_request(&parent, &mut Run).unwrap();
    let mut tile_viewport = viewport();
    tile_viewport.width = 1;
    tile_viewport.height = 1;
    tile_viewport.view_width = 1.;
    tile_viewport.view_height = 1.;
    tile_viewport.aspect.none = true;
    let prefix_tile = b.new_scene(tile_viewport).unwrap();
    b.seal(prefix_tile, &mut Run).unwrap();
    let mut input = shape(doc.source());
    input.shape.source.element = Span {
        start: path_start,
        end: path_end,
    };
    let shape_id = b.append_shape(root, input, &mut Run).unwrap();

    let child_scene = b.new_scene(viewport()).unwrap();
    let child = b.begin_draft(child_scene).unwrap();
    let child_request = b
        .begin_nested_request(&parent, parent_request, &child, &mut Run)
        .unwrap();
    let mut child_input = shape(doc.source());
    child_input.shape.source.element = Span {
        start: path_start,
        end: path_end,
    };
    let child_shape = b.append_shape(child_scene, child_input, &mut Run).unwrap();
    let child_mapped = b
        .traversal_scene(child_scene)
        .unwrap()
        .shape(child_shape)
        .unwrap()
        .source;
    let tile = b.new_scene(tile_viewport).unwrap();
    b.seal(tile, &mut Run).unwrap();
    let descriptor = PatternDescriptor {
        definition: pattern_element,
        width: 1,
        height: 1,
        tile_width: 1.,
        tile_height: 1.,
        content_transform: identity(),
        error_offset: 0,
    };
    let mapping = PatternMapping {
        tile_origin: [0.; 2],
        user_to_base: identity(),
        pattern_transform: identity(),
        user_inverse: identity(),
        viewport: [100.; 2],
        bounds: [0., 0., 1., 1.],
        source_map: child_mapped,
        opacity: 1.,
    };
    let mut job =
        RasterJob::new(ChargeLedger::new(doc.source_owner(), 1 << 20, 4096).unwrap()).unwrap();
    let mut tickets = PatternTickets::new().unwrap();
    assert!(tickets
        .request_mapped_static(
            &b,
            &parent,
            &source,
            tile,
            descriptor,
            mapping,
            shape_id,
            PaintChannel::Fill,
            &job,
            &mut Run,
        )
        .is_err()); // parent draft is not the current nested draft
    assert!(tickets
        .request_mapped_static(
            &b,
            &child,
            &source,
            prefix_tile,
            descriptor,
            mapping,
            child_shape,
            PaintChannel::Fill,
            &job,
            &mut Run,
        )
        .is_err()); // valid child scope and mapping, but tile predates its request mark
    let request = tickets
        .request_mapped_static(
            &b,
            &child,
            &source,
            tile,
            descriptor,
            mapping,
            child_shape,
            PaintChannel::Fill,
            &job,
            &mut Run,
        )
        .unwrap();
    let stale_ticket = request.ticket();
    assert_eq!(
        request.child().input().static_request(),
        Some(child_request)
    );
    drop(request);
    b.commit_request(&child, child_request, &mut Run).unwrap();
    b.commit_draft(&child, &mut Run).unwrap();

    let stale_pixels = job.buffer(RasterPool::Output, 4, &mut Run).unwrap();
    job.extend(stale_pixels, &[1, 2, 3, 255], &mut Run).unwrap();
    let stale_output = JobRenderOutput {
        generation: 7,
        pixels: stale_pixels,
        width: 1,
        height: 1,
        stride: 4,
    };
    assert!(tickets
        .complete_mapped(
            stale_ticket,
            &mut b,
            &parent,
            &mut job,
            Ok(stale_output),
            &mut Run
        )
        .is_err());

    let parent_mapping = PatternMapping {
        source_map: b
            .traversal_scene(root)
            .unwrap()
            .shape(shape_id)
            .unwrap()
            .source,
        ..mapping
    };
    let parent_request_view = tickets
        .request_mapped_static(
            &b,
            &parent,
            &source,
            tile,
            descriptor,
            parent_mapping,
            shape_id,
            PaintChannel::Fill,
            &job,
            &mut Run,
        )
        .unwrap();
    let valid_ticket = parent_request_view.ticket();
    drop(parent_request_view);
    let pixels = job.buffer(RasterPool::Output, 4, &mut Run).unwrap();
    job.extend(pixels, &[4, 5, 6, 255], &mut Run).unwrap();
    let output = JobRenderOutput {
        generation: 7,
        pixels,
        width: 1,
        height: 1,
        stride: 4,
    };
    tickets
        .complete_mapped(
            valid_ticket,
            &mut b,
            &parent,
            &mut job,
            Ok(output),
            &mut Run,
        )
        .unwrap();
    assert_eq!(b.traversal_scene(root).unwrap().patterns.len(), 1);
    b.commit_request(&parent, parent_request, &mut Run).unwrap();
    b.commit_draft(&parent, &mut Run).unwrap();
}
