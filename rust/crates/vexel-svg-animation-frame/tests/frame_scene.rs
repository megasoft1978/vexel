use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use vexel_scene::{ErrorKind, Limits, SceneBuilder, Stop, Viewport, WorkControl, WorkKind};
use vexel_svg_animation::parse_owned;

struct Control {
    canceled: AtomicBool,
    work: u64,
    limit: u64,
}
impl Control {
    fn new(limit: u64) -> Self {
        Self {
            canceled: AtomicBool::new(false),
            work: 0,
            limit,
        }
    }
}
impl WorkControl for Control {
    fn charge(&mut self, _: WorkKind, units: u64) -> Result<(), Stop> {
        if self.canceled.load(Ordering::Acquire) {
            return Err(Stop::Canceled);
        }
        let next = self.work.checked_add(units).ok_or(Stop::Overflow)?;
        if next > self.limit {
            return Err(Stop::Limit);
        }
        self.work = next;
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        if self.canceled.load(Ordering::Acquire) {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}

fn graph() -> vexel_scene::SceneGraph {
    let source = Arc::new(b"<svg/>".to_vec());
    let animation = parse_owned(&source, &[]).unwrap();
    let viewport = Viewport {
        width: 4,
        height: 4,
        view_x: 0.0,
        view_y: 0.0,
        view_width: 4.0,
        view_height: 4.0,
        aspect: Default::default(),
    };
    SceneBuilder::new(source, animation, 1, viewport, Limits::default())
        .unwrap()
        .freeze()
        .unwrap()
}

#[test]
fn empty_owned_scene_evaluates_without_copying_or_losing_child_lookup() {
    let graph = graph();
    let mut control = Control::new(u64::MAX);
    let frame = vexel_svg_animation_frame::evaluate_frame(
        graph.root(),
        0.0,
        128 * 1024 * 1024,
        &mut control,
    )
    .unwrap();
    let view = frame.view();
    assert!(view.shapes.is_empty());
    assert!(view.scene(view.id).is_some());
    assert_eq!(frame.memory_stats().live_bytes, 0);
}

#[test]
fn invalid_time_and_cancellation_are_reported_without_a_frame() {
    let graph = graph();
    let mut control = Control::new(u64::MAX);
    assert_eq!(
        vexel_svg_animation_frame::evaluate_frame(
            graph.root(),
            f64::NAN,
            128 * 1024 * 1024,
            &mut control
        )
        .unwrap_err()
        .kind,
        ErrorKind::InvalidArgument,
    );
    control.canceled.store(true, Ordering::Release);
    assert_eq!(
        vexel_svg_animation_frame::evaluate_frame(
            graph.root(),
            0.0,
            128 * 1024 * 1024,
            &mut control
        )
        .unwrap_err()
        .kind,
        ErrorKind::Canceled,
    );
}
