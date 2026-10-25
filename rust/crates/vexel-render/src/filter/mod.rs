//! SVG filter chain over one straight-alpha RGBA8 layer.
//!
//! A pure pixel port of the C `raster_shape_filter_apply` (`shape_filter.c`):
//! the primitives `feGaussianBlur`, `feOffset`, `feFlood`, `feBlend`,
//! `feComposite`, `feMerge`, `feColorMatrix`, `feDropShadow` and
//! `feTurbulence` (SVG 1.1 reference noise, see [`turbulence`]), evaluated in
//! linear RGB (the C always sets `color-interpolation-filters: linearRGB`),
//! clipped to the filter region afterwards. Creating the layer and
//! compositing the result into the parent is the group machinery, not this
//! module. The unsupported primitives never reach this module: the definition
//! parser rejects them with typed `ErrorKind::UnsupportedFe*` diagnostics.
//!
//! Failure mapping from the C: scaled blur/offset magnitude, work and scratch
//! bounds are [`RenderError::LimitExceeded`]; a dangling input reference or
//! inconsistent chain is [`RenderError::InvalidGeometry`]; `control` returning
//! true is [`RenderError::Cancelled`]. On any error the layer is unchanged.

mod blur;
mod boxblur;
mod color;
mod region;
mod turbulence;

use crate::{RenderError, ViewMap};
use color::{composite_pixel, linear_table, source_over, srgb_table};
use vexel_scene::{Filter, FilterInput, FilterKind, FilterPrimitive};

/// Hard cap on the total scratch bytes of one filter application (the C
/// limit was 64 MiB; raised with the region-bounded window, see below).
pub const FILTER_SCRATCH_LIMIT: usize = 768 * 1024 * 1024;
/// C `FILTER_WORK_LIMIT`: weighted pixel visits up to which a blur uses the
/// exact Gaussian kernel. Larger blurs use the O(pixels) box approximation
/// ([`boxblur`]), so the exact outputs are unchanged.
pub const FILTER_WORK_LIMIT: usize = 50_000_000;
/// Hard cap on the weighted pixel visits of one whole filter application.
pub const FILTER_TOTAL_WORK_LIMIT: usize = 2_000_000_000;
/// Hard cap on the pixels one filter application works on (16 Mpx). The work
/// window is the filter region plus the reach of the chain, clipped to the
/// layer, not the whole canvas.
pub const MAX_FILTER_PIXELS: usize = 16 * 1024 * 1024;
/// C `SVG_SHAPE_MAX_FILTER_PRIMITIVES`.
pub const MAX_FILTER_PRIMITIVES: usize = 16;
/// Largest scaled `stdDeviation` (C `values[0|1] > 32`).
pub const MAX_STD_DEVIATION: f64 = 32.0;
/// Largest scaled `dx`/`dy` magnitude (C `fabs(values[2|3]) > 4096`).
pub const MAX_OFFSET: f64 = 4096.0;

