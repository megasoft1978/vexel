//! WO-RUST-UI-MOTION-01 acceptance 5 and 6 through the loop adapter, with an
//! injected clock (headless, no window).
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;
use vexel_app::model::{Layout, Mode, Target, Tool};
use vexel_app::motion::{FOCUS_ORDER, MotionAdapter, MotionFrame, control_id, tool_id};
use vexel_platform_apple::PointerKind;
use vexel_ui::motion::Clock;

#[derive(Clone)]
struct TestClock(Rc<Cell<u64>>);
impl Clock for TestClock {
    fn now_ns(&self) -> u64 {
        self.0.get()
    }
}
fn setup(reduced: bool) -> (MotionAdapter<TestClock>, Rc<Cell<u64>>, Layout) {
    let time = Rc::new(Cell::new(1_000_000_000));
    let adapter = MotionAdapter::with_clock(TestClock(time.clone()), reduced);
    (adapter, time, Layout::new(1280, 820, Mode::Draw).unwrap())
}
fn center(layout: &Layout, tool: usize) -> (i32, i32) {
    let r = layout.tools[tool];
    ((r.x + r.width / 2) as i32, (r.y + r.height / 2) as i32)
}
const MS: u64 = 1_000_000;

#[test]
fn ac5_idle_requests_no_redraw_and_activity_requests_one() {
    let (mut m, time, layout) = setup(false);
    assert_eq!(m.redraw_after(), None);
    let p = center(&layout, 1);
    let (_, changed) = m.pointer(&layout, PointerKind::Move, p, true);
    assert!(changed);
    assert_eq!(m.redraw_after(), Some(Duration::from_millis(16)));
    // Same position again: nothing changes, nothing new to draw.
    assert!(!m.pointer(&layout, PointerKind::Move, p, true).1);
    time.set(time.get() + 121 * MS);
    assert_eq!(m.redraw_after(), None);
    // Leaving (90 ms) animates, then the adapter is idle again.
    m.pointer(&layout, PointerKind::Move, (-1, -1), true);
    assert!(m.redraw_after().is_some());
    time.set(time.get() + 91 * MS);
    assert_eq!(m.redraw_after(), None);
}

#[test]
fn ac4_reduced_motion_adapter_never_schedules() {
    let (mut m, _time, layout) = setup(true);
    m.pointer(&layout, PointerKind::Move, center(&layout, 0), true);
    m.pointer(&layout, PointerKind::Down, center(&layout, 0), true);
    assert_eq!(m.redraw_after(), None);
    let v = m.frame(true, Mode::Draw).get(tool_id(Tool::Select));
    assert_eq!((v.hover, v.press), (1.0, 1.0));
}

#[test]
fn ac6_press_is_set_and_action_fires_on_the_first_event() {
    let (mut m, _time, layout) = setup(false);
    let p = center(&layout, 2);
    // First event ever: visual progress is 0, the hit-test action is already there.
    let (hit, changed) = m.pointer(&layout, PointerKind::Down, p, true);
    assert_eq!(hit, Some(Target::Tool(Tool::Rectangle)));
    assert_eq!(hit, layout.hit_test((p.0 as u32, p.1 as u32)));
    assert!(changed);
    assert_eq!(
        m.registry().pressed(),
        Some(control_id(Target::Tool(Tool::Rectangle)))
    );
    let v = m.frame(true, Mode::Draw).get(tool_id(Tool::Rectangle));
    assert_eq!(v.press, 0.0, "visual has not started");
    // Secondary buttons neither press nor delay.
    let (_, c2) = m.pointer(&layout, PointerKind::Down, center(&layout, 3), false);
    assert!(!c2);
}

#[test]
fn press_release_hover_focus_cycle_and_frame_values() {
    let (mut m, time, layout) = setup(false);
    m.pointer(&layout, PointerKind::Down, center(&layout, 0), true);
    time.set(time.get() + 80 * MS);
    assert_eq!(
        m.frame(true, Mode::Draw).get(tool_id(Tool::Select)).press,
        1.0
    );
    m.pointer(&layout, PointerKind::Up, center(&layout, 0), true);
    time.set(time.get() + 130 * MS);
    assert_eq!(
        m.frame(true, Mode::Draw).get(tool_id(Tool::Select)).press,
        0.0
    );
    // Tab walks the whole focus order and wraps; a pointer press clears focus.
    for expect in FOCUS_ORDER.iter().chain(&FOCUS_ORDER[..1]) {
        assert!(m.tab(false));
        assert_eq!(m.registry().focused(), Some(*expect));
    }
    assert!(m.tab(true));
    assert_eq!(m.registry().focused(), Some(FOCUS_ORDER[6]));
    time.set(time.get() + 130 * MS);
    assert_eq!(m.frame(true, Mode::Draw).get(FOCUS_ORDER[6]).focus, 1.0);
    m.pointer(&layout, PointerKind::Down, (2000, 2000), true);
    assert_eq!(m.registry().focused(), None);
    // The rest frame is the static look.
    let rest = MotionFrame::rest(true, Mode::Draw);
    assert!(rest.get(control_id(Target::DrawMode)).is_rest());
}
