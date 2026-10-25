//! Polyline simplification (Douglas-Peucker), corner detection and cubic
//! Bezier fitting (Schneider, Graphics Gems 1990, public algorithm).
//! Only `sqrt` and basic arithmetic are used.

pub type P = [f64; 2];

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: P, b: P) -> P {
    [a[0] + b[0], a[1] + b[1]]
}
fn mul(a: P, s: f64) -> P {
    [a[0] * s, a[1] * s]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn len(a: P) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: P) -> P {
    let l = len(a);
    if l < 1e-12 {
        [0.0, 0.0]
    } else {
        mul(a, 1.0 / l)
    }
}

/// Distance from `p` to the segment a-b.
fn seg_dist(p: P, a: P, b: P) -> f64 {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    if l2 < 1e-18 {
        return len(sub(p, a));
    }
    let t = (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0);
    len(sub(p, add(a, mul(ab, t))))
}

/// Douglas-Peucker on the closed polygon `pts`; returns kept indices, sorted.
pub fn simplify_closed(pts: &[P], eps: f64) -> Vec<usize> {
    let n = pts.len();
    if n < 4 {
        return (0..n).collect();
    }
    let far = (1..n)
        .max_by(|&a, &b| {
            len(sub(pts[a], pts[0]))
                .total_cmp(&len(sub(pts[b], pts[0])))
                .then(b.cmp(&a))
        })
        .unwrap_or(1);
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[far] = true;
    for (lo, hi) in [(0, far), (far, n)] {
        let mut stack = vec![(lo, hi)];
        while let Some((l, h)) = stack.pop() {
            let (a, b) = (pts[l], pts[h % n]);
            let mut best = (0.0, 0);
            for i in l + 1..h {
                let d = seg_dist(pts[i], a, b);
                if d > best.0 {
                    best = (d, i);
                }
            }
            if best.0 > eps {
                keep[best.1] = true;
                stack.push((l, best.1));
                stack.push((best.1, h));
            }
        }
    }
    (0..n).filter(|&i| keep[i]).collect()
}

/// Arc reach (pixels) used to measure how sharply the outline turns.
const REACH: f64 = 7.0;

/// Dense index `k` steps from `i` (cyclic) that is first at least `REACH`
/// away, walking `dir` (+1 or -1) but never more than half the loop.
fn reach(pts: &[P], i: usize, dir: isize) -> usize {
    let n = pts.len();
    let mut j = i;
    for _ in 0..n / 2 {
        j = (j as isize + dir).rem_euclid(n as isize) as usize;
        if len(sub(pts[j], pts[i])) >= REACH {
            break;
        }
    }
    j
}

/// Dense indices among `kept` where the outline turns by more than the angle
/// whose cosine is `cos_limit`, measured over a few pixels so staircase noise
/// does not count; only the sharpest vertex of a cluster is kept.
pub fn corners(pts: &[P], kept: &[usize], cos_limit: f64) -> Vec<usize> {
    let mut cand: Vec<(f64, usize)> = Vec::new();
    for &i in kept {
        let a = sub(pts[i], pts[reach(pts, i, -1)]);
        let b = sub(pts[reach(pts, i, 1)], pts[i]);
        let (la, lb) = (len(a), len(b));
        if la > 1e-9 && lb > 1e-9 {
            let c = dot(a, b) / (la * lb);
            if c < cos_limit {
                cand.push((c, i));
            }
        }
    }
    cand.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
    let mut out: Vec<usize> = Vec::new();
    for (_, i) in cand {
        if out.iter().all(|&o| len(sub(pts[o], pts[i])) >= REACH) {
            out.push(i);
        }
    }
    out.sort_unstable();
    out
}

pub type Bez = [P; 4];

/// A path segment ready to print.
pub enum Seg {
    Line(P),
    Curve(P, P, P),
}

/// Fits the closed outline `pts` (dense, already collinear-free) and returns
/// the start point plus segments, error within `tol` pixels.
pub fn fit_closed(pts: &[P], tol: f64, cos_corner: f64) -> (P, Vec<Seg>) {
    let n = pts.len();
    let (mut lo, mut hi) = (pts[0], pts[0]);
    for p in pts {
        lo = [lo[0].min(p[0]), lo[1].min(p[1])];
        hi = [hi[0].max(p[0]), hi[1].max(p[1])];
    }
    if (hi[0] - lo[0]).max(hi[1] - lo[1]) < 3.0 * REACH {
        // Too small to fit: keep the lattice outline as a polygon.
        return (pts[0], pts[1..].iter().map(|&p| Seg::Line(p)).collect());
    }
    let kept = simplify_closed(pts, (tol * 0.7).max(0.5));
    let mut breaks = corners(pts, &kept, cos_corner);
    let smooth_loop = breaks.is_empty();
    if smooth_loop {
        breaks = vec![0, n / 2];
    }
    let mut segs = Vec::new();
    let nb = breaks.len();
    for bi in 0..nb {
        let (i, j) = (breaks[bi], breaks[(bi + 1) % nb]);
        let count = if j > i { j - i } else { j + n - i } + 1;
        if count < 2 {
            continue;
        }
        let piece: Vec<P> = (0..count).map(|k| pts[(i + k) % n]).collect();
        let corner_start = !smooth_loop;
        let k = 3.min(count - 1).max(1);
        let t1 = if corner_start {
            unit(sub(piece[k], piece[0]))
        } else {
            unit(sub(pts[(i + k) % n], pts[(i + n - k) % n]))
        };
        let t2 = if corner_start {
            unit(sub(piece[count - 1 - k], piece[count - 1]))
        } else {
            unit(sub(pts[(j + n - k) % n], pts[(j + k) % n]))
        };
        fit_piece(&piece, t1, t2, tol, &mut segs);
    }
    (pts[breaks[0]], segs)
}