/// Applies `chain` (the primitives of `filter`) to `layer` in place.
///
/// `layer` is straight RGBA8 of `view.width() * view.height()` pixels and
/// `view` maps user space to it (used for user-space primitive scaling and the
/// `userSpaceOnUse` region). `available_scratch` bounds the working memory
/// like the C parameter of the same name (clamped to
/// [`FILTER_SCRATCH_LIMIT`]). `control` is polled per primitive and pixel row.
pub fn apply_filter_chain(
    layer: &mut [u8],
    view: &ViewMap,
    filter: &Filter,
    chain: &[FilterPrimitive],
    available_scratch: usize,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let (width, height) = (view.width() as usize, view.height() as usize);
    if chain.is_empty()
        || chain.len() > MAX_FILTER_PRIMITIVES
        || chain.len() != filter.primitives.count
    {
        return Err(RenderError::InvalidGeometry);
    }
    let pixels = width
        .checked_mul(height)
        .ok_or(RenderError::LimitExceeded)?;
    let byte_count = pixels.checked_mul(4).ok_or(RenderError::LimitExceeded)?;
    if width == 0 || height == 0 || layer.len() != byte_count {
        return Err(RenderError::InvalidGeometry);
    }
    let bounds = region::alpha_bounds(layer, width, height);
    let (box_width, box_height) = bounds.map_or((0.0, 0.0), |b| (b[2] - b[0], b[3] - b[1]));
    let mut scaled = chain.to_vec();
    for primitive in &mut scaled {
        if !matches!(
            primitive.kind,
            FilterKind::GaussianBlur | FilterKind::DropShadow | FilterKind::Offset
        ) {
            continue;
        }
        let (scale_x, scale_y) = if filter.primitive_units_object_bounding_box {
            (box_width, box_height)
        } else {
            (view.scale_x, view.scale_y)
        };
        primitive.values[0] *= scale_x;
        primitive.values[1] *= scale_y;
        primitive.values[2] *= scale_x;
        primitive.values[3] *= scale_y;
        let v = &primitive.values;
        if !(v[..4].iter().all(|x| x.is_finite()))
            || v[0] > MAX_STD_DEVIATION
            || v[1] > MAX_STD_DEVIATION
            || v[2].abs() > MAX_OFFSET
            || v[3].abs() > MAX_OFFSET
        {
            return Err(RenderError::LimitExceeded);
        }
    }
    // Everything outside the filter region is cleared at the end, and a
    // pixel only depends on inputs within the summed reach of the chain, so
    // the work is bounded to the region plus that reach.
    let region = bounds.map(|b| region::device_region(view, filter, b));
    let tile = region::user_region(view, filter, bounds);
    let [x0, y0, x1, y1] = window_of(&scaled, region, width, height);
    let (window_w, window_h) = (x1 - x0, y1 - y0);
    let pixels = window_w * window_h;
    if pixels == 0 {
        layer.fill(0);
        return Ok(());
    }
    if pixels > MAX_FILTER_PIXELS {
        return Err(RenderError::LimitExceeded);
    }
    check_work(&scaled, pixels)?;
    let has_alpha = chain.iter().any(uses_alpha_input);
    let has_shadow = chain.iter().any(|p| p.kind == FilterKind::DropShadow);
    let byte_count = pixels * 4;
    let float_bytes = pixels * 4 * std::mem::size_of::<f32>();
    let full = window_w == width && window_h == height;
    // results + graphic + alpha + source copy + finished, f32 work, shadow
    let mut scratch = (chain.len() + 2 + usize::from(has_alpha) + usize::from(!full))
        .checked_mul(byte_count)
        .and_then(|bytes| bytes.checked_add(float_bytes))
        .ok_or(RenderError::LimitExceeded)?;
    if has_shadow {
        scratch += pixels;
    }
    if scratch > FILTER_SCRATCH_LIMIT || scratch > available_scratch {
        return Err(RenderError::LimitExceeded);
    }
    let local_view = ViewMap {
        width: window_w as u32,
        height: window_h as u32,
        offset_x: view.offset_x - x0 as f64,
        offset_y: view.offset_y - y0 as f64,
        ..*view
    };
    let local_region = region.map(|r| [r[0] - x0 as f64, r[1] - y0 as f64, r[2] - x0 as f64, r[3] - y0 as f64]);
    if full {
        let finished = run_chain(layer, &local_view, filter, &scaled, local_region, tile, has_alpha, has_shadow, control)?;
        layer.copy_from_slice(&finished);
        return Ok(());
    }
    let mut source = reserve(byte_count)?;
    for y in y0..y1 {
        source.extend_from_slice(&layer[(y * width + x0) * 4..(y * width + x1) * 4]);
    }
    let finished = run_chain(&source, &local_view, filter, &scaled, local_region, tile, has_alpha, has_shadow, control)?;
    layer.fill(0);
    for (row, line) in finished.chunks_exact(window_w * 4).enumerate() {
        let at = ((y0 + row) * width + x0) * 4;
        layer[at..at + window_w * 4].copy_from_slice(line);
    }
    Ok(())
}

/// Pixel window `[x0, y0, x1, y1)` the chain must be evaluated on: the
/// filter `region` grown by the summed blur/offset reach, clipped to the layer.
fn window_of(chain: &[FilterPrimitive], region: Option<[f64; 4]>, width: usize, height: usize) -> [usize; 4] {
    let Some(r) = region else {
        return [0; 4];
    };
    if !r.iter().all(|v| v.is_finite()) {
        return [0, 0, width, height];
    }
    let (mut reach_x, mut reach_y) = (1.0f64, 1.0f64);
    for p in chain {
        let v = &p.values;
        match p.kind {
            FilterKind::GaussianBlur => {
                reach_x += (3.0 * v[0]).ceil();
                reach_y += (3.0 * v[1]).ceil();
            }
            FilterKind::DropShadow => {
                reach_x += (3.0 * v[0]).ceil() + v[2].abs().ceil() + 1.0;
                reach_y += (3.0 * v[1]).ceil() + v[3].abs().ceil() + 1.0;
            }
            FilterKind::Offset => {
                reach_x += v[2].abs().ceil() + 1.0;
                reach_y += v[3].abs().ceil() + 1.0;
            }
            _ => {}
        }
    }
    let lo = |v: f64, limit: usize| v.floor().clamp(0.0, limit as f64) as usize;
    let hi = |v: f64, limit: usize| v.ceil().clamp(0.0, limit as f64) as usize;
    let (x0, y0) = (lo(r[0] - reach_x, width), lo(r[1] - reach_y, height));
    let (x1, y1) = (hi(r[2] + reach_x, width).max(x0), hi(r[3] + reach_y, height).max(y0));
    [x0, y0, x1, y1]
}

