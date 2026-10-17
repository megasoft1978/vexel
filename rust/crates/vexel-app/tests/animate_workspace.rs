//! Draw <-> Animate workspace switches (WO-RUST-V1-ANIMATE-FIX-01): the stage
//! keeps the document and the Timeline text follows the open document. All
//! headless: session, compose worker and driver, no window.
use std::path::PathBuf;
use std::time::{Duration, Instant};
use vexel_app::animation::driver::Driver;
use vexel_app::animation::playback::MonotonicClock;
use vexel_app::animation::{stage, strip};
use vexel_app::chrome::{
    AppState, NO_ANIMATION, NO_DOCUMENT_TO_ANIMATE, chrome_text, timeline_text,
};
use vexel_app::compose::{DocumentView, blit_document, compose_document};
use vexel_app::model::{Layout, Mode, Model};
use vexel_app::render::render;
use vexel_app::session::AppDocument;
use vexel_platform_apple::RgbaFrame;

const W: u32 = 1280;
const H: u32 = 820;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../../fixtures/v1/{rel}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn open(rel: &str) -> AppDocument {
    let mut app = AppDocument::new(10, 10).unwrap();
    app.open_path(&fixture(rel)).unwrap();
    app
}

fn layout(mode: Mode) -> Layout {
    Layout::new(W, H, mode).unwrap()
}

fn stage_of(mode: Mode) -> (u32, u32) {
    let l = layout(mode);
    (l.stage.width, l.stage.height)
}

/// One shell iteration at `mode`: sync, then deliver any finished compose.
fn step(
    driver: &mut Driver<MonotonicClock>,
    app: &AppDocument,
    view: &mut DocumentView,
    mode: Mode,
) {
    stage::sync(driver, app, view, stage_of(mode));
    view.poll();
}

