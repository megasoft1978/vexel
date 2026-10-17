//! WO-RUST-UI-TIMELINE-01 acceptance 4 and 5: scrubbing, keys and hit-testing.
//! Assertions are on driver and selection state, never on pixels.
#[path = "ui_timeline_support.rs"]
mod support;

use support::*;
use vexel_app::timeline::layout::{Hit, TimelineLayout};
use vexel_app::timeline::model::KeyRef;
use vexel_app::timeline::{Phase, key};
use vexel_platform_apple::Modifiers;

const NONE: Modifiers = Modifiers(0);
const FRAME: f64 = 1.0 / 24.0;

fn setup() -> (
    vexel_app::animation::driver::Driver<vexel_app::animation::playback::ManualClock>,
    vexel_app::timeline::Timeline,
    TimelineLayout,
) {
    let d = driver_with(LOOP);
    let tl = timeline_of(LOOP, &d);
    (d, tl, TimelineLayout::new(rect(54, 500, 900, 200)))
}

fn at(l: &TimelineLayout, t: f64, loop_s: f64) -> i32 {
    l.time_to_x(t, loop_s).round() as i32
}

#[test]
fn dragging_the_ruler_scrubs_clamped_and_release_keeps_the_time() {
    let (mut d, mut tl, l) = setup();
    d.play();
    let y = (l.ruler.y + 12) as i32;
    let out = tl.pointer(Phase::Down, (at(&l, 0.5, 1.0), y), &l, &mut d);
    assert!(out.consumed && out.redraw);
    assert!(!d.is_playing(), "scrubbing pauses");
    assert!((d.time() - 0.5).abs() < 0.02);
    tl.pointer(Phase::Move, (at(&l, 0.8, 1.0), y + 3), &l, &mut d);
    assert!((d.time() - 0.8).abs() < 0.02);
    // Far past the right edge and left of the lane: clamped to the loop.
    tl.pointer(Phase::Move, (5000, y), &l, &mut d);
    assert_eq!(d.time(), 1.0);
    tl.pointer(Phase::Move, (0, y), &l, &mut d);
    assert_eq!(d.time(), 0.0);
    tl.pointer(Phase::Move, (at(&l, 0.3, 1.0), y), &l, &mut d);
    tl.pointer(Phase::Up, (at(&l, 0.3, 1.0), y), &l, &mut d);
    let ended = d.time();
    assert!((ended - 0.3).abs() < 0.02);
    // After release the pointer no longer moves the time.
    tl.pointer(Phase::Move, (at(&l, 0.9, 1.0), y), &l, &mut d);
    assert_eq!(d.time(), ended);
    assert!(!d.is_playing());
}

#[test]
fn arrow_keys_step_one_24fps_frame_and_wrap_per_loop_mode() {
    let (mut d, _, _) = setup();
    assert!(key(124, NONE, &mut d));
    assert!((d.time() - FRAME).abs() < 1e-9);
    assert!(key(124, NONE, &mut d));
    assert!((d.time() - 2.0 * FRAME).abs() < 1e-9);
    assert!(key(123, NONE, &mut d));
    assert!((d.time() - FRAME).abs() < 1e-9);
    // Looping: back from frame 0 wraps to the last frame (23 of 24).
    d.seek(0.0);
    assert!(d.is_looping());
    key(123, NONE, &mut d);
    assert!((d.time() - 23.0 * FRAME).abs() < 1e-9);
    key(124, NONE, &mut d);
    assert_eq!(d.time(), 0.0);
    // Play once: clamps at both ends (the end frame is the loop end).
    d.toggle_looping();
    assert!(!d.is_looping());
    key(123, NONE, &mut d);
    assert_eq!(d.time(), 0.0);
    d.seek(1.0);
    key(124, NONE, &mut d);
    assert_eq!(d.time(), 1.0);
}

#[test]
fn space_home_end_and_l_use_the_driver_and_chords_are_ignored() {
    let (mut d, _, _) = setup();
    assert!(!d.is_playing());
    key(49, NONE, &mut d);
    assert!(d.is_playing());
    key(49, NONE, &mut d);
    assert!(!d.is_playing());
    d.seek(0.4);
    key(119, NONE, &mut d);
    assert_eq!(d.time(), 1.0);
    key(115, NONE, &mut d);
    assert_eq!(d.time(), 0.0);
    key(37, NONE, &mut d);
    assert!(!d.is_looping());
    assert!(!key(49, Modifiers(Modifiers::COMMAND), &mut d));
    assert!(!key(0, NONE, &mut d));
    // A static document ignores every playback key.
    let mut s = driver_with(STATIC);
    assert!(!key(49, NONE, &mut s) && !key(124, NONE, &mut s));
}

