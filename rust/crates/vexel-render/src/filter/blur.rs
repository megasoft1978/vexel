//! `feGaussianBlur`, `feOffset` and `feDropShadow` (C `blur`, `offset_surface`,
//! `drop_shadow`).
//!
//! The blur is a true separable Gaussian with kernel radius `ceil(3 * sigma)`,
//! weights normalised over the whole kernel, and samples outside the layer
//! skipped (no renormalisation). The horizontal pass is stored as `f32`
//! premultiplied-alpha intermediate exactly as the C does.

use super::color::{clamp01, quantize, source_over};
use crate::RenderError;
use vexel_scene::FilterPrimitive;

/// Largest kernel radius (C rejects `ceil(3 * sigma) > 96`).
pub(super) const MAX_RADIUS: f64 = 96.0;

fn weights(sigma: f64, radius: i32) -> Vec<f64> {
    (-radius..=radius)
        .map(|k| {
            if sigma == 0.0 {
                1.0
            } else {
                let k = f64::from(k);
                (-(k * k) / (2.0 * sigma * sigma)).exp()
            }
        })
        .collect()
}

fn radius_of(sigma: f64) -> Result<i32, RenderError> {
    let radius = (3.0 * sigma).ceil();
    if radius.is_nan() || radius > MAX_RADIUS {
        return Err(RenderError::LimitExceeded);
    }
    Ok(radius as i32)
}

/// Kernel visits per pixel of the exact blur.
pub(super) fn exact_per_pixel(radius_x: i32, radius_y: i32) -> usize {
    (2 * radius_x + 1) as usize + (2 * radius_y + 1) as usize
}

/// True when the exact kernel fits [`super::FILTER_WORK_LIMIT`] for `pixels`;
/// larger blurs use the O(pixels) box approximation.
pub(super) fn is_exact(pixels: usize, radius_x: i32, radius_y: i32) -> bool {
    pixels.saturating_mul(exact_per_pixel(radius_x, radius_y)) <= super::FILTER_WORK_LIMIT
}

/// Gaussian blur of straight RGBA `input` into `output`; `work` holds
/// `width * height * 4` floats.
pub(super) fn blur(
    input: &[u8],
    output: &mut [u8],
    width: usize,
    height: usize,
    sigma: (f64, f64),
    work: &mut [f32],
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let (radius_x, radius_y) = (radius_of(sigma.0)?, radius_of(sigma.1)?);
    if !is_exact(width * height, radius_x, radius_y) {
        return super::boxblur::blur(input, output, width, height, sigma, work, control);
    }
    let (weights_x, weights_y) = (weights(sigma.0, radius_x), weights(sigma.1, radius_y));
    let total_x: f64 = weights_x.iter().sum();
    let total_y: f64 = weights_y.iter().sum();
    if total_x <= 0.0 || total_y <= 0.0 {
        return Err(RenderError::InvalidGeometry);
    }
    // Same arithmetic, same per-pixel summation order (kernel offset
    // ascending) as the per-pixel formulation, but accumulated a row at a
    // time and without the taps that are exactly zero (transparent pixels
    // add `+0.0`, which leaves a non-negative sum bit-for-bit unchanged).
    let kernel_x: Vec<f64> = weights_x.iter().map(|w| w / total_x).collect();
    let kernel_y: Vec<f64> = weights_y.iter().map(|w| w / total_y).collect();
    let mut premul = vec![[0.0f64; 4]; width];
    let mut acc = vec![[0.0f64; 4]; width];
    // Columns that can be non-zero in each row of `work`.
    let mut bounds: Vec<Option<(usize, usize)>> = vec![None; height];
    for y in 0..height {
        if control() {
            return Err(RenderError::Cancelled);
        }
        let row = &input[y * width * 4..(y + 1) * width * 4];
        let (mut first, mut last) = (usize::MAX, 0);
        for (x, (p, px)) in premul.iter_mut().zip(row.chunks_exact(4)).enumerate() {
            let alpha = f64::from(px[3]) / 255.0;
            *p = [
                f64::from(px[0]) / 255.0 * alpha,
                f64::from(px[1]) / 255.0 * alpha,
                f64::from(px[2]) / 255.0 * alpha,
                alpha,
            ];
            if px[3] != 0 {
                first = first.min(x);
                last = x;
            }
        }
        let out = &mut work[y * width * 4..(y + 1) * width * 4];
        if first == usize::MAX {
            out.fill(0.0);
            continue;
        }
        let lo = first.saturating_sub(radius_x as usize);
        let hi = (last + radius_x as usize).min(width - 1);
        bounds[y] = Some((lo, hi));
        acc[lo..=hi].fill([0.0; 4]);
        for k in -radius_x..=radius_x {
            let weight = kernel_x[(k + radius_x) as usize];
            // Output columns whose source column x + k lies in first..=last.
            let from = (first as i64 - i64::from(k)).max(lo as i64) as usize;
            let to = (last as i64 - i64::from(k)).min(hi as i64);
            if to < from as i64 {
                continue;
            }
            for x in from..=to as usize {
                let source = &premul[(x as i64 + i64::from(k)) as usize];
                let sum = &mut acc[x];
                sum[0] += source[0] * weight;
                sum[1] += source[1] * weight;
                sum[2] += source[2] * weight;
                sum[3] += source[3] * weight;
            }
        }
        out.fill(0.0);
        for x in lo..=hi {
            for c in 0..4 {
                out[x * 4 + c] = acc[x][c] as f32;
            }
        }
    }
    for y in 0..height {
        if control() {
            return Err(RenderError::Cancelled);
        }
        acc.fill([0.0; 4]);
        for k in -radius_y..=radius_y {
            let sy = y as i64 + i64::from(k);
            if sy < 0 || sy >= height as i64 {
                continue;
            }
            let Some((lo, hi)) = bounds[sy as usize] else {
                continue;
            };
            let weight = kernel_y[(k + radius_y) as usize];
            let row = &work[sy as usize * width * 4..(sy as usize + 1) * width * 4];
            for x in lo..=hi {
                let sum = &mut acc[x];
                sum[0] += f64::from(row[x * 4]) * weight;
                sum[1] += f64::from(row[x * 4 + 1]) * weight;
                sum[2] += f64::from(row[x * 4 + 2]) * weight;
                sum[3] += f64::from(row[x * 4 + 3]) * weight;
            }
        }
        let out = &mut output[y * width * 4..(y + 1) * width * 4];
        for (x, sum) in acc.iter().enumerate() {
            let alpha = clamp01(sum[3]);
            let px = &mut out[x * 4..x * 4 + 4];
            if alpha <= 0.0 {
                px.fill(0);
                continue;
            }
            for c in 0..3 {
                px[c] = quantize(clamp01(sum[c] / alpha));
            }
            px[3] = quantize(alpha);
        }
    }
    Ok(())
}

