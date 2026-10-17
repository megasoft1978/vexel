//! 16-unit icon mask rasteriser with device-pixel snapping, and the centred tool-rail helper.
//!
//! Rules (all icons, constants not per icon):
//! * Authoring grid is 16x16 units; the mask for scale `s` is `16*s` pixels square.
//! * Nominal stroke is `STROKE_UNITS` = 1.5 units with round caps and joins (capsule distance test).
//! * Pixel snapping: the device stroke width is the nominal width rounded UP to an even pixel
//!   count, `ceil(1.5*s/2)*2` = 2, 4, 6 px at 1x, 2x, 3x (2.0 units effective). Every segment uses
//!   that one width, so optical weight is equal across lines, curves and diagonals. Exactly
//!   horizontal or vertical segments also snap both endpoints to the device grid (an even width is
//!   centred on a pixel boundary), so they cover whole pixels and render crisp instead of with
//!   half-covered edge rows. Segments that continue a snapped one start on its snapped point.
//!   Why even, not 1.5 px: a 1.5 px stroke cannot be both crisp and scale as 4x in covered
//!   pixels (AC2/AC4 of the order); 2/4/6 px gives exact 4x scaling and no partial rows.
//! * Anti-aliasing for curves and diagonals: 8x8 samples per pixel; pixels with 10 or fewer of the
//!   64 samples covered are dropped (fringe clean-up) so faint halos do not smear small glyphs.
use crate::{
    ControlState, IconKind, Rect, Tokens, WidgetCanvas,
    icons::{IconError, Segment, distance2},
};

pub(crate) const GRID: usize = 16;
/// Stroke width in grid units; one constant for every icon.
pub(crate) const STROKE_UNITS: f64 = 1.5;
#[allow(dead_code)]
pub(crate) const ROUND_CAPS: bool = true;
#[allow(dead_code)]
pub(crate) const ROUND_JOINS: bool = true;

/// Supersampling grid is N x N per pixel.
const N: u32 = 8;
const FRINGE_SAMPLES: u32 = 10;

/// Only the keyframe marker is a filled closed path; every other icon is stroke only.
pub(crate) fn is_filled(icon: IconKind) -> bool {
    icon == IconKind::KEYFRAME_FILLED
}

/// Device stroke width in pixels (even), shared by every segment.
pub(crate) fn snapped_width(scale: usize) -> f64 {
    ((STROKE_UNITS * scale as f64) / 2.0).ceil() * 2.0
}

fn snap(c: f64, width: f64) -> f64 {
    if (width as u64) % 2 == 1 {
        c.floor() + 0.5
    } else {
        c.round()
    }
}

pub(crate) fn state_color(state: ControlState, t: &Tokens) -> u32 {
    match state {
        ControlState::Disabled => t.disabled_text,
        ControlState::Pressed | ControlState::Selected => t.accent,
        ControlState::Focus => t.focus_ring,
        ControlState::Invalid => t.error,
        ControlState::Rest | ControlState::Hover => t.text_primary,
    }
}

struct Prim {
    seg: Segment,
    r: f64,
}

fn inside(x: f64, y: f64, segs: &[Segment]) -> bool {
    let mut odd = false;
    for s in segs {
        if (s.y0 > y) != (s.y1 > y) {
            let cross = s.x0 + (y - s.y0) / (s.y1 - s.y0) * (s.x1 - s.x0);
            if x < cross {
                odd = !odd;
            }
        }
    }
    odd
}

