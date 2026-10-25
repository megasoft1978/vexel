//! Resampling composite of a cached layer bitmap: the preview path of the
//! layer cache. Exports and `render_png` never call this.
//!
//! A bitmap is a straight-alpha RGBA8 image placed at `region` in the pixel
//! space it was rasterised in. [`warp_over`] draws it onto a destination tile
//! through an affine map (source pixels to destination pixels): an integer
//! translation copies rows exactly like a layer composite; anything else
//! samples bilinearly in premultiplied space (so edges do not gain halos) and
//! blends source-over.

use super::layer_geom::{IRect, apply, inverse};
use super::layers::{alpha_table, composite_row, source_over};
use vexel_geometry::Affine;
use vexel_raster::Tile;
use vexel_render::RenderError;

/// A rasterised layer: `rgba` covers `region` (row 0 at `region.y0`).
pub(super) struct Bitmap {
    pub region: IRect,
    pub rgba: Vec<u8>,
}

impl Bitmap {
    pub fn bytes(&self) -> usize {
        self.rgba.len()
    }

    fn texel(&self, x: i32, y: i32) -> [u32; 4] {
        let (w, h) = (self.region.width() as i32, self.region.height() as i32);
        if x < 0 || y < 0 || x >= w || y >= h {
            return [0; 4];
        }
        let at = (y as usize * w as usize + x as usize) * 4;
        let p = &self.rgba[at..at + 4];
        [
            u32::from(p[0]),
            u32::from(p[1]),
            u32::from(p[2]),
            u32::from(p[3]),
        ]
    }
}

/// Pixels per task below which one thread does the whole composite.
const PARALLEL_PIXELS: usize = 60_000;
const MAX_THREADS: usize = 8;

/// Source-over of `bitmap`, mapped by `m`, times `opacity`, onto `dest`.
pub(super) fn warp_over(
    dest: &mut Tile,
    bitmap: &Bitmap,
    m: Affine,
    opacity: f64,
    cancel: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let canvas = IRect {
        x0: 0,
        y0: 0,
        x1: dest.width() as i32,
        y1: dest.height() as i32,
    };
    let Some(inv) = inverse(m) else {
        return Err(RenderError::InvalidGeometry);
    };
    let Some(mapped) = bitmap.region.map(m) else {
        return Err(RenderError::InvalidGeometry);
    };
    let area = mapped.grow(1, 1).intersect(canvas);
    if area.is_empty() || bitmap.region.is_empty() {
        return Ok(());
    }
    if cancel() {
        return Err(RenderError::Cancelled);
    }
    let table = alpha_table(opacity);
    if let Some((dx, dy)) = integer_shift(m) {
        copy_shifted(dest, bitmap, dx, dy, &table);
        return Ok(());
    }
    let row_bytes = canvas.width() * 4;
    let rows = area.height();
    let threads = if rows * area.width() < PARALLEL_PIXELS {
        1
    } else {
        std::thread::available_parallelism()
            .map_or(4, usize::from)
            .min(MAX_THREADS)
            .min(rows.div_ceil(8))
    };
    let band = rows.div_ceil(threads);
    let body = &mut dest.rgba_mut()[area.y0 as usize * row_bytes..area.y1 as usize * row_bytes];
    std::thread::scope(|scope| {
        for (n, chunk) in body.chunks_mut(band * row_bytes).enumerate() {
            let first = area.y0 + (n * band) as i32;
            let mut work = move || sample_rows(chunk, row_bytes, first, area, bitmap, inv, &table);
            if threads == 1 {
                work();
            } else {
                scope.spawn(work);
            }
        }
    });
    Ok(())
}

/// `m` as a pure integer translation, if it is one.
fn integer_shift(m: Affine) -> Option<(i32, i32)> {
    let near = |v: f64, to: f64| (v - to).abs() < 1e-9;
    let whole = |v: f64| near(v, v.round()).then_some(v.round() as i32);
    (near(m.a, 1.0) && near(m.d, 1.0) && near(m.b, 0.0) && near(m.c, 0.0))
        .then(|| Some((whole(m.e)?, whole(m.f)?)))?
}

/// Exact row copy for a translation by whole pixels.
fn copy_shifted(dest: &mut Tile, bitmap: &Bitmap, dx: i32, dy: i32, table: &[u8; 256]) {
    let (w, h) = (dest.width() as i32, dest.height() as i32);
    let r = bitmap.region;
    let (x0, x1) = ((r.x0 + dx).max(0), (r.x1 + dx).min(w));
    let (y0, y1) = ((r.y0 + dy).max(0), (r.y1 + dy).min(h));
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let src_w = r.width();
    for y in y0..y1 {
        let sy = (y - dy - r.y0) as usize;
        let sx = (x0 - dx - r.x0) as usize;
        let src = &bitmap.rgba[(sy * src_w + sx) * 4..(sy * src_w + sx + (x1 - x0) as usize) * 4];
        let at = (y as usize * w as usize + x0 as usize) * 4;
        composite_row(
            &mut dest.rgba_mut()[at..at + (x1 - x0) as usize * 4],
            src,
            table,
        );
    }
}

