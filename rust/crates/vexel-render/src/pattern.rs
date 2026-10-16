//! Pattern paint (`fill`/`stroke` `url(#pattern)`), a port of the C reference
//! (`src/svg/shape/pattern.c` tile rasterization and
//! `src/raster/shape_render_sample.c` `pattern_sample`).
//!
//! A pattern is rendered once into a bounded RGBA tile by the scalar
//! rasterizer ([`render_pattern_tile`]) and then sampled per anti-aliasing
//! sample with the mapped inverse transform: shape-local point, then
//! `pattern_inverse` (tile pixels), a half-pixel shifted bilinear lookup that
//! wraps in both axes, and the pattern opacity. Only the renderer side lives
//! here; the decoder supplies the tile children and the mapping.

use crate::render::{ShapePaints, render_core};
use crate::stroke::invert;
use crate::{PreserveAspect, RenderError, RenderShape, ViewBox, ViewMap};
use std::ops::Range;
use vexel_geometry::{Affine, Point, affine_map_point, affine_valid};
use vexel_raster::Tile;
use vexel_scene::{Paint, PaintKind, Pattern, PatternId, Shape};

/// Most pattern tiles rendered or referenced at once (C `SHAPE_PATTERN_MAX_INSTANCES`).
pub const PATTERN_MAX_INSTANCES: usize = 1024;
/// Largest tile area in pixels (C `SHAPE_PATTERN_MAX_PIXELS`).
pub const PATTERN_MAX_PIXELS: usize = 1_048_576;
/// Largest tile width or height in pixels (C `SHAPE_PATTERN_MAX_AXIS`).
pub const PATTERN_MAX_AXIS: u32 = 4096;
/// Largest tile extent in user units (C check `tile_width > 65536.0`).
pub const PATTERN_MAX_TILE_EXTENT: f64 = 65536.0;

/// One pattern channel of a shape.
///
/// `inverse` maps the shape's local space (the space the element transform
/// maps to document space) to tile pixels, C `pattern_inverse`
/// (`vexel_scene::Paint::pattern_inverse`). `opacity` is
/// `fill-opacity`/`stroke-opacity`, in `[0, 1]`. A `tile` of `None` is a
/// pattern that paints nothing (for example a zero-sized tile): the channel
/// is painted with no pixels and never falls back to the solid colour.
#[derive(Debug, Clone, Copy)]
pub struct PatternPaint<'a> {
    pub tile: Option<&'a Tile>,
    pub inverse: Affine,
    pub opacity: f64,
}

/// A [`RenderShape`] whose fill and/or stroke is a pattern. The solid colours
/// of `shape` are used for every channel whose pattern is `None`.
#[derive(Debug, Clone, Copy)]
pub struct PatternPaintShape<'a> {
    pub shape: RenderShape<'a>,
    pub fill: Option<PatternPaint<'a>>,
    pub stroke: Option<PatternPaint<'a>>,
}

impl<'a> From<RenderShape<'a>> for PatternPaintShape<'a> {
    fn from(shape: RenderShape<'a>) -> Self {
        Self {
            shape,
            fill: None,
            stroke: None,
        }
    }
}

/// Size of one pattern tile: pixel size and user-space extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternTileSpec {
    pub width: u32,
    pub height: u32,
    pub tile_width: f64,
    pub tile_height: f64,
}

/// Pixel size of a pattern tile for a pixel `density` (C `pixel_width =
/// ceil(tile_width * density)`, at least 1). Over-axis or over-area tiles are
/// [`RenderError::LimitExceeded`]; a non-positive or non-finite extent or
/// density is [`RenderError::InvalidGeometry`].
pub fn pattern_pixel_size(
    tile_width: f64,
    tile_height: f64,
    density: f64,
) -> Result<(u32, u32), RenderError> {
    if !(tile_width.is_finite() && tile_height.is_finite() && tile_width > 0.0 && tile_height > 0.0)
        || !density.is_finite()
        || density <= 0.0
    {
        return Err(RenderError::InvalidGeometry);
    }
    if tile_width > PATTERN_MAX_TILE_EXTENT || tile_height > PATTERN_MAX_TILE_EXTENT {
        return Err(RenderError::LimitExceeded);
    }
    let width = (tile_width * density).ceil().max(1.0);
    let height = (tile_height * density).ceil().max(1.0);
    if !width.is_finite()
        || !height.is_finite()
        || width > f64::from(PATTERN_MAX_AXIS)
        || height > f64::from(PATTERN_MAX_AXIS)
        || width * height > PATTERN_MAX_PIXELS as f64
    {
        return Err(RenderError::LimitExceeded);
    }
    Ok((width as u32, height as u32))
}

fn check_pixels(width: u32, height: u32) -> Result<(), RenderError> {
    if width == 0 || height == 0 {
        return Err(RenderError::InvalidGeometry);
    }
    if width > PATTERN_MAX_AXIS
        || height > PATTERN_MAX_AXIS
        || (width as usize) * (height as usize) > PATTERN_MAX_PIXELS
    {
        return Err(RenderError::LimitExceeded);
    }
    Ok(())
}

