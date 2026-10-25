//! Box-blur approximation of the Gaussian for large `stdDeviation` (SVG
//! Filter Effects 1, `feGaussianBlur`: three successive box blurs of size
//! `d = floor(sigma * 3 * sqrt(2 * pi) / 4 + 0.5)`).
//!
//! Cost is O(pixels), independent of sigma, unlike the exact kernel in
//! [`super::blur`] whose cost grows with the radius. It is only used when the
//! exact kernel would exceed [`super::FILTER_WORK_LIMIT`], so every output the
//! exact path could produce is unchanged. Samples outside the layer count as
//! transparent, matching the exact path (no renormalisation).

use super::color::{clamp01, quantize};
use crate::RenderError;

/// Weighted cost per pixel of one box blur (two axes, three passes each).
pub(super) const BOX_COST_PER_PIXEL: usize = 24;
/// Columns processed together in the vertical pass (cache blocking).
const LANES: usize = 8;

/// Box size for `sigma` (SVG spec); `<= 1` means the axis is left alone.
pub(super) fn box_size(sigma: f64) -> usize {
    (sigma * 3.0 * (2.0 * std::f64::consts::PI).sqrt() / 4.0 + 0.5).floor() as usize
}

/// One sliding-window pass: `dst[i] = mean(src[i - offset .. i - offset + size])`
/// per channel, zero outside `0..n`. Samples hold `w` floats.
fn pass(src: &[f32], dst: &mut [f32], n: usize, w: usize, size: usize, offset: usize) {
    let mut acc = vec![0.0f64; w];
    let first_hi = (size - offset).min(n);
    for j in 0..first_hi {
        for (a, v) in acc.iter_mut().zip(&src[j * w..(j + 1) * w]) {
            *a += f64::from(*v);
        }
    }
    let scale = 1.0 / size as f64;
    for i in 0..n {
        for (out, a) in dst[i * w..(i + 1) * w].iter_mut().zip(&acc) {
            *out = (a * scale) as f32;
        }
        let add = i + size - offset;
        if add < n {
            for (a, v) in acc.iter_mut().zip(&src[add * w..(add + 1) * w]) {
                *a += f64::from(*v);
            }
        }
        if i >= offset {
            let sub = i - offset;
            for (a, v) in acc.iter_mut().zip(&src[sub * w..(sub + 1) * w]) {
                *a -= f64::from(*v);
            }
        }
    }
}

/// Three box passes of total size `d` over `n` samples of `w` floats.
fn passes(buf: &mut [f32], tmp: &mut [f32], n: usize, w: usize, d: usize) {
    if d <= 1 {
        return;
    }
    let plan: [(usize, usize); 3] = if d % 2 == 1 {
        [(d, d / 2); 3]
    } else {
        [(d, d / 2), (d, d / 2 - 1), (d + 1, d / 2)]
    };
    pass(buf, tmp, n, w, plan[0].0, plan[0].1);
    pass(tmp, buf, n, w, plan[1].0, plan[1].1);
    pass(buf, tmp, n, w, plan[2].0, plan[2].1);
    buf.copy_from_slice(&tmp[..buf.len()]);
}

/// Box-approximated Gaussian blur of straight RGBA `input` into `output`;
/// `work` holds `width * height * 4` floats.
pub(super) fn blur(
    input: &[u8],
    output: &mut [u8],
    width: usize,
    height: usize,
    sigma: (f64, f64),
    work: &mut [f32],
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let (dx, dy) = (box_size(sigma.0), box_size(sigma.1));
    let mut line = vec![0.0f32; width * 4];
    let mut tmp = vec![0.0f32; width * 4];
    for y in 0..height {
        if control() {
            return Err(RenderError::Cancelled);
        }
        let row = &input[y * width * 4..(y + 1) * width * 4];
        for (slot, pixel) in line.chunks_exact_mut(4).zip(row.chunks_exact(4)) {
            let alpha = f32::from(pixel[3]) / 255.0;
            for c in 0..3 {
                slot[c] = f32::from(pixel[c]) / 255.0 * alpha;
            }
            slot[3] = alpha;
        }
        passes(&mut line, &mut tmp, width, 4, dx);
        work[y * width * 4..(y + 1) * width * 4].copy_from_slice(&line);
    }
    let mut strip = vec![0.0f32; height * LANES * 4];
    let mut strip_tmp = vec![0.0f32; height * LANES * 4];
    for x0 in (0..width).step_by(LANES) {
        if control() {
            return Err(RenderError::Cancelled);
        }
        let lanes = LANES.min(width - x0);
        let w = lanes * 4;
        for y in 0..height {
            let from = (y * width + x0) * 4;
            strip[y * w..(y + 1) * w].copy_from_slice(&work[from..from + w]);
        }
        passes(&mut strip[..height * w], &mut strip_tmp[..height * w], height, w, dy);
        for y in 0..height {
            for k in 0..lanes {
                let s = &strip[y * w + k * 4..y * w + k * 4 + 4];
                let alpha = clamp01(f64::from(s[3]));
                let at = (y * width + x0 + k) * 4;
                if alpha <= 0.0 {
                    output[at..at + 4].fill(0);
                    continue;
                }
                for c in 0..3 {
                    output[at + c] = quantize(clamp01(f64::from(s[c]) / alpha));
                }
                output[at + 3] = quantize(alpha);
            }
        }
    }
    Ok(())
}
