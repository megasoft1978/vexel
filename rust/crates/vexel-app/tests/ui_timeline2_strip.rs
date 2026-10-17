//! WO-RUST-UI-TIMELINE-02 acceptance 7 and 8: the Draw-mode strip placement
//! and look, and the review candidates of the polished footer and strip.
//!
//! Candidates are never accepted by the worker: `VEXEL_BLESS_GOLDENS=1` writes
//! PNGs to `artifacts/WO-RUST-UI-TIMELINE-02/review/`; normal runs only check
//! that every candidate renders at the right size.
#[path = "ui_timeline_support.rs"]
mod support;

use support::*;
use vexel_app::animation::strip::{self, READOUT_PT};
use vexel_app::canvas::{controls, paint_cluster};
use vexel_app::model::{Layout, Mode, Model};
use vexel_app::render::render;
use vexel_png::{Compression, encode_rgba8};
use vexel_ui::{Rect, type_scale};

const TWELVE: &[u8] = include_bytes!("../../../fixtures/ui/timeline2/twelve.svg");
const REVIEW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../artifacts/WO-RUST-UI-TIMELINE-02/review"
);

fn disjoint(a: Rect, b: Rect) -> bool {
    a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y
}

fn within(inner: Rect, outer: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
}

#[test]
fn the_strip_zoom_cluster_and_status_bar_are_pairwise_disjoint() {
    for (w, h) in [(720, 480), (1280, 800)] {
        let l = Layout::new(w, h, Mode::Draw).unwrap();
        assert!(l.footer.is_none(), "Draw mode has no footer");
        let bar = strip::layout(l.stage).expect("strip fits").bar;
        let c = controls(l.stage.width, l.stage.height, 100)
            .unwrap()
            .container;
        let cluster = Rect {
            x: c.x + l.stage.x,
            y: c.y + l.stage.y,
            ..c
        };
        assert!(within(bar, l.stage) && within(cluster, l.stage));
        assert!(
            disjoint(bar, cluster),
            "{w}x{h}: strip over the zoom cluster"
        );
        assert!(disjoint(bar, l.status) && disjoint(cluster, l.status));
        assert_eq!(
            bar.y + bar.height + 8,
            cluster.y,
            "8 px gap above the cluster"
        );
        assert_eq!(bar.height, 28);
        let s = strip::layout(l.stage).unwrap();
        for (a, b) in [(s.button, s.scrub), (s.scrub, s.label), (s.button, s.label)] {
            assert!(disjoint(a, b));
        }
        assert!(within(s.button, bar) && within(s.scrub, bar) && within(s.label, bar));
        // Only Draw has a strip: Animate has the footer instead.
        assert!(Layout::new(w, h, Mode::Animate).unwrap().footer.is_some());
    }
}

fn draw_window(size: (u32, u32), dark: bool, scale: u32, t: f64) -> (u32, u32, Vec<u8>, Rect) {
    let mut d = driver_with(LOOP);
    d.seek(t);
    let mut model = Model::new(dark);
    model.mode = Mode::Draw;
    let layout = model.layout(size.0, size.1).unwrap();
    let base = render(size.0, size.1, scale, &model).unwrap();
    let (w, h) = (base.width(), base.height());
    let mut pixels = base.pixels().to_vec();
    paint_cluster(&mut pixels, (w, h), scale, layout.stage, dark, 100, true);
    let frame = vexel_platform_apple::RgbaFrame::new(w, h, f64::from(scale), pixels).unwrap();
    let bar = strip::layout(layout.stage).unwrap();
    let out = strip::draw(frame, &bar, &d, dark);
    (w, h, out.pixels().to_vec(), bar.bar)
}

#[test]
fn the_strip_has_no_accent_block_only_a_thin_thumb_and_a_body_size_readout() {
    assert_eq!(
        READOUT_PT,
        u32::from(type_scale().body_pt),
        "readout is body size"
    );
    for (size, dark, scale) in [
        ((720, 480), true, 1),
        ((1280, 800), false, 2),
        ((1280, 800), true, 1),
    ] {
        let (w, _, px, bar) = draw_window(size, dark, scale, 0.5);
        let accent = tokens(dark).accent.to_be_bytes();
        let mut widest = 0u32;
        let mut seen = 0u32;
        for y in bar.y * scale..(bar.y + bar.height) * scale {
            let mut run = 0u32;
            for x in bar.x * scale..(bar.x + bar.width) * scale {
                let at = ((y * w + x) * 4) as usize;
                if px[at..at + 3] == accent[..3] {
                    run += 1;
                    seen += 1;
                } else {
                    widest = widest.max(run);
                    run = 0;
                }
            }
            widest = widest.max(run);
        }
        assert!(seen > 0, "{size:?}: the thumb is accent");
        assert!(
            widest <= 4 * scale,
            "{size:?} dark={dark}: accent run of {widest} device px (thumb only)"
        );
    }
}

#[test]
fn the_strip_scrub_keys_and_buttons_still_behave() {
    use vexel_app::animation::strip::{Strip, StripPointer};
    let mut d = driver_with(LOOP);
    let layout = Layout::new(1280, 800, Mode::Draw).unwrap();
    let bar = strip::layout(layout.stage).unwrap();
    let mut s = Strip::default();
    let y = (bar.scrub.y + bar.scrub.height / 2) as i32;
    let mid = (bar.scrub.x + bar.scrub.width / 2) as i32;
    assert!(s.pointer(StripPointer::Down, (mid, y), &bar, &mut d));
    assert!((d.time() - 0.5).abs() < 0.05, "{}", d.time());
    let k = (d.time() * 30.0).round();
    assert!((d.time() - k / 30.0).abs() < 1e-9, "scrub lands on a frame");
    assert!(s.pointer(StripPointer::Up, (mid, y), &bar, &mut d));
    let play = ((bar.button.x + 4) as i32, (bar.button.y + 4) as i32);
    assert!(s.pointer(StripPointer::Down, play, &bar, &mut d));
    assert!(d.is_playing());
}

// ---- 8: review candidates ---------------------------------------------------------

fn animate(source: &[u8], t: f64, dark: bool, scale: u32) -> (u32, u32, Vec<u8>) {
    let mut d = driver_with(source);
    d.seek(t);
    frame((1280, 800), dark, scale, timeline_of(source, &d))
}

#[test]
fn review_candidates_render_and_are_blessed_only_on_request() {
    let bless = std::env::var_os("VEXEL_BLESS_GOLDENS").is_some_and(|v| v == "1");
    if bless {
        std::fs::create_dir_all(REVIEW).unwrap();
    }
    for dark in [false, true] {
        for scale in [1, 2] {
            let theme = if dark { "dark" } else { "light" };
            let shots = [
                ("animate-loop-1s", animate(LOOP, 0.4, dark, scale)),
                ("animate-12-tracks", animate(TWELVE, 0.8, dark, scale)),
                ("draw-loop-strip", {
                    let (w, h, px, _) = draw_window((1280, 800), dark, scale, 0.4);
                    (w, h, px)
                }),
            ];
            for (name, (w, h, rgba)) in shots {
                assert_eq!((w, h), (1280 * scale, 800 * scale), "{name}");
                assert!(rgba.chunks_exact(4).any(|p| p[3] == 255));
                if bless {
                    let png = encode_rgba8(w, h, &rgba, Compression::Fixed).unwrap();
                    std::fs::write(format!("{REVIEW}/{name}-{theme}@{scale}x.png"), png).unwrap();
                }
            }
        }
    }
}
