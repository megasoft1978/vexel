//! Deterministic k-means++ colour quantisation.

use super::{MAX_COLOURS, Rgba, TraceError};

/// Label of transparent pixels.
pub const NONE: u8 = 255;
const SAMPLE_CAP: usize = 60_000;
const ITERATIONS: usize = 14;

/// Palette plus one label per pixel (`NONE` where the pixel is transparent).
#[derive(Clone, Debug)]
pub struct Quantized {
    pub width: usize,
    pub height: usize,
    pub palette: Vec<[u8; 3]>,
    pub labels: Vec<u8>,
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn dist2(a: [i32; 3], b: [i32; 3]) -> i64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    i64::from(d[0] * d[0] + d[1] * d[1] + d[2] * d[2])
}

fn nearest(c: &[[i32; 3]], p: [i32; 3]) -> usize {
    let mut best = (i64::MAX, 0);
    for (i, q) in c.iter().enumerate() {
        let d = dist2(*q, p);
        if d < best.0 {
            best = (d, i);
        }
    }
    best.1
}

/// Quantises the opaque pixels (alpha >= 128) of `img` to at most `k` colours.
/// k-means++ seeding from `seed`, then Lloyd iterations on an even sample;
/// every pixel takes its nearest centre and the palette becomes the mean of
/// its members. A 3x3 majority pass removes single-pixel anti-aliasing noise.
pub fn quantize(img: &Rgba<'_>, k: usize, seed: u64) -> Result<Quantized, TraceError> {
    if k == 0 || k > MAX_COLOURS {
        return Err(TraceError::BadColours);
    }
    let n = img.width * img.height;
    let opaque = |i: usize| img.data[i * 4 + 3] >= 128;
    let rgb = |i: usize| {
        [
            i32::from(img.data[i * 4]),
            i32::from(img.data[i * 4 + 1]),
            i32::from(img.data[i * 4 + 2]),
        ]
    };
    let total = (0..n).filter(|&i| opaque(i)).count();
    let mut labels = vec![NONE; n];
    if total == 0 {
        return Ok(Quantized {
            width: img.width,
            height: img.height,
            palette: Vec::new(),
            labels,
        });
    }
    let stride = total.div_ceil(SAMPLE_CAP).max(1);
    let sample: Vec<[i32; 3]> = (0..n)
        .filter(|&i| opaque(i))
        .step_by(stride)
        .map(rgb)
        .collect();
    let mut rng = Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1);
    let mut centres: Vec<[i32; 3]> = vec![sample[(rng.next() % sample.len() as u64) as usize]];
    let mut mind: Vec<i64> = sample.iter().map(|&p| dist2(p, centres[0])).collect();
    while centres.len() < k.min(sample.len()) {
        let sum: f64 = mind.iter().map(|&d| d as f64).sum();
        if sum <= 0.0 {
            break;
        }
        let target = rng.unit() * sum;
        let (mut acc, mut pick) = (0.0, sample.len() - 1);
        for (i, &d) in mind.iter().enumerate() {
            acc += d as f64;
            if acc > target {
                pick = i;
                break;
            }
        }
        let c = sample[pick];
        centres.push(c);
        for (m, &p) in mind.iter_mut().zip(&sample) {
            *m = (*m).min(dist2(p, c));
        }
    }
    for _ in 0..ITERATIONS {
        let mut sums = vec![[0i64; 4]; centres.len()];
        for &p in &sample {
            let c = nearest(&centres, p);
            sums[c][0] += i64::from(p[0]);
            sums[c][1] += i64::from(p[1]);
            sums[c][2] += i64::from(p[2]);
            sums[c][3] += 1;
        }
        let mut moved = false;
        for (c, s) in centres.iter_mut().zip(&sums) {
            if s[3] > 0 {
                let m = [
                    (s[0] / s[3]) as i32,
                    (s[1] / s[3]) as i32,
                    (s[2] / s[3]) as i32,
                ];
                moved |= m != *c;
                *c = m;
            }
        }
        if !moved {
            break;
        }
    }
    for i in 0..n {
        if opaque(i) {
            labels[i] = nearest(&centres, rgb(i)) as u8;
        }
    }
    majority(&mut labels, img.width, img.height);
    let mut sums = vec![[0u64; 4]; centres.len()];
    for (i, &l) in labels.iter().enumerate() {
        if l != NONE {
            let p = rgb(i);
            let s = &mut sums[usize::from(l)];
            s[0] += p[0] as u64;
            s[1] += p[1] as u64;
            s[2] += p[2] as u64;
            s[3] += 1;
        }
    }
    let palette = sums
        .iter()
        .zip(&centres)
        .map(|(s, c)| {
            if s[3] == 0 {
                [c[0] as u8, c[1] as u8, c[2] as u8]
            } else {
                [
                    (s[0] / s[3]) as u8,
                    (s[1] / s[3]) as u8,
                    (s[2] / s[3]) as u8,
                ]
            }
        })
        .collect();
    Ok(Quantized {
        width: img.width,
        height: img.height,
        palette,
        labels,
    })
}

/// One 3x3 majority pass; a label changes only when another label holds a
/// dominant majority (7+ of the 9 cells), so real edges and corners survive.
fn majority(labels: &mut [u8], w: usize, h: usize) {
    let src = labels.to_vec();
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let mut cell = [0u8; 9];
            let mut i = 0;
            for yy in y - 1..=y + 1 {
                for xx in x - 1..=x + 1 {
                    cell[i] = src[yy * w + xx];
                    i += 1;
                }
            }
            let cur = cell[4];
            if cur == NONE {
                continue;
            }
            for &cand in &cell {
                if cand != cur && cand != NONE && cell.iter().filter(|&&c| c == cand).count() >= 7 {
                    labels[y * w + x] = cand;
                    break;
                }
            }
        }
    }
}
