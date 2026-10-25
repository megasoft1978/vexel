//! Ignored timing test: `cargo test -p vexel-gpu-scene --release --lib bench
//! -- --ignored --nocapture`. Prints encode (whole, including binning) and bin
//! alone for N shapes of ~6 edges on a 1920x1080 canvas.

use crate::classify::classify;
use crate::encode::{Encoded, Encoder, encode, encode_timed};
use crate::input::{GpuColorStop, GpuGradient, GpuPaint, GpuScene, GpuSegment, GpuSpread};
use std::time::Instant;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / f64::from(1u32 << 31)
    }
}

fn scene(count: usize) -> GpuScene {
    let mut rng = Lcg(7);
    let mut scene = GpuScene::new(1920, 1080);
    for i in 0..count {
        let (cx, cy) = (rng.next() * 1920.0, rng.next() * 1080.0);
        let r = 3.0 + rng.next() * 30.0;
        let n = 6;
        let pts: Vec<(f64, f64)> = (0..n)
            .map(|k| {
                let a = (f64::from(k) + rng.next() * 0.5) / f64::from(n) * std::f64::consts::TAU;
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        let edges = (0..n as usize)
            .map(|k| GpuSegment {
                x0: pts[k].0,
                y0: pts[k].1,
                x1: pts[(k + 1) % n as usize].0,
                y1: pts[(k + 1) % n as usize].1,
            })
            .collect();
        let paint = if i % 8 == 0 {
            GpuPaint::Linear(GpuGradient {
                inverse: [0.01, 0.0, 0.0, 0.0, 0.0, 0.0],
                spread: GpuSpread::Repeat,
                stops: vec![
                    GpuColorStop {
                        offset: 0.0,
                        rgba: [255, 0, 0, 255],
                    },
                    GpuColorStop {
                        offset: 1.0,
                        rgba: [0, 0, 255, 255],
                    },
                ],
                focal_ratio: 0.0,
            })
        } else {
            GpuPaint::Solid([(i % 251) as u8, 90, 200, 255])
        };
        scene.push_fill(edges, i % 3 == 0, paint);
    }
    scene
}

#[test]
#[ignore]
fn bench_encode_and_bin() {
    for count in [500, 2000, 5000, 20000] {
        let s = scene(count);
        let t = Instant::now();
        assert!(classify(&s).is_supported());
        let classify_ms = t.elapsed().as_secs_f64() * 1e3;
        let (mut best_encode, mut best_bin) = (f64::MAX, f64::MAX);
        let mut encoded = None;
        let (mut warm, mut best_warm) = (Encoder::default(), f64::MAX);
        let mut reused = Encoded::default();
        for _ in 0..9 {
            let t = Instant::now();
            warm.encode_into(&s, &mut reused).unwrap();
            best_warm = best_warm.min(t.elapsed().as_secs_f64() * 1e3);
            let t = Instant::now();
            let e = encode(&s).unwrap();
            best_encode = best_encode.min(t.elapsed().as_secs_f64() * 1e3);
            encoded = Some(e);
            // Binning share, measured with per-shape timers (adds overhead).
            let (_, bin) = encode_timed(&s).unwrap();
            best_bin = best_bin.min(bin.as_secs_f64() * 1e3);
        }
        let e = encoded.unwrap();
        assert_eq!(e, reused);
        println!(
            "shapes={count:6} edges={:7} entries={:7} classify={classify_ms:.2}ms encode(cold, incl. bin)={best_encode:.2}ms encode(warm, buffers reused)={best_warm:.2}ms bin(timer overhead incl.)={best_bin:.2}ms",
            e.edges.len(),
            e.tile_list.len()
        );
    }
}

/// One huge shape (a 2000-edge ring over a 1920x1080 canvas): clipped work
/// against the unclipped per-tile edge visits.
#[test]
#[ignore]
fn bench_large_shape_work() {
    let n = 2000usize;
    let ring: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let a = k as f64 / n as f64 * std::f64::consts::TAU;
            (960.0 + 900.0 * a.cos(), 540.0 + 500.0 * a.sin())
        })
        .collect();
    let edges = (0..n)
        .map(|k| GpuSegment {
            x0: ring[k].0,
            y0: ring[k].1,
            x1: ring[(k + 1) % n].0,
            y1: ring[(k + 1) % n].1,
        })
        .collect();
    let mut scene = GpuScene::new(1920, 1080);
    scene.push_fill(edges, false, GpuPaint::Solid([9, 9, 9, 255]));
    let t = Instant::now();
    let clipped = encode(&scene).unwrap();
    let clipped_ms = t.elapsed().as_secs_f64() * 1e3;
    // Unclipped visits: every source edge in every tile of the shape's box
    // (the unclipped encoding itself overflows the edge pool here).
    let shape = &clipped.shapes[0];
    let fixed = shape
        .bbox
        .map(|v| (v * (1 << crate::layout::EDGE_FIXED_SHIFT) as f32) as i32);
    let r = crate::bin::tile_range(fixed, clipped.header.tiles_x, clipped.header.tiles_y).unwrap();
    let tiles = u64::from(r[2] - r[0] + 1) * u64::from(r[3] - r[1] + 1);
    let unclipped = u64::from(shape.edge_count) * tiles;
    let clipped_visits: u64 = clipped
        .tile_list
        .iter()
        .map(|t| u64::from(t.edge_count))
        .sum();
    println!(
        "ring {n} edges over {tiles} tiles: entries={} edge visits clipped={clipped_visits} unclipped={unclipped} ({:.0}x) encode={clipped_ms:.2}ms",
        clipped.tile_list.len(),
        unclipped as f64 / clipped_visits as f64
    );
}