/// Steps until the document is composed at the stage size of `mode`.
fn settle(
    driver: &mut Driver<MonotonicClock>,
    app: &AppDocument,
    view: &mut DocumentView,
    mode: Mode,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let size = stage_of(mode);
    loop {
        step(driver, app, view, mode);
        // One more sync so a just-delivered compose is presented.
        stage::sync(driver, app, view, size);
        let composed = view.composed();
        if composed.is_some_and(|c| (c.frame.width, c.frame.height) == size) {
            return;
        }
        assert!(Instant::now() < deadline, "compose timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn screen(view: &DocumentView, mode: Mode) -> RgbaFrame {
    let frame = render(W, H, 1, &Model::new(true)).unwrap();
    blit_document(frame, view, &layout(mode), true)
}

/// RGBA of the stage region of `frame`.
fn stage_pixels(frame: &RgbaFrame, mode: Mode) -> Vec<u8> {
    let s = layout(mode).stage;
    let mut out = Vec::new();
    for y in s.y..s.y + s.height {
        let start = ((y * W + s.x) * 4) as usize;
        out.extend_from_slice(&frame.pixels()[start..start + (s.width * 4) as usize]);
    }
    out
}

fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn well_color(mode: Mode) -> Vec<u8> {
    // A view with a status but nothing composed paints the empty canvas well.
    let mut app = AppDocument::new(10, 10).unwrap();
    let mut empty = DocumentView::new();
    empty.open(&mut app, &fixture("missing.svg"), stage_of(mode));
    assert!(empty.status().is_some() && empty.composed().is_none());
    stage_pixels(&screen(&empty, mode), mode)
}

/// Number of stage pixels that differ from the empty stage content.
fn painted(view: &DocumentView, mode: Mode) -> usize {
    let empty = well_color(mode);
    stage_pixels(&screen(view, mode), mode)
        .chunks_exact(4)
        .zip(empty.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count()
}

fn state(app: &AppDocument, view: &DocumentView, driver: &Driver<MonotonicClock>) -> AppState {
    let mut state = AppState::from_session(app, view);
    if driver.knows(app.revision()) {
        state.loop_seconds = driver.loop_seconds();
    }
    state
}

// ---- AC1 ---------------------------------------------------------------------

#[test]
fn timeline_text_has_three_distinct_states() {
    let none = timeline_text(false, 0.0);
    let stat = timeline_text(true, 0.0);
    let anim = timeline_text(true, 2.0);
    assert_eq!(none, NO_DOCUMENT_TO_ANIMATE);
    assert_eq!(none, "No document to animate");
    assert_eq!(stat, NO_ANIMATION);
    assert_eq!(stat, "No animation in this document");
    assert_eq!(anim, "Animated, loop 0:02.00");
    assert!(none != stat && stat != anim && none != anim);
    // An open document never claims "No document to animate".
    assert_ne!(timeline_text(true, 0.0), NO_DOCUMENT_TO_ANIMATE);
    assert_ne!(timeline_text(true, 1.0), NO_DOCUMENT_TO_ANIMATE);
}

// ---- AC2 ---------------------------------------------------------------------

#[test]
fn animate_layout_state_for_static_and_animated_documents() {
    for (rel, animated) in [("compose/compose-v1.svg", false), ("anim/loop.svg", true)] {
        let app = open(rel);
        let mut driver = Driver::new(MonotonicClock::new());
        let mut view = DocumentView::new();
        settle(&mut driver, &app, &mut view, Mode::Animate);
        let l = layout(Mode::Animate);
        assert!(l.stage.width > 0 && l.stage.height > 0);
        let text = chrome_text(&state(&app, &view, &driver));
        assert_eq!(
            strip::layout(l.stage).is_some() && driver.strip_visible(),
            animated
        );
        assert_eq!(driver.strip_visible(), animated, "{rel}");
        if animated {
            assert_eq!(text.timeline, timeline_text(true, driver.loop_seconds()));
            assert!(
                text.timeline.starts_with("Animated, loop "),
                "{}",
                text.timeline
            );
            assert!(strip::layout(l.stage).is_some());
        } else {
            assert_eq!(text.timeline, NO_ANIMATION);
        }
    }
    // No document: the placeholder.
    let driver = Driver::new(MonotonicClock::new());
    let blank = AppDocument::new(10, 10).unwrap();
    let text = chrome_text(&state(&blank, &DocumentView::new(), &driver));
    assert_eq!(text.timeline, NO_DOCUMENT_TO_ANIMATE);
}

#[test]
fn timeline_text_is_right_before_the_static_compose_lands() {
    let app = open("anim/loop.svg");
    let mut driver = Driver::new(MonotonicClock::new());
    let mut view = DocumentView::new();
    stage::sync(&mut driver, &app, &mut view, stage_of(Mode::Animate));
    assert!(view.composed().is_none());
    let text = chrome_text(&state(&app, &view, &driver));
    assert!(
        text.timeline.starts_with("Animated, loop "),
        "{}",
        text.timeline
    );
}

// ---- AC3 ---------------------------------------------------------------------

#[test]
fn static_stage_in_animate_equals_compose_at_that_size() {
    let app = open("compose/compose-v1.svg");
    let bytes = app.source_bytes().unwrap();
    let mut driver = Driver::new(MonotonicClock::new());
    let mut view = DocumentView::new();
    settle(&mut driver, &app, &mut view, Mode::Animate);
    let (w, h) = stage_of(Mode::Animate);
    let expect = compose_document(&bytes, w, h, &|| false).unwrap().frame;
    let composed = &view.composed().unwrap().frame;
    assert_eq!(composed.rgba, expect.rgba);
    // The blitted stage is that frame composited over white.
    let over: Vec<u8> = expect
        .rgba
        .chunks_exact(4)
        .flat_map(|p| {
            let a = u32::from(p[3]);
            let f = |c: u8| ((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
            [f(p[0]), f(p[1]), f(p[2]), 255]
        })
        .collect();
    let shown = stage_pixels(&screen(&view, Mode::Animate), Mode::Animate);
    assert_eq!(hash(&shown), hash(&over));
    // Draw mode, composed at the same (Animate) size, shows the same pixels.
    let mut draw = DocumentView::new();
    draw.set_name("d");
    draw.show(&bytes, app.revision(), (w, h));
    while draw.composed().is_none() {
        draw.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(draw.composed().unwrap().frame.rgba, expect.rgba);
}

// ---- AC4 ---------------------------------------------------------------------

#[test]
fn no_blank_gap_on_the_frame_after_a_mode_switch() {
    let app = open("compose/compose-v1.svg");
    let mut driver = Driver::new(MonotonicClock::new());
    let mut view = DocumentView::new();
    settle(&mut driver, &app, &mut view, Mode::Draw);
    assert!(painted(&view, Mode::Draw) > 1000);
    // Switch: the shell starts the compose at the new stage size; the worker
    // result has not been delivered (no poll).
    stage::sync(&mut driver, &app, &mut view, stage_of(Mode::Animate));
    assert!(
        painted(&view, Mode::Animate) > 1000,
        "the stage must keep showing the document while the compose is pending"
    );
}

// ---- AC5 ---------------------------------------------------------------------

#[test]
fn repeated_switching_never_blanks_and_ends_composed() {
    let app = open("compose/compose-v1.svg");
    let mut driver = Driver::new(MonotonicClock::new());
    let mut view = DocumentView::new();
    settle(&mut driver, &app, &mut view, Mode::Draw);
    for mode in [
        Mode::Animate,
        Mode::Draw,
        Mode::Animate,
        Mode::Draw,
        Mode::Animate,
    ] {
        stage::sync(&mut driver, &app, &mut view, stage_of(mode));
        assert!(
            painted(&view, mode) > 1000,
            "blank right after switch to {mode:?}"
        );
        settle(&mut driver, &app, &mut view, mode);
        assert!(
            painted(&view, mode) > 1000,
            "blank once settled in {mode:?}"
        );
    }
    let (w, h) = stage_of(Mode::Animate);
    let c = view.composed().unwrap();
    assert_eq!((c.frame.width, c.frame.height), (w, h));
    let (status, error) = view.status().unwrap();
    assert!(!error && !status.starts_with("Composing"), "{status}");
}

// ---- AC6 ---------------------------------------------------------------------

#[test]
fn animated_document_stays_animated_across_switches() {
    let app = open("anim/loop.svg");
    let mut driver = Driver::new(MonotonicClock::new());
    let mut view = DocumentView::new();
    settle(&mut driver, &app, &mut view, Mode::Draw);
    for mode in [Mode::Animate, Mode::Draw, Mode::Animate] {
        stage::sync(&mut driver, &app, &mut view, stage_of(mode));
        assert!(painted(&view, mode) > 100, "blank after switch to {mode:?}");
        settle(&mut driver, &app, &mut view, mode);
        // The next sync presents the animated frame at the new size.
        stage::sync(&mut driver, &app, &mut view, stage_of(mode));
        assert!(view.animated_shown(), "driver frame refused in {mode:?}");
        assert!(painted(&view, mode) > 100);
    }
    assert!(driver.strip_visible());
    let label = strip::label(driver.time(), driver.loop_seconds());
    assert!(label.ends_with("/ 0:01.00"), "{label}");
}
