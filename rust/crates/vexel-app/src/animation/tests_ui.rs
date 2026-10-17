//! Keyboard map and timeline strip tests (WO-RUST-V1-PLAYBACK-01 AC6, AC8).

use super::driver::{Driver, FRAME_STEP};
use super::keys::{PlaybackKey, apply, playback_key};
use super::playback::ManualClock;
use super::strip::{
    STRIP_HEIGHT, Strip, StripHit, StripLayout, StripPointer, format_time, hit, label, layout,
};
use super::tests_driver::animated_app;
use crate::model::{Layout, Mode};
use crate::tools::{PointerInput, ScaleMap, ToolController};
use vexel_platform_apple::Modifiers;
use vexel_ui::Rect;

const NONE: Modifiers = Modifiers(0);

fn loaded() -> Driver<ManualClock> {
    let app = animated_app();
    let mut driver = Driver::new(ManualClock::default());
    assert!(driver.sync(app.revision(), &app.source_bytes().unwrap()));
    driver
}

fn stage() -> Rect {
    Layout::new(900, 600, Mode::Draw).unwrap().stage
}

fn bar() -> StripLayout {
    layout(stage()).unwrap()
}

fn centre(r: Rect) -> (i32, i32) {
    ((r.x + r.width / 2) as i32, (r.y + r.height / 2) as i32)
}

// ---- AC6: keyboard ----------------------------------------------------------

#[test]
fn the_playback_key_map_is_exact() {
    assert_eq!(playback_key(49, NONE), Some(PlaybackKey::Toggle));
    assert_eq!(playback_key(115, NONE), Some(PlaybackKey::Home));
    assert_eq!(playback_key(123, NONE), Some(PlaybackKey::StepBack));
    assert_eq!(playback_key(124, NONE), Some(PlaybackKey::StepForward));
    assert_eq!(playback_key(37, NONE), Some(PlaybackKey::ToggleLoop));
}

#[test]
fn playback_keys_do_not_collide_with_tools_modes_or_chords() {
    // Escape, Delete (both), Return (both), D and A (modes).
    for code in [53, 51, 117, 36, 76, 2, 0] {
        assert_eq!(playback_key(code, NONE), None, "code {code}");
    }
    for modifiers in [
        Modifiers::COMMAND,
        Modifiers::CONTROL,
        Modifiers::OPTION,
        Modifiers::SHIFT,
    ] {
        for code in [49, 115, 123, 124, 37] {
            assert_eq!(playback_key(code, Modifiers(modifiers)), None);
        }
    }
    // Cmd+Z and Cmd+S keep their meaning.
    let cmd = Modifiers(Modifiers::COMMAND);
    assert!(crate::session::edit_shortcut(6, cmd).is_some());
    assert_eq!(playback_key(6, cmd), None);
    assert_eq!(playback_key(1, cmd), None);
    assert!(crate::project_session::shortcut_action(1, cmd).is_some());
}

#[test]
fn keys_drive_the_playback_state() {
    let mut driver = loaded();
    assert!(apply(&mut driver, PlaybackKey::Toggle, false, false));
    assert!(driver.is_playing());
    // Stepping while playing pauses first, then steps one frame.
    assert!(apply(&mut driver, PlaybackKey::StepForward, false, false));
    assert!(!driver.is_playing());
    assert!((driver.time() - FRAME_STEP).abs() < 1e-9);
    assert!(driver.seek(0.5));
    assert!(apply(&mut driver, PlaybackKey::StepForward, false, false));
    assert!((driver.time() - (0.5 + FRAME_STEP)).abs() < 1e-9);
    assert!(apply(&mut driver, PlaybackKey::StepBack, false, false));
    assert!(apply(&mut driver, PlaybackKey::StepBack, false, false));
    assert!((driver.time() - (0.5 - FRAME_STEP)).abs() < 1e-9);
    assert!(apply(&mut driver, PlaybackKey::Home, false, false));
    assert_eq!(driver.time(), 0.0);
    // Looping: stepping back at 0 wraps to the last frame of the 2 s loop.
    assert!(driver.is_looping());
    assert!(apply(&mut driver, PlaybackKey::StepBack, false, false));
    assert!((driver.time() - 59.0 * FRAME_STEP).abs() < 1e-9);
    assert!(driver.seek(0.0));
    assert!(apply(&mut driver, PlaybackKey::ToggleLoop, false, false));
    assert!(!driver.is_looping());
    // Home keeps the play state.
    assert!(apply(&mut driver, PlaybackKey::Toggle, false, false));
    driver.playback().clock().advance(0.3);
    assert!(apply(&mut driver, PlaybackKey::Home, false, false));
    assert!(driver.is_playing() && driver.time() == 0.0);
}

