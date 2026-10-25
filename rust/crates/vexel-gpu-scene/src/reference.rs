//! Pure-Rust reference rasterizer of the ENCODED scene: the oracle for the
//! Metal kernel and proof that the encoding is complete. It does what the
//! kernel will do, per 16x16 tile and pixel: 4x4 samples at the layout's
//! sample locations, exact integer crossing tests on the tile entry's
//! tile-local fixed-point edges,
//! gradient colours in `f32`, one source-over per shape per sample, then the
//! CPU's integer resolve and pixel source-over (`vexel-render` `composite`).

use crate::encode::Encoded;
use crate::layout::{
    self, EDGE_FIXED_SHIFT, FILL_EVENODD, GpuEdge, GpuStop, PAINT_RADIAL, PAINT_SOLID,
    SAMPLE_COUNT, SAMPLES_PER_AXIS, SPREAD_PAD, SPREAD_REFLECT, TILE_SIZE,
};

/// Fixed-point distance between neighbouring samples (a quarter pixel).
const SAMPLE_UNIT: i32 = 1 << (EDGE_FIXED_SHIFT - 2);

/// Renders to straight-alpha RGBA8, row-major, byte-comparable with the CPU
/// rasterizer's tile.
pub fn render_straight(e: &Encoded) -> Vec<u8> {
    let (w, h) = (e.header.width as usize, e.header.height as usize);
    let bg = e.header.background_rgba.to_le_bytes();
    let mut out = Vec::with_capacity(w * h * 4);
    for _ in 0..w * h {
        out.extend_from_slice(&bg);
    }
    let tile_px = TILE_SIZE as usize;
    let mut spans: Vec<GpuEdge> = Vec::new();
    for ty in 0..e.header.tiles_y as usize {
        for tx in 0..e.header.tiles_x as usize {
            let t = e.tiles[ty * e.header.tiles_x as usize + tx];
            let list = &e.tile_list[t.list_start as usize..][..t.list_count as usize];
            let (x_px0, y_px0) = (tx * tile_px, ty * tile_px);
            let x_end = (x_px0 + tile_px).min(w);
            let y_end = (y_px0 + tile_px).min(h);
            for entry in list {
                let shape = &e.shapes[entry.shape as usize];
                let edges = &e.edges[entry.edge_start as usize..][..entry.edge_count as usize];
                let mut masks = [0u16; (TILE_SIZE * TILE_SIZE) as usize];
                for sr in 0..tile_px * SAMPLES_PER_AXIS as usize {
                    // Tile-local sample row centre.
                    let py = (sr as i32) * SAMPLE_UNIT + SAMPLE_UNIT / 2;
                    spans.clear();
                    spans.extend(edges.iter().copied().filter(|g| spans_row(g, py)));
                    if spans.is_empty() {
                        continue;
                    }
                    let sample_row = (y_px0 * 4 + sr) as u32;
                    if sample_row < e.header.content_y0 || sample_row >= e.header.content_y1 {
                        continue;
                    }
                    let sy = sr % 4;
                    let row_px = y_px0 + sr / 4;
                    if row_px >= y_end {
                        continue;
                    }
                    for col in x_px0..x_end {
                        for sx in 0..4 {
                            let sample_col = (col * 4 + sx) as u32;
                            if sample_col < e.header.content_x0 || sample_col >= e.header.content_x1
                            {
                                continue;
                            }
                            let px =
                                (((col - x_px0) * 4 + sx) as i32) * SAMPLE_UNIT + SAMPLE_UNIT / 2;
                            if covers(&spans, px, py, shape.fill_rule == FILL_EVENODD) {
                                masks[(row_px - y_px0) * tile_px + (col - x_px0)] |=
                                    1 << (sy * 4 + sx);
                            }
                        }
                    }
                }
                shade(
                    e,
                    shape,
                    &masks,
                    [x_px0, y_px0],
                    [x_end, y_end],
                    w,
                    &mut out,
                );
            }
        }
    }
    out
}

