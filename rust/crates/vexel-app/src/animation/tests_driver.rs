//! Headless driver tests (WO-RUST-V1-PLAYBACK-01): ManualClock, offscreen
//! frames, no window.

use super::driver::{AnimFrame, Driver, FRAME_STEP, QUEUE_CAPACITY, TickOutcome};
use super::frame::{AnimatedScene, AnimationError, decode};
use super::playback::{ManualClock, Playback};
use crate::compose::{self, DocumentView, Frame};
use crate::model::{Layout, Mode, Model};
use crate::session::{AppDocument, PlaybackHost};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const NEVER: AtomicBool = AtomicBool::new(false);
const ANIM: &[u8] = b"<animate attributeName=\"x\" values=\"10;20\" dur=\"2s\"/>";

pub(super) fn fixture_path() -> PathBuf {
    PathBuf::from(format!(
        "{}/../../fixtures/v1/playback/loop2.svg",
        env!("CARGO_MANIFEST_DIR")
    ))
}

pub(super) fn animated_app() -> AppDocument {
    let mut app = AppDocument::new(10, 10).unwrap();
    app.open_path(&fixture_path()).unwrap();
    app
}

fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn layout() -> Layout {
    Layout::new(900, 600, Mode::Draw).unwrap()
}

fn stage() -> (u32, u32) {
    let l = layout();
    (l.stage.width, l.stage.height)
}