#[test]
fn keys_are_ignored_for_text_fields_tools_and_static_documents() {
    let mut driver = loaded();
    for key in [
        PlaybackKey::Toggle,
        PlaybackKey::Home,
        PlaybackKey::StepForward,
        PlaybackKey::ToggleLoop,
    ] {
        assert!(!apply(&mut driver, key, true, false), "text field owns");
        assert!(!apply(&mut driver, key, false, true), "tool owns");
    }
    assert!(!driver.is_playing() && driver.time() == 0.0 && driver.is_looping());
    let mut empty = Driver::new(ManualClock::default());
    assert!(!apply(&mut empty, PlaybackKey::Toggle, false, false));
}

#[test]
fn a_tool_gesture_owns_the_keys_until_it_ends() {
    let mut tools = ToolController::new();
    assert!(!tools.owns_keys());
    let mut doc = vexel_v1_document::V1Document::new(160, 120).unwrap();
    let map = ScaleMap::new((0.0, 0.0), 1.0, (160.0, 120.0)).unwrap();
    tools.set_tool(crate::model::Tool::Rectangle);
    tools
        .pointer(PointerInput::Down { x: 10.0, y: 10.0 }, &mut doc, &map)
        .unwrap();
    assert!(tools.owns_keys());
    tools
        .pointer(PointerInput::Up { x: 60.0, y: 50.0 }, &mut doc, &map)
        .unwrap();
    assert!(!tools.owns_keys());
}

// ---- AC8: strip -------------------------------------------------------------

#[test]
fn strip_layout_sits_at_the_stage_bottom_with_disjoint_controls() {
    let (stage, bar) = (stage(), bar());
    assert_eq!(bar.bar.height, STRIP_HEIGHT);
    // The bar floats 8 px inside the stage and 8 px above the zoom cluster.
    let cluster = crate::canvas::controls(stage.width, stage.height, 100).unwrap();
    assert_eq!(
        bar.bar.y + bar.bar.height + 8,
        stage.y + cluster.container.y
    );
    assert_eq!((bar.bar.x, bar.bar.width), (stage.x + 8, stage.width - 16));
    assert!(bar.button.x + bar.button.width <= bar.scrub.x);
    assert!(bar.scrub.x + bar.scrub.width <= bar.label.x);
    assert!(bar.label.x + bar.label.width <= bar.bar.x + bar.bar.width);
    assert!(
        layout(Rect {
            x: 0,
            y: 0,
            width: 200,
            height: 300
        })
        .is_none()
    );
    assert!(
        layout(Rect {
            x: 0,
            y: 0,
            width: 800,
            height: 60
        })
        .is_none()
    );
}

#[test]
fn strip_hit_testing_separates_button_scrub_and_bar() {
    let bar = bar();
    let (bx, by) = centre(bar.button);
    let (sx, sy) = centre(bar.scrub);
    assert_eq!(hit(&bar, bx, by), Some(StripHit::Button));
    assert_eq!(hit(&bar, sx, sy), Some(StripHit::Scrub));
    assert_eq!(hit(&bar, bar.bar.x as i32 + 1, by), Some(StripHit::Bar));
    assert_eq!(hit(&bar, sx, bar.bar.y as i32 - 1), None);
    assert_eq!(hit(&bar, -5, -5), None);
}