/// [`render_straight`] converted to premultiplied RGBA8 (`(c * a + 127) / 255`).
pub fn render(e: &Encoded) -> Vec<u8> {
    let mut px = render_straight(e);
    for p in px.chunks_exact_mut(4) {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
    px
}

fn spans_row(g: &GpuEdge, py: i32) -> bool {
    (g.y0 <= py && g.y1 > py) || (g.y1 <= py && g.y0 > py)
}

/// Winding or parity of the spanning `edges` for a sample (all span `py`).
fn covers(edges: &[GpuEdge], px: i32, py: i32, evenodd: bool) -> bool {
    let (mut winding, mut parity) = (0i32, false);
    for g in edges {
        // crossing x > px  <=>  ((x0-px)*dy + (py-y0)*dx) / dy > 0
        // i64 throughout: differences of coordinates up to 2^30 exceed i32.
        let dy = i64::from(g.y1) - i64::from(g.y0);
        let num = (i64::from(g.x0) - i64::from(px)) * dy
            + (i64::from(py) - i64::from(g.y0)) * (i64::from(g.x1) - i64::from(g.x0));
        if (dy > 0 && num > 0) || (dy < 0 && num < 0) {
            winding += if dy > 0 { 1 } else { -1 };
            parity = !parity;
        }
    }
    if evenodd { parity } else { winding != 0 }
}

/// Source-over of one shape's coverage masks onto the pixel buffer.
fn shade(
    e: &Encoded,
    shape: &layout::GpuShape,
    masks: &[u16],
    origin: [usize; 2],
    end: [usize; 2],
    width: usize,
    out: &mut [u8],
) {
    let solid = shape.color_rgba.to_le_bytes();
    let gradient = (shape.paint_kind != PAINT_SOLID).then(|| {
        (
            &e.gradients[shape.gradient_index as usize],
            shape.paint_kind,
        )
    });
    for y in origin[1]..end[1] {
        for x in origin[0]..end[0] {
            let mask = masks[(y - origin[1]) * TILE_SIZE as usize + (x - origin[0])];
            if mask == 0 {
                continue;
            }
            let mut samples = [[0u8; 4]; SAMPLE_COUNT as usize];
            for (i, sample) in samples.iter_mut().enumerate() {
                if mask & (1 << i) == 0 {
                    continue;
                }
                let color = match gradient {
                    None => solid,
                    Some((g, kind)) => {
                        let (sx, sy) = (i % 4, i / 4);
                        let fx = x as f32 + layout::SAMPLE_LOCATIONS[sx];
                        let fy = y as f32 + layout::SAMPLE_LOCATIONS[sy];
                        match gradient_color(g, kind, &e.stops, fx, fy) {
                            Some(c) => c,
                            None => continue,
                        }
                    }
                };
                source_over(sample, color);
            }
            let source = resolve_samples(&samples);
            let offset = (y * width + x) * 4;
            let pixel: &mut [u8; 4] = (&mut out[offset..offset + 4]).try_into().unwrap();
            source_over(pixel, source);
        }
    }
}

/// Gradient colour at a device point; `None` when the point is not placed.
fn gradient_color(
    g: &layout::GpuGradient,
    kind: u32,
    all_stops: &[GpuStop],
    x: f32,
    y: f32,
) -> Option<[u8; 4]> {
    let m = g.inverse;
    let (gx, gy) = (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
    if !(gx.is_finite() && gy.is_finite()) {
        return None;
    }
    let t = if kind == PAINT_RADIAL {
        radial_t(g.focal_ratio, gx, gy)
    } else {
        gx
    };
    if !t.is_finite() {
        return None;
    }
    let t = spread(t, g.spread);
    let stops = &all_stops[g.stop_start as usize..][..g.stop_count as usize];
    let last = stops.len() - 1;
    let bytes = |s: &GpuStop| s.rgba.map(|v| v as u8);
    if t <= stops[0].offset {
        return Some(bytes(&stops[0]));
    }
    if t >= stops[last].offset {
        return Some(bytes(&stops[last]));
    }
    let mut right = 1;
    while right < last && t > stops[right].offset {
        right += 1;
    }
    let (l, r) = (&stops[right - 1], &stops[right]);
    let range = r.offset - l.offset;
    let amount = if range <= 0.0 {
        1.0
    } else {
        (t - l.offset) / range
    };
    let mut rgba = [0u8; 4];
    for c in 0..4 {
        rgba[c] = ((1.0 - amount) * l.rgba[c] + amount * r.rgba[c])
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    Some(rgba)
}

/// Focal radial `t` in unit-circle space: centre `(0,0)`, radius 1, focal
/// `(fr, 0)`; same root selection as the CPU sampler.
fn radial_t(fr: f32, gx: f32, gy: f32) -> f32 {
    let a = fr * fr - 1.0;
    let (pdx, pdy) = (gx - fr, gy);
    let b = 2.0 * pdx * fr; // -2 * (pd . (c - f)), c - f = (-fr, 0)
    let c = pdx * pdx + pdy * pdy;
    if a.abs() <= 1e-15 {
        if b.abs() <= 1e-15 {
            return f32::NAN;
        }
        return -c / b;
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 || !disc.is_finite() {
        return f32::NAN;
    }
    let root = disc.sqrt();
    let t0 = (-b - root) / (2.0 * a);
    let t1 = (-b + root) / (2.0 * a);
    let t = t0.max(t1);
    if t < 0.0 { t0.min(t1) } else { t }
}

fn spread(t: f32, mode: u32) -> f32 {
    match mode {
        SPREAD_PAD => t,
        SPREAD_REFLECT => {
            let mut v = t % 2.0;
            if v < 0.0 {
                v += 2.0;
            }
            if v > 1.0 { 2.0 - v } else { v }
        }
        _ => t - t.floor(),
    }
}

/// `vexel-render` `composite::source_over`.
fn source_over(destination: &mut [u8; 4], source: [u8; 4]) {
    let sa = u32::from(source[3]);
    if sa == 0 {
        return;
    }
    let da = u32::from(destination[3]);
    let inverse = 255 - sa;
    let alpha_num = sa * 255 + da * inverse;
    for c in 0..3 {
        let num = u32::from(source[c]) * sa * 255 + u32::from(destination[c]) * da * inverse;
        destination[c] = ((num + alpha_num / 2) / alpha_num) as u8;
    }
    destination[3] = ((alpha_num + 127) / 255) as u8;
}

/// `vexel-render` `composite::resolve_samples`.
fn resolve_samples(samples: &[[u8; 4]; SAMPLE_COUNT as usize]) -> [u8; 4] {
    let mut alpha_sum = 0u32;
    let mut premultiplied = [0u32; 3];
    for p in samples {
        let a = u32::from(p[3]);
        alpha_sum += a;
        for c in 0..3 {
            premultiplied[c] += u32::from(p[c]) * a;
        }
    }
    if alpha_sum == 0 {
        return [0; 4];
    }
    let mut rgba = [0u8; 4];
    for c in 0..3 {
        rgba[c] = ((premultiplied[c] + alpha_sum / 2) / alpha_sum) as u8;
    }
    rgba[3] = ((alpha_sum + SAMPLE_COUNT / 2) / SAMPLE_COUNT) as u8;
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::encode;
    use crate::input::{GpuPaint, GpuScene, GpuSegment};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GpuSegment> {
        let s = |a, b, c, d| GpuSegment {
            x0: a,
            y0: b,
            x1: c,
            y1: d,
        };
        vec![
            s(x0, y0, x1, y0),
            s(x1, y0, x1, y1),
            s(x1, y1, x0, y1),
            s(x0, y1, x0, y0),
        ]
    }

    fn px(buf: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
        buf[(y * w + x) * 4..][..4].try_into().unwrap()
    }

    #[test]
    fn solid_rect_with_half_pixel_edges() {
        let mut scene = GpuScene::new(20, 20);
        scene.push_fill(
            rect(2.5, 3.0, 10.0, 8.5),
            false,
            GpuPaint::Solid([200, 100, 50, 255]),
        );
        let img = render_straight(&encode(&scene).unwrap());
        assert_eq!(px(&img, 20, 5, 5), [200, 100, 50, 255]);
        assert_eq!(px(&img, 20, 1, 5), [0; 4]);
        // Half-covered column x = 2: samples 2,3 of each row => alpha 128.
        assert_eq!(px(&img, 20, 2, 5), [200, 100, 50, 128]);
        // Half-covered bottom row y = 8.
        assert_eq!(px(&img, 20, 5, 8), [200, 100, 50, 128]);
        assert_eq!(px(&img, 20, 5, 9), [0; 4]);
    }

    #[test]
    fn evenodd_hole_and_nonzero_fill() {
        let mut edges = rect(1.0, 1.0, 15.0, 15.0);
        edges.extend(rect(5.0, 5.0, 10.0, 10.0));
        let mut scene = GpuScene::new(16, 16);
        scene.push_fill(edges.clone(), true, GpuPaint::Solid([255, 0, 0, 255]));
        let img = render_straight(&encode(&scene).unwrap());
        assert_eq!(px(&img, 16, 7, 7), [0; 4]);
        assert_eq!(px(&img, 16, 3, 3), [255, 0, 0, 255]);
        let mut scene = GpuScene::new(16, 16);
        scene.push_fill(edges, false, GpuPaint::Solid([255, 0, 0, 255]));
        let img = render_straight(&encode(&scene).unwrap());
        assert_eq!(px(&img, 16, 7, 7), [255, 0, 0, 255]);
    }

    #[test]
    fn z_order_and_translucent_blend() {
        let mut scene = GpuScene::new(32, 32);
        scene.push_fill(
            rect(0.0, 0.0, 32.0, 32.0),
            false,
            GpuPaint::Solid([0, 0, 255, 255]),
        );
        scene.push_fill(
            rect(10.0, 10.0, 30.0, 30.0),
            false,
            GpuPaint::Solid([255, 0, 0, 128]),
        );
        let img = render_straight(&encode(&scene).unwrap());
        assert_eq!(px(&img, 32, 2, 2), [0, 0, 255, 255]);
        let mixed = px(&img, 32, 20, 20);
        assert_eq!(mixed[3], 255);
        assert!(mixed[0] > 100 && mixed[2] > 100, "{mixed:?}");
        let pre = render(&encode(&scene).unwrap());
        assert_eq!(px(&pre, 32, 2, 2), [0, 0, 255, 255]);
    }

    #[test]
    fn premultiplied_output() {
        let mut scene = GpuScene::new(16, 16);
        scene.push_fill(
            rect(0.0, 0.0, 16.0, 16.0),
            false,
            GpuPaint::Solid([200, 100, 50, 128]),
        );
        let pre = render(&encode(&scene).unwrap());
        assert_eq!(px(&pre, 16, 4, 4), [100, 50, 25, 128]);
    }
}
