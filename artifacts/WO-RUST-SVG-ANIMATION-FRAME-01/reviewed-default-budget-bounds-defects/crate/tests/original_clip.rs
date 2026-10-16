#[path = "support/mod.rs"]
mod common;
use common::{Control, clip_fixture, edges, fixture};
use std::sync::atomic::Ordering;
use vexel_scene::ErrorKind;

#[test]
fn original_clip_overlay_updates_real_owned_clip_and_rolls_back_to_base() {
    let fixture = fixture(
        "<animate attributeName='width' from='8' to='4' dur='2s' fill='freeze'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        true,
    );
    let clip = fixture.clip.unwrap();
    let base = fixture.graph.root();
    let original_edges = base.edges.to_vec();
    let original_clip_bounds = base.clip(clip).unwrap().bounds;
    let mut control = Control::new(u64::MAX);
    let frame = vexel_svg_animation_frame::evaluate_frame(base, 1.0, &mut control).unwrap();
    let updated = frame.view();
    let updated_edges = edges(&frame, fixture.shape);
    assert_eq!(updated_edges[0].a.x, 1.0);
    assert_eq!(updated_edges[1].a.x, 7.0);
    assert_eq!(updated.clips.len(), 1);
    assert_eq!(updated.clip(clip).unwrap().bounds, [1.0, 1.0, 7.0, 9.0]);
    assert_eq!(
        updated.clip(clip).unwrap().owner_node,
        base.clip(clip).unwrap().owner_node
    );
    assert_eq!(base.edges, original_edges);
    assert_eq!(base.clip(clip).unwrap().bounds, original_clip_bounds);
}

#[test]
fn configured_frame_budget_rejects_small_limit_and_allows_default_cap() {
    let fixture = fixture(
        "<animate attributeName='width' from='8' to='4' dur='2s' fill='freeze'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        true,
    );
    let base = fixture.graph.root();
    let original_edges = base.edges.to_vec();
    let clip = fixture.clip.unwrap();
    let original_bounds = base.clip(clip).unwrap().bounds;
    let mut control = Control::new(u64::MAX);
    assert_eq!(
        vexel_svg_animation_frame::evaluate_frame_with_budget(base, 1.0, 0, &mut control)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(vexel_scene::LimitKind::ReservedBytes)
    );
    assert_eq!(base.edges, original_edges);
    assert_eq!(base.clip(clip).unwrap().bounds, original_bounds);

    let mut control = Control::new(u64::MAX);
    assert_eq!(
        vexel_svg_animation_frame::evaluate_frame_with_budget(base, 1.0, usize::MAX, &mut control,)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(vexel_scene::LimitKind::ReservedBytes)
    );

    let mut control = Control::new(u64::MAX);
    let frame = vexel_svg_animation_frame::evaluate_frame_with_budget(
        base,
        1.0,
        vexel_svg_animation_frame::DEFAULT_FRAME_BYTE_BUDGET,
        &mut control,
    )
    .unwrap();
    assert!(
        frame.memory_stats().live_bytes <= vexel_svg_animation_frame::DEFAULT_FRAME_BYTE_BUDGET
    );
    assert_eq!(
        frame.view().clip(clip).unwrap().bounds,
        [1.0, 1.0, 7.0, 9.0]
    );
}

