//! Stroke rendering of the reference rasterizer against the real `vexel-render`
//! CPU output, on seeded stroke scenes (widths 0.5-40, all joins and caps,
//! dashes, transforms, fill under stroke). Bounds are the ADR-0035 tolerances
//! from `policy`; they are never loosened here. `-- --nocapture` prints metrics.
#![allow(dead_code)]

use vexel_gpu_scene::policy::{
    PARITY_MEAN_ABS_ERROR_MAX, PARITY_OVER_2_MAX_FRACTION, PARITY_OVER_16_MAX_FRACTION,
};
use vexel_gpu_scene::{encode, reference};

include!("common/strokes.rs");

fn compare(a: &[u8], b: &[u8]) -> (f64, f64, f64, u8) {
    assert_eq!(a.len(), b.len());
    let pixels = a.len() / 4;
    let (mut over_2, mut over_16, mut sum, mut max_diff) = (0usize, 0usize, 0u64, 0u8);
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let worst = (0..4).map(|c| pa[c].abs_diff(pb[c])).max().unwrap();
        over_2 += usize::from(worst > 2);
        over_16 += usize::from(worst > 16);
        max_diff = max_diff.max(worst);
        sum += (0..4).map(|c| u64::from(pa[c].abs_diff(pb[c]))).sum::<u64>();
    }
    (
        over_2 as f64 / pixels as f64,
        over_16 as f64 / pixels as f64,
        sum as f64 / (pixels * 4) as f64 / 255.0,
        max_diff,
    )
}

fn check(name: &str, seed: u64, count: usize, size: u32, min_w: f64, max_w: f64) {
    let side = f64::from(size);
    let cases = stroke_cases(seed, count, side, side, min_w, max_w);
    let (scene, cpu) = stroke_scene(&cases, size, size);
    assert!(vexel_gpu_scene::classify(&scene).is_supported(), "{name}");
    let encoded = encode(&scene).unwrap();
    let gpu = reference::render_straight(&encoded);
    let (o2, o16, mae, max) = compare(&gpu, &cpu);
    println!(
        "{name}: {size}px shapes={} prims={} >2/255={:.5}% >16/255={:.5}% mae={:.6} max={max}",
        cases.len(),
        encoded.prims.len(),
        o2 * 100.0,
        o16 * 100.0,
        mae
    );
    assert!(o2 <= PARITY_OVER_2_MAX_FRACTION, "{name} over 2: {o2}");
    assert!(o16 <= PARITY_OVER_16_MAX_FRACTION, "{name} over 16: {o16}");
    assert!(mae <= PARITY_MEAN_ABS_ERROR_MAX, "{name} mae: {mae}");
}

#[test]
fn thin_strokes_match_cpu() {
    check("thin", 0x51, 60, 480, 0.5, 3.0);
}

#[test]
fn medium_strokes_match_cpu() {
    check("medium", 0x52, 50, 480, 3.0, 14.0);
}

#[test]
fn wide_strokes_match_cpu() {
    check("wide", 0x53, 40, 480, 14.0, 40.0);
}

#[test]
fn mixed_strokes_match_cpu_large_canvas() {
    check("mixed-900", 0x54, 120, 900, 0.5, 40.0);
}

#[test]
fn many_seeds_match_cpu() {
    for seed in 0..6u64 {
        check(&format!("seed{seed}"), 0x1000 + seed, 40, 480, 0.5, 40.0);
    }
}
