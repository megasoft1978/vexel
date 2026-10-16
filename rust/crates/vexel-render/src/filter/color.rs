//! Per-pixel colour arithmetic of the C filter pipeline (`shape_filter.c`).
//!
//! Layers are straight (non-premultiplied) RGBA8. All maths is `f64` with the
//! same operation order as the C so that results round identically.

use vexel_scene::{FilterKind, FilterPrimitive};

pub(super) fn clamp01(value: f64) -> f64 {
    if !value.is_finite() || value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
}

/// C `floor(value * 255.0 + 0.5)` narrowed to a byte.
pub(super) fn quantize(value: f64) -> u8 {
    (value * 255.0 + 0.5).floor() as u8
}

fn unit(byte: u8) -> f64 {
    f64::from(byte) / 255.0
}

fn decode_srgb_unit(value: u8) -> f64 {
    let sample = unit(value);
    if sample <= 0.04045 {
        sample / 12.92
    } else {
        ((sample + 0.055) / 1.055).powf(2.4)
    }
}

fn encode_srgb_unit(value: f64) -> u8 {
    let value = clamp01(value);
    let sample = if value <= 0.0031308 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    quantize(clamp01(sample))
}

/// sRGB byte to linear byte (`floor(decode * 255 + 0.5)`).
pub(super) fn linear_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    for (value, slot) in table.iter_mut().enumerate() {
        *slot = quantize(decode_srgb_unit(value as u8));
    }
    table
}

/// Linear byte to sRGB byte (C `encode_srgb(byte / 255.0)`).
pub(super) fn srgb_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    for (value, slot) in table.iter_mut().enumerate() {
        *slot = encode_srgb_unit(unit(value as u8));
    }
    table
}

/// C `source_over(destination, source)` for straight alpha, in `f64`.
pub(super) fn source_over(destination: &mut [u8], source: &[u8]) {
    let source_alpha = unit(source[3]);
    let backdrop_alpha = unit(destination[3]);
    let output_alpha = source_alpha + backdrop_alpha - source_alpha * backdrop_alpha;
    if output_alpha <= 0.0 {
        destination[..4].fill(0);
        return;
    }
    for channel in 0..3 {
        let foreground = unit(source[channel]);
        let backdrop = unit(destination[channel]);
        let premultiplied =
            foreground * source_alpha + backdrop * backdrop_alpha * (1.0 - source_alpha);
        destination[channel] = quantize(clamp01(premultiplied / output_alpha));
    }
    destination[3] = quantize(output_alpha);
}

fn blend_component(source: f64, backdrop: f64, mode: u8) -> f64 {
    match mode {
        1 => source * backdrop,
        2 => source + backdrop - source * backdrop,
        3 => {
            if source < backdrop {
                source
            } else {
                backdrop
            }
        }
        4 => {
            if source > backdrop {
                source
            } else {
                backdrop
            }
        }
        _ => source,
    }
}

fn blend_pixel(output: &mut [u8], source: &[u8], backdrop: &[u8], mode: u8) {
    let sa = unit(source[3]);
    let ba = unit(backdrop[3]);
    let oa = sa + ba - sa * ba;
    if oa <= 0.0 {
        output[..4].fill(0);
        return;
    }
    for c in 0..3 {
        let sc = unit(source[c]);
        let bc = unit(backdrop[c]);
        let mixed = blend_component(sc, bc, mode);
        let premultiplied = sc * sa * (1.0 - ba) + bc * ba * (1.0 - sa) + mixed * sa * ba;
        output[c] = quantize(clamp01(premultiplied / oa));
    }
    output[3] = quantize(oa);
}

