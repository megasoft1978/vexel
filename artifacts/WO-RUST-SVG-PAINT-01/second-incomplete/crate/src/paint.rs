use vexel_geometry::{affine_map_point, Edge, Point};
use vexel_scene::{Paint, PaintKind, PaintStop, Spread};

pub const MAX_STOPS: usize = 65_536;
pub const MAX_GRADIENT_STOPS: usize = 256;
pub const MAX_TOKEN_VISITS: usize = 4_194_304;

/// Checked equivalent of the legacy pointer span helper. `formed` is true
/// only when a nonempty in-range slice was formed.
pub fn edge_span<'a>(edges: &'a [Edge], first: usize, count: usize, formed: &mut bool) -> Option<&'a [Edge]> {
    *formed = false;
    if count == 0 { return None; }
    let end = first.checked_add(count)?;
    let result = edges.get(first..end)?;
    *formed = true;
    Some(result)
}

/// Validates a stop span before sampling. Invalid untrusted paint data or a
/// nonfinite intermediate leaves `rgba` unchanged, matching the C no-write
/// fallbacks without allowing an out-of-range access.
pub fn sample(paint: &Paint, stops: &[PaintStop], x: f64, y: f64, rgba: &mut [u8; 4]) {
    let Some(end) = paint.stops.first.checked_add(paint.stops.count) else { return; };
    if paint.stops.count < 2 || end > stops.len() { return; }
    let section = &stops[paint.stops.first..end];
    if !valid_stops(section) { return; }
    let Some(local) = affine_map_point(paint.user_inverse, Point { x, y }) else { return; };
    let (mut gx, mut gy) = (local.x, local.y);
    if paint.object_bounding_box {
        let (w, h) = (paint.bounds[2] - paint.bounds[0], paint.bounds[3] - paint.bounds[1]);
        if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 { return; }
        gx = (local.x - paint.bounds[0]) / w;
        gy = (local.y - paint.bounds[1]) / h;
    }
    let Some(point) = affine_map_point(paint.gradient_inverse, Point { x: gx, y: gy }) else { return; };
    gx = point.x;
    gy = point.y;
    let t = match paint.kind {
        PaintKind::Linear => {
            let dx = paint.coordinates[2] - paint.coordinates[0];
            let dy = paint.coordinates[3] - paint.coordinates[1];
            let length = dx * dx + dy * dy;
            if length == 0.0 { 1.0 } else {
                ((gx - paint.coordinates[0]) * dx + (gy - paint.coordinates[1]) * dy) / length
            }
        }
        PaintKind::Radial => radial_t(paint, gx, gy),
        _ => return,
    };
    if !t.is_finite() { return; }
    let t = spread(t, paint.spread);
    if t <= section[0].offset { *rgba = section[0].rgba; return; }
    let last = section.len() - 1;
    if t >= section[last].offset { *rgba = section[last].rgba; return; }
    let mut right = 1;
    while right < last && t > section[right].offset { right += 1; }
    let left = right - 1;
    let range = section[right].offset - section[left].offset;
    let amount = if range <= 0.0 { 1.0 } else { (t - section[left].offset) / range };
    let mut output = [0; 4];
    for channel in 0..4 {
        output[channel] = ((1.0 - amount) * f64::from(section[left].rgba[channel])
            + amount * f64::from(section[right].rgba[channel]))
            .round().clamp(0.0, 255.0) as u8;
    }
    *rgba = output;
}

fn valid_stops(stops: &[PaintStop]) -> bool {
    stops.len() >= 2
        && stops.iter().all(|stop| stop.offset.is_finite() && (0.0..=1.0).contains(&stop.offset))
        && stops.windows(2).all(|pair| pair[1].offset >= pair[0].offset)
}

fn spread(t: f64, spread: Spread) -> f64 {
    match spread {
        Spread::Pad => t,
        Spread::Repeat => t - t.floor(),
        Spread::Reflect => {
            let mut value = t % 2.0;
            if value < 0.0 { value += 2.0; }
            if value > 1.0 { 2.0 - value } else { value }
        }
    }
}

fn radial_t(paint: &Paint, gx: f64, gy: f64) -> f64 {
    let (cx, cy, radius, fx, fy) = (paint.coordinates[0], paint.coordinates[1], paint.coordinates[2], paint.coordinates[3], paint.coordinates[4]);
    let (center_dx, center_dy, point_dx, point_dy) = (cx - fx, cy - fy, gx - fx, gy - fy);
    let a = center_dx * center_dx + center_dy * center_dy - radius * radius;
    let b = -2.0 * (point_dx * center_dx + point_dy * center_dy);
    let c = point_dx * point_dx + point_dy * point_dy;
    if a.abs() <= 1e-15 {
        if b.abs() <= 1e-15 { return f64::NAN; }
        return -c / b;
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 || !discriminant.is_finite() { return f64::NAN; }
    let root = discriminant.sqrt();
    let t0 = (-b - root) / (2.0 * a);
    let t1 = (-b + root) / (2.0 * a);
    let t = t0.max(t1);
    if t < 0.0 { t0.min(t1) } else { t }
}
