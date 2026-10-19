//! WO-RUST-UI-WIRING-01 acceptance 8: golden candidates for the Export popover
//! and the Undo hover frame, 1280x800 Draw, compose-v1.svg plus one edit (so
//! Undo is enabled), light and dark, 1x and 2x. `VEXEL_BLESS_GOLDENS=1` writes
//! candidates (`*.rle` plus review PNGs) into
//! `artifacts/WO-RUST-UI-WIRING-01/review/`; the worker never accepts them.
//! Once a human has accepted them they are copied to `fixtures/ui/wiring/` and
//! normal runs compare against those (CHROME policy: at most 0.5% of pixels
//! differ, no channel by more than 24).
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};
use vexel_app::animation::driver::Driver;
use vexel_app::animation::playback::MonotonicClock;
use vexel_app::animation::stage;
use vexel_app::chrome::{AppState, chrome_text};
use vexel_app::compose::{DocumentView, blit_document};
use vexel_app::model::Model;
use vexel_app::motion::MotionAdapter;
use vexel_app::render::render_with;
use vexel_app::session::AppDocument;
use vexel_platform_apple::PointerKind;
use vexel_png::{Compression, encode_rgba8};
use vexel_ui::motion::Clock;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/ui/wiring");
const REVIEW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../artifacts/WO-RUST-UI-WIRING-01/review"
);
const SIZE: (u32, u32) = (1280, 800);

#[derive(Clone)]
struct TestClock(Rc<Cell<u64>>);
impl Clock for TestClock {
    fn now_ns(&self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    ExportMenu,
    HoverUndo,
}

fn compose_v1() -> PathBuf {
    PathBuf::from(format!(
        "{}/../../fixtures/v1/compose/compose-v1.svg",
        env!("CARGO_MANIFEST_DIR")
    ))
}

fn frame(kind: Kind, dark: bool, scale: u32) -> (u32, u32, Vec<u8>) {
    let mut model = Model::new(dark);
    let mut app = AppDocument::new(10, 10).unwrap();
    let mut view = DocumentView::new();
    let l = model.layout(SIZE.0, SIZE.1).unwrap();
    let stage_size = (l.stage.width, l.stage.height);
    app.open_path(&compose_v1()).unwrap();
    app.edit(|d| d.add_rectangle_with_id(8.0, 8.0, 40.0, 30.0, "#2060e0", "wiring-rect"))
        .unwrap();
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
    model.sync_history(&app);
    let layout = model.layout(SIZE.0, SIZE.1).unwrap();
    let time = Rc::new(Cell::new(1_000_000_000));
    let mut motion = MotionAdapter::with_clock(TestClock(time.clone()), false);
    match kind {
        Kind::ExportMenu => model.export_open = true,
        Kind::HoverUndo => {
            let r = layout.buttons[0];
            let at = ((r.x + r.width / 2) as i32, (r.y + r.height / 2) as i32);
            motion.pointer_in(&layout, PointerKind::Move, at, true, &model);
            time.set(time.get() + 300_000_000);
        }
    }
    let raw = render_with(
        SIZE.0,
        SIZE.1,
        scale,
        &model,
        &motion.frame(dark, model.mode),
    )
    .expect("shell renders");
    let out = blit_document(raw, &view, &layout, dark);
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

fn name(kind: Kind, dark: bool, scale: u32) -> String {
    let k = match kind {
        Kind::ExportMenu => "export-menu",
        Kind::HoverUndo => "hover-undo",
    };
    let theme = if dark { "dark" } else { "light" };
    format!("1280x800-draw-{theme}-{k}@{scale}x")
}

#[test]
fn wiring_frames_match_accepted_goldens_or_write_candidates() {
    let bless = std::env::var_os("VEXEL_BLESS_GOLDENS").is_some_and(|v| v == "1");
    let mut failures = Vec::new();
    let mut compared = 0;
    for kind in [Kind::ExportMenu, Kind::HoverUndo] {
        for dark in [false, true] {
            for scale in [1, 2] {
                let (w, h, rgba) = frame(kind, dark, scale);
                assert_eq!(rgba, frame(kind, dark, scale).2, "repeatable");
                let file = name(kind, dark, scale);
                if bless {
                    std::fs::create_dir_all(REVIEW).unwrap();
                    std::fs::write(format!("{REVIEW}/{file}.rle"), rle(w, h, &rgba)).unwrap();
                    let png = encode_rgba8(w, h, &rgba, Compression::Fixed).unwrap();
                    std::fs::write(format!("{REVIEW}/{file}.png"), png).unwrap();
                    continue;
                }
                let Ok(golden) = std::fs::read(format!("{FIXTURES}/{file}.rle")) else {
                    continue; // candidate not accepted yet
                };
                compared += 1;
                let (gw, gh, want) = unrle(&golden);
                assert_eq!((gw, gh), (w, h), "{file}: dimensions");
                if let Err(why) = compare(&want, &rgba) {
                    failures.push(format!("{file}: {why}"));
                }
            }
        }
    }
    eprintln!("compared {compared} accepted goldens");
    assert!(
        failures.is_empty(),
        "golden mismatches:\n{}",
        failures.join("\n")
    );
}
