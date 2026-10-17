//! WO-RUST-UI-CHROME-01 acceptance 8: full-window goldens.
//!
//! 1280x800 and 960x640, Draw and Animate, dark and light, 1x and 2x, each
//! with the compose-v1.svg state and the empty (no document) state: 32 frames.
//! Goldens are first-party (the Rust renderer, never a C oracle), stored as
//! run-length encoded RGBA (`*.rle`: u32 LE width, height, then (count, pixel)
//! pairs) because the repository has no PNG decoder in the test path.
//!
//! Tolerance (the frame carries CoreText glyphs, which can differ by OS
//! version): at most 0.5% of pixels may differ at all, and no channel may
//! differ by more than 24. Geometry-only assertions live in `layout::tests`.
//!
//! Candidates stay candidates until a human has viewed the review PNGs:
//! `VEXEL_BLESS_GOLDENS=1` writes fixtures plus
//! `artifacts/WO-RUST-UI-CHROME-01/review/*.png`; normal runs never write.
use std::path::PathBuf;
use std::time::{Duration, Instant};
use vexel_app::animation::driver::Driver;
use vexel_app::animation::playback::MonotonicClock;
use vexel_app::animation::stage;
use vexel_app::chrome::{AppState, chrome_text};
use vexel_app::compose::{DocumentView, blit_document};
use vexel_app::model::{Mode, Model};
use vexel_app::render::render;
use vexel_app::session::AppDocument;
use vexel_png::{Compression, encode_rgba8};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/ui/chrome");
const REVIEW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../artifacts/WO-RUST-UI-CHROME-01/review"
);

fn compose_v1() -> PathBuf {
    PathBuf::from(format!(
        "{}/../../fixtures/v1/compose/compose-v1.svg",
        env!("CARGO_MANIFEST_DIR")
    ))
}

/// One frame of the shell at `size`, as `main.rs` would present it.
fn frame(
    size: (u32, u32),
    mode: Mode,
    dark: bool,
    scale: u32,
    with_doc: bool,
) -> (u32, u32, Vec<u8>) {
    let mut model = Model::new(dark);
    model.mode = mode;
    let mut app = AppDocument::new(10, 10).unwrap();
    let mut view = DocumentView::new();
    if with_doc {
        let stage_size = {
            let l = model.layout(size.0, size.1).unwrap();
            (l.stage.width, l.stage.height)
        };
        app.open_path(&compose_v1()).unwrap();
        let mut driver = Driver::new(MonotonicClock::new());
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            stage::sync(&mut driver, &app, &mut view, stage_size);
            view.poll();
            if view
                .composed()
                .is_some_and(|c| (c.frame.width, c.frame.height) == stage_size)
            {
                break;
            }
            assert!(Instant::now() < deadline, "compose timed out");
            std::thread::sleep(Duration::from_millis(2));
        }
        model.chrome = chrome_text(&AppState::from_session(&app, &view));
    }
    let raw = render(size.0, size.1, scale, &model).expect("shell renders");
    let layout = model.layout(size.0, size.1).unwrap();
    let out = if with_doc {
        blit_document(raw, &view, &layout, dark)
    } else {
        raw
    };
    (out.width(), out.height(), out.pixels().to_vec())
}

fn rle(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = [w.to_le_bytes(), h.to_le_bytes()].concat();
    let mut px = rgba.chunks_exact(4);
    let Some(first) = px.next() else { return out };
    let (mut cur, mut n) = (first, 1u32);
    for p in px {
        if p == cur {
            n += 1;
        } else {
            out.extend(n.to_le_bytes());
            out.extend(cur);
            (cur, n) = (p, 1);
        }
    }
    out.extend(n.to_le_bytes());
    out.extend(cur);
    out
}

fn unrle(data: &[u8]) -> (u32, u32, Vec<u8>) {
    let word = |i: usize| u32::from_le_bytes(data[i..i + 4].try_into().unwrap());
    let (w, h) = (word(0), word(4));
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for run in data[8..].chunks_exact(8) {
        let n = u32::from_le_bytes(run[..4].try_into().unwrap());
        for _ in 0..n {
            out.extend_from_slice(&run[4..]);
        }
    }
    (w, h, out)
}

