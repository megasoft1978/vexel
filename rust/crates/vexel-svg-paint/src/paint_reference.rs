//! The pre-optimisation sampler, kept verbatim as the byte-identity oracle for
//! [`crate::PreparedPaint`].
use crate::paint::MAX_GRADIENT_STOPS;
use vexel_geometry::{Point, affine_map_point};
use vexel_scene::{Paint, PaintKind, PaintStop, Spread};

/// Validates a stop span before sampling. Invalid untrusted paint data or a
/// nonfinite intermediate leaves `rgba` unchanged, matching the C no-write
/// fallbacks without allowing an out-of-range access.
pub(crate) fn sample_reference(
    paint: &Paint,
    stops: &[PaintStop],
    x: f64,
    y: f64,
    rgba: &mut [u8; 4],
) {
    let Some(end) = paint.stops.first.checked_add(paint.stops.count) else {
        return;
    };
    if paint.stops.count < 2 || paint.stops.count > MAX_GRADIENT_STOPS || end > stops.len() {
        return;
    }
    let section = &stops[paint.stops.first..end];
    if !valid_stops(section) {
        return;
    }
    let Some(local) = affine_map_point(paint.user_inverse, Point { x, y }) else {
        return;
    };
    let (mut gx, mut gy) = (local.x, local.y);
    if paint.object_bounding_box {
        let (w, h) = (
            paint.bounds[2] - paint.bounds[0],
            paint.bounds[3] - paint.bounds[1],
        );
        if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
            return;
        }
        gx = (local.x - paint.bounds[0]) / w;
        gy = (local.y - paint.bounds[1]) / h;
    }
    let Some(point) = affine_map_point(paint.gradient_inverse, Point { x: gx, y: gy }) else {
        return;
    };
    gx = point.x;
    gy = point.y;
    let t = match paint.kind {
        PaintKind::Linear => {
            let dx = paint.coordinates[2] - paint.coordinates[0];
            let dy = paint.coordinates[3] - paint.coordinates[1];
            let length = dx * dx + dy * dy;
            if length == 0.0 {
                1.0
            } else {
                ((gx - paint.coordinates[0]) * dx + (gy - paint.coordinates[1]) * dy) / length
            }
        }
        PaintKind::Radial => radial_t(paint, gx, gy),
        _ => return,
    };
    if !t.is_finite() {
        return;
    }
    let t = spread(t, paint.spread);
    if t <= section[0].offset {
        *rgba = section[0].rgba;
        return;
    }
    let last = section.len() - 1;
    if t >= section[last].offset {
        *rgba = section[last].rgba;
        return;
    }
    let mut right = 1;
    while right < last && t > section[right].offset {
        right += 1;
    }
    let left = right - 1;
    let range = section[right].offset - section[left].offset;
    let amount = if range <= 0.0 {
        1.0
    } else {
        (t - section[left].offset) / range
    };
    let mut output = [0; 4];
    for channel in 0..4 {
        output[channel] = ((1.0 - amount) * f64::from(section[left].rgba[channel])
            + amount * f64::from(section[right].rgba[channel]))
        .round()
        .clamp(0.0, 255.0) as u8;
    }
    *rgba = output;
}

fn valid_stops(stops: &[PaintStop]) -> bool {
    stops.len() >= 2
        && stops
            .iter()
            .all(|stop| stop.offset.is_finite() && (0.0..=1.0).contains(&stop.offset))
        && stops
            .windows(2)
            .all(|pair| pair[1].offset >= pair[0].offset)
}

fn spread(t: f64, spread: Spread) -> f64 {
    match spread {
        Spread::Pad => t,
        Spread::Repeat => t - t.floor(),
        Spread::Reflect => {
            let mut value = t % 2.0;
            if value < 0.0 {
                value += 2.0;
            }
            if value > 1.0 { 2.0 - value } else { value }
        }
    }
}

