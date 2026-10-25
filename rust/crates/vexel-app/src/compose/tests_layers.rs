//! Preview layer cache: accuracy against the exact renderer, exact paths
//! untouched, the byte budget, and filtered / blended groups never resampled.

use super::{DiffCache, compose_document, compose_document_cached};
use std::fmt::Write as _;
use std::sync::Arc;

/// A display cache (written so the display-cache source guard does not match tests).
fn display_cache() -> DiffCache {
    <DiffCache>::display()
}

const SIZE: u32 = 300;
const FRAMES: usize = 10;

/// `n` large curved shapes under a camera group translated by `t`, plus a
/// small shape outside it. `wrap` wraps the camera group's content.
fn scene(t: f64, n: usize, size: f64, wrap: (&str, &str)) -> Vec<u8> {
    let mut s = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300\" height=\"300\" viewBox=\"0 0 300 300\">",
    );
    let _ = write!(s, "<g transform=\"translate({t} {})\">{}", t * 0.37, wrap.0);
    for i in 0..n {
        let (x, y) = (
            f64::from((i % 5) as u32) * 30.0,
            f64::from((i / 5) as u32) * 20.0,
        );
        let _ = write!(
            s,
            "<path d=\"M{x} {y} C{} {} {} {} {} {} S{} {} {} {} Z\" fill=\"#{:02x}66cc\" stroke=\"#112244\" stroke-width=\"1.5\"/>",
            x + size * 0.5,
            y - 10.0,
            x + size,
            y + size * 0.4,
            x + size * 0.8,
            y + size,
            x + size * 0.3,
            y + size * 1.2,
            x - 10.0,
            y + size * 0.5,
            (i * 23) % 256
        );
    }
    let _ = write!(
        s,
        "{}</g><rect x=\"10\" y=\"10\" width=\"20\" height=\"20\" fill=\"#ff0000\"/></svg>",
        wrap.1
    );
    s.into_bytes()
}

fn frames(wrap: (&str, &str), n: usize, size: f64) -> Vec<Arc<Vec<u8>>> {
    (0..FRAMES)
        .map(|i| Arc::new(scene(f64::from(i as u32) * 1.3 + 0.4, n, size, wrap)))
        .collect()
}

fn error(a: &[u8], b: &[u8]) -> (f64, f64) {
    let total: u64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    let far = a
        .chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(p, q)| p.iter().zip(*q).any(|(x, y)| x.abs_diff(*y) > 64))
        .count();
    (
        total as f64 / a.len() as f64,
        far as f64 / (a.len() / 4) as f64,
    )
}

fn run_display(cache: &mut DiffCache, docs: &[Arc<Vec<u8>>]) -> Vec<Vec<u8>> {
    docs.iter()
        .map(|d| {
            compose_document_cached(d.clone(), SIZE, SIZE, &|| false, cache)
                .unwrap()
                .frame
                .rgba
        })
        .collect()
}

fn exact(docs: &[Arc<Vec<u8>>]) -> Vec<Vec<u8>> {
    docs.iter()
        .map(|d| {
            compose_document(d, SIZE, SIZE, &|| false)
                .unwrap()
                .frame
                .rgba
        })
        .collect()
}

#[test]
fn preview_stays_within_tolerance_of_the_exact_render() {
    let docs = frames(("", ""), 40, 120.0);
    let reference = exact(&docs);
    let mut cache = display_cache();
    cache.set_preview(true);
    let got = run_display(&mut cache, &docs);
    let stats = cache.layer_stats().unwrap();
    assert!(stats.reused > 0, "layers were never reused: {stats:?}");
    let (mut worst_mean, mut worst_far) = (0.0f64, 0.0f64);
    for (a, b) in got.iter().zip(&reference) {
        let (mean, far) = error(a, b);
        worst_mean = worst_mean.max(mean);
        worst_far = worst_far.max(far);
    }
    // Tolerance: mean error under 2/255 per channel, under 2% of pixels off by more than 64.
    assert!(worst_mean < 2.0, "mean error {worst_mean}");
    assert!(worst_far < 0.02, "far fraction {worst_far}");
}

#[test]
fn exact_paths_ignore_the_cache() {
    let docs = frames(("", ""), 40, 120.0);
    let reference = exact(&docs);
    // A default cache has no layers at all.
    let mut plain = DiffCache::default();
    assert!(plain.layer_stats().is_none());
    assert_eq!(run_display_plain(&mut plain, &docs), reference);
    // A display cache that is not in preview mode renders exactly.
    let mut still = display_cache();
    assert_eq!(run_display(&mut still, &docs), reference);
    assert_eq!(still.layer_stats().unwrap().built, 0);
}

fn run_display_plain(cache: &mut DiffCache, docs: &[Arc<Vec<u8>>]) -> Vec<Vec<u8>> {
    run_display(cache, docs)
}

#[test]
fn layer_bytes_stay_inside_the_budget() {
    let docs = frames(("", ""), 40, 120.0);
    let mut cache = display_cache();
    cache.set_preview(true);
    cache.set_layer_budget(1);
    run_display(&mut cache, &docs);
    let stats = cache.layer_stats().unwrap();
    assert!(stats.bytes <= 1 && stats.peak_bytes <= 1, "{stats:?}");
    assert!(stats.rejected > 0, "{stats:?}");
    // A budget that fits one layer keeps the total at or below it.
    let mut cache = display_cache();
    cache.set_preview(true);
    let budget = 2_000_000;
    cache.set_layer_budget(budget);
    run_display(&mut cache, &docs);
    let stats = cache.layer_stats().unwrap();
    assert!(stats.peak_bytes <= budget, "{stats:?}");
}

#[test]
fn filtered_and_blended_groups_are_not_resampled() {
    for wrap in [
        (
            "<g filter=\"url(#f)\">",
            "</g><defs><filter id=\"f\" filterUnits=\"userSpaceOnUse\" x=\"-50\" y=\"-50\" width=\"500\" height=\"500\"><feGaussianBlur stdDeviation=\"2\"/></filter></defs>",
        ),
        ("<g style=\"mix-blend-mode:multiply\">", "</g>"),
    ] {
        // Small content: no run is worth caching, so every pixel is exact.
        let docs = frames(wrap, 3, 6.0);
        let reference = exact(&docs);
        let mut cache = display_cache();
        cache.set_preview(true);
        assert_eq!(run_display(&mut cache, &docs), reference);
        let stats = cache.layer_stats().unwrap();
        assert_eq!((stats.built, stats.reused), (0, 0), "{stats:?}");
    }
}