/// Rasterise unit-space segments into a `16*scale` square coverage mask.
pub(crate) fn rasterize(
    segs: &[Segment],
    filled: bool,
    scale: usize,
) -> Result<Vec<u8>, IconError> {
    let s = scale as f64;
    let axis = GRID * scale;
    let len = axis.checked_mul(axis).ok_or(IconError::Allocation)?;
    let mut mask = Vec::new();
    mask.try_reserve_exact(len)
        .map_err(|_| IconError::Allocation)?;
    mask.resize(len, 0u8);
    let w = snapped_width(scale);
    let mut prims = Vec::new();
    prims
        .try_reserve(segs.len())
        .map_err(|_| IconError::Allocation)?;
    let mut dev = Vec::new();
    dev.try_reserve(segs.len())
        .map_err(|_| IconError::Allocation)?;
    let aligned = |g: &Segment| (g.x0 == g.x1) != (g.y0 == g.y1);
    let snapped: Vec<Segment> = segs
        .iter()
        .map(|g| Segment {
            x0: snap(g.x0 * s, w),
            y0: snap(g.y0 * s, w),
            x1: snap(g.x1 * s, w),
            y1: snap(g.y1 * s, w),
        })
        .collect();
    for (i, g) in segs.iter().enumerate() {
        let d = Segment {
            x0: g.x0 * s,
            y0: g.y0 * s,
            x1: g.x1 * s,
            y1: g.y1 * s,
        };
        dev.push(d);
        if aligned(g) {
            prims.push(Prim {
                seg: snapped[i],
                r: w / 2.0,
            });
        } else {
            // A free segment that continues an axis-aligned one starts/ends on its snapped point,
            // so rounded corners join straight edges without a notch.
            let mut q = d;
            if i > 0 && aligned(&segs[i - 1]) && segs[i - 1].x1 == g.x0 && segs[i - 1].y1 == g.y0 {
                q.x0 = snapped[i - 1].x1;
                q.y0 = snapped[i - 1].y1;
            }
            if i + 1 < segs.len()
                && aligned(&segs[i + 1])
                && segs[i + 1].x0 == g.x1
                && segs[i + 1].y0 == g.y1
            {
                q.x1 = snapped[i + 1].x0;
                q.y1 = snapped[i + 1].y0;
            }
            prims.push(Prim { seg: q, r: w / 2.0 });
        }
    }
    for py in 0..axis {
        for px in 0..axis {
            let (fx, fy) = (px as f64, py as f64);
            let near: Vec<&Prim> = prims
                .iter()
                .filter(|p| {
                    let sg = &p.seg;
                    sg.x0.min(sg.x1) - p.r < fx + 1.0
                        && sg.x0.max(sg.x1) + p.r > fx
                        && sg.y0.min(sg.y1) - p.r < fy + 1.0
                        && sg.y0.max(sg.y1) + p.r > fy
                })
                .collect();
            let mut covered = 0u32;
            for sy in 0..N {
                for sx in 0..N {
                    let x = fx + (sx as f64 + 0.5) / N as f64;
                    let y = fy + (sy as f64 + 0.5) / N as f64;
                    if near.iter().any(|p| distance2(x, y, &p.seg) <= p.r * p.r)
                        || (filled && inside(x, y, &dev))
                    {
                        covered += 1;
                    }
                }
            }
            // Fringe clean-up: coverage of FRINGE_SAMPLES/64 or less is dropped so faint halos do not smear the glyph.
            if covered > FRINGE_SAMPLES {
                mask[py * axis + px] = ((covered * 255 + N * N / 2) / (N * N)) as u8;
            }
        }
    }
    Ok(mask)
}

/// Paint a 16 pt glyph centred in `rect` (a 32 pt tool-rail target is the intended size).
/// Equal margins on every side; the glyph lands on integer device pixels at integer scales.
/// Colour comes from `tokens` by `state`. Returns false when `rect` is smaller than the glyph.
pub fn icon_centered(
    canvas: &mut WidgetCanvas<'_>,
    rect: Rect,
    kind: IconKind,
    state: ControlState,
    tokens: &Tokens,
) -> bool {
    let g = GRID as u32;
    if rect.width < g || rect.height < g {
        return false;
    }
    let glyph = Rect {
        x: rect.x + (rect.width - g) / 2,
        y: rect.y + (rect.height - g) / 2,
        width: g,
        height: g,
    };
    canvas.paint_icon_color(kind, glyph, state_color(state, tokens))
}