/// Rasterizes the pattern content `children` (already in tile user space, that
/// is with the content transform applied) into a fresh tile, as C
/// `decode_pattern_content` does: view box `0 0 tile_width tile_height`
/// mapped onto the pixel grid with `preserveAspectRatio="none"`.
///
/// Returns `Ok(None)` when `tile_width` or `tile_height` is zero, negative or
/// not finite: such a pattern paints nothing. `control` has the semantics of
/// [`render_shapes`](crate::render_shapes) and is polled per child and row.
pub fn render_pattern_tile(
    spec: PatternTileSpec,
    children: &[PatternPaintShape<'_>],
    control: &dyn Fn() -> bool,
) -> Result<Option<Tile>, RenderError> {
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if !(positive(spec.tile_width) && positive(spec.tile_height)) {
        return Ok(None);
    }
    if spec.tile_width > PATTERN_MAX_TILE_EXTENT || spec.tile_height > PATTERN_MAX_TILE_EXTENT {
        return Err(RenderError::LimitExceeded);
    }
    check_pixels(spec.width, spec.height)?;
    let bytes = (spec.width as usize) * (spec.height as usize) * 4;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(bytes)
        .map_err(|_| RenderError::LimitExceeded)?;
    rgba.resize(bytes, 0);
    let mut tile = Tile::from_owned_rgba(spec.width, spec.height, rgba)
        .map_err(|_| RenderError::LimitExceeded)?;
    let view = ViewMap::from_viewbox(
        ViewBox {
            x: 0.0,
            y: 0.0,
            width: spec.tile_width,
            height: spec.tile_height,
        },
        spec.width,
        spec.height,
        Some(PreserveAspect {
            align_x: 0.0,
            align_y: 0.0,
            none: true,
            slice: false,
        }),
    )?;
    render_pattern_shapes(children, view, &mut tile, control)?;
    Ok(Some(tile))
}

/// Renders every `(spec, children)` request in order; more than
/// [`PATTERN_MAX_INSTANCES`] requests is [`RenderError::LimitExceeded`].
pub fn render_pattern_tiles(
    requests: &[(PatternTileSpec, &[PatternPaintShape<'_>])],
    control: &dyn Fn() -> bool,
) -> Result<Vec<Option<Tile>>, RenderError> {
    if requests.len() > PATTERN_MAX_INSTANCES {
        return Err(RenderError::LimitExceeded);
    }
    let mut tiles = Vec::new();
    tiles
        .try_reserve_exact(requests.len())
        .map_err(|_| RenderError::LimitExceeded)?;
    for (spec, children) in requests {
        if control() {
            return Err(RenderError::Cancelled);
        }
        tiles.push(render_pattern_tile(*spec, children, control)?);
    }
    Ok(tiles)
}

fn validate_paint(paint: &PatternPaint<'_>) -> Result<(), RenderError> {
    if !affine_valid(paint.inverse)
        || !paint.opacity.is_finite()
        || !(0.0..=1.0).contains(&paint.opacity)
    {
        return Err(RenderError::InvalidGeometry);
    }
    if let Some(tile) = paint.tile {
        check_pixels(tile.width(), tile.height())?;
    }
    Ok(())
}

fn validate_patterns(shapes: &[PatternPaintShape<'_>]) -> Result<(), RenderError> {
    let mut distinct: Vec<*const Tile> = Vec::new();
    for entry in shapes {
        for paint in [entry.fill.as_ref(), entry.stroke.as_ref()]
            .into_iter()
            .flatten()
        {
            validate_paint(paint)?;
            if invert(entry.shape.transform).is_none() {
                return Err(RenderError::InvalidGeometry);
            }
            if let Some(tile) = paint.tile {
                let address: *const Tile = tile;
                if !distinct.contains(&address) {
                    if distinct.len() == PATTERN_MAX_INSTANCES {
                        return Err(RenderError::LimitExceeded);
                    }
                    distinct.push(address);
                }
            }
        }
    }
    Ok(())
}

/// [`render_shapes`](crate::render_shapes) with pattern paint. A shape without
/// pattern paint renders exactly as through `render_shapes`. More than
/// [`PATTERN_MAX_INSTANCES`] distinct tiles, an invalid inverse or opacity, or
/// a pattern on a shape whose element transform is singular is an error before
/// any pixel is written.
pub fn render_pattern_shapes(
    shapes: &[PatternPaintShape<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    validate_patterns(shapes)?;
    let plain: Vec<RenderShape<'_>> = shapes.iter().map(|s| s.shape).collect();
    let paints: Vec<ShapePaints<'_>> = if shapes
        .iter()
        .any(|s| s.fill.is_some() || s.stroke.is_some())
    {
        shapes.iter().map(|s| (s.fill, s.stroke)).collect()
    } else {
        Vec::new()
    };
    render_core(&plain, &[], &[], &paints, view, tile, control)
}

fn wrap_index(index: i64, dimension: u32) -> usize {
    index.rem_euclid(i64::from(dimension)) as usize
}

/// C `pattern_sample`: the pattern colour at a shape-local point, or `None`
/// when the point cannot be mapped (the C function then paints nothing).
pub(crate) fn sample(paint: &PatternPaint<'_>, local: Point) -> Option<[u8; 4]> {
    let tile = paint.tile?;
    let mapped = affine_map_point(paint.inverse, local)?;
    if !mapped.x.is_finite()
        || !mapped.y.is_finite()
        || mapped.x.abs() > 1e12
        || mapped.y.abs() > 1e12
    {
        return None;
    }
    let (width, height) = (f64::from(tile.width()), f64::from(tile.height()));
    let mut sample_x = (mapped.x - 0.5) % width;
    let mut sample_y = (mapped.y - 0.5) % height;
    if sample_x < 0.0 {
        sample_x += width;
    }
    if sample_y < 0.0 {
        sample_y += height;
    }
    let x0 = sample_x.floor() as i64;
    let y0 = sample_y.floor() as i64;
    let (ix0, iy0) = (wrap_index(x0, tile.width()), wrap_index(y0, tile.height()));
    let (ix1, iy1) = (
        wrap_index(x0 + 1, tile.width()),
        wrap_index(y0 + 1, tile.height()),
    );
    let fx = sample_x - sample_x.floor();
    let fy = sample_y - sample_y.floor();
    let stride = tile.width() as usize;
    let at = |ix: usize, iy: usize| -> &[u8] {
        let offset = (iy * stride + ix) * 4;
        &tile.rgba()[offset..offset + 4]
    };
    let samples = [at(ix0, iy0), at(ix1, iy0), at(ix0, iy1), at(ix1, iy1)];
    let weights = [
        (1.0 - fx) * (1.0 - fy),
        fx * (1.0 - fy),
        (1.0 - fx) * fy,
        fx * fy,
    ];
    let mut alpha = 0.0f64;
    let mut premultiplied = [0.0f64; 3];
    for (pixel, weight) in samples.iter().zip(weights) {
        let a = f64::from(pixel[3]) / 255.0;
        alpha += a * weight;
        for (channel, slot) in premultiplied.iter_mut().enumerate() {
            *slot += f64::from(pixel[channel]) / 255.0 * a * weight;
        }
    }
    let opacity = paint.opacity;
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return None;
    }
    alpha *= opacity;
    if alpha <= 0.0 {
        return Some([0; 4]);
    }
    let divisor = if opacity == 0.0 { 1.0 } else { alpha / opacity };
    let mut rgba = [0u8; 4];
    for (channel, value) in premultiplied.iter().enumerate() {
        rgba[channel] = (((value / divisor) * 255.0).clamp(0.0, 255.0) + 0.5).floor() as u8;
    }
    rgba[3] = (alpha.min(1.0) * 255.0 + 0.5).floor() as u8;
    Some(rgba)
}

fn pattern_paint<'a>(
    index: Option<usize>,
    paints: &[Paint],
    resolve: &dyn Fn(PatternId) -> Option<&'a Pattern>,
) -> Result<Option<PatternPaint<'a>>, RenderError> {
    let Some(index) = index else {
        return Ok(None);
    };
    let paint = paints.get(index).ok_or(RenderError::InvalidGeometry)?;
    if paint.kind != PaintKind::Pattern {
        return Ok(None);
    }
    let pattern = paint
        .pattern
        .and_then(resolve)
        .ok_or(RenderError::InvalidGeometry)?;
    Ok(Some(PatternPaint {
        tile: Some(&pattern.tile),
        inverse: paint.pattern_inverse,
        opacity: paint.pattern_opacity,
    }))
}

/// [`from_scene_shape`](crate::from_scene_shape) for shapes that may use
/// pattern paint on fill and/or stroke. `resolve` looks up the completed
/// `vexel_scene::Pattern` of a paint's `PatternId`. Every other deferred
/// feature is still reported as `Unsupported`; a pattern paint that is not
/// resolvable is `InvalidGeometry`.
#[allow(clippy::too_many_arguments)]
pub fn from_scene_shape_with_pattern<'a>(
    shape: &Shape,
    fill_rgba: [u8; 4],
    fill_paints: &[Paint],
    stroke_paints: &[Paint],
    resolve: &dyn Fn(PatternId) -> Option<&'a Pattern>,
    edges: &'a [crate::FlatEdge],
    subpaths: &'a [Range<usize>],
    subpath_closed: &'a [bool],
) -> Result<PatternPaintShape<'a>, RenderError> {
    let fill = pattern_paint(shape.fill_paint, fill_paints, resolve)?;
    let stroke = pattern_paint(shape.stroke_paint, stroke_paints, resolve)?;
    // The solid adapter validates everything else; pattern channels are
    // presented to it as default solid paint.
    let mut solid = *shape;
    if fill.is_some() {
        solid.fill_paint = None;
    }
    if stroke.is_some() {
        solid.stroke_paint = None;
        solid.stroke.paint_server = false;
    }
    let solid_shape = crate::from_scene_shape(
        &solid,
        fill_rgba,
        fill_paints,
        stroke_paints,
        edges,
        subpaths,
        subpath_closed,
    )?;
    Ok(PatternPaintShape {
        shape: solid_shape,
        fill,
        stroke,
    })
}
