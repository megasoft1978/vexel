//! WO-RUST-UI-TIMELINE-02 acceptance 1 and 6: one frame grid for stepping,
//! seeking and the readout; one step definition shared by Draw and Animate.
#[path = "ui_timeline_support.rs"]
mod support;

use std::path::Path;
use support::*;
use vexel_app::animation::driver::{Driver, FRAME_STEP, quantize};
use vexel_app::animation::keys::{PlaybackKey, apply};
use vexel_app::animation::playback::ManualClock;
use vexel_app::layout::status_hint;
use vexel_app::model::{Mode, Model, Tool};
use vexel_app::timeline::key;
use vexel_platform_apple::Modifiers;

const ODD: &[u8] = include_bytes!("../../../fixtures/ui/timeline2/odd.svg");
const NONE: Modifiers = Modifiers(0);
const LEFT: u16 = 123;
const RIGHT: u16 = 124;

fn count_in(dir: &Path, needle: &str, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            count_in(&path, needle, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            for (n, line) in text.lines().enumerate() {
                if line.contains(needle) {
                    found.push(format!("{}:{}", path.display(), n + 1));
                }
            }
        }
    }
}

#[test]
fn one_frame_step_constant_exists() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let (mut steps, mut fps) = (Vec::new(), Vec::new());
    count_in(&src, "const FRAME_STEP", &mut steps);
    count_in(&src, "const FPS", &mut fps);
    assert_eq!(steps.len(), 1, "{steps:?}");
    assert!(fps.is_empty(), "24 fps constant is gone: {fps:?}");
    assert!((FRAME_STEP - 1.0 / 30.0).abs() < 1e-15);
}

/// The old timeline rule, written out independently of the driver.
fn expected(time: f64, forward: bool, looping: bool, loop_s: f64) -> f64 {
    let frames = ((loop_s * 30.0) - 1e-9).ceil().max(1.0) as i64;
    let current = (time * 30.0).round() as i64;
    let next = current + if forward { 1 } else { -1 };
    let next = if looping {
        next.rem_euclid(frames)
    } else {
        next.clamp(0, frames)
    };
    (next as f64 / 30.0).min(loop_s)
}

fn driver_of(source: &[u8]) -> Driver<ManualClock> {
    driver_with(source)
}

#[test]
fn both_workspaces_step_identically_on_the_30fps_grid() {
    let docs: [(&[u8], f64, bool); 4] = [
        (LOOP, 1.0, true),
        (LOOP2, 2.0, true),
        (LONG, 4.0, true),
        (ODD, 1.01, false),
    ];
    let mut seed = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut on_grid = 0;
    let mut end_clamps = 0;
    for _ in 0..5000 {
        let (source, loop_s, multiple) = docs[(next() * 4.0) as usize % 4];
        let time = next() * loop_s;
        let forward = next() < 0.5;
        let looping = next() < 0.5;
        let playing = next() < 0.5;
        let mut draw = driver_of(source);
        let mut animate = driver_of(source);
        for d in [&mut draw, &mut animate] {
            assert!((d.loop_seconds() - loop_s).abs() < 1e-9);
            d.seek(time);
            if !looping {
                d.toggle_looping();
            }
            if playing {
                d.play();
            }
        }
        let at = draw.time();
        let want = expected(at, forward, looping, loop_s);
        let key_draw = if forward {
            PlaybackKey::StepForward
        } else {
            PlaybackKey::StepBack
        };
        assert!(apply(&mut draw, key_draw, false, false));
        assert!(key(if forward { RIGHT } else { LEFT }, NONE, &mut animate));
        assert!(
            !draw.is_playing() && !animate.is_playing(),
            "stepping pauses"
        );
        assert!(
            (draw.time() - animate.time()).abs() < 1e-12,
            "workspaces agree"
        );
        assert!(
            (draw.time() - want).abs() < 1e-9,
            "{time} fwd={forward} loop={looping}: {} vs {want}",
            draw.time()
        );
        let k = (draw.time() * 30.0).round();
        if (draw.time() - k / 30.0).abs() < 1e-9 {
            on_grid += 1;
        } else {
            // Only the end of a loop that is not a whole number of frames.
            assert!(!multiple && (draw.time() - loop_s).abs() < 1e-9);
            end_clamps += 1;
        }
    }
    assert!(on_grid > 4000, "{on_grid} results on k/30");
    assert!(end_clamps > 0, "odd loop end exercised");
}

#[test]
fn seeking_and_the_readout_quantise_to_the_grid() {
    assert_eq!(quantize(0.0, 1.0), 0.0);
    assert!((quantize(0.51, 1.0) - 15.0 / 30.0).abs() < 1e-12);
    assert_eq!(quantize(9.0, 1.0), 1.0);
    assert_eq!(quantize(-3.0, 1.0), 0.0);
    // Scrubbing the ruler lands on a frame.
    let mut d = driver_with(LOOP);
    let mut tl = timeline_of(LOOP, &d);
    let l = vexel_app::timeline::layout::TimelineLayout::new(rect(54, 500, 900, 200));
    let y = (l.ruler.y + 12) as i32;
    for x in (l.lane_x0 as i32..l.lane_x1 as i32).step_by(7) {
        tl.pointer(vexel_app::timeline::Phase::Down, (x, y), &l, &mut d);
        let k = (d.time() * 30.0).round();
        assert!((d.time() - k / 30.0).abs() < 1e-9, "x={x} t={}", d.time());
        tl.pointer(vexel_app::timeline::Phase::Up, (x, y), &l, &mut d);
    }
}

#[test]
fn hints_follow_the_new_wording() {
    let mut m = Model::new(false);
    m.chrome.state.open = true;
    m.mode = Mode::Animate;
    assert_eq!(status_hint(&m), "Space plays, drag the ruler to scrub");
    m.mode = Mode::Draw;
    m.tool = Tool::Select;
    assert_eq!(status_hint(&m), "Click a shape to select it");
    m.tool = Tool::Pen;
    assert_eq!(
        status_hint(&m),
        "Click to add points, double-click or Return to finish"
    );
}

#[test]
fn chrome_timeline_text_keeps_its_reader() {
    // `chrome.timeline` is still read by tests/animate_workspace.rs (a protected
    // test), so the field stays; this records the decision.
    let text = vexel_app::chrome::timeline_text(true, 2.0);
    assert!(text.starts_with("Animated, loop "));
}