/// Evaluates the chain on `source` (`view.width() x view.height()` straight
/// RGBA8) and returns the sRGB result clipped to `region`.
#[allow(clippy::too_many_arguments)]
fn run_chain(
    source: &[u8],
    view: &ViewMap,
    filter: &Filter,
    scaled: &[FilterPrimitive],
    region: Option<[f64; 4]>,
    tile: [f64; 4],
    has_alpha: bool,
    has_shadow: bool,
    control: &dyn Fn() -> bool,
) -> Result<Vec<u8>, RenderError> {
    let (width, height) = (view.width() as usize, view.height() as usize);
    let pixels = width * height;
    let byte_count = pixels * 4;
    let linear = linear_table();
    let linear_rgb = filter.linear_rgb;
    let mut graphic = reserve(byte_count)?;
    graphic.extend_from_slice(source);
    if linear_rgb {
        for pixel in graphic.chunks_exact_mut(4) {
            for c in &mut pixel[..3] {
                *c = linear[usize::from(*c)];
            }
        }
    }
    let alpha_layer = if has_alpha {
        let mut surface = reserve(byte_count)?;
        surface.resize(byte_count, 0);
        for (target, source) in surface.chunks_exact_mut(4).zip(graphic.chunks_exact(4)) {
            target[3] = source[3];
        }
        surface
    } else {
        Vec::new()
    };
    let mut work: Vec<f32> = Vec::new();
    work.try_reserve_exact(pixels * 4)
        .map_err(|_| RenderError::LimitExceeded)?;
    work.resize(pixels * 4, 0.0);
    let mut shadow_alpha: Vec<u8> = Vec::new();
    if has_shadow {
        shadow_alpha
            .try_reserve_exact(pixels)
            .map_err(|_| RenderError::LimitExceeded)?;
        shadow_alpha.resize(pixels, 0);
    }
    let mut results: Vec<Vec<u8>> = Vec::new();
    results
        .try_reserve_exact(scaled.len())
        .map_err(|_| RenderError::LimitExceeded)?;
    for _ in scaled {
        let mut surface = reserve(byte_count)?;
        surface.resize(byte_count, 0);
        results.push(surface);
    }

    for (index, primitive) in scaled.iter().enumerate() {
        if control() {
            return Err(RenderError::Cancelled);
        }
        let (done, rest) = results.split_at_mut(index);
        let output = &mut rest[0];
        let lookup = Lookup {
            graphic: &graphic,
            alpha: &alpha_layer,
            done,
            index,
        };
        let input = lookup.resolve(primitive.input)?;
        match primitive.kind {
            FilterKind::GaussianBlur => blur::blur(
                input,
                output,
                width,
                height,
                (primitive.values[0], primitive.values[1]),
                &mut work,
                control,
            )?,
            FilterKind::Offset => blur::offset(
                input,
                output,
                width,
                height,
                primitive.values[2],
                primitive.values[3],
            ),
            FilterKind::Flood => {
                let mut flood = primitive.rgba;
                if linear_rgb {
                    for c in &mut flood[..3] {
                        *c = linear[usize::from(*c)];
                    }
                }
                for pixel in output.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&flood);
                }
            }
            FilterKind::Blend | FilterKind::Composite => {
                let second = lookup.resolve(primitive.input2)?;
                for (row, (a, b)) in input
                    .chunks(width * 4)
                    .zip(second.chunks(width * 4))
                    .enumerate()
                {
                    if control() {
                        return Err(RenderError::Cancelled);
                    }
                    let out_row = &mut output[row * width * 4..(row + 1) * width * 4];
                    for ((a, b), out) in a
                        .chunks_exact(4)
                        .zip(b.chunks_exact(4))
                        .zip(out_row.chunks_exact_mut(4))
                    {
                        composite_pixel(primitive, a, b, out);
                    }
                }
            }
            FilterKind::Merge => {
                let count = usize::from(primitive.merge_count).min(primitive.merge_inputs.len());
                for merge in &primitive.merge_inputs[..count] {
                    let source = lookup.resolve(*merge)?;
                    for (row, rows) in source.chunks(width * 4).enumerate() {
                        if control() {
                            return Err(RenderError::Cancelled);
                        }
                        let out_row = &mut output[row * width * 4..(row + 1) * width * 4];
                        for (out, src) in out_row.chunks_exact_mut(4).zip(rows.chunks_exact(4)) {
                            source_over(out, src);
                        }
                    }
                }
            }
            FilterKind::ColorMatrix => {
                if !color::color_matrix(primitive, input, output, control, width * 4) {
                    return Err(RenderError::Cancelled);
                }
            }
            FilterKind::Turbulence => {
                let params = turbulence::Params {
                    base: [primitive.values[0], primitive.values[1]],
                    octaves: primitive.values[2] as u32,
                    seed: primitive.values[3] as i64,
                    turbulence: primitive.mode & 1 != 0,
                    stitch: primitive.mode & 2 != 0,
                    tile,
                };
                turbulence::render(&params, view, output);
            }
            FilterKind::DropShadow => blur::drop_shadow(
                primitive,
                input,
                output,
                &mut shadow_alpha,
                &mut work,
                (width, height),
                &linear,
                linear_rgb,
                control,
            )?,
        }
    }

    let last = results.last().ok_or(RenderError::InvalidGeometry)?;
    let mut finished = reserve(byte_count)?;
    finished.extend_from_slice(last);
    if linear_rgb {
        let encode = srgb_table();
        for pixel in finished.chunks_exact_mut(4) {
            for c in &mut pixel[..3] {
                *c = encode[usize::from(*c)];
            }
        }
    }
    region::clip(&mut finished, width, height, region);
    Ok(finished)
}

