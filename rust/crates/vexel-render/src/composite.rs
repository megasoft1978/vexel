//! Source-over blending and sample resolution (C `shape_render.c`).

use crate::SAMPLES_PER_AXIS;

pub(crate) const SAMPLE_COUNT: usize = SAMPLES_PER_AXIS * SAMPLES_PER_AXIS;

/// C `composite_source_over`: straight-alpha RGBA8, integer arithmetic.
pub(crate) fn source_over(destination: &mut [u8; 4], source: [u8; 4]) {
    let source_alpha = u32::from(source[3]);
    if source_alpha == 0 {
        return;
    }
    let destination_alpha = u32::from(destination[3]);
    let inverse_source_alpha = 255 - source_alpha;
    let output_alpha_numerator = source_alpha * 255 + destination_alpha * inverse_source_alpha;
    for channel in 0..3 {
        let numerator = u32::from(source[channel]) * source_alpha * 255
            + u32::from(destination[channel]) * destination_alpha * inverse_source_alpha;
        destination[channel] = ((numerator + output_alpha_numerator / 2) / output_alpha_numerator) as u8;
    }
    destination[3] = ((output_alpha_numerator + 127) / 255) as u8;
}

/// C `resolve_shape_sample`: RGB averaged over alpha weight, alpha over all samples.
pub(crate) fn resolve_samples(samples: &[[u8; 4]; SAMPLE_COUNT]) -> [u8; 4] {
    let mut alpha_sum = 0u32;
    let mut premultiplied = [0u32; 3];
    for pixel in samples {
        let alpha = u32::from(pixel[3]);
        alpha_sum += alpha;
        for channel in 0..3 {
            premultiplied[channel] += u32::from(pixel[channel]) * alpha;
        }
    }
    if alpha_sum == 0 {
        return [0; 4];
    }
    let mut rgba = [0u8; 4];
    for channel in 0..3 {
        rgba[channel] = ((premultiplied[channel] + alpha_sum / 2) / alpha_sum) as u8;
    }
    rgba[3] = ((alpha_sum + SAMPLE_COUNT as u32 / 2) / SAMPLE_COUNT as u32) as u8;
    rgba
}

/// [`resolve_samples`] of a pixel whose sample row `r` is `SAMPLES_PER_AXIS`
/// copies of `rows[r]`: the sums are those of the full array, term for term.
pub(crate) fn resolve_rows(rows: &[[u8; 4]; SAMPLES_PER_AXIS]) -> [u8; 4] {
    let mut alpha_sum = 0u32;
    let mut premultiplied = [0u32; 3];
    for pixel in rows {
        let alpha = u32::from(pixel[3]);
        alpha_sum += alpha * SAMPLES_PER_AXIS as u32;
        for channel in 0..3 {
            premultiplied[channel] += u32::from(pixel[channel]) * alpha * SAMPLES_PER_AXIS as u32;
        }
    }
    if alpha_sum == 0 {
        return [0; 4];
    }
    let mut rgba = [0u8; 4];
    for channel in 0..3 {
        rgba[channel] = ((premultiplied[channel] + alpha_sum / 2) / alpha_sum) as u8;
    }
    rgba[3] = ((alpha_sum + SAMPLE_COUNT as u32 / 2) / SAMPLE_COUNT as u32) as u8;
    rgba
}

/// True when every byte is zero; chunked so the scan vectorizes.
pub(crate) fn all_zero(bytes: &[u8]) -> bool {
    let mut chunks = bytes.chunks_exact(64);
    let mut acc = 0u8;
    for chunk in &mut chunks {
        let mut word = 0u8;
        for byte in chunk {
            word |= *byte;
        }
        if word != 0 {
            return false;
        }
    }
    for byte in chunks.remainder() {
        acc |= *byte;
    }
    acc == 0
}
