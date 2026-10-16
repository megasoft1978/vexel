#[path = "support/mod.rs"]
mod common;
use common::{Control, clip_fixture_with_suffix, fixture};
use vexel_scene::{Clip, ClipGeometry, Edge, ErrorKind, LimitKind, Stop, WorkControl, WorkKind};

fn capacity_clip(source: &Clip, capacity_bytes: usize) -> Clip {
    let mut edges = Vec::new();
    edges
        .try_reserve_exact(capacity_bytes / std::mem::size_of::<Edge>())
        .unwrap();
    edges.extend_from_slice(&source.geometry.edges);
    Clip {
        geometry: ClipGeometry {
            edges,
            shapes: source.geometry.shapes.clone(),
        },
        parent: source.parent,
        owner_node: source.owner_node,
        owner_before_suffix: source.owner_before_suffix,
        object_bounding_box: source.object_bounding_box,
        bounds: source.bounds,
        user_inverse: source.user_inverse,
    }
}
#[test]
fn normal_entrypoint_honors_policy_above_64mib_with_separate_clip_cap() {
    let fixture = fixture(
        "<animate attributeName='width' from='8' to='4' dur='2s' fill='freeze'/>",
        "M1 1 L9 1 L9 9 L1 9 Z",
        true,
    );
    let policy = 128 * 1024 * 1024;
    let mut frame = vexel_svg_animation_frame::evaluate_frame(
        fixture.graph.root(),
        1.0,
        policy,
        &mut Control::new(u64::MAX),
    )
    .unwrap();
    for _ in 0..2 {
        let replacement = vec![capacity_clip(&frame.view().clips[0], 33 * 1024 * 1024)];
        frame
            .replace_clips(replacement, &mut Control::new(u64::MAX))
            .unwrap();
    }
    assert!(frame.memory_stats().peak_live_bytes > 64 * 1024 * 1024);
    assert!(frame.memory_stats().peak_live_bytes <= policy);
    let before = frame.memory_stats();
    let bounds = frame.view().clips[0].bounds;
    let excessive_clip = vec![capacity_clip(&frame.view().clips[0], 65 * 1024 * 1024)];
    assert_eq!(
        frame
            .replace_clips(excessive_clip, &mut Control::new(u64::MAX))
            .unwrap_err()
            .kind,
        ErrorKind::Limit(LimitKind::Animation)
    );
    assert_eq!(frame.memory_stats(), before);
    assert_eq!(frame.view().clips[0].bounds, bounds);
    assert_eq!(fixture.graph.root().clips[0].bounds, [1.0, 1.0, 9.0, 9.0]);
}
struct DuringSuffix {
    saw_edge: bool,
    shapes_after_edge: usize,
    stop_after: Option<usize>,
}
impl WorkControl for DuringSuffix {
    fn charge(&mut self, kind: WorkKind, _: u64) -> Result<(), Stop> {
        if kind == WorkKind::ClipEdge {
            self.saw_edge = true;
        }
        if self.saw_edge && kind == WorkKind::ReferenceProbe {
            if self.stop_after == Some(self.shapes_after_edge) {
                return Err(Stop::Canceled);
            }
            self.shapes_after_edge += 1;
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}
#[test]
fn bounds_scan_observes_cancel_in_zero_node_zero_edge_suffix_atomically() {
    let (graph, shape, _, _, full) = clip_fixture_with_suffix(1024);
    let mut frame = common::evaluate_frame(graph.root(), 0.0, &mut Control::new(u64::MAX)).unwrap();
    let before = frame.memory_stats();
    let inverse = frame.view().clip(full).unwrap().user_inverse;
    let mut cancel = DuringSuffix {
        saw_edge: false,
        shapes_after_edge: 0,
        stop_after: Some(16),
    };
    assert_eq!(
        vexel_svg_animation_frame::update_animation_clips(&mut frame, shape, true, &mut cancel)
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(cancel.shapes_after_edge, 16);
    assert_eq!(frame.memory_stats(), before);
    assert_eq!(frame.view().clip(full).unwrap().user_inverse, inverse);
    assert_eq!(graph.root().clip(full).unwrap().user_inverse, inverse);
    let mut success = DuringSuffix {
        saw_edge: false,
        shapes_after_edge: 0,
        stop_after: None,
    };
    vexel_svg_animation_frame::update_animation_clips(&mut frame, shape, true, &mut success)
        .unwrap();
    assert!(success.shapes_after_edge >= 1024);
    assert_eq!(frame.view().clip(full).unwrap().user_inverse.e, -19.0);
    assert_eq!(
        frame.view().clip(full).unwrap().bounds,
        [0.0, 0.0, 2.0, 2.0]
    );
    assert_eq!(graph.root().clip(full).unwrap().user_inverse, inverse);
}