/// C `llround`: halves round away from zero.
fn round_shift(value: f64) -> i64 {
    value.round() as i64
}

/// `feOffset`: integer shift by the rounded `dx`, `dy`; vacated pixels are
/// transparent.
pub(super) fn offset(
    input: &[u8],
    output: &mut [u8],
    width: usize,
    height: usize,
    dx: f64,
    dy: f64,
) {
    let (shift_x, shift_y) = (round_shift(dx), round_shift(dy));
    output.fill(0);
    for y in 0..height {
        let destination_y = y as i64 + shift_y;
        if destination_y < 0 || destination_y >= height as i64 {
            continue;
        }
        for x in 0..width {
            let destination_x = x as i64 + shift_x;
            if destination_x < 0 || destination_x >= width as i64 {
                continue;
            }
            let to = (destination_y as usize * width + destination_x as usize) * 4;
            let from = (y * width + x) * 4;
            output[to..to + 4].copy_from_slice(&input[from..from + 4]);
        }
    }
}

/// `feDropShadow`: blurred alpha, shifted, tinted by `flood-color`
/// (converted to linear when `linear_rgb`) and composited under the input.
pub(super) fn drop_shadow(
    primitive: &FilterPrimitive,
    input: &[u8],
    output: &mut [u8],
    shadow_alpha: &mut [u8],
    work: &mut [f32],
    size: (usize, usize),
    linear: &[u8; 256],
    linear_rgb: bool,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let (width, height) = size;
    blur(
        input,
        output,
        width,
        height,
        (primitive.values[0], primitive.values[1]),
        work,
        control,
    )?;
    for (alpha, pixel) in shadow_alpha.iter_mut().zip(output.chunks_exact(4)) {
        *alpha = pixel[3];
    }
    let mut color = [0u8; 3];
    for c in 0..3 {
        color[c] = if linear_rgb {
            linear[usize::from(primitive.rgba[c])]
        } else {
            primitive.rgba[c]
        };
    }
    let color_alpha = f64::from(primitive.rgba[3]) / 255.0;
    let (dx, dy) = (
        round_shift(primitive.values[2]),
        round_shift(primitive.values[3]),
    );
    output.fill(0);
    for y in 0..height {
        if control() {
            return Err(RenderError::Cancelled);
        }
        for x in 0..width {
            let sx = x as i64 - dx;
            let sy = y as i64 - dy;
            if sx < 0 || sy < 0 || sx >= width as i64 || sy >= height as i64 {
                continue;
            }
            let source_pixel = sy as usize * width + sx as usize;
            let destination = (y * width + x) * 4;
            output[destination..destination + 3].copy_from_slice(&color);
            output[destination + 3] =
                quantize(f64::from(shadow_alpha[source_pixel]) / 255.0 * color_alpha);
        }
    }
    for (out, src) in output.chunks_exact_mut(4).zip(input.chunks_exact(4)) {
        source_over(out, src);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The original per-pixel formulation, kept as the byte-exact reference.
    fn reference(input: &[u8], width: usize, height: usize, sigma: (f64, f64)) -> Vec<u8> {
        let (radius_x, radius_y) = (radius_of(sigma.0).unwrap(), radius_of(sigma.1).unwrap());
        let (weights_x, weights_y) = (weights(sigma.0, radius_x), weights(sigma.1, radius_y));
        let total_x: f64 = weights_x.iter().sum();
        let total_y: f64 = weights_y.iter().sum();
        let mut work = vec![0.0f32; width * height * 4];
        let mut output = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0f64; 4];
                for k in -radius_x..=radius_x {
                    let sx = x as i64 + i64::from(k);
                    if sx < 0 || sx >= width as i64 {
                        continue;
                    }
                    let at = (y * width + sx as usize) * 4;
                    let pixel = &input[at..at + 4];
                    let alpha = f64::from(pixel[3]) / 255.0;
                    let weight = weights_x[(k + radius_x) as usize] / total_x;
                    sum[0] += f64::from(pixel[0]) / 255.0 * alpha * weight;
                    sum[1] += f64::from(pixel[1]) / 255.0 * alpha * weight;
                    sum[2] += f64::from(pixel[2]) / 255.0 * alpha * weight;
                    sum[3] += alpha * weight;
                }
                let index = (y * width + x) * 4;
                for c in 0..4 {
                    work[index + c] = sum[c] as f32;
                }
            }
        }
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0f64; 4];
                for k in -radius_y..=radius_y {
                    let sy = y as i64 + i64::from(k);
                    if sy < 0 || sy >= height as i64 {
                        continue;
                    }
                    let index = (sy as usize * width + x) * 4;
                    let weight = weights_y[(k + radius_y) as usize] / total_y;
                    for c in 0..4 {
                        sum[c] += f64::from(work[index + c]) * weight;
                    }
                }
                let alpha = clamp01(sum[3]);
                let index = (y * width + x) * 4;
                if alpha <= 0.0 {
                    continue;
                }
                for c in 0..3 {
                    output[index + c] = quantize(clamp01(sum[c] / alpha));
                }
                output[index + 3] = quantize(alpha);
            }
        }
        output
    }

    fn image(width: usize, height: usize, seed: u32, sparse: bool) -> Vec<u8> {
        let mut state = seed;
        let mut next = move || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        };
        let mut px = vec![0u8; width * height * 4];
        for (i, p) in px.chunks_exact_mut(4).enumerate() {
            let (x, y) = (i % width, i / width);
            let inside = !sparse || ((x / 7 + y / 5) % 4 == 0 && x > 9 && y < height - 6);
            if inside {
                p.copy_from_slice(&[next(), next(), next(), next() | 1]);
            } else if next() < 3 {
                p.copy_from_slice(&[next(), next(), next(), 0]);
            }
        }
        px
    }

    #[test]
    fn row_wise_blur_is_byte_identical_to_the_per_pixel_formulation() {
        for (w, h, sigma, sparse) in [
            (37usize, 29usize, (2.0, 3.5), false),
            (64, 48, (4.0, 1.0), true),
            (50, 50, (0.0, 0.0), true),
            (40, 33, (9.0, 9.0), true),
            (9, 7, (6.0, 2.5), false),
        ] {
            let input = image(w, h, w as u32 * 31 + h as u32, sparse);
            let mut output = vec![9u8; w * h * 4];
            let mut work = vec![7.0f32; w * h * 4];
            blur(&input, &mut output, w, h, sigma, &mut work, &|| false).unwrap();
            assert_eq!(output, reference(&input, w, h, sigma), "{w}x{h} {sigma:?}");
        }
        // Fully transparent input.
        let (w, h) = (20, 12);
        let mut output = vec![9u8; w * h * 4];
        let mut work = vec![7.0f32; w * h * 4];
        blur(&vec![0u8; w * h * 4], &mut output, w, h, (3.0, 3.0), &mut work, &|| false).unwrap();
        assert!(output.iter().all(|&b| b == 0));
    }
}
