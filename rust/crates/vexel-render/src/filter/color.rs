//! Per-pixel colour arithmetic of the C filter pipeline (`shape_filter.c`).
//!
//! Layers are straight (non-premultiplied) RGBA8. All maths is `f64` with the
//! same operation order as the C so that results round identically.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::ScopedJoinHandle;
use std::time::Duration;

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
        5 => hard_light(source, backdrop),
        6 => {
            if backdrop == 0.0 {
                0.0
            } else if source >= 1.0 {
                1.0
            } else {
                (backdrop / (1.0 - source)).min(1.0)
            }
        }
        7 => {
            if backdrop >= 1.0 {
                1.0
            } else if source <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - backdrop) / source).min(1.0)
            }
        }
        8 => hard_light(backdrop, source),
        9 => soft_light(source, backdrop),
        10 => (source - backdrop).abs(),
        11 => source + backdrop - 2.0 * source * backdrop,
        _ => source,
    }
}

/// W3C `HardLight(cb, cs)`.
fn hard_light(backdrop: f64, source: f64) -> f64 {
    if source <= 0.5 {
        backdrop * 2.0 * source
    } else {
        let s = 2.0 * source - 1.0;
        backdrop + s - backdrop * s
    }
}

/// W3C `SoftLight(cb, cs)`.
fn soft_light(source: f64, backdrop: f64) -> f64 {
    if source <= 0.5 {
        backdrop - (1.0 - 2.0 * source) * backdrop * (1.0 - backdrop)
    } else {
        let d = if backdrop <= 0.25 {
            ((16.0 * backdrop - 12.0) * backdrop + 4.0) * backdrop
        } else {
            backdrop.sqrt()
        };
        backdrop + (2.0 * source - 1.0) * (d - backdrop)
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

/// One libm `sin` or `cos` per call. Pairing them on the same argument lets
/// optimized Apple builds merge them into `__sincos_stret`, whose `sin` can
/// differ by one ulp from `f64::sin`. `inline(never)` prevents the merge.
#[inline(never)]
fn sin_unpaired(angle: f64) -> f64 {
    angle.sin()
}

#[inline(never)]
fn cos_unpaired(angle: f64) -> f64 {
    angle.cos()
}

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
            let (s, c) = (sin_unpaired(angle), cos_unpaired(angle));
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
    // `unit(b)` for every byte, and the previous pixel's result: flat-colour
    // art repeats pixels, and the same input always gives the same output.
    let units: Vec<f64> = (0..=255u8).map(unit).collect();
    let rows = input.len().div_ceil(row_bytes.max(1));
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(MAX_BANDS)
        .min(rows / MIN_BAND_ROWS)
        .max(1);
    if threads == 1 {
        for (rows_in, rows_out) in input.chunks(row_bytes).zip(output.chunks_mut(row_bytes)) {
            if control() {
                return false;
            }
            matrix_pixels(primitive, &matrix, &units, rows_in, rows_out);
        }
        return true;
    }
    // Pixels are independent, so bands give the bytes of the serial loop.
    // `control` is not shareable across threads: the calling thread polls it
    // while the bands run and raises `cancel`, which bands check per row.
    if control() {
        return false;
    }
    let band = rows.div_ceil(threads) * row_bytes;
    let (matrix, units) = (&matrix, &units);
    let cancel = AtomicBool::new(false);
    let mut cancelled = false;
    std::thread::scope(|scope| {
        let handles: Vec<_> = input
            .chunks(band)
            .zip(output.chunks_mut(band))
            .map(|(rows_in, rows_out)| {
                let cancel = &cancel;
                scope.spawn(move || {
                    matrix_band(
                        primitive, matrix, units, row_bytes, rows_in, rows_out, cancel,
                    )
                })
            })
            .collect();
        while !handles.iter().all(ScopedJoinHandle::is_finished) {
            if !cancelled && control() {
                cancelled = true;
                cancel.store(true, Ordering::Relaxed);
            }
            std::thread::sleep(BAND_POLL);
        }
    });
    !cancelled && !control()
}

/// How often the caller polls `control` while bands run.
const BAND_POLL: Duration = Duration::from_micros(500);

/// One thread band, row by row, stopping early once `cancel` is raised.
fn matrix_band(
    primitive: &FilterPrimitive,
    matrix: &[f64; 20],
    units: &[f64],
    row_bytes: usize,
    input: &[u8],
    output: &mut [u8],
    cancel: &AtomicBool,
) {
    for (rows_in, rows_out) in input.chunks(row_bytes).zip(output.chunks_mut(row_bytes)) {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        matrix_pixels(primitive, matrix, units, rows_in, rows_out);
    }
}

/// Bands never get fewer rows than this (thread spawn cost).
const MIN_BAND_ROWS: usize = 64;
const MAX_BANDS: usize = 8;

fn matrix_pixels(
    primitive: &FilterPrimitive,
    matrix: &[f64; 20],
    units: &[f64],
    input: &[u8],
    output: &mut [u8],
) {
    let mut last: Option<([u8; 4], [u8; 4])> = None;
    for (source, destination) in input.chunks_exact(4).zip(output.chunks_exact_mut(4)) {
        let key = [source[0], source[1], source[2], source[3]];
        if let Some((seen, result)) = last {
            if seen == key {
                destination.copy_from_slice(&result);
                continue;
            }
        }
        let rgba = [
            units[usize::from(source[0])],
            units[usize::from(source[1])],
            units[usize::from(source[2])],
            units[usize::from(source[3])],
        ];
        if primitive.mode == 3 {
            destination[..3].fill(0);
            destination[3] = quantize(clamp01(
                LUMINANCE[0] * rgba[0] + LUMINANCE[1] * rgba[1] + LUMINANCE[2] * rgba[2],
            ));
        } else if source[3] == 0 {
            destination.fill(0);
        } else {
            for row in 0..4 {
                let mut value = matrix[row * 5 + 4];
                for (column, sample) in rgba.iter().enumerate() {
                    value += matrix[row * 5 + column] * sample;
                }
                destination[row] = quantize(clamp01(value));
            }
        }
        last = Some((
            key,
            [
                destination[0],
                destination[1],
                destination[2],
                destination[3],
            ],
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vexel_scene::FilterInput;

    #[test]
    fn unpaired_trig_matches_std_bits() {
        for angle in [0.0, 0.3, 1.0, 2.5, -4.25, 6.0] {
            assert_eq!(sin_unpaired(angle).to_bits(), f64::sin(angle).to_bits());
            assert_eq!(cos_unpaired(angle).to_bits(), f64::cos(angle).to_bits());
        }
    }

    fn primitive(mode: u8) -> FilterPrimitive {
        FilterPrimitive {
            kind: FilterKind::ColorMatrix,
            input: FilterInput::Graphic,
            input2: FilterInput::Graphic,
            merge_inputs: [FilterInput::Graphic; 8],
            merge_count: 0,
            mode,
            rgba: [0; 4],
            values: [
                0.3, 0.6, 0.1, 0.0, 0.05, 0.2, 0.5, 0.3, 0.0, -0.1, 0.1, 0.4, 0.9, 0.0, 0.2, 0.0,
                0.0, 0.0, 1.0, 0.0,
            ],
            source_offset: 0,
        }
    }

    /// 12x8 layer built from runs of identical pixels, alternating pairs and
    /// alpha-0 pixels, so the last-pixel memo is hit and missed.
    fn runs_image() -> Vec<u8> {
        let palette: [[u8; 4]; 6] = [
            [10, 200, 30, 255],
            [0, 0, 0, 0],
            [255, 128, 1, 0],
            [77, 77, 77, 128],
            [1, 2, 3, 255],
            [200, 10, 90, 37],
        ];
        let mut out = Vec::new();
        for i in 0..96usize {
            let pixel = match i % 6 {
                0..=2 => palette[(i / 12) % 6],
                3 => palette[1],
                4 => palette[4],
                _ => palette[(i / 3) % 6],
            };
            out.extend_from_slice(&pixel);
        }
        out
    }

    /// Per-pixel reference: no memo, no byte lookup table.
    fn reference(primitive: &FilterPrimitive, input: &[u8]) -> Vec<u8> {
        let matrix = matrix_for(primitive);
        let mut out = Vec::with_capacity(input.len());
        for source in input.chunks_exact(4) {
            let rgba: [f64; 4] = std::array::from_fn(|c| unit(source[c]));
            if primitive.mode == 3 {
                out.extend_from_slice(&[0, 0, 0]);
                out.push(quantize(clamp01(
                    LUMINANCE[0] * rgba[0] + LUMINANCE[1] * rgba[1] + LUMINANCE[2] * rgba[2],
                )));
            } else if source[3] == 0 {
                out.extend_from_slice(&[0; 4]);
            } else {
                for row in 0..4 {
                    let mut value = matrix[row * 5 + 4];
                    for (column, sample) in rgba.iter().enumerate() {
                        value += matrix[row * 5 + column] * sample;
                    }
                    out.push(quantize(clamp01(value)));
                }
            }
        }
        out
    }

    #[test]
    fn color_matrix_matches_a_per_pixel_reference_on_runs() {
        let input = runs_image();
        let row_bytes = 12 * 4;
        for mode in [0u8, 3] {
            let primitive = primitive(mode);
            let mut output = vec![0u8; input.len()];
            assert!(color_matrix(
                &primitive,
                &input,
                &mut output,
                &|| false,
                row_bytes
            ));
            assert_eq!(output, reference(&primitive, &input), "mode {mode}");
        }
    }

    #[test]
    fn banded_color_matrix_matches_the_reference_and_honours_cancel() {
        // 12 px wide, 1003 rows: several bands, the last one shorter.
        let tile = runs_image();
        let input: Vec<u8> = (0..1003 / 8 + 1)
            .flat_map(|_| tile.clone())
            .take(12 * 4 * 1003)
            .collect();
        for mode in [0u8, 3] {
            let primitive = primitive(mode);
            let mut output = vec![0u8; input.len()];
            assert!(color_matrix(
                &primitive,
                &input,
                &mut output,
                &|| false,
                12 * 4
            ));
            assert_eq!(output, reference(&primitive, &input), "mode {mode}");
        }
        let mut output = vec![0u8; input.len()];
        assert!(!color_matrix(
            &primitive(0),
            &input,
            &mut output,
            &|| true,
            12 * 4
        ));
    }

    #[test]
    fn banded_color_matrix_stops_when_control_cancels_midway() {
        // Only the first poll reports "keep going"; every later one cancels,
        // so the bands are cancelled while they run (or right after).
        let tile = runs_image();
        let input: Vec<u8> = (0..1003 / 8 + 1)
            .flat_map(|_| tile.clone())
            .take(12 * 4 * 1003)
            .collect();
        let polls = std::sync::atomic::AtomicUsize::new(0);
        let control = || polls.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 1;
        let mut output = vec![0u8; input.len()];
        assert!(!color_matrix(
            &primitive(0),
            &input,
            &mut output,
            &control,
            12 * 4
        ));
    }
}
