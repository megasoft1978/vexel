//! Per-tile edge clipping must not change a single byte of the rendered
//! frame, and must cut the edge visits of large shapes.

use vexel_gpu_scene::input::{GpuPaint, GpuScene, GpuSegment};
use vexel_gpu_scene::{Encoded, encode, encode_unclipped, reference};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / f64::from(1u32 << 31)
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

/// A big polygon (`n` vertices, jittered radius), optionally a self-crossing
/// star (every `step`-th vertex), centred anywhere including off canvas.
fn big_shape(rng: &mut Lcg, w: f64, h: f64, n: usize, step: usize) -> Vec<GpuSegment> {
    let (cx, cy) = (rng.range(-0.2 * w, 1.2 * w), rng.range(-0.2 * h, 1.2 * h));
    let r = rng.range(0.3 * w, 1.1 * w);
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let a = k as f64 / n as f64 * std::f64::consts::TAU;
            let rr = r * rng.range(0.7, 1.0);
            (cx + rr * a.cos(), cy + rr * a.sin())
        })
        .collect();
    let mut order = Vec::new();
    let mut k = 0;
    for _ in 0..n {
        order.push(k);
        k = (k + step) % n;
    }
    (0..n)
        .map(|i| {
            let (a, b) = (pts[order[i]], pts[order[(i + 1) % n]]);
            GpuSegment {
                x0: a.0,
                y0: a.1,
                x1: b.0,
                y1: b.1,
            }
        })
        .collect()
}

fn visits(e: &Encoded) -> u64 {
    e.tile_list.iter().map(|t| u64::from(t.edge_count)).sum()
}

#[test]
fn clipped_equals_unclipped_exactly() {
    let (w, h) = (400u32, 300u32);
    for seed in 1..=4u64 {
        let mut rng = Lcg(seed);
        let mut scene = GpuScene::new(w, h);
        for i in 0..8 {
            let n = 20 + (rng.next() * 60.0) as usize;
            // Step 1: simple ring; step 2 (n even): self-crossing double loop.
            let step = if i % 3 == 2 && n % 2 == 1 { 2 } else { 1 };
            let edges = big_shape(&mut rng, f64::from(w), f64::from(h), n, step);
            scene.push_fill(
                edges,
                i % 2 == 0,
                GpuPaint::Solid([(i * 20) as u8, 90, 200, 140 + i as u8]),
            );
        }
        let clipped = encode(&scene).unwrap();
        let unclipped = encode_unclipped(&scene).unwrap();
        assert_eq!(
            reference::render_straight(&clipped),
            reference::render_straight(&unclipped),
            "seed {seed}"
        );
        let (c, u) = (visits(&clipped), visits(&unclipped));
        println!(
            "seed {seed}: entries {} vs {}, edge visits clipped={c} unclipped={u} ({:.1}x)",
            clipped.tile_list.len(),
            unclipped.tile_list.len(),
            u as f64 / c as f64
        );
        assert!(c * 4 < u, "work should drop at least 4x: {c} vs {u}");
        assert!(clipped.tile_list.len() <= unclipped.tile_list.len());
    }
}

/// A single large ring over many tiles: every tile keeps few edges.
#[test]
fn large_ring_work_drops() {
    let n = 600usize;
    let mut scene = GpuScene::new(512, 384);
    let edges: Vec<GpuSegment> = (0..n)
        .map(|k| {
            let p = |k: usize| {
                let a = (k % n) as f64 / n as f64 * std::f64::consts::TAU;
                (256.0 + 240.0 * a.cos(), 192.0 + 180.0 * a.sin())
            };
            let (a, b) = (p(k), p(k + 1));
            GpuSegment {
                x0: a.0,
                y0: a.1,
                x1: b.0,
                y1: b.1,
            }
        })
        .collect();
    scene.push_fill(edges, false, GpuPaint::Solid([200, 30, 30, 255]));
    let clipped = encode(&scene).unwrap();
    let unclipped = encode_unclipped(&scene).unwrap();
    assert_eq!(
        reference::render_straight(&clipped),
        reference::render_straight(&unclipped)
    );
    let (c, u) = (visits(&clipped), visits(&unclipped));
    println!(
        "ring: visits clipped={c} unclipped={u} ({:.0}x)",
        u as f64 / c as f64
    );
    assert!(c * 50 < u);
}
