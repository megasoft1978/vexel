//! Paints the canvas well: level-0 fill, artboard shadow, hairline,
//! checkerboard, the document pixels and the selection overlays.
//!
//! Everything is software-rasterised with no blur dependency, so the output is
//! exactly reproducible (the goldens use tolerance 0).

use super::map::Bounds;
use super::overlay::{self, DeviceRect};
use super::size::{artboard_pixels, frame_matches};
use super::surface::{Clip, Surface, rgb};
use super::view::View;
use crate::compose::Frame;
use vexel_ui::{Rect, Tokens};

/// Checkerboard square, in logical points.
pub const CHECKER: i64 = 8;
/// Shadow layers `(feather in points, alpha)`: soft rounded rectangles the
/// size of the artboard, offset 2 points down, each fading linearly over its
/// feather around the artboard edge. Stacked they reach about 35% black under
/// the artboard (0 2 12 rgba(0,0,0,.35)) and about half that at its edge.
pub const SHADOW_LAYERS: [(f64, f64); 2] = [(12.0, 0.22), (5.0, 0.15)];
pub const SHADOW_DROP: f64 = 2.0;
const SHADOW_RADIUS: f64 = 4.0;

/// The well fill. Light mode sits a ramp step below the panels (level 2) so
/// the stage reads as a recessed area; dark keeps the level 0 well.
pub fn well_fill(tokens: &Tokens) -> u32 {
    let luma =
        |c: u32| (299 * (c >> 24 & 255) + 587 * (c >> 16 & 255) + 114 * (c >> 8 & 255)) / 1000;
    if luma(tokens.window) > 128 {
        tokens.surface[2]
    } else {
        tokens.canvas_well
    }
}

/// What to draw into the stage.
pub struct StagePaint<'a> {
    pub view: View,
    /// Artboard size at 100%.
    pub artboard: (f64, f64),
    /// The cached compose to present (straight alpha), if any.
    pub frame: Option<&'a Frame>,
    /// Stage-local view bounds of the selected / hovered shape.
    pub selection: Option<Bounds>,
    pub hover: Option<Bounds>,
}

/// Device rectangle of the artboard: `(x, y, width, height)`.
pub fn artboard_device_rect(
    view: View,
    artboard: (f64, f64),
    stage: Rect,
    scale: u32,
) -> (i64, i64, i64, i64) {
    let s = i64::from(scale);
    let (w, h) = artboard_pixels(artboard, view.zoom, scale);
    (
        i64::from(stage.x) * s + (view.pan.0 * f64::from(scale)).round() as i64,
        i64::from(stage.y) * s + (view.pan.1 * f64::from(scale)).round() as i64,
        i64::from(w),
        i64::from(h),
    )
}

/// Fills `stage` with the well and, when there is a document, draws the
/// artboard and overlays. `pixels` is the whole window buffer, `width` its
/// width in device pixels and `scale` the backing scale.
pub fn paint_stage(
    pixels: &mut [u8],
    width: u32,
    scale: u32,
    stage: Rect,
    tokens: &Tokens,
    paint: Option<&StagePaint<'_>>,
) {
    let s = i64::from(scale);
    let height = (pixels.len() / 4 / width.max(1) as usize) as i64;
    let clip = Clip {
        x0: i64::from(stage.x) * s,
        y0: i64::from(stage.y) * s,
        x1: ((i64::from(stage.x) + i64::from(stage.width)) * s).min(i64::from(width)),
        y1: ((i64::from(stage.y) + i64::from(stage.height)) * s).min(height),
    };
    let mut surface = Surface {
        px: pixels,
        width: i64::from(width),
        clip,
        scale: s,
    };
    surface.fill(clip.x0, clip.y0, clip.x1, clip.y1, rgb(well_fill(tokens)));
    if let Some(paint) = paint {
        paint_document(&mut surface, stage, scale, tokens, paint);
    }
    // 1 px edge on the stage's inner border, over everything else.
    let edge = rgb(tokens.border);
    surface.fill(clip.x0, clip.y0, clip.x1, clip.y0 + s, edge);
    surface.fill(clip.x0, clip.y1 - s, clip.x1, clip.y1, edge);
    surface.fill(clip.x0, clip.y0, clip.x0 + s, clip.y1, edge);
    surface.fill(clip.x1 - s, clip.y0, clip.x1, clip.y1, edge);
}