#[test]
fn time_label_format_is_minutes_seconds_centiseconds() {
    assert_eq!(format_time(0.0), "0:00.00");
    assert_eq!(format_time(61.5), "1:01.50");
    assert_eq!(label(1.5, 2.0), "0:01.50 / 0:02.00");
    assert_eq!(label(0.014, 10.0), "0:00.01 / 0:10.00");
    assert_eq!(format_time(-1.0), "0:00.00");
}

#[test]
fn scrub_maps_pointer_x_to_clamped_seconds() {
    let (bar, mut driver, mut strip) = (bar(), loaded(), Strip::default());
    let y = centre(bar.scrub).1;
    let left = bar.scrub.x as i32;
    let right = (bar.scrub.x + bar.scrub.width) as i32;
    assert!(!strip.pointer(StripPointer::Down, (left - 100, y), &bar, &mut driver));
    // Far left and far right of the bar clamp to 0 and the loop end.
    assert!(strip.pointer(StripPointer::Down, (left, y), &bar, &mut driver));
    assert_eq!(driver.time(), 0.0);
    assert!(strip.pointer(StripPointer::Move, (right + 500, y), &bar, &mut driver));
    assert_eq!(driver.time(), 2.0);
    assert!(strip.pointer(StripPointer::Move, (left - 500, y), &bar, &mut driver));
    assert_eq!(driver.time(), 0.0);
    // Middle of the usable track is half the loop.
    let mid = centre(bar.scrub).0;
    assert!(strip.pointer(StripPointer::Move, (mid, y), &bar, &mut driver));
    assert!((driver.time() - 1.0).abs() < 0.05, "{}", driver.time());
    assert!(strip.pointer(StripPointer::Up, (mid, y), &bar, &mut driver));
    assert!(!strip.is_dragging());
}

#[test]
fn dragging_the_scrub_bar_pauses_and_release_stays_paused() {
    let (bar, mut driver, mut strip) = (bar(), loaded(), Strip::default());
    assert!(driver.play());
    driver.playback().clock().advance(0.25);
    let (x, y) = centre(bar.scrub);
    assert!(strip.pointer(StripPointer::Down, (x, y), &bar, &mut driver));
    assert!(!driver.is_playing() && strip.is_dragging());
    driver.playback().clock().advance(5.0);
    assert!(strip.pointer(StripPointer::Move, (x + 40, y), &bar, &mut driver));
    let moved = driver.time();
    assert!(moved > 1.0 && !driver.is_playing());
    assert!(strip.pointer(StripPointer::Up, (x + 40, y), &bar, &mut driver));
    assert!(!driver.is_playing() && !strip.is_dragging());
    assert_eq!(driver.time(), moved);
    // Moves without a drag fall through to the tools.
    assert!(!strip.pointer(StripPointer::Move, (x, y), &bar, &mut driver));
}

#[test]
fn the_button_toggles_play_and_the_bar_swallows_clicks() {
    let (bar, mut driver, mut strip) = (bar(), loaded(), Strip::default());
    let (bx, by) = centre(bar.button);
    assert!(strip.pointer(StripPointer::Down, (bx, by), &bar, &mut driver));
    assert!(driver.is_playing());
    assert!(strip.pointer(StripPointer::Down, (bx, by), &bar, &mut driver));
    assert!(!driver.is_playing());
    assert!(strip.pointer(
        StripPointer::Down,
        (bar.bar.x as i32 + 1, by),
        &bar,
        &mut driver
    ));
    assert!(!driver.is_playing());
    // Above the strip: not consumed.
    assert!(!strip.pointer(
        StripPointer::Down,
        (bx, bar.bar.y as i32 - 3),
        &bar,
        &mut driver
    ));
}

#[test]
fn the_strip_is_hidden_and_inert_for_static_documents() {
    let driver = Driver::new(ManualClock::default());
    assert!(!driver.strip_visible());
    let mut driver = driver;
    let mut strip = Strip::default();
    let bar = bar();
    let (bx, by) = centre(bar.button);
    assert!(!strip.pointer(StripPointer::Down, (bx, by), &bar, &mut driver));
    assert!(!driver.is_playing());
    assert!(loaded().strip_visible());
}
