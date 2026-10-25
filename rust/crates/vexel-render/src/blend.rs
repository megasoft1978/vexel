//! `mix-blend-mode` pixel compositing: W3C Compositing and Blending Level 1
//! on straight RGBA8 with integer arithmetic only.
//!
//! With `B` the separable blend function, backdrop `(cb, ab)` and source
//! `(cs, as)` (all bytes, straight alpha), the spec gives
//! `Cr = (1 - ab) * cs + ab * B(cb, cs)` and the source-over result
//! `co = as * Cr + (1 - as) * ab * cb`, `ao = as + ab - as * ab`. Every term
//! is evaluated as an exact integer numerator (`B` at scale 255^2) and divided
//! once with round-half-up, so there is no float and no platform dependence.
//! `plus-lighter` is `co = min(1, as * cs + ab * cb)`, `ao = min(1, as + ab)`.

/// Mode code of `mix-blend-mode: plus-lighter` (see `vexel_scene::BLEND_MODE_NAMES`).
pub const PLUS_LIGHTER: u8 = 12;

const S: i64 = 255 * 255;

fn isqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut x = (value as f64).sqrt() as u64;
    while x * x > value {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= value {
        x += 1;
    }
    x
}

/// `screen(cb, cs)` at scale 255^2.
fn screen(cb: i64, cs: i64) -> i64 {
    255 * (cb + cs) - cb * cs
}

/// `HardLight(cb, cs)` at scale 255^2.
fn hard_light(cb: i64, cs: i64) -> i64 {
    if 2 * cs <= 255 {
        cb * 2 * cs
    } else {
        screen(cb, 2 * cs - 255)
    }
}

/// `SoftLight(cb, cs)` at scale 255^2.
fn soft_light(cb: i64, cs: i64) -> i64 {
    if 2 * cs <= 255 {
        255 * cb - ((255 - 2 * cs) * cb * (255 - cb) + 127) / 255
    } else {
        let d = if 4 * cb <= 255 {
            (((16 * cb - 3060) * cb + 260_100) * cb + 127) / 255
        } else {
            isqrt(16_581_375 * cb as u64) as i64
        };
        255 * cb + ((2 * cs - 255) * (d - 255 * cb)) / 255
    }
}

/// The separable blend function `B(cb, cs)` at scale 255^2, `mode` 0..=11.
fn separable(mode: u8, cb: i64, cs: i64) -> i64 {
    match mode {
        1 => cb * cs,
        2 => screen(cb, cs),
        3 => cb.min(cs) * 255,
        4 => cb.max(cs) * 255,
        5 => hard_light(cs, cb),
        6 => {
            if cb == 0 {
                0
            } else if cs == 255 {
                S
            } else {
                (cb * S / (255 - cs)).min(S)
            }
        }
        7 => {
            if cb == 255 {
                S
            } else if cs == 0 {
                0
            } else {
                S - ((255 - cb) * S / cs).min(S)
            }
        }
        8 => hard_light(cb, cs),
        9 => soft_light(cb, cs),
        10 => (cb - cs).abs() * 255,
        11 => 255 * (cb + cs) - 2 * cb * cs,
        _ => cs * 255,
    }
}