/// `Err` with the numbers when `got` is outside the documented tolerance.
fn compare(want: &[u8], got: &[u8]) -> Result<(), String> {
    if want.len() != got.len() {
        return Err(format!("size {} vs {}", want.len(), got.len()));
    }
    let (mut differing, mut worst) = (0usize, 0u8);
    for (a, b) in want.chunks_exact(4).zip(got.chunks_exact(4)) {
        let d = a
            .iter()
            .zip(b)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        if d > 0 {
            differing += 1;
            worst = worst.max(d);
        }
    }
    let total = want.len() / 4;
    if differing * 200 > total || worst > 24 {
        return Err(format!(
            "{differing}/{total} pixels differ, worst channel delta {worst}"
        ));
    }
    Ok(())
}

fn name(size: (u32, u32), mode: Mode, dark: bool, scale: u32, with_doc: bool) -> String {
    format!(
        "{}x{}-{}-{}-{}@{}x",
        size.0,
        size.1,
        if mode == Mode::Draw {
            "draw"
        } else {
            "animate"
        },
        if dark { "dark" } else { "light" },
        if with_doc { "compose-v1" } else { "empty" },
        scale
    )
}

#[test]
fn chrome_frames_match_first_party_goldens() {
    let bless = std::env::var_os("VEXEL_BLESS_GOLDENS").is_some_and(|v| v == "1");
    let mut failures = Vec::new();
    for size in [(1280, 800), (960, 640)] {
        for mode in [Mode::Draw, Mode::Animate] {
            for dark in [true, false] {
                for with_doc in [true, false] {
                    for scale in [1, 2] {
                        let (w, h, rgba) = frame(size, mode, dark, scale, with_doc);
                        let file = name(size, mode, dark, scale, with_doc);
                        if bless {
                            std::fs::create_dir_all(FIXTURES).unwrap();
                            std::fs::create_dir_all(REVIEW).unwrap();
                            std::fs::write(format!("{FIXTURES}/{file}.rle"), rle(w, h, &rgba))
                                .unwrap();
                            let png = encode_rgba8(w, h, &rgba, Compression::Fixed).unwrap();
                            std::fs::write(format!("{REVIEW}/{file}.png"), png).unwrap();
                            continue;
                        }
                        let golden = std::fs::read(format!("{FIXTURES}/{file}.rle"))
                            .unwrap_or_else(|e| panic!("{file}: {e}"));
                        let (gw, gh, want) = unrle(&golden);
                        assert_eq!((gw, gh), (w, h), "{file}: dimensions");
                        if let Err(why) = compare(&want, &rgba) {
                            failures.push(format!("{file}: {why}"));
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "golden mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn rle_round_trips_and_tolerance_rejects_large_changes() {
    let rgba: Vec<u8> = (0..400u32)
        .flat_map(|i| [(i / 7) as u8, 1, 2, 255])
        .collect();
    let (w, h, back) = unrle(&rle(20, 20, &rgba));
    assert_eq!((w, h), (20, 20));
    assert_eq!(back, rgba);
    assert!(compare(&rgba, &rgba).is_ok());
    let mut one = rgba.clone();
    for px in 0..3 {
        one[px * 4 + 1] = 9;
    }
    assert!(
        compare(&rgba, &one).is_err(),
        "3 of 400 pixels exceeds 0.5%"
    );
    let big: Vec<u8> = vec![0; 4 * 1000];
    let mut near = big.clone();
    near[0] = 4;
    assert!(compare(&big, &near).is_ok());
    near[4] = 25;
    assert!(compare(&big, &near).is_err(), "delta above 24 is rejected");
}

#[test]
fn chrome_frames_are_repeatable_and_differ_by_state() {
    let a = frame((960, 640), Mode::Draw, true, 1, false);
    assert_eq!(a, frame((960, 640), Mode::Draw, true, 1, false));
    assert_ne!(a.2, frame((960, 640), Mode::Animate, true, 1, false).2);
    assert_ne!(a.2, frame((960, 640), Mode::Draw, false, 1, false).2);
}