fn reserve(bytes: usize) -> Result<Vec<u8>, RenderError> {
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(bytes)
        .map_err(|_| RenderError::LimitExceeded)?;
    Ok(buffer)
}

fn uses_alpha_input(primitive: &FilterPrimitive) -> bool {
    let count = usize::from(primitive.merge_count).min(primitive.merge_inputs.len());
    primitive.input == FilterInput::Alpha
        || primitive.input2 == FilterInput::Alpha
        || primitive.merge_inputs[..count].contains(&FilterInput::Alpha)
}

/// Weighted pixel visits of the chain: bounded by [`FILTER_TOTAL_WORK_LIMIT`].
/// A blur costs its exact kernel visits while those fit [`FILTER_WORK_LIMIT`],
/// else the flat box-blur cost.
fn check_work(chain: &[FilterPrimitive], pixels: usize) -> Result<(), RenderError> {
    let mut total = 0usize;
    for primitive in chain {
        let per_pixel = match primitive.kind {
            FilterKind::GaussianBlur | FilterKind::DropShadow => {
                let rx = (3.0 * primitive.values[0]).ceil();
                let ry = (3.0 * primitive.values[1]).ceil();
                if !rx.is_finite()
                    || !ry.is_finite()
                    || rx > blur::MAX_RADIUS
                    || ry > blur::MAX_RADIUS
                {
                    return Err(RenderError::LimitExceeded);
                }
                let (rx, ry) = (rx as i32, ry as i32);
                if blur::is_exact(pixels, rx, ry) {
                    blur::exact_per_pixel(rx, ry)
                } else {
                    boxblur::BOX_COST_PER_PIXEL
                }
            }
            FilterKind::Merge => usize::from(primitive.merge_count),
            FilterKind::Turbulence => (primitive.values[2] as usize).max(1),
            _ => 1,
        };
        total = pixels
            .checked_mul(per_pixel)
            .and_then(|amount| total.checked_add(amount))
            .filter(|total| *total <= FILTER_TOTAL_WORK_LIMIT)
            .ok_or(RenderError::LimitExceeded)?;
    }
    Ok(())
}

struct Lookup<'a> {
    graphic: &'a [u8],
    alpha: &'a [u8],
    done: &'a [Vec<u8>],
    index: usize,
}

impl<'a> Lookup<'a> {
    /// C `resolve_input`: `Previous` is the preceding result (or the source
    /// graphic for the first primitive); a result must be an earlier one.
    fn resolve(&self, input: FilterInput) -> Result<&'a [u8], RenderError> {
        let input = match input {
            FilterInput::Previous if self.index == 0 => FilterInput::Graphic,
            FilterInput::Previous => FilterInput::Result(self.index - 1),
            other => other,
        };
        match input {
            FilterInput::Graphic => Ok(self.graphic),
            FilterInput::Alpha => Ok(self.alpha),
            FilterInput::Result(i) if i < self.index => Ok(&self.done[i]),
            _ => Err(RenderError::InvalidGeometry),
        }
    }
}
