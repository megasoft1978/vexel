//! Seeded colour segmentation, a bounded GrabCut-style alternative: a small
//! k-means colour model of the foreground is grown from the seed points by
//! flood fill, then refined from the grown region a few times. Polygons
//! restrict (`include`) or forbid (`exclude`) areas.

use super::{Rgba, TraceError};

pub const MAX_SEEDS: usize = 64;
pub const MAX_POLYGONS: usize = 16;
pub const MAX_POLYGON_POINTS: usize = 512;
const MODEL_COLOURS: usize = 4;
const REFINE_ROUNDS: usize = 3;
const SAMPLE_CAP: usize = 20_000;

/// What to cut out.
#[derive(Clone, Debug)]
pub struct Spec {
    /// Foreground seed pixels (x, y).
    pub seeds: Vec<(usize, usize)>,
    /// Polygons (image pixels); when present the mask stays inside their union.
    pub include: Vec<Vec<(f64, f64)>>,
    /// Polygons that are never part of the mask.
    pub exclude: Vec<Vec<(f64, f64)>>,
    /// Colour distance (0..441) from the seed model that still counts as foreground.
    pub tolerance: f64,
    /// Fill background pockets that do not touch the image border.
    pub fill_holes: bool,
}

/// Binary mask, row-major.
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub width: usize,
    pub height: usize,
    pub bits: Vec<bool>,
}

/// Summary of a mask.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub area: usize,
    /// `[x0, y0, x1, y1]`, end exclusive; all zero when empty.
    pub bbox: [usize; 4],
}

impl Mask {
    pub fn stats(&self) -> Stats {
        let (mut area, mut b) = (0, [usize::MAX, usize::MAX, 0, 0]);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.bits[y * self.width + x] {
                    area += 1;
                    b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
                }
            }
        }
        Stats {
            area,
            bbox: if area == 0 { [0; 4] } else { b },
        }
    }
}

