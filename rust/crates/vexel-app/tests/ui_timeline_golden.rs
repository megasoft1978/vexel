//! WO-RUST-UI-TIMELINE-01 acceptance 8: footer goldens (crop of the 1280x800
//! Animate window) for playing, paused with a selected key, and a long track
//! list scrolled; 1x and 2x, dark and light: 12 images.
//!
//! Goldens are first-party (the Rust renderer, never a C oracle), stored as
//! run-length encoded RGBA (u32 LE width, height, then (count, pixel) pairs).
//! The footer carries CoreText glyphs, so the tolerance follows the policy:
//! at most 0.5% of pixels may differ at all and no channel may differ by more
//! than 24. Geometry-only assertions live in `ui_timeline_model.rs`.
//!
//! Candidates stay candidates until a human has viewed the review PNGs:
//! `VEXEL_BLESS_GOLDENS=1` writes the fixtures plus
//! `artifacts/WO-RUST-UI-TIMELINE-01/review/*.png`; normal runs never write.
#[path = "ui_timeline_support.rs"]
mod support;

use support::*;
use vexel_app::timeline::model::KeyRef;
use vexel_png::{Compression, encode_rgba8};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/ui/timeline");
const REVIEW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../artifacts/WO-RUST-UI-TIMELINE-01/review"
);
const SIZE: (u32, u32) = (1280, 800);

fn state(name: &str) -> vexel_app::timeline::Timeline {
    match name {
        "playing" => {
            let mut d = driver_with(LOOP);
            d.seek(0.8);
            d.play();
            let mut tl = timeline_of(LOOP, &d);
            tl.model.playing = true;
            tl
        }
        "paused-selected" => {
            let mut d = driver_with(LOOP);
            d.seek(0.5);
            let mut tl = timeline_of(LOOP, &d);
            tl.selection = Some(KeyRef {
                element: 0,
                prop: Some(0),
                index: 1,
            });
            tl
        }
        _ => {
            let mut d = driver_with(LONG);
            d.seek(1.3);
            let mut tl = timeline_of(LONG, &d);
            tl.scroll = 2;
            tl
        }
    }
}

fn crop(name: &str, dark: bool, scale: u32) -> (u32, u32, Vec<u8>) {
    let f = footer(SIZE);
    let full = frame(SIZE, dark, scale, state(name));
    let (w, h) = (f.width * scale, f.height * scale);
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in f.y * scale..f.y * scale + h {
        let at = ((y * full.0 + f.x * scale) * 4) as usize;
        out.extend_from_slice(&full.2[at..at + (w * 4) as usize]);
    }
    (w, h, out)
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

#[test]
fn footer_frames_match_first_party_goldens() {
    let bless = std::env::var_os("VEXEL_BLESS_GOLDENS").is_some_and(|v| v == "1");
    let mut failures = Vec::new();
    for name in ["playing", "paused-selected", "long-scrolled"] {
        for dark in [true, false] {
            for scale in [1, 2] {
                let (w, h, rgba) = crop(name, dark, scale);
                let file = format!(
                    "footer-{name}-{}@{scale}x",
                    if dark { "dark" } else { "light" }
                );
                if bless {
                    std::fs::create_dir_all(FIXTURES).unwrap();
                    std::fs::create_dir_all(REVIEW).unwrap();
                    std::fs::write(format!("{FIXTURES}/{file}.rle"), rle(w, h, &rgba)).unwrap();
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
    assert!(
        failures.is_empty(),
        "golden mismatches:\n{}",
        failures.join("\n")
    );
}

#[test]
fn rle_round_trips_and_tolerance_rejects_large_changes() {
    let img: Vec<u8> = (0..64u32)
        .flat_map(|i| [(i / 8) as u8, 7, 9, 255])
        .collect();
    assert_eq!(unrle(&rle(8, 8, &img)), (8, 8, img.clone()));
    assert!(compare(&img, &img).is_ok());
    let mut bad = img.clone();
    bad[0] = bad[0].wrapping_add(200);
    assert!(compare(&img, &bad).is_err());
    let _ = REVIEW;
}