#[test]
fn clip_replacement_budget_counts_old_plus_new_capacity_overlap() {
    let fixture = fixture(
        "<animate attributeName='width' from='8' to='4' dur='2s' fill='freeze'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        true,
    );
    let base = fixture.graph.root();
    let shape = fixture.shape;
    let clip = fixture.clip.unwrap();
    let generous = vexel_svg_animation_frame::DEFAULT_FRAME_BYTE_BUDGET;
    let mut control = Control::new(u64::MAX);
    let measured =
        vexel_svg_animation_frame::evaluate_frame_with_budget(base, 1.0, generous, &mut control)
            .unwrap()
            .memory_stats()
            .live_bytes;

    let mut control = Control::new(u64::MAX);
    let mut constrained =
        vexel_svg_animation_frame::evaluate_frame_with_budget(base, 1.0, measured, &mut control)
            .unwrap();
    let before_stats = constrained.memory_stats();
    let before_clip = {
        let current = constrained.view().clip(clip).unwrap();
        (
            current.bounds,
            current.user_inverse,
            current.owner_node,
            current.geometry.edges.len(),
            current.geometry.shapes.len(),
        )
    };
    let mut control = Control::new(u64::MAX);
    assert_eq!(
        vexel_svg_animation_frame::update_animation_clips(
            &mut constrained,
            shape,
            true,
            &mut control,
        )
        .unwrap_err()
        .kind,
        ErrorKind::Limit(vexel_scene::LimitKind::ReservedBytes)
    );
    assert_eq!(constrained.memory_stats(), before_stats);
    let after_clip = constrained.view().clip(clip).unwrap();
    assert_eq!(after_clip.bounds, before_clip.0);
    assert_eq!(after_clip.user_inverse, before_clip.1);
    assert_eq!(after_clip.owner_node, before_clip.2);
    assert_eq!(after_clip.geometry.edges.len(), before_clip.3);
    assert_eq!(after_clip.geometry.shapes.len(), before_clip.4);
    assert_eq!(base.clip(clip).unwrap().bounds, [1.0, 1.0, 9.0, 9.0]);

    let mut control = Control::new(u64::MAX);
    let mut room =
        vexel_svg_animation_frame::evaluate_frame_with_budget(base, 1.0, generous, &mut control)
            .unwrap();
    let mut control = Control::new(u64::MAX);
    vexel_svg_animation_frame::update_animation_clips(&mut room, shape, true, &mut control)
        .unwrap();
    assert_eq!(room.view().clip(clip).unwrap().bounds, before_clip.0);
}

#[test]
fn actual_clip_operation_preserves_ancestor_before_suffix_and_full_owner_products() {
    let (graph, shape, parent, child_before, child_full) = clip_fixture();
    let base_view = graph.root();
    let alias = base_view.shape_id(1).unwrap();
    let zero_edge_shape = base_view.shape_id(2).unwrap();
    assert_eq!(base_view.shape(shape).unwrap().clip, Some(child_full));
    assert_eq!(base_view.shape(alias).unwrap().clip, Some(child_full));
    assert!(base_view.shape_id(2).is_some());
    assert_eq!(
        base_view
            .shape(base_view.shape_id(2).unwrap())
            .unwrap()
            .edges
            .count,
        0
    );
    let mut control = Control::new(u64::MAX);
    let mut frame =
        vexel_svg_animation_frame::evaluate_frame(graph.root(), 0.0, &mut control).unwrap();
    let base_inverse = frame.view().clip(child_full).unwrap().user_inverse;
    let mut control = Control::new(u64::MAX);
    vexel_svg_animation_frame::update_animation_clips(
        &mut frame,
        zero_edge_shape,
        true,
        &mut control,
    )
    .unwrap();
    let updated = frame.view();
    assert_eq!(updated.clips.len(), 3);
    assert!(frame.memory_stats().live_bytes > 0);
    assert!(frame.memory_stats().peak_live_bytes >= frame.memory_stats().live_bytes);
    assert_eq!(updated.clip(parent).unwrap().user_inverse.e, -5.0);
    assert_eq!(updated.clip(child_before).unwrap().user_inverse.e, -15.0);
    assert_eq!(updated.clip(child_full).unwrap().user_inverse.e, -19.0);
    assert_eq!(updated.clip(parent).unwrap().bounds, [7.0, 0.0, 9.0, 2.0]);
    assert_eq!(
        updated.clip(child_before).unwrap().bounds,
        [0.0, 0.0, 2.0, 2.0]
    );
    assert_eq!(
        updated.clip(child_full).unwrap().bounds,
        [0.0, 0.0, 2.0, 2.0]
    );
    assert_eq!(base_inverse.e, 0.0);
}