fn fit_piece(piece: &[P], t1: P, t2: P, tol: f64, out: &mut Vec<Seg>) {
    let end = piece[piece.len() - 1];
    let a = piece[0];
    if piece
        .iter()
        .all(|&p| seg_dist(p, a, end) <= tol * 0.5 + 0.2)
    {
        out.push(Seg::Line(end));
        return;
    }
    let mut beziers = Vec::new();
    // The lattice outline zigzags up to ~0.75 px around the true edge.
    let fit_tol = tol + 0.75;
    fit_cubic(piece, t1, t2, fit_tol * fit_tol, 0, &mut beziers);
    for b in beziers {
        out.push(Seg::Curve(b[1], b[2], b[3]));
    }
}

fn fit_cubic(pts: &[P], t1: P, t2: P, err2: f64, depth: u32, out: &mut Vec<Bez>) {
    let n = pts.len();
    let (p0, p3) = (pts[0], pts[n - 1]);
    if n == 2 || depth > 20 {
        let d = len(sub(p3, p0)) / 3.0;
        out.push([p0, add(p0, mul(t1, d)), add(p3, mul(t2, d)), p3]);
        return;
    }
    let mut u = chord_params(pts);
    let mut bez = generate(pts, &u, t1, t2);
    let (mut max_err, mut split) = max_error(pts, &bez, &u);
    if max_err < err2 {
        out.push(bez);
        return;
    }
    if max_err < err2 * 16.0 {
        for _ in 0..4 {
            u = reparam(pts, &u, &bez);
            bez = generate(pts, &u, t1, t2);
            (max_err, split) = max_error(pts, &bez, &u);
            if max_err < err2 {
                out.push(bez);
                return;
            }
        }
    }
    let split = split.clamp(1, n - 2);
    let tc = unit(sub(pts[split - 1], pts[split + 1]));
    fit_cubic(&pts[..=split], t1, tc, err2, depth + 1, out);
    fit_cubic(&pts[split..], mul(tc, -1.0), t2, err2, depth + 1, out);
}

fn chord_params(pts: &[P]) -> Vec<f64> {
    let mut u = vec![0.0; pts.len()];
    for i in 1..pts.len() {
        u[i] = u[i - 1] + len(sub(pts[i], pts[i - 1]));
    }
    let total = u[pts.len() - 1];
    for (i, v) in u.iter_mut().enumerate() {
        *v = if total > 0.0 {
            *v / total
        } else {
            i as f64 / (pts.len() - 1) as f64
        };
    }
    u
}

fn basis(t: f64) -> [f64; 4] {
    let m = 1.0 - t;
    [m * m * m, 3.0 * t * m * m, 3.0 * t * t * m, t * t * t]
}

fn eval(b: &Bez, t: f64) -> P {
    let w = basis(t);
    let mut r = [0.0, 0.0];
    for i in 0..4 {
        r = add(r, mul(b[i], w[i]));
    }
    r
}

fn generate(pts: &[P], u: &[f64], t1: P, t2: P) -> Bez {
    let (p0, p3) = (pts[0], pts[pts.len() - 1]);
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, &t) in u.iter().enumerate() {
        let w = basis(t);
        let a0 = mul(t1, w[1]);
        let a1 = mul(t2, w[2]);
        c00 += dot(a0, a0);
        c01 += dot(a0, a1);
        c11 += dot(a1, a1);
        let tmp = sub(pts[i], add(mul(p0, w[0] + w[1]), mul(p3, w[2] + w[3])));
        x0 += dot(a0, tmp);
        x1 += dot(a1, tmp);
    }
    let det = c00 * c11 - c01 * c01;
    let (al, ar) = if det.abs() < 1e-12 {
        (0.0, 0.0)
    } else {
        ((x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det)
    };
    let seg = len(sub(p3, p0));
    if al < 1e-6 * seg || ar < 1e-6 * seg {
        let d = seg / 3.0;
        [p0, add(p0, mul(t1, d)), add(p3, mul(t2, d)), p3]
    } else {
        [p0, add(p0, mul(t1, al)), add(p3, mul(t2, ar)), p3]
    }
}

fn max_error(pts: &[P], b: &Bez, u: &[f64]) -> (f64, usize) {
    let mut best = (0.0, pts.len() / 2);
    for i in 1..pts.len() - 1 {
        let d = sub(eval(b, u[i]), pts[i]);
        let e = dot(d, d);
        if e > best.0 {
            best = (e, i);
        }
    }
    best
}

fn reparam(pts: &[P], u: &[f64], b: &Bez) -> Vec<f64> {
    let d1: Vec<P> = (0..3).map(|i| mul(sub(b[i + 1], b[i]), 3.0)).collect();
    let d2: Vec<P> = (0..2).map(|i| mul(sub(d1[i + 1], d1[i]), 2.0)).collect();
    u.iter()
        .zip(pts)
        .map(|(&t, &p)| {
            let m = 1.0 - t;
            let q1 = add(
                add(mul(d1[0], m * m), mul(d1[1], 2.0 * m * t)),
                mul(d1[2], t * t),
            );
            let q2 = add(mul(d2[0], m), mul(d2[1], t));
            let diff = sub(eval(b, t), p);
            let num = dot(diff, q1);
            let den = dot(q1, q1) + dot(diff, q2);
            if den.abs() < 1e-12 {
                t
            } else {
                (t - num / den).clamp(0.0, 1.0)
            }
        })
        .collect()
}
