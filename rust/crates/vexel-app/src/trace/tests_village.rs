//! Real-art check: skipped unless the village reference render is present.

use super::*;
use crate::animation::frame;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const SOURCE: &str = "/private/tmp/vexel-toon/ref2/shot3_village.png";

#[test]
fn village_traces_within_the_stated_error_and_limits() {
    let Ok(bytes) = std::fs::read(SOURCE) else {
        eprintln!("skipped: {SOURCE} not present");
        return;
    };
    let px = crate::compose::decode_png(&bytes).expect("decode");
    let (w, h) = (px.width as usize, px.height as usize);
    let img = Rgba::new(w, h, &px.rgba).unwrap();
    // The tool's defaults for a 1536 px image: minArea = pixels / 12000, tolerance 1.5.
    let opts = TraceOptions {
        min_area: (w * h / 12_000) as u32,
        hole_area: (w * h / 24_000) as u32,
        tolerance: 1.5,
        ..TraceOptions::default()
    };
    let started = Instant::now();
    let layers = trace_image(&img, 24, &opts, None).unwrap();
    let elapsed = started.elapsed().as_secs_f64();
    let bytes_out: usize = layers.iter().map(|l| l.d.len()).sum();
    let contours: usize = layers.iter().map(|l| l.contours).sum();
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">"
    );
    for l in &layers {
        svg.push_str(&format!("<path d=\"{}\" fill=\"{}\"/>", l.d, l.hex()));
    }
    svg.push_str("</svg>");
    let scene = frame::decode(svg.as_bytes()).expect("traced svg decodes");
    let out = scene
        .render_sized(0.0, (w as u32, h as u32), &AtomicBool::new(false))
        .expect("render");
    let (mut sum, mut n) = (0u64, 0u64);
    for (a, b) in px.rgba.chunks_exact(4).zip(out.rgba().chunks_exact(4)) {
        for c in 0..3 {
            sum += u64::from(a[c].abs_diff(b[c]));
            n += 1;
        }
    }
    let mean = sum as f64 / n as f64;
    println!(
        "village {w}x{h} k=24: {elapsed:.2}s, {} layers, {contours} contours, {bytes_out} bytes, mean colour error {mean:.2}/255 (build: {})",
        layers.len(),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    assert!(layers.len() <= 24 && layers.len() <= 4096);
    assert!(bytes_out < 4 * 1024 * 1024 - 64 * 160);
    assert!(mean < 12.0, "mean colour error {mean}");
    if !cfg!(debug_assertions) {
        assert!(elapsed < 3.0, "trace took {elapsed:.2}s");
    }
}