#[test]
fn clip_update_capacity_and_cancel_failures_leave_the_frame_unpublished() {
    let (graph, shape, _, _, child_full) = clip_fixture();
    let mut base_control = Control::new(u64::MAX);
    let initial =
        vexel_svg_animation_frame::evaluate_frame(graph.root(), 0.0, &mut base_control).unwrap();
    let original_stats = initial.memory_stats();
    let original_inverse = initial.view().clip(child_full).unwrap().user_inverse;
    let mut limited = initial;
    let mut control = Control::new(0);
    assert_eq!(
        vexel_svg_animation_frame::update_animation_clips(&mut limited, shape, true, &mut control,)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(vexel_scene::LimitKind::Work)
    );
    assert_eq!(limited.memory_stats().live_bytes, original_stats.live_bytes);
    assert_eq!(
        limited.view().clip(child_full).unwrap().user_inverse,
        original_inverse
    );

    let mut canceled =
        vexel_svg_animation_frame::evaluate_frame(graph.root(), 0.0, &mut Control::new(u64::MAX))
            .unwrap();
    let mut control = Control::new(u64::MAX);
    control.canceled.store(true, Ordering::Release);
    assert_eq!(
        vexel_svg_animation_frame::update_animation_clips(
            &mut canceled,
            shape,
            true,
            &mut control,
        )
        .unwrap_err()
        .kind,
        ErrorKind::Canceled
    );
    assert_eq!(
        canceled.view().clip(child_full).unwrap().user_inverse,
        original_inverse
    );
}

#[test]
fn clip_update_near_publication_work_limit_keeps_all_old_clip_values() {
    let (graph, shape, _, _, child_full) = clip_fixture();
    let mut success_frame =
        vexel_svg_animation_frame::evaluate_frame(graph.root(), 0.0, &mut Control::new(u64::MAX))
            .unwrap();
    let mut success_control = Control::new(u64::MAX);
    vexel_svg_animation_frame::update_animation_clips(
        &mut success_frame,
        shape,
        true,
        &mut success_control,
    )
    .unwrap();
    assert!(success_control.work > 1);

    let mut frame =
        vexel_svg_animation_frame::evaluate_frame(graph.root(), 0.0, &mut Control::new(u64::MAX))
            .unwrap();
    let before_stats = frame.memory_stats();
    let before_clips = frame.view().clips.len();
    let before_inverse = frame.view().clip(child_full).unwrap().user_inverse;
    let mut control = Control::new(success_control.work - 1);
    assert!(
        vexel_svg_animation_frame::update_animation_clips(&mut frame, shape, true, &mut control,)
            .is_err()
    );
    assert_eq!(frame.memory_stats().live_bytes, before_stats.live_bytes);
    assert_eq!(frame.view().clips.len(), before_clips);
    assert_eq!(
        frame.view().clip(child_full).unwrap().user_inverse,
        before_inverse
    );
}

#[test]
fn public_clip_update_can_refresh_inverse_without_replacing_object_bounds() {
    let (graph, shape, parent, child_before, child_full) = clip_fixture();
    let mut frame =
        vexel_svg_animation_frame::evaluate_frame(graph.root(), 0.0, &mut Control::new(u64::MAX))
            .unwrap();
    let original = frame
        .view()
        .clips
        .iter()
        .map(|clip| clip.bounds)
        .collect::<Vec<_>>();
    let mut control = Control::new(u64::MAX);
    vexel_svg_animation_frame::update_animation_clips(&mut frame, shape, false, &mut control)
        .unwrap();
    let updated = frame.view();
    assert_eq!(updated.clip(parent).unwrap().user_inverse.e, -5.0);
    assert_eq!(updated.clip(child_before).unwrap().user_inverse.e, -15.0);
    assert_eq!(updated.clip(child_full).unwrap().user_inverse.e, -19.0);
    assert_eq!(
        updated
            .clips
            .iter()
            .map(|clip| clip.bounds)
            .collect::<Vec<_>>(),
        original
    );
}

#[test]
fn limit_and_cancellation_do_not_publish_a_partial_real_frame() {
    let fixture = fixture(
        "<animate attributeName='x' from='1' to='5' dur='2s' fill='freeze'/>",
        "M1 1 L3 1 L3 5 L1 5 Z",
        true,
    );
    let mut limited = Control::new(0);
    assert_eq!(
        vexel_svg_animation_frame::evaluate_frame(fixture.graph.root(), 1.0, &mut limited)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(vexel_scene::LimitKind::Work)
    );
    let mut canceled = Control::new(u64::MAX);
    canceled.canceled.store(true, Ordering::Release);
    assert_eq!(
        vexel_svg_animation_frame::evaluate_frame(fixture.graph.root(), 1.0, &mut canceled)
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(
        fixture
            .graph
            .root()
            .clip(fixture.clip.unwrap())
            .unwrap()
            .bounds,
        [1.0, 1.0, 9.0, 9.0]
    );
}