/// Composites straight RGBA8 `source` (alpha already scaled by the group
/// opacity) over `destination` with blend `mode` (0 = normal).
pub fn blend_pixel(mode: u8, destination: &mut [u8], source: [u8; 4]) {
    let (a_s, a_b) = (i64::from(source[3]), i64::from(destination[3]));
    if a_s == 0 {
        return;
    }
    if mode == PLUS_LIGHTER {
        let alpha = (a_s + a_b).min(255);
        for c in 0..3 {
            let q = (i64::from(source[c]) * a_s + i64::from(destination[c]) * a_b).min(S);
            destination[c] = ((2 * q + alpha) / (2 * alpha)).min(255) as u8;
        }
        destination[3] = alpha as u8;
        return;
    }
    let numerator = 255 * a_s + 255 * a_b - a_s * a_b;
    for c in 0..3 {
        let (cs, cb) = (i64::from(source[c]), i64::from(destination[c]));
        let mixed = (255 - a_b) * cs * 255 + a_b * separable(mode, cb, cs);
        let premultiplied = a_s * mixed + 255 * (255 - a_s) * a_b * cb;
        let divisor = 255 * numerator;
        destination[c] = ((2 * premultiplied + divisor) / (2 * divisor)).min(255) as u8;
    }
    destination[3] = ((numerator + 127) / 255) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Float oracle straight from the W3C text.
    fn oracle(mode: u8, cb: f64, cs: f64) -> f64 {
        let hard = |b: f64, s: f64| {
            if s <= 0.5 { b * 2.0 * s } else { b + (2.0 * s - 1.0) - b * (2.0 * s - 1.0) }
        };
        match mode {
            1 => cb * cs,
            2 => cb + cs - cb * cs,
            3 => cb.min(cs),
            4 => cb.max(cs),
            5 => hard(cs, cb),
            6 => if cb == 0.0 { 0.0 } else if cs >= 1.0 { 1.0 } else { (cb / (1.0 - cs)).min(1.0) },
            7 => if cb >= 1.0 { 1.0 } else if cs <= 0.0 { 0.0 } else { 1.0 - ((1.0 - cb) / cs).min(1.0) },
            8 => hard(cb, cs),
            9 => {
                if cs <= 0.5 {
                    cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
                } else {
                    let d = if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() };
                    cb + (2.0 * cs - 1.0) * (d - cb)
                }
            }
            10 => (cb - cs).abs(),
            11 => cb + cs - 2.0 * cb * cs,
            _ => cs,
        }
    }

    #[test]
    fn opaque_pixels_match_the_spec_formulas_for_every_mode() {
        for mode in 0..=11u8 {
            for cb in (0..=255u32).step_by(5) {
                for cs in (0..=255u32).step_by(5) {
                    let mut dst = [cb as u8, 0, 0, 255];
                    blend_pixel(mode, &mut dst, [cs as u8, 0, 0, 255]);
                    let expect = oracle(mode, f64::from(cb) / 255.0, f64::from(cs) / 255.0) * 255.0;
                    assert!(
                        (f64::from(dst[0]) - expect).abs() <= 1.0,
                        "mode {mode} cb {cb} cs {cs}: {} vs {expect}",
                        dst[0]
                    );
                    assert_eq!(dst[3], 255);
                }
            }
        }
    }

    #[test]
    fn translucent_source_matches_the_full_formula() {
        for mode in 0..=11u8 {
            let (cb, cs, ab, a_s) = (0.3f64, 0.8f64, 0.6f64, 0.5f64);
            let to = |v: f64| (v * 255.0).round() as u8;
            let mut dst = [to(cb), to(cb), to(cb), to(ab)];
            blend_pixel(mode, &mut dst, [to(cs), to(cs), to(cs), to(a_s)]);
            let (cbq, csq, abq, asq) = (
                f64::from(to(cb)) / 255.0,
                f64::from(to(cs)) / 255.0,
                f64::from(to(ab)) / 255.0,
                f64::from(to(a_s)) / 255.0,
            );
            let cr = (1.0 - abq) * csq + abq * oracle(mode, cbq, csq);
            let ao = asq + abq - asq * abq;
            let co = asq * cr + (1.0 - asq) * abq * cbq;
            assert!((f64::from(dst[0]) - co / ao * 255.0).abs() <= 1.0, "mode {mode}");
            assert!((f64::from(dst[3]) - ao * 255.0).abs() <= 1.0, "alpha mode {mode}");
        }
    }

    #[test]
    fn normal_mode_equals_source_over() {
        let mut dst = [10, 200, 90, 130];
        blend_pixel(0, &mut dst, [250, 20, 30, 77]);
        let (a_s, a_b) = (77u32, 130u32);
        let n = a_s * 255 + a_b * (255 - a_s);
        let expect = ((250 * a_s * 255 + 10 * a_b * (255 - a_s) + n / 2) / n) as u8;
        assert_eq!(dst[0], expect);
        assert_eq!(dst[3], ((n + 127) / 255) as u8);
    }

    #[test]
    fn plus_lighter_adds_premultiplied_and_clamps() {
        let mut dst = [100, 100, 100, 255];
        blend_pixel(PLUS_LIGHTER, &mut dst, [200, 50, 0, 255]);
        assert_eq!(dst, [255, 150, 100, 255]);
        let mut half = [0, 0, 0, 0];
        blend_pixel(PLUS_LIGHTER, &mut half, [90, 40, 10, 128]);
        assert_eq!(half, [90, 40, 10, 128]);
        let mut glow = [60, 60, 60, 128];
        blend_pixel(PLUS_LIGHTER, &mut glow, [200, 200, 200, 128]);
        assert_eq!(glow[3], 255);
        // premult sum (60+200)*128/255 = 130.5 over alpha 1.
        assert!((i32::from(glow[0]) - 130).abs() <= 1);
    }

    #[test]
    fn known_analytic_values() {
        let mut dst = [128, 128, 128, 255];
        blend_pixel(1, &mut dst, [128, 128, 128, 255]);
        assert_eq!(dst[0], 64); // multiply: 0.502^2 = 0.252
        let mut dst = [128, 0, 255, 255];
        blend_pixel(10, &mut dst, [200, 0, 255, 255]);
        assert_eq!(dst, [72, 0, 0, 255]);
        let mut dst = [100, 100, 100, 255];
        blend_pixel(2, &mut dst, [0, 0, 0, 255]);
        assert_eq!(dst[0], 100); // screen with black is identity
    }
}