fn radial_t(paint: &Paint, gx: f64, gy: f64) -> f64 {
    let (cx, cy, radius, fx, fy) = (
        paint.coordinates[0],
        paint.coordinates[1],
        paint.coordinates[2],
        paint.coordinates[3],
        paint.coordinates[4],
    );
    let (center_dx, center_dy, point_dx, point_dy) = (cx - fx, cy - fy, gx - fx, gy - fy);
    let a = center_dx * center_dx + center_dy * center_dy - radius * radius;
    let b = -2.0 * (point_dx * center_dx + point_dy * center_dy);
    let c = point_dx * point_dx + point_dy * point_dy;
    if a.abs() <= 1e-15 {
        if b.abs() <= 1e-15 {
            return f64::NAN;
        }
        return -c / b;
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 || !discriminant.is_finite() {
        return f64::NAN;
    }
    let root = discriminant.sqrt();
    let t0 = (-b - root) / (2.0 * a);
    let t1 = (-b + root) / (2.0 * a);
    let t = t0.max(t1);
    if t < 0.0 { t0.min(t1) } else { t }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PreparedPaint;
    use vexel_geometry::Affine;
    use vexel_scene::TableRange;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn unit(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
        fn range(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (hi - lo) * self.unit()
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
        /// Mostly ordinary numbers, sometimes degenerate or non-finite ones.
        fn wild(&mut self, lo: f64, hi: f64) -> f64 {
            match self.below(40) {
                0 => 0.0,
                1 => f64::NAN,
                2 => f64::INFINITY,
                3 => 1.0e13,
                4 => -0.0,
                _ => self.range(lo, hi),
            }
        }
        fn affine(&mut self) -> Affine {
            if self.below(3) == 0 {
                return vexel_geometry::affine_identity();
            }
            Affine {
                a: self.wild(-2.0, 2.0),
                b: self.wild(-1.0, 1.0),
                c: self.wild(-1.0, 1.0),
                d: self.wild(-2.0, 2.0),
                e: self.wild(-300.0, 300.0),
                f: self.wild(-300.0, 300.0),
            }
        }
    }

    #[test]
    fn prepared_sampling_is_bit_identical_to_the_original_sampler() {
        let mut rng = Rng(0x1234_5678_9ABC_DEF1);
        let (mut written, mut skipped) = (0u32, 0u32);
        for _ in 0..4000 {
            let count = 1 + rng.below(6);
            let mut offset = 0.0f64;
            let stops: Vec<PaintStop> = (0..count + 2)
                .map(|_| {
                    offset = if rng.below(30) == 0 {
                        rng.wild(-0.5, 1.5)
                    } else {
                        (offset + rng.range(0.0, 0.4)).min(1.0)
                    };
                    PaintStop {
                        offset,
                        rgba: [
                            rng.below(256) as u8,
                            rng.below(256) as u8,
                            rng.below(256) as u8,
                            rng.below(256) as u8,
                        ],
                    }
                })
                .collect();
            let paint = Paint {
                kind: [PaintKind::Linear, PaintKind::Radial, PaintKind::Solid][rng.below(3)],
                spread: [Spread::Pad, Spread::Reflect, Spread::Repeat][rng.below(3)],
                object_bounding_box: rng.below(2) == 0,
                coordinates: [
                    rng.wild(-50.0, 150.0),
                    rng.wild(-50.0, 150.0),
                    rng.wild(-50.0, 150.0),
                    rng.wild(-50.0, 150.0),
                    rng.wild(-50.0, 150.0),
                    0.0,
                ],
                bounds: [
                    rng.wild(-20.0, 20.0),
                    rng.wild(-20.0, 20.0),
                    rng.wild(30.0, 120.0),
                    rng.wild(30.0, 120.0),
                ],
                user_inverse: rng.affine(),
                gradient_inverse: rng.affine(),
                stops: TableRange {
                    first: rng.below(3),
                    count: rng.below(8),
                },
                pattern: None,
                pattern_inverse: vexel_geometry::affine_identity(),
                pattern_opacity: 1.0,
            };
            let prepared = PreparedPaint::new(&paint, &stops);
            for _ in 0..40 {
                let (x, y) = (rng.wild(-200.0, 300.0), rng.wild(-200.0, 300.0));
                let (mut old, mut new) = ([1u8, 2, 3, 4], [1u8, 2, 3, 4]);
                sample_reference(&paint, &stops, x, y, &mut old);
                if let Some(p) = &prepared {
                    p.sample(x, y, &mut new);
                }
                assert_eq!(old, new, "{paint:?} at {x} {y}");
                if old == [1, 2, 3, 4] {
                    skipped += 1
                } else {
                    written += 1
                }
            }
        }
        assert!(
            written > 10_000 && skipped > 10_000,
            "{written} written, {skipped} skipped"
        );
    }
}