fn paint_document(
    surface: &mut Surface<'_>,
    stage: Rect,
    scale: u32,
    tokens: &Tokens,
    paint: &StagePaint<'_>,
) {
    let s = surface.scale;
    let clip = surface.clip;
    let (ax, ay, aw, ah) = artboard_device_rect(paint.view, paint.artboard, stage, scale);
    shadow(surface, (ax, ay, aw, ah));
    let edge = rgb(tokens.control_edge);
    surface.fill(ax - s, ay - s, ax + aw + s, ay, edge);
    surface.fill(ax - s, ay + ah, ax + aw + s, ay + ah + s, edge);
    surface.fill(ax - s, ay, ax, ay + ah, edge);
    surface.fill(ax + aw, ay, ax + aw + s, ay + ah, edge);
    artboard(surface, (ax, ay, aw, ah), tokens, paint.frame);
    let origin = (clip.x0 as f64, clip.y0 as f64);
    let device = |b: Bounds| -> DeviceRect {
        let f = s as f64;
        (
            origin.0 + b.0 * f,
            origin.1 + b.1 * f,
            origin.0 + b.2 * f,
            origin.1 + b.3 * f,
        )
    };
    let accent = tokens.accent;
    if let Some(hover) = paint.hover.filter(|h| Some(*h) != paint.selection) {
        overlay::outline(
            surface,
            device(hover),
            overlay::HOVER_WIDTH * s as f64,
            accent,
            overlay::HOVER_OPACITY,
        );
    }
    if let Some(selection) = paint.selection {
        let rect = device(selection);
        overlay::outline(
            surface,
            rect,
            overlay::SELECTION_WIDTH * s as f64,
            accent,
            1.0,
        );
        overlay::handles(surface, rect, accent);
    }
}

/// Linear-ramp coverage of the pixel centre against a rounded rectangle: 1
/// `feather / 2` inside its edge, 0 `feather / 2` outside it.
fn soft_coverage(x: i64, y: i64, r: (f64, f64, f64, f64), radius: f64, feather: f64) -> f64 {
    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
    let radius = radius.min((r.2 - r.0) / 2.0).min((r.3 - r.1) / 2.0);
    let (cx, cy) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0);
    let (hx, hy) = ((r.2 - r.0) / 2.0 - radius, (r.3 - r.1) / 2.0 - radius);
    let (qx, qy) = ((px - cx).abs() - hx, (py - cy).abs() - hy);
    let outside = qx.max(0.0).hypot(qy.max(0.0));
    let sd = outside + qx.max(qy).min(0.0) - radius;
    (0.5 - sd / feather).clamp(0.0, 1.0)
}

/// Two layered soft rounded rectangles around and below the artboard; never
/// drawn inside it.
fn shadow(surface: &mut Surface<'_>, art: (i64, i64, i64, i64)) {
    let f = surface.scale as f64;
    for (feather, alpha) in SHADOW_LAYERS {
        let (feather, reach) = (feather * f, feather * f / 2.0);
        let rect = (
            art.0 as f64,
            art.1 as f64 + SHADOW_DROP * f,
            (art.0 + art.2) as f64,
            (art.1 + art.3) as f64 + SHADOW_DROP * f,
        );
        let c = surface.clip.intersect(
            (rect.0 - reach).floor() as i64,
            (rect.1 - reach).floor() as i64,
            (rect.2 + reach).ceil() as i64,
            (rect.3 + reach).ceil() as i64,
        );
        for y in c.y0..c.y1 {
            let in_rows = y >= art.1 && y < art.1 + art.3;
            let mut x = c.x0;
            while x < c.x1 {
                if in_rows && x >= art.0 && x < art.0 + art.2 {
                    x = art.0 + art.2;
                    continue;
                }
                let cover = soft_coverage(x, y, rect, SHADOW_RADIUS * f, feather);
                surface.blend(x, y, [0, 0, 0], alpha * cover);
                x += 1;
            }
        }
    }
}

