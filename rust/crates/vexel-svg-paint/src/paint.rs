use vexel_geometry::{Affine, Edge, affine_valid};
use vexel_scene::{Paint, PaintKind, PaintStop, Spread};

pub const MAX_STOPS: usize = 65_536;
pub const MAX_GRADIENT_STOPS: usize = 256;
pub const MAX_TOKEN_VISITS: usize = 4_194_304;

/// Checked equivalent of the legacy pointer span helper. `formed` is true
/// only when a nonempty in-range slice was formed.
pub fn edge_span<'a>(
    edges: &'a [Edge],
    first: usize,
    count: usize,
    formed: &mut bool,
) -> Option<&'a [Edge]> {
    *formed = false;
    if count == 0 {
        return None;
    }
    let end = first.checked_add(count)?;
    let result = edges.get(first..end)?;
    *formed = true;
    Some(result)
}

/// A gradient paint with everything that does not depend on the sample point
/// checked or computed once: stop validation, matrix validity, bounding-box
/// size and the per-kind constants. Per sample it performs the same floating
/// point operations in the same order as the original single-call sampler, so
/// the colours are bit-identical.
#[derive(Debug, Clone, Copy)]
pub struct PreparedPaint<'a> {
    paint: &'a Paint,
    section: &'a [PaintStop],
    /// `(width, height)` of the bounding box when `object_bounding_box`.
    bbox: Option<(f64, f64)>,
    /// Linear: `(dx, dy, length)`; radial: `(center_dx, center_dy, a)`.
    constants: [f64; 3],
}

impl<'a> PreparedPaint<'a> {
    /// `None` when sampling could never write a colour: bad stop span or
    /// stops, an invalid matrix, an empty bounding box or a non-gradient kind.
    pub fn new(paint: &'a Paint, stops: &'a [PaintStop]) -> Option<Self> {
        let end = paint.stops.first.checked_add(paint.stops.count)?;
        if paint.stops.count < 2 || paint.stops.count > MAX_GRADIENT_STOPS || end > stops.len() {
            return None;
        }
        let section = &stops[paint.stops.first..end];
        if !valid_stops(section)
            || !affine_valid(paint.user_inverse)
            || !affine_valid(paint.gradient_inverse)
        {
            return None;
        }
        let bbox = if paint.object_bounding_box {
            let (w, h) = (
                paint.bounds[2] - paint.bounds[0],
                paint.bounds[3] - paint.bounds[1],
            );
            if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
                return None;
            }
            Some((w, h))
        } else {
            None
        };
        let c = &paint.coordinates;
        let constants = match paint.kind {
            PaintKind::Linear => {
                let (dx, dy) = (c[2] - c[0], c[3] - c[1]);
                [dx, dy, dx * dx + dy * dy]
            }
            PaintKind::Radial => {
                let (center_dx, center_dy) = (c[0] - c[3], c[1] - c[4]);
                [
                    center_dx,
                    center_dy,
                    center_dx * center_dx + center_dy * center_dy - c[2] * c[2],
                ]
            }
            _ => return None,
        };
        Some(Self {
            paint,
            section,
            bbox,
            constants,
        })
    }

    /// Writes the colour at `(x, y)`, or leaves `rgba` unchanged when the point
    /// cannot be placed.
    pub fn sample(&self, x: f64, y: f64, rgba: &mut [u8; 4]) {
        let paint = self.paint;
        let section = self.section;
        let Some((mut gx, mut gy)) = map_valid(paint.user_inverse, x, y) else {
            return;
        };
        if let Some((w, h)) = self.bbox {
            gx = (gx - paint.bounds[0]) / w;
            gy = (gy - paint.bounds[1]) / h;
        }
        let Some((gx, gy)) = map_valid(paint.gradient_inverse, gx, gy) else {
            return;
        };
        let t = if paint.kind == PaintKind::Linear {
            let [dx, dy, length] = self.constants;
            if length == 0.0 {
                1.0
            } else {
                ((gx - paint.coordinates[0]) * dx + (gy - paint.coordinates[1]) * dy) / length
            }
        } else {
            radial_t(paint, self.constants, gx, gy)
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
}

/// [`affine_map_point`] for a matrix already known to be valid.
#[inline]
fn map_valid(m: Affine, x: f64, y: f64) -> Option<(f64, f64)> {
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    let (mx, my) = (m.a * x + m.c * y + m.e, m.b * x + m.d * y + m.f);
    (mx.is_finite() && my.is_finite() && mx.abs() <= 1.0e6 && my.abs() <= 1.0e6).then_some((mx, my))
}

/// Validates a stop span before sampling. Invalid untrusted paint data or a
/// nonfinite intermediate leaves `rgba` unchanged, matching the C no-write
/// fallbacks without allowing an out-of-range access.
pub fn sample(paint: &Paint, stops: &[PaintStop], x: f64, y: f64, rgba: &mut [u8; 4]) {
    if let Some(prepared) = PreparedPaint::new(paint, stops) {
        prepared.sample(x, y, rgba);
    }
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

fn radial_t(paint: &Paint, constants: [f64; 3], gx: f64, gy: f64) -> f64 {
    let (fx, fy) = (paint.coordinates[3], paint.coordinates[4]);
    let [center_dx, center_dy, a] = constants;
    let (point_dx, point_dy) = (gx - fx, gy - fy);
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
