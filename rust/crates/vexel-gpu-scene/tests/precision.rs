//! Edge representation study (ADR-0035 addendum): mismatch rate of samples
//! next to an edge crossing, against exact `f64` crossings for fixed point at several shifts and for
//! `f32` coordinates relative to a per-shape origin. Run with
//! `cargo test -p vexel-gpu-scene --release --test precision -- --ignored --nocapture`.

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

type Ring = Vec<(f64, f64)>;

fn ring(rng: &mut Lcg, cx: f64, cy: f64, r: f64) -> Ring {
    let n = 12;
    (0..n)
        .map(|k| {
            let a = (f64::from(k) + rng.next() * 0.8) / f64::from(n) * std::f64::consts::TAU;
            let rr = r * (0.4 + 0.6 * rng.next());
            (cx + rr * a.cos(), cy + rr * a.sin())
        })
        .collect()
}

/// Non-zero winding at (px, py) from a crossing predicate; edges as in the CPU.
fn winding(ring: &Ring, mut cross_right: impl FnMut(usize, usize) -> Option<bool>) -> bool {
    let mut w = 0i32;
    for k in 0..ring.len() {
        if let Some(right) = cross_right(k, (k + 1) % ring.len()) {
            if right {
                w += if ring[(k + 1) % ring.len()].1 > ring[k].1 {
                    1
                } else {
                    -1
                };
            }
        }
    }
    w != 0
}

#[test]
#[ignore]
fn edge_representation_study() {
    println!(
        "radius  samples      shift8   shift10  shift12  shift14  f32-rel   (mismatching near-edge samples per million)"
    );
    for radius in [40.0, 400.0, 4000.0] {
        let mut rng = Lcg(5);
        let mut bad = [0u64; 5];
        let mut total = 0u64;
        for _ in 0..40 {
            let (cx, cy) = (rng.next() * 5000.0, rng.next() * 5000.0);
            let r = ring(&mut rng, cx, cy, radius);
            let (ox, oy) = r
                .iter()
                .fold((f64::MAX, f64::MAX), |a, p| (a.0.min(p.0), a.1.min(p.1)));
            // Sample points: for every edge and every sample row it spans, the two
            // lattice columns (quarter pixel, offset 1/8) nearest its crossing.
            let mut points: Vec<(f64, f64)> = Vec::new();
            for k in 0..r.len() {
                let (p, q) = (r[k], r[(k + 1) % r.len()]);
                let (lo, hi) = (p.1.min(q.1), p.1.max(q.1));
                let mut row = ((lo * 4.0 - 0.5).ceil()).max(f64::MIN);
                while (row + 0.5) / 4.0 < hi {
                    let py = (row + 0.5) / 4.0;
                    let x = p.0 + (py - p.1) * (q.0 - p.0) / (q.1 - p.1);
                    let col = (x * 4.0 - 0.5).floor();
                    points.push(((col + 0.5) / 4.0, py));
                    points.push(((col + 1.5) / 4.0, py));
                    row += 1.0;
                }
            }
            for (px, py) in points {
                {
                    total += 1;
                    let exact = winding(&r, |a, b| {
                        let (p, q) = (r[a], r[b]);
                        let spans = (p.1 <= py && q.1 > py) || (q.1 <= py && p.1 > py);
                        spans.then(|| {
                            let x = p.0 + (py - p.1) * (q.0 - p.0) / (q.1 - p.1);
                            x > px
                        })
                    });
                    for (i, shift) in [8u32, 10, 12, 14].into_iter().enumerate() {
                        let u = f64::from(1u32 << shift);
                        let fx = |v: f64| (v * u).round() as i64;
                        let (sx, sy) = (fx(px), fx(py));
                        let got = winding(&r, |a, b| {
                            let (x0, y0, x1, y1) = (fx(r[a].0), fx(r[a].1), fx(r[b].0), fx(r[b].1));
                            let spans = (y0 <= sy && y1 > sy) || (y1 <= sy && y0 > sy);
                            spans.then(|| {
                                let dy = y1 - y0;
                                let num = (x0 - sx) * dy + (sy - y0) * (x1 - x0);
                                (dy > 0 && num > 0) || (dy < 0 && num < 0)
                            })
                        });
                        bad[i] += u64::from(got != exact);
                    }
                    let rel = |x: f64, o: f64| (x - o) as f32;
                    let (spx, spy) = (rel(px, ox), rel(py, oy));
                    let got = winding(&r, |a, b| {
                        let (x0, y0) = (rel(r[a].0, ox), rel(r[a].1, oy));
                        let (x1, y1) = (rel(r[b].0, ox), rel(r[b].1, oy));
                        let spans = (y0 <= spy && y1 > spy) || (y1 <= spy && y0 > spy);
                        spans.then(|| x0 + (spy - y0) * ((x1 - x0) / (y1 - y0)) > spx)
                    });
                    bad[4] += u64::from(got != exact);
                }
            }
        }
        let ppm = |n: u64| n as f64 * 1e6 / total as f64;
        println!(
            "{radius:6}  {total:9}  {:8.1} {:8.1} {:8.1} {:8.1} {:8.1}",
            ppm(bad[0]),
            ppm(bad[1]),
            ppm(bad[2]),
            ppm(bad[3]),
            ppm(bad[4])
        );
    }
}
