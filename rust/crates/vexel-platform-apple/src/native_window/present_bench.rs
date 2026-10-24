//! Headless bench of the pure-Rust and CoreGraphics parts of one present
//! (premultiply + image preparation). The NSWindow/CALayer hand-off needs a
//! live window and is not measured here.
//! `cargo test -p vexel-platform-apple --release present_bench -- --ignored --nocapture`
use super::frame::RgbaFrame;
use super::window::prepare_rgba_image;
use std::time::Instant;

const W: u32 = 2560;
const H: u32 = 1440;

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * p).round() as usize]
}

/// Mostly opaque UI-like frame with a few translucent shadow rows.
fn ui_pixels() -> Vec<u8> {
    let mut p = vec![0u8; (W * H * 4) as usize];
    for (i, px) in p.chunks_exact_mut(4).enumerate() {
        let y = i as u32 / W;
        let a = if y % 97 == 0 { 96 } else { 255 };
        px.copy_from_slice(&[(i % 251) as u8, (i % 241) as u8, (i % 239) as u8, a]);
    }
    p
}

#[test]
#[ignore]
fn present_cpu_parts() {
    let src = ui_pixels();
    let (mut pre, mut img, mut all) = (vec![], vec![], vec![]);
    for _ in 0..60 {
        let frame = RgbaFrame::new(W, H, 2.0, src.clone()).unwrap();
        let t = Instant::now();
        let px = frame.into_premultiplied_pixels();
        let a = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let image = prepare_rgba_image(W, H, px);
        let b = t.elapsed().as_secs_f64() * 1e3;
        assert!(image.is_some());
        pre.push(a);
        img.push(b);
        all.push(a + b);
    }
    for (name, v) in [
        ("premultiply", &mut pre),
        ("cgimage", &mut img),
        ("total", &mut all),
    ] {
        println!(
            "present_bench {name}: p50 {:.2} ms p95 {:.2} ms",
            pct(v, 0.5),
            pct(v, 0.95)
        );
    }
}