/// A view whose static compose of `app` at the stage size has finished.
fn ready_view(app: &AppDocument) -> DocumentView {
    let mut view = DocumentView::new();
    view.set_name("doc");
    view.show(&app.source_bytes().unwrap(), app.revision(), stage());
    let deadline = Instant::now() + Duration::from_secs(30);
    while view.composed().is_none() {
        assert!(Instant::now() < deadline, "compose timed out");
        view.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    view
}

/// Hash of the whole window frame after the compose blit.
fn screen_hash(view: &DocumentView) -> u64 {
    let layout = layout();
    let frame = crate::render::render(layout.width, layout.height, 1, &Model::new(true)).unwrap();
    hash(compose::blit_document(frame, view, &layout, true).pixels())
}

fn as_frame(tile: &vexel_raster::Tile, size: (u32, u32)) -> Frame {
    Frame {
        width: size.0,
        height: size.1,
        rgba: tile.rgba().to_vec(),
        disclosures: Vec::new(),
    }
}

/// Blit hash of `scene.render_sized(time)` presented on a fresh view of `app`.
fn reference_hash(app: &AppDocument, scene: &AnimatedScene, time: f64) -> u64 {
    let mut view = ready_view(app);
    let tile = scene.render_sized(time, stage(), &NEVER).unwrap();
    assert!(view.present_animated(app.revision(), as_frame(&tile, stage())));
    screen_hash(&view)
}

fn frame_of(outcome: TickOutcome) -> AnimFrame {
    match outcome {
        TickOutcome::Frame(frame) => frame,
        other => panic!("expected a frame, got {other:?}"),
    }
}

/// Presents `frame` on a fresh view of `app` and returns the blit hash.
fn presented_hash(app: &AppDocument, frame: AnimFrame) -> u64 {
    let mut view = ready_view(app);
    assert!(view.present_animated(frame.revision, frame.frame));
    screen_hash(&view)
}

fn new_driver() -> Driver<ManualClock> {
    let mut driver = Driver::new(ManualClock::default());
    driver.set_size(stage());
    driver
}

fn tick(driver: &mut Driver<ManualClock>, app: &AppDocument) -> TickOutcome {
    driver.tick(app.revision(), &app.source_bytes().unwrap())
}

// ---- AC1 --------------------------------------------------------------------

#[test]
fn tick_frames_equal_scene_render_through_the_compose_blit() {
    let app = animated_app();
    let scene = decode(&app.source_bytes().unwrap()).unwrap();
    assert!(scene.loop_seconds() >= 2.0);
    let mut driver = new_driver();
    let mut seen = Vec::new();
    let mut check = |driver: &mut Driver<ManualClock>, expect: f64| {
        let frame = frame_of(tick(driver, &app));
        assert!(
            (frame.time - expect).abs() < 1e-9,
            "{} vs {expect}",
            frame.time
        );
        let time = frame.time;
        let got = presented_hash(&app, frame);
        assert_eq!(got, reference_hash(&app, &scene, time), "t={expect}");
        seen.push(got);
    };
    check(&mut driver, 0.0);
    assert!(driver.play());
    driver.playback().clock().advance(0.5);
    check(&mut driver, 0.5);
    driver.playback().clock().advance(0.5);
    check(&mut driver, 1.0);
    driver.pause();
    assert!(driver.seek(2.0));
    check(&mut driver, 2.0); // the exact end frame
    assert!(driver.seek(1.9));
    assert!(driver.play());
    driver.playback().clock().advance(0.2);
    check(&mut driver, 0.1); // wrapped past the end
    assert!(seen[0] != seen[1] && seen[1] != seen[2] && seen[0] != seen[2]);
}

// ---- AC2 --------------------------------------------------------------------

#[test]
fn play_pause_seek_follow_the_clock_and_clamp() {
    let app = animated_app();
    let mut driver = new_driver();
    tick(&mut driver, &app);
    assert!(!driver.is_playing() && driver.time() == 0.0);
    assert!(driver.play());
    driver.playback().clock().advance(0.75);
    assert!((driver.time() - 0.75).abs() < 1e-9);
    driver.pause();
    driver.playback().clock().advance(5.0);
    assert!((driver.time() - 0.75).abs() < 1e-9, "paused time holds");
    assert!(driver.seek(99.0));
    assert_eq!(driver.time(), 2.0);
    assert!(driver.seek(-3.0));
    assert_eq!(driver.time(), 0.0);
    assert!(driver.seek(1.0));
    assert!(!driver.seek(f64::NAN) && !driver.seek(f64::INFINITY));
    assert_eq!(driver.time(), 1.0);
    // Seeking keeps the play state.
    assert!(driver.play());
    assert!(driver.seek(0.5));
    assert!(driver.is_playing());
    driver.playback().clock().advance(0.25);
    assert!((driver.time() - 0.75).abs() < 1e-9);
}

#[test]
fn play_on_a_static_document_is_a_no_op() {
    let mut app = AppDocument::new(160, 120).unwrap();
    app.edit(|d| d.add_rectangle_with_id(10.0, 10.0, 50.0, 50.0, "#ff0000", "r1"))
        .unwrap();
    let mut driver = new_driver();
    assert_eq!(tick(&mut driver, &app), TickOutcome::NoAnimation);
    assert!(!driver.play());
    assert!(!driver.is_playing() && !driver.strip_visible());
    assert!(!driver.toggle() && !driver.step(true));
}

#[test]
fn loop_wraps_and_play_once_stops_at_the_end_frame() {
    let app = animated_app();
    let mut driver = new_driver();
    tick(&mut driver, &app);
    assert!(driver.is_looping() && driver.play());
    driver.playback().clock().advance(2.5);
    assert!((driver.time() - 0.5).abs() < 1e-9);
    assert!(driver.is_playing());

    driver.toggle_looping();
    assert!(!driver.is_looping());
    driver.playback().clock().advance(5.0);
    let end = frame_of(tick(&mut driver, &app));
    assert_eq!(end.time, 2.0, "play-once stops on the end frame");
    assert!(!driver.is_playing());
    assert_eq!(tick(&mut driver, &app), TickOutcome::Unchanged);
    // Playing again restarts from the beginning.
    assert!(driver.play());
    assert_eq!(driver.time(), 0.0);
}

// ---- AC3 --------------------------------------------------------------------

#[test]
fn pacing_skips_sub_frame_advances_and_never_backlogs() {
    let app = animated_app();
    let mut driver = new_driver();
    frame_of(tick(&mut driver, &app));
    assert!(driver.play());
    driver.playback().clock().advance(FRAME_STEP * 0.4);
    assert_eq!(tick(&mut driver, &app), TickOutcome::Unchanged);
    driver.playback().clock().advance(FRAME_STEP);
    let next = frame_of(tick(&mut driver, &app));
    assert!((next.time - FRAME_STEP * 1.4).abs() < 1e-9);
    assert_eq!(driver.queue_len(), 0);
    // A ~10 s stall (not a whole number of loops) yields one frame at the
    // current time, not one per missed step.
    driver.playback().clock().advance(10.25);
    let stalled = frame_of(tick(&mut driver, &app));
    assert!((stalled.time - driver.time()).abs() < 1e-9);
    assert_eq!(tick(&mut driver, &app), TickOutcome::Unchanged);
    assert_eq!(driver.queue_len(), 0);
}

#[test]
fn the_queue_never_exceeds_its_capacity() {
    let app = animated_app();
    let source = app.source_bytes().unwrap();
    let mut driver = new_driver();
    for i in 0..10 {
        driver.request_frame(&source, app.revision(), f64::from(i) * 0.1);
        assert!(driver.queue_len() <= QUEUE_CAPACITY);
    }
    assert_eq!(driver.queue_len(), QUEUE_CAPACITY);
    // One tick returns only the newest and empties the queue.
    driver.seek(1.9);
    let frame = frame_of(driver.tick(app.revision(), &source));
    assert!((frame.time - 1.9).abs() < 1e-9, "{}", frame.time);
    assert_eq!(driver.queue_len(), 0);
}

// ---- AC4 --------------------------------------------------------------------

#[test]
fn an_edit_cancels_old_work_and_renders_the_edited_document() {
    let mut app = animated_app();
    let mut driver = new_driver();
    frame_of(tick(&mut driver, &app));
    assert!(driver.play());
    driver.playback().clock().advance(0.5);
    let before = frame_of(tick(&mut driver, &app));
    let old_revision = app.revision();
    // A queued frame of the old revision must not survive the edit.
    driver.request_frame(&app.source_bytes().unwrap(), old_revision, 0.2);
    assert_eq!(driver.queue_len(), 1);
    app.edit(|d| d.add_rectangle_with_id(100.0, 80.0, 140.0, 110.0, "#0000ff", "extra"))
        .unwrap();
    assert!(app.revision() > old_revision);
    let after = frame_of(tick(&mut driver, &app));
    assert_eq!(after.revision, app.revision());
    assert_eq!(driver.queue_len(), 0);
    assert_ne!(after.revision, before.revision);
    // Same clock time, edited document.
    assert!((after.time - before.time).abs() < 1e-9);
    let scene = decode(&app.source_bytes().unwrap()).unwrap();
    let expected = scene.render_sized(after.time, stage(), &NEVER).unwrap();
    assert_eq!(after.frame.rgba, expected.rgba());
    assert_ne!(after.frame.rgba, before.frame.rgba);
    // Nothing of the old revision comes back.
    driver.playback().clock().advance(0.2);
    assert_eq!(frame_of(tick(&mut driver, &app)).revision, app.revision());
}

#[test]
fn a_pre_set_cancel_flag_presents_nothing() {
    let app = animated_app();
    let scene = decode(&app.source_bytes().unwrap()).unwrap();
    let flag = AtomicBool::new(true);
    assert_eq!(
        scene.render(0.5, &flag).unwrap_err(),
        AnimationError::Cancelled
    );
    assert!(scene.render_sized(0.5, stage(), &flag).is_err());

    let mut driver = new_driver();
    frame_of(tick(&mut driver, &app));
    driver.seek(1.0);
    driver.cancel_flag().store(true, Ordering::SeqCst);
    assert_eq!(tick(&mut driver, &app), TickOutcome::Cancelled);
    assert_eq!(driver.queue_len(), 0);
    // The flag is consumed; the next tick renders again.
    assert_eq!(frame_of(tick(&mut driver, &app)).time, 1.0);
}

// ---- AC5 --------------------------------------------------------------------

#[test]
fn a_vanishing_loop_pauses_at_zero_and_hides_the_strip() {
    let mut app = AppDocument::new(160, 120).unwrap();
    app.edit(|d| d.add_rectangle_with_id(10.0, 10.0, 50.0, 50.0, "#ff0000", "r1"))
        .unwrap();
    let mut driver = new_driver();
    assert_eq!(tick(&mut driver, &app), TickOutcome::NoAnimation);
    assert!(!driver.strip_visible());
    app.edit(|d| d.add_animation_fragment("r1", ANIM)).unwrap();
    frame_of(tick(&mut driver, &app));
    assert!(driver.strip_visible());
    assert!(driver.seek(1.0) && driver.play());
    app.undo().unwrap();
    assert_eq!(tick(&mut driver, &app), TickOutcome::NoAnimation);
    assert!(!driver.strip_visible() && !driver.is_playing());
    assert_eq!(driver.time(), 0.0);
    app.redo().unwrap();
    frame_of(tick(&mut driver, &app));
    assert!(driver.strip_visible());
    assert!(!driver.is_playing() && driver.time() == 0.0);
}

#[test]
fn a_shorter_loop_clamps_the_time() {
    let mut p = Playback::new(ManualClock::default(), 2.0);
    assert!(p.seek(1.8));
    p.set_duration(1.0);
    assert_eq!(p.time(), 1.0);
    assert!(p.play());
    p.set_duration(0.0);
    assert!(!p.is_playing() && p.time() == 0.0);
}