/// `feBlend` and `feComposite` for one pixel (C `composite`).
pub(super) fn composite_pixel(primitive: &FilterPrimitive, a: &[u8], b: &[u8], out: &mut [u8]) {
    if primitive.kind == FilterKind::Blend {
        blend_pixel(out, a, b, primitive.mode);
        return;
    }
    let alpha_a = unit(a[3]);
    let alpha_b = unit(b[3]);
    if primitive.mode == 6 {
        for c in 0..4 {
            let av = unit(a[c]);
            let bv = unit(b[c]);
            let value = primitive.values[0] * av * bv
                + primitive.values[1] * av
                + primitive.values[2] * bv
                + primitive.values[3];
            out[c] = quantize(clamp01(value));
        }
        return;
    }
    let alpha = match primitive.mode {
        0 => alpha_a + alpha_b * (1.0 - alpha_a),
        1 => alpha_a * alpha_b,
        2 => alpha_a * (1.0 - alpha_b),
        3 => alpha_b,
        4 => alpha_a * (1.0 - alpha_b) + alpha_b * (1.0 - alpha_a),
        _ => (alpha_a + alpha_b).min(1.0),
    };
    if alpha <= 0.0 {
        out[..4].fill(0);
        return;
    }
    for c in 0..3 {
        let ca = unit(a[c]);
        let cb = unit(b[c]);
        let premultiplied = match primitive.mode {
            0 => ca * alpha_a + cb * alpha_b * (1.0 - alpha_a),
            1 => ca * alpha_a * alpha_b,
            2 => ca * alpha_a * (1.0 - alpha_b),
            3 => ca * alpha_a * alpha_b + cb * alpha_b * (1.0 - alpha_a),
            4 => ca * alpha_a * (1.0 - alpha_b) + cb * alpha_b * (1.0 - alpha_a),
            _ => alpha.min(ca * alpha_a + cb * alpha_b),
        };
        out[c] = quantize(clamp01(premultiplied / alpha));
    }
    out[3] = quantize(clamp01(alpha));
}

const LUMINANCE: [f64; 3] = [0.2126, 0.7152, 0.0722];

fn matrix_for(primitive: &FilterPrimitive) -> [f64; 20] {
    let mut matrix = [0.0f64; 20];
    match primitive.mode {
        0 => matrix.copy_from_slice(&primitive.values),
        1 => {
            let s = primitive.values[0];
            matrix[0] = 0.213 + 0.787 * s;
            matrix[1] = 0.715 - 0.715 * s;
            matrix[2] = 0.072 - 0.072 * s;
            matrix[5] = 0.213 - 0.213 * s;
            matrix[6] = 0.715 + 0.285 * s;
            matrix[7] = 0.072 - 0.072 * s;
            matrix[10] = 0.213 - 0.213 * s;
            matrix[11] = 0.715 - 0.715 * s;
            matrix[12] = 0.072 + 0.928 * s;
            matrix[18] = 1.0;
        }
        2 => {
            let angle = primitive.values[0] * (std::f64::consts::PI / 180.0);
            let (s, c) = (angle.sin(), angle.cos());
            matrix[0] = 0.213 + c * 0.787 - s * 0.213;
            matrix[1] = 0.715 - c * 0.715 - s * 0.715;
            matrix[2] = 0.072 - c * 0.072 + s * 0.928;
            matrix[5] = 0.213 - c * 0.213 + s * 0.143;
            matrix[6] = 0.715 + c * 0.285 + s * 0.140;
            matrix[7] = 0.072 - c * 0.072 - s * 0.283;
            matrix[10] = 0.213 - c * 0.213 - s * 0.787;
            matrix[11] = 0.715 - c * 0.715 + s * 0.715;
            matrix[12] = 0.072 + c * 0.928 + s * 0.072;
            matrix[18] = 1.0;
        }
        _ => {
            matrix[15] = LUMINANCE[0];
            matrix[16] = LUMINANCE[1];
            matrix[17] = LUMINANCE[2];
        }
    }
    matrix
}

/// `feColorMatrix` over a whole layer (C `color_matrix`). Returns false when
/// `control` requested cancellation.
pub(super) fn color_matrix(
    primitive: &FilterPrimitive,
    input: &[u8],
    output: &mut [u8],
    control: &dyn Fn() -> bool,
    row_bytes: usize,
) -> bool {
    let matrix = matrix_for(primitive);
    for (rows_in, rows_out) in input.chunks(row_bytes).zip(output.chunks_mut(row_bytes)) {
        if control() {
            return false;
        }
        for (source, destination) in rows_in.chunks_exact(4).zip(rows_out.chunks_exact_mut(4)) {
            let rgba = [
                unit(source[0]),
                unit(source[1]),
                unit(source[2]),
                unit(source[3]),
            ];
            if primitive.mode == 3 {
                destination[..3].fill(0);
                destination[3] = quantize(clamp01(
                    LUMINANCE[0] * rgba[0] + LUMINANCE[1] * rgba[1] + LUMINANCE[2] * rgba[2],
                ));
                continue;
            }
            if source[3] == 0 {
                destination.fill(0);
                continue;
            }
            for row in 0..4 {
                let mut value = matrix[row * 5 + 4];
                for (column, sample) in rgba.iter().enumerate() {
                    value += matrix[row * 5 + column] * sample;
                }
                destination[row] = quantize(clamp01(value));
            }
        }
    }
    true
}
