//! Mask tracing and colour layering.

use super::contour::{self, Loop};
use super::fit::{self, P, Seg};
use super::quant::{NONE, quantize};
use super::{MAX_CONTOURS, MAX_OUTPUT_BYTES, Rgba, TraceError};

/// Tracing knobs.
#[derive(Clone, Copy, Debug)]
pub struct TraceOptions {
    /// Largest distance in pixels between the curve and the pixel outline.
    pub tolerance: f64,
    /// Outer contours smaller than this many pixels are dropped.
    pub min_area: u32,
    /// Holes smaller than this many pixels are filled (never above half `min_area`).
    pub hole_area: u32,
    /// Outline turns sharper than this stay corners (cosine of the angle).
    pub corner_cos: f64,
    /// Seed of the colour quantiser.
    pub seed: u64,
    /// Draw a near-black colour last so line art stays on top.
    pub ink_last: bool,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            tolerance: 1.0,
            min_area: 30,
            hole_area: 20,
            // 48 degrees
            corner_cos: 0.669_130_6,
            seed: 1,
            ink_last: true,
        }
    }
}

/// One colour layer, drawn bottom to top.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub color: [u8; 3],
    /// Path data of every contour of the union of this and all later colours.
    pub d: String,
    /// Pixels of this colour alone.
    pub area: u64,
    pub contours: usize,
}

impl Layer {
    pub fn hex(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            self.color[0], self.color[1], self.color[2]
        )
    }
}

#[derive(Default)]
struct Budget {
    contours: usize,
    bytes: usize,
}

fn num(v: f64, out: &mut String) {
    let r = (v * 10.0).round() / 10.0;
    let mut s = format!("{r:.1}");
    if s.ends_with(".0") {
        s.truncate(s.len() - 2);
    }
    if s == "-0" {
        s = "0".into();
    }
    out.push_str(&s);
}

fn pair(p: P, o: (i32, i32), out: &mut String) {
    num(p[0] + f64::from(o.0), out);
    out.push(' ');
    num(p[1] + f64::from(o.1), out);
}

fn emit(lp: &Loop, opts: &TraceOptions, origin: (i32, i32), out: &mut String) -> bool {
    let pts: Vec<P> = lp
        .points
        .iter()
        .map(|&(x, y)| [f64::from(x), f64::from(y)])
        .collect();
    if pts.len() < 3 {
        return false;
    }
    let (start, segs) = fit::fit_closed(&pts, opts.tolerance, opts.corner_cos);
    out.push('M');
    pair(start, origin, out);
    for (i, s) in segs.iter().enumerate() {
        match s {
            Seg::Line(p) if i + 1 == segs.len() && *p == start => {}
            Seg::Line(p) => {
                out.push('L');
                pair(*p, origin, out);
            }
            Seg::Curve(a, b, c) => {
                out.push('C');
                pair(*a, origin, out);
                out.push(' ');
                pair(*b, origin, out);
                out.push(' ');
                pair(*c, origin, out);
            }
        }
    }
    out.push('Z');
    true
}

fn trace_into(
    mask: &[bool],
    w: usize,
    h: usize,
    opts: &TraceOptions,
    origin: (i32, i32),
    budget: &mut Budget,
) -> Result<(String, usize), TraceError> {
    let room = MAX_CONTOURS.saturating_sub(budget.contours);
    let loops = contour::loops(mask, w, h, room.saturating_mul(4).max(16))
        .ok_or(TraceError::TooManyContours)?;
    let hole_min = i64::from(opts.hole_area.min(opts.min_area / 2));
    let outer_min = i64::from(opts.min_area);
    let mut d = String::new();
    let mut count = 0;
    for lp in &loops {
        let keep = if lp.area > 0 {
            lp.area >= outer_min
        } else {
            -lp.area >= hole_min
        };
        if !keep {
            continue;
        }
        if budget.contours + count >= MAX_CONTOURS {
            return Err(TraceError::TooManyContours);
        }
        if emit(lp, opts, origin, &mut d) {
            count += 1;
        }
        if budget.bytes + d.len() > MAX_OUTPUT_BYTES {
            return Err(TraceError::OutputTooLarge);
        }
    }
    budget.contours += count;
    budget.bytes += d.len();
    Ok((d, count))
}

/// Traces one mask (`w` x `h`, row-major, `true` = foreground) into path data.
/// Outer contours run clockwise and holes anticlockwise, so the default
/// nonzero fill rule renders the holes. Returns the data and contour count.
pub fn trace(
    mask: &[bool],
    w: usize,
    h: usize,
    opts: &TraceOptions,
) -> Result<(String, usize), TraceError> {
    if w == 0 || h == 0 || w.checked_mul(h) != Some(mask.len()) || mask.len() > super::MAX_PIXELS {
        return Err(TraceError::BadImage);
    }
    trace_into(mask, w, h, opts, (0, 0), &mut Budget::default())
}

/// Quantises `img` to `k` colours and traces each into a stacked layer
/// (largest area first, so every later layer sits on top of a union that
/// already covers it and no gaps open between neighbours).
/// `region` = `[x0, y0, x1, y1]` crops first and offsets the output.
pub fn trace_image(
    img: &Rgba<'_>,
    k: usize,
    opts: &TraceOptions,
    region: Option<[usize; 4]>,
) -> Result<Vec<Layer>, TraceError> {
    let (cropped, origin);
    let view = match region {
        None => {
            origin = (0, 0);
            *img
        }
        Some([x0, y0, x1, y1]) => {
            if x0 >= x1 || y0 >= y1 || x1 > img.width || y1 > img.height {
                return Err(TraceError::BadRegion);
            }
            let (w, h) = (x1 - x0, y1 - y0);
            let mut data = Vec::with_capacity(w * h * 4);
            for y in y0..y1 {
                data.extend_from_slice(
                    &img.data[(y * img.width + x0) * 4..(y * img.width + x1) * 4],
                );
            }
            cropped = data;
            origin = (x0 as i32, y0 as i32);
            Rgba::new(w, h, &cropped)?
        }
    };
    let q = quantize(&view, k, opts.seed)?;
    let mut areas = vec![0u64; q.palette.len()];
    for &l in &q.labels {
        if l != NONE {
            areas[usize::from(l)] += 1;
        }
    }
    let lum = |c: [u8; 3]| u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2]);
    let mut order: Vec<usize> = (0..q.palette.len()).filter(|&c| areas[c] > 0).collect();
    order.sort_by(|&a, &b| areas[b].cmp(&areas[a]).then(a.cmp(&b)));
    if opts.ink_last && order.len() > 1 {
        let ink = *order
            .iter()
            .min_by_key(|&&c| (lum(q.palette[c]), c))
            .unwrap_or(&order[0]);
        if lum(q.palette[ink]) < 165 {
            order.retain(|&c| c != ink);
            order.push(ink);
        }
    }
    let mut rank = vec![0u8; q.palette.len()];
    for (r, &c) in order.iter().enumerate() {
        rank[c] = r as u8;
    }
    let mut budget = Budget::default();
    let mut layers = Vec::new();
    for (r, &c) in order.iter().enumerate() {
        let mask: Vec<bool> = q
            .labels
            .iter()
            .map(|&l| l != NONE && usize::from(rank[usize::from(l)]) >= r)
            .collect();
        let (d, contours) = trace_into(&mask, q.width, q.height, opts, origin, &mut budget)?;
        if contours > 0 {
            layers.push(Layer {
                color: q.palette[c],
                d,
                area: areas[c],
                contours,
            });
        }
    }
    Ok(layers)
}