/// Bilinear rows `first..` of the destination held in `chunk`.
fn sample_rows(
    chunk: &mut [u8],
    row_bytes: usize,
    first: i32,
    area: IRect,
    bitmap: &Bitmap,
    inv: Affine,
    table: &[u8; 256],
) {
    let (w, h) = (bitmap.region.width() as i32, bitmap.region.height() as i32);
    for (n, row) in chunk.chunks_exact_mut(row_bytes).enumerate() {
        let y = first + n as i32;
        // Source position of the first pixel centre; texel centres sit at +0.5.
        let (sx0, sy0) = apply(inv, f64::from(area.x0) + 0.5, f64::from(y) + 0.5);
        let (mut u, mut v) = (
            sx0 - 0.5 - f64::from(bitmap.region.x0),
            sy0 - 0.5 - f64::from(bitmap.region.y0),
        );
        for x in area.x0..area.x1 {
            let (iu, iv) = (u.floor(), v.floor());
            let (ix, iy) = (iu as i32, iv as i32);
            let fx = (((u - iu) * 256.0) as u32).min(256);
            let fy = (((v - iv) * 256.0) as u32).min(256);
            u += inv.a;
            v += inv.b;
            if ix < -1 || iy < -1 || ix >= w || iy >= h {
                continue;
            }
            let weights = [
                (256 - fx) * (256 - fy),
                fx * (256 - fy),
                (256 - fx) * fy,
                fx * fy,
            ];
            let texels = if ix >= 0 && iy >= 0 && ix + 1 < w && iy + 1 < h {
                // Interior: all four texels exist, no per-texel bounds test.
                let at = (iy as usize * w as usize + ix as usize) * 4;
                let below = at + w as usize * 4;
                let t = |i: usize| {
                    let p = &bitmap.rgba[i..i + 4];
                    [
                        u32::from(p[0]),
                        u32::from(p[1]),
                        u32::from(p[2]),
                        u32::from(p[3]),
                    ]
                };
                [t(at), t(at + 4), t(below), t(below + 4)]
            } else {
                [
                    bitmap.texel(ix, iy),
                    bitmap.texel(ix + 1, iy),
                    bitmap.texel(ix, iy + 1),
                    bitmap.texel(ix + 1, iy + 1),
                ]
            };
            if texels[0][3] | texels[1][3] | texels[2][3] | texels[3][3] == 0 {
                continue;
            }
            let mut alpha = 0u32;
            let mut sum = [0u32; 3];
            for (wt, t) in weights.iter().zip(&texels) {
                let wa = wt * t[3];
                alpha += wa;
                for c in 0..3 {
                    sum[c] += wa * t[c];
                }
            }
            if alpha == 0 {
                continue;
            }
            let a8 = ((alpha + 32768) >> 16).min(255) as usize;
            let a = table[a8];
            if a == 0 {
                continue;
            }
            let px = [
                ((sum[0] + alpha / 2) / alpha) as u8,
                ((sum[1] + alpha / 2) / alpha) as u8,
                ((sum[2] + alpha / 2) / alpha) as u8,
                a,
            ];
            let at = x as usize * 4;
            source_over(&mut row[at..at + 4], px);
        }
    }
}

/// Composites `bitmap` (placed at its region, which is the canvas) onto
/// `dest` inside `area` with the group's `opacity` and blend `mode`: the same
/// arithmetic as compositing a rendered group layer.
pub(super) fn blend_over(
    dest: &mut Tile,
    bitmap: &Bitmap,
    opacity: f64,
    mode: u8,
    area: IRect,
    cancel: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let r = bitmap.region;
    let width = dest.width() as usize;
    if area.is_empty() || r.x0 != 0 || r.y0 != 0 || r.width() != width {
        return Ok(());
    }
    let table = alpha_table(opacity);
    for y in area.y0..area.y1 {
        if cancel() {
            return Err(RenderError::Cancelled);
        }
        let range = (y as usize * width + area.x0 as usize) * 4
            ..(y as usize * width + area.x1 as usize) * 4;
        let src = &bitmap.rgba[range.clone()];
        let dst = &mut dest.rgba_mut()[range];
        if mode == 0 {
            composite_row(dst, src, &table);
            continue;
        }
        for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
            let alpha = table[usize::from(s[3])];
            if alpha != 0 {
                vexel_render::blend::blend_pixel(mode, d, [s[0], s[1], s[2], alpha]);
            }
        }
    }
    Ok(())
}