/// Pixels whose centre lies inside the even-odd union of `polys`.
pub fn rasterize(polys: &[Vec<(f64, f64)>], w: usize, h: usize) -> Vec<bool> {
    let mut out = vec![false; w * h];
    for poly in polys {
        let n = poly.len();
        for y in 0..h {
            let cy = y as f64 + 0.5;
            let mut xs: Vec<f64> = Vec::new();
            for i in 0..n {
                let (a, b) = (poly[i], poly[(i + 1) % n]);
                if (a.1 <= cy) != (b.1 <= cy) {
                    xs.push(a.0 + (cy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(f64::total_cmp);
            for pair in xs.chunks_exact(2) {
                let lo = (pair[0] - 0.5).ceil().max(0.0) as usize;
                let hi = ((pair[1] - 0.5).floor() + 1.0).clamp(0.0, w as f64) as usize;
                for x in lo..hi {
                    out[y * w + x] = true;
                }
            }
        }
    }
    out
}

fn valid_polygons(polys: &[Vec<(f64, f64)>]) -> bool {
    polys.len() <= MAX_POLYGONS
        && polys.iter().all(|p| {
            (3..=MAX_POLYGON_POINTS).contains(&p.len())
                && p.iter().all(|q| {
                    q.0.is_finite() && q.1.is_finite() && q.0.abs() < 1e7 && q.1.abs() < 1e7
                })
        })
}

type Rgb = [f64; 3];

fn colour(img: &Rgba<'_>, i: usize) -> Rgb {
    [
        f64::from(img.data[i * 4]),
        f64::from(img.data[i * 4 + 1]),
        f64::from(img.data[i * 4 + 2]),
    ]
}

fn dist(a: Rgb, b: Rgb) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Colour model: centres with their RMS spread.
struct Model(Vec<(Rgb, f64)>);

impl Model {
    /// Deterministic farthest-point seeding, then a few Lloyd rounds.
    fn fit(samples: &[Rgb]) -> Self {
        let mut centres = vec![samples[0]];
        while centres.len() < MODEL_COLOURS.min(samples.len()) {
            let far = samples
                .iter()
                .enumerate()
                .map(|(i, &s)| {
                    (
                        centres.iter().map(|&c| dist(s, c)).fold(f64::MAX, f64::min),
                        i,
                    )
                })
                .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
                .map_or(0, |m| m.1);
            if centres.contains(&samples[far]) {
                break;
            }
            centres.push(samples[far]);
        }
        let mut spread = vec![0.0; centres.len()];
        for _ in 0..4 {
            let mut acc = vec![([0.0; 3], 0.0f64, 0.0f64); centres.len()];
            for &s in samples {
                let (c, d) = nearest(&centres, s);
                let a = &mut acc[c];
                for k in 0..3 {
                    a.0[k] += s[k];
                }
                a.1 += 1.0;
                a.2 += d * d;
            }
            for (c, a) in acc.iter().enumerate() {
                if a.1 > 0.0 {
                    centres[c] = [a.0[0] / a.1, a.0[1] / a.1, a.0[2] / a.1];
                    spread[c] = (a.2 / a.1).sqrt();
                }
            }
        }
        Self(centres.into_iter().zip(spread).collect())
    }

    fn accepts(&self, p: Rgb, tolerance: f64) -> bool {
        self.0
            .iter()
            .any(|&(c, rms)| dist(p, c) <= tolerance + 1.5 * rms)
    }
}

fn nearest(centres: &[Rgb], p: Rgb) -> (usize, f64) {
    let mut best = (0, f64::MAX);
    for (i, &c) in centres.iter().enumerate() {
        let d = dist(p, c);
        if d < best.1 {
            best = (i, d);
        }
    }
    best
}

fn flood(
    img: &Rgba<'_>,
    seeds: &[(usize, usize)],
    allowed: &[bool],
    model: &Model,
    tol: f64,
) -> Vec<bool> {
    let (w, h) = (img.width, img.height);
    let mut out = vec![false; w * h];
    let mut stack: Vec<usize> = Vec::new();
    for &(x, y) in seeds {
        let i = y * w + x;
        if !out[i] {
            out[i] = true;
            stack.push(i);
        }
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % w, i / w);
        let around = [
            (x > 0).then(|| i - 1),
            (x + 1 < w).then(|| i + 1),
            (y > 0).then(|| i - w),
            (y + 1 < h).then(|| i + w),
        ];
        for j in around.into_iter().flatten() {
            if !out[j]
                && allowed[j]
                && img.data[j * 4 + 3] >= 128
                && model.accepts(colour(img, j), tol)
            {
                out[j] = true;
                stack.push(j);
            }
        }
    }
    out
}

/// Cuts the seeded region out of `img`.
pub fn segment(img: &Rgba<'_>, spec: &Spec) -> Result<Mask, TraceError> {
    let (w, h) = (img.width, img.height);
    if spec.seeds.is_empty()
        || spec.seeds.len() > MAX_SEEDS
        || spec.seeds.iter().any(|&(x, y)| x >= w || y >= h)
        || !valid_polygons(&spec.include)
        || !valid_polygons(&spec.exclude)
        || !spec.tolerance.is_finite()
        || !(0.0..=441.0).contains(&spec.tolerance)
    {
        return Err(TraceError::BadRegion);
    }
    let mut allowed = if spec.include.is_empty() {
        vec![true; w * h]
    } else {
        rasterize(&spec.include, w, h)
    };
    if !spec.exclude.is_empty() {
        for (a, e) in allowed.iter_mut().zip(rasterize(&spec.exclude, w, h)) {
            *a &= !e;
        }
    }
    if spec.seeds.iter().any(|&(x, y)| !allowed[y * w + x]) {
        return Err(TraceError::BadRegion);
    }
    let mut samples: Vec<Rgb> = Vec::new();
    for &(sx, sy) in &spec.seeds {
        for y in sy.saturating_sub(2)..(sy + 3).min(h) {
            for x in sx.saturating_sub(2)..(sx + 3).min(w) {
                samples.push(colour(img, y * w + x));
            }
        }
    }
    let model = Model::fit(&samples);
    let mut mask = flood(img, &spec.seeds, &allowed, &model, spec.tolerance);
    let mut area = mask.iter().filter(|&&b| b).count();
    for _ in 0..REFINE_ROUNDS {
        let stride = area.div_ceil(SAMPLE_CAP).max(1);
        let grown: Vec<Rgb> = (0..w * h)
            .filter(|&i| mask[i])
            .step_by(stride)
            .map(|i| colour(img, i))
            .collect();
        let refined = flood(
            img,
            &spec.seeds,
            &allowed,
            &Model::fit(&grown),
            spec.tolerance,
        );
        let next = refined.iter().filter(|&&b| b).count();
        // A refinement that balloons has started to leak into the background.
        if next * 5 > area * 8 || next == area && refined == mask {
            break;
        }
        mask = refined;
        area = next;
    }
    if spec.fill_holes {
        fill_holes(&mut mask, &allowed, w, h);
    }
    Ok(Mask {
        width: w,
        height: h,
        bits: mask,
    })
}

/// Sets every background pocket that cannot reach the image border.
fn fill_holes(mask: &mut [bool], allowed: &[bool], w: usize, h: usize) {
    let mut outside = vec![false; w * h];
    let mut stack = Vec::new();
    let edge = (0..w)
        .flat_map(|x| [x, (h - 1) * w + x])
        .chain((0..h).flat_map(|y| [y * w, y * w + w - 1]));
    for i in edge {
        if !mask[i] && !outside[i] {
            outside[i] = true;
            stack.push(i);
        }
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % w, i / w);
        let around = [
            (x > 0).then(|| i - 1),
            (x + 1 < w).then(|| i + 1),
            (y > 0).then(|| i - w),
            (y + 1 < h).then(|| i + w),
        ];
        for j in around.into_iter().flatten() {
            if !mask[j] && !outside[j] {
                outside[j] = true;
                stack.push(j);
            }
        }
    }
    for i in 0..w * h {
        if !mask[i] && !outside[i] && allowed[i] {
            mask[i] = true;
        }
    }
}
