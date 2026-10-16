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
    let (weights_x, weights_y) = (weights(sigma.0, radius_x), weights(sigma.1, radius_y));
    let total_x: f64 = weights_x.iter().sum();
    let total_y: f64 = weights_y.iter().sum();
    if total_x <= 0.0 || total_y <= 0.0 {
        return Err(RenderError::InvalidGeometry);
    }
    for y in 0..height {
        if control() {
            return Err(RenderError::Cancelled);
        }
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
        if control() {
            return Err(RenderError::Cancelled);
        }
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
                output[index..index + 4].fill(0);
                continue;
            }
            for c in 0..3 {
                output[index + c] = quantize(clamp01(sum[c] / alpha));
            }
            output[index + 3] = quantize(alpha);
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