fn artboard(
    surface: &mut Surface<'_>,
    art: (i64, i64, i64, i64),
    tokens: &Tokens,
    frame: Option<&Frame>,
) {
    let (ax, ay, aw, ah) = art;
    let cell = CHECKER * surface.scale;
    let (a, b) = (rgb(tokens.checker_a), rgb(tokens.checker_b));
    let frame = frame.filter(|f| {
        f.rgba.len() == f.width as usize * f.height as usize * 4
            && frame_matches((f.width, f.height), (aw as f64, ah as f64))
    });
    let direct = frame.is_some_and(|f| (i64::from(f.width), i64::from(f.height)) == (aw, ah));
    let c = surface.clip.intersect(ax, ay, ax + aw, ay + ah);
    for y in c.y0..c.y1 {
        for x in c.x0..c.x1 {
            let (dx, dy) = (x - ax, y - ay);
            let bg = if (dx / cell + dy / cell) % 2 == 0 {
                a
            } else {
                b
            };
            let out = match frame {
                Some(f) if direct => {
                    let at = (dy as usize * f.width as usize + dx as usize) * 4;
                    over(&f.rgba[at..at + 4], bg)
                }
                Some(f) => resampled(f, (dx, dy), (aw, ah), bg),
                None => bg,
            };
            surface.set(x, y, out);
        }
    }
}

/// Straight-alpha source over an opaque background, rounded.
pub fn over(px: &[u8], bg: [u8; 3]) -> [u8; 3] {
    let a = u32::from(px[3]);
    let mix = |s: u8, d: u8| ((u32::from(s) * a + u32::from(d) * (255 - a) + 127) / 255) as u8;
    [mix(px[0], bg[0]), mix(px[1], bg[1]), mix(px[2], bg[2])]
}

/// Bilinear sample in premultiplied alpha at frame position `(u, v)` in pixel
/// units (pixel centres at +0.5).
fn bilinear(f: &Frame, u: f64, v: f64) -> [f32; 4] {
    let (fx, fy) = (u - 0.5, v - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = ((fx - x0) as f32, (fy - y0) as f32);
    let at = |x: f64, y: f64| {
        let xi = (x.max(0.0) as usize).min(f.width as usize - 1);
        let yi = (y.max(0.0) as usize).min(f.height as usize - 1);
        let i = (yi * f.width as usize + xi) * 4;
        let a = f32::from(f.rgba[i + 3]) / 255.0;
        [
            f32::from(f.rgba[i]) * a,
            f32::from(f.rgba[i + 1]) * a,
            f32::from(f.rgba[i + 2]) * a,
            a,
        ]
    };
    let (p00, p10, p01, p11) = (
        at(x0, y0),
        at(x0 + 1.0, y0),
        at(x0, y0 + 1.0),
        at(x0 + 1.0, y0 + 1.0),
    );
    let mut out = [0.0; 4];
    for (i, o) in out.iter_mut().enumerate() {
        let top = p00[i] + (p10[i] - p00[i]) * tx;
        let bottom = p01[i] + (p11[i] - p01[i]) * tx;
        *o = top + (bottom - top) * ty;
    }
    out
}

/// One artboard pixel from a differently sized frame: bilinear when enlarging,
/// a box average of up to 4x4 taps when reducing.
fn resampled(f: &Frame, (dx, dy): (i64, i64), (aw, ah): (i64, i64), bg: [u8; 3]) -> [u8; 3] {
    let (rx, ry) = (
        f64::from(f.width) / aw as f64,
        f64::from(f.height) / ah as f64,
    );
    let taps = |r: f64| if r > 1.0 { (r.ceil() as u32).min(4) } else { 1 };
    let (nx, ny) = (taps(rx), taps(ry));
    let mut sum = [0.0f32; 4];
    for j in 0..ny {
        for i in 0..nx {
            let u = (dx as f64 + (f64::from(i) + 0.5) / f64::from(nx)) * rx;
            let v = (dy as f64 + (f64::from(j) + 0.5) / f64::from(ny)) * ry;
            let p = bilinear(f, u, v);
            for (s, p) in sum.iter_mut().zip(p) {
                *s += p;
            }
        }
    }
    let n = (nx * ny) as f32;
    let keep = 1.0 - sum[3] / n;
    let mix = |c: usize| {
        (sum[c] / n + f32::from(bg[c]) * keep)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    [mix(0), mix(1), mix(2)]
}
