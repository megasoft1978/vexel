//! Ignored timing test: `cargo test -p vexel-gpu-scene --release --lib bench
//! -- --ignored --nocapture`. Prints encode (whole, including binning) and bin
//! alone for N shapes of ~6 edges on a 1920x1080 canvas.

use crate::bin::{FixedBox, bin_tiles};
use crate::classify::classify;
use crate::encode::encode;
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
        let mut best_encode = f64::MAX;
        let mut encoded = None;
        for _ in 0..5 {
            let t = Instant::now();
            let e = encode(&s).unwrap();
            best_encode = best_encode.min(t.elapsed().as_secs_f64() * 1e3);
            encoded = Some(e);
        }
        let e = encoded.unwrap();
        let boxes: Vec<FixedBox> = e
            .shapes
            .iter()
            .map(|sh| {
                sh.bbox
                    .map(|v| (v * (1 << crate::layout::EDGE_FIXED_SHIFT) as f32) as i32)
            })
            .collect();
        let mut best_bin = f64::MAX;
        for _ in 0..5 {
            let t = Instant::now();
            let r = bin_tiles(&boxes, e.header.tiles_x, e.header.tiles_y).unwrap();
            best_bin = best_bin.min(t.elapsed().as_secs_f64() * 1e3);
            assert_eq!(r.1.len(), e.tile_list.len());
        }
        println!(
            "shapes={count:6} edges={:7} entries={:7} classify={classify_ms:.2}ms encode(total)={best_encode:.2}ms bin={best_bin:.2}ms",
            e.edges.len(),
            e.tile_list.len()
        );
    }
}