#[test]
fn transport_buttons_fire_on_release_over_the_pressed_button() {
    let (mut d, mut tl, l) = setup();
    for (hit, rc) in l.buttons() {
        assert!(rc.width >= 28 && rc.height >= 28, "{hit:?}");
    }
    let centre = |rc: vexel_ui::Rect| ((rc.x + rc.width / 2) as i32, (rc.y + rc.height / 2) as i32);
    let play = centre(l.play.unwrap());
    tl.pointer(Phase::Down, play, &l, &mut d);
    assert!(!d.is_playing(), "press alone does nothing");
    tl.pointer(Phase::Up, play, &l, &mut d);
    assert!(d.is_playing());
    // Press then release elsewhere cancels.
    tl.pointer(Phase::Down, play, &l, &mut d);
    tl.pointer(Phase::Up, centre(l.skip_end.unwrap()), &l, &mut d);
    assert!(d.is_playing());
    d.pause();
    let press = |tl: &mut vexel_app::timeline::Timeline, d: &mut _, p| {
        tl.pointer(Phase::Down, p, &l, d);
        tl.pointer(Phase::Up, p, &l, d);
    };
    press(&mut tl, &mut d, centre(l.skip_end.unwrap()));
    assert_eq!(d.time(), 1.0);
    press(&mut tl, &mut d, centre(l.skip_start.unwrap()));
    assert_eq!(d.time(), 0.0);
    press(&mut tl, &mut d, centre(l.step_forward.unwrap()));
    assert!((d.time() - FRAME).abs() < 1e-9);
    press(&mut tl, &mut d, centre(l.loop_btn.unwrap()));
    assert!(!d.is_looping());
    // Disabled controls are hit but inert, and carry a hint.
    let before = d.time();
    press(&mut tl, &mut d, centre(l.add_key.unwrap()));
    press(&mut tl, &mut d, centre(l.ease.unwrap()));
    assert_eq!(d.time(), before);
    assert!(Hit::AddKey.disabled_hint().is_some() && Hit::Ease.disabled_hint().is_some());
}

#[test]
fn clicking_a_diamond_selects_it_and_empty_lane_space_does_not() {
    let (mut d, mut tl, l) = setup();
    let rows = tl.rows();
    assert_eq!(rows.len(), 4, "two elements, one property each");
    // Row 1 is box.x with keys at 0, 0.5, 1.0.
    let row = l.row_rect(1);
    let y = (row.y + row.height / 2) as i32;
    let key_x = at(&l, 0.5, 1.0);
    let out = tl.pointer(Phase::Down, (key_x + 2, y), &l, &mut d);
    assert!(out.consumed);
    let want = KeyRef {
        element: 0,
        prop: Some(0),
        index: 1,
    };
    assert_eq!(tl.selection, Some(want));
    tl.pointer(Phase::Up, (key_x, y), &l, &mut d);
    let doc_time = d.time();
    assert_eq!(doc_time, 0.0, "selecting a key does not scrub");
    // Empty lane space between keys: scrubs, selection unchanged.
    tl.pointer(Phase::Down, (at(&l, 0.25, 1.0), y), &l, &mut d);
    tl.pointer(Phase::Up, (at(&l, 0.25, 1.0), y), &l, &mut d);
    assert_eq!(tl.selection, Some(want));
    assert!((d.time() - 0.25).abs() < 0.02);
    assert_eq!(
        l.hit_test(&tl.model, &rows, 0, at(&l, 0.25, 1.0), y),
        Hit::Lane
    );
    // The summary row selects with prop = None.
    let top = l.row_rect(0);
    tl.pointer(
        Phase::Down,
        (at(&l, 1.0, 1.0), (top.y + 10) as i32),
        &l,
        &mut d,
    );
    assert_eq!(
        tl.selection,
        Some(KeyRef {
            element: 0,
            prop: None,
            index: 2
        })
    );
}

#[test]
fn chevrons_collapse_rows_and_long_lists_scroll_in_whole_rows() {
    let d = driver_with(LONG);
    let mut tl = timeline_of(LONG, &d);
    let l = TimelineLayout::new(rect(54, 500, 900, 120));
    let mut d = d;
    let total = tl.rows().len();
    assert!(
        total > l.visible_rows(),
        "{total} rows vs {}",
        l.visible_rows()
    );
    assert!(tl.scroll_rows(100, &l));
    assert_eq!(tl.scroll, total - l.visible_rows());
    assert!(!tl.scroll_rows(1, &l), "already at the end");
    assert!(tl.scroll_rows(-100, &l) && tl.scroll == 0);
    // Pixel deltas accumulate to rows.
    assert!(!tl.scroll_by_points(-10.0, &l));
    assert!(tl.scroll_by_points(-20.0, &l));
    assert_eq!(tl.scroll, 1);
    tl.scroll_rows(-1, &l);
    // Collapse the first element through its chevron.
    let row = l.row_rect(0);
    let p = ((row.x + 14) as i32, (row.y + 14) as i32);
    tl.pointer(Phase::Down, p, &l, &mut d);
    assert_eq!(tl.collapsed[0], true);
    assert_eq!(tl.rows().len(), total - tl.model.elements[0].props.len());
    assert_eq!(tl.selection, None);
}
