use crate::*;
use vexel_geometry::{Point, affine_map_point};
use vexel_scene::{
    AlphaMode, ControlledKernelResources, Paint, PaintKind, Result, SceneView, ShapeId,
    ValidatedDashView, WorkControl,
};

pub struct RasterSampleInput<'a> {
    pub resources: &'a dyn ControlledKernelResources,
    pub active_edges: &'a [u32],
    pub local_edges: &'a [vexel_geometry::Edge],
    pub dash_seams: &'a [bool],
    pub dash_cache: ValidatedDashView<'a>,
    pub px: f64,
    pub py: f64,
    pub paint_x: f64,
    pub paint_y: f64,
}
impl RasterSampleInput<'_> {
    fn paint_sample(
        &self,
        paint: &Paint,
        stops: &[vexel_scene::PaintStop],
        control: &mut dyn WorkControl,
    ) -> Result<[u8; 4]> {
        control.checkpoint().map_err(crate::stopped)?;
        self.resources
            .paint_sample(paint, stops, self.paint_x, self.paint_y, control)
    }
}

pub(crate) fn edge_crosses_right(edge: &vexel_geometry::Edge, px: f64, py: f64) -> bool {
    let up = edge.a.y <= py && edge.b.y > py;
    let down = edge.b.y <= py && edge.a.y > py;
    if !up && !down {
        return false;
    }
    let cross = edge.a.x + (py - edge.a.y) * (edge.b.x - edge.a.x) / (edge.b.y - edge.a.y);
    cross > px
}

pub(crate) fn wrap_index(value: isize, extent: usize) -> Option<usize> {
    let extent = isize::try_from(extent).ok()?;
    (extent > 0).then(|| value.rem_euclid(extent) as usize)
}

pub(crate) fn fill_rule_contains(even_odd: bool, winding: i32, parity: u8) -> bool {
    if even_odd { parity != 0 } else { winding != 0 }
}

pub(super) fn over(dst: &mut [u8; 4], src: [u8; 4]) {
    let sa = src[3] as u32;
    if sa == 0 {
        return;
    }
    let da = dst[3] as u32;
    let inv = 255 - sa;
    let an = sa * 255 + da * inv;
    for c in 0..3 {
        let n = src[c] as u32 * sa * 255 + dst[c] as u32 * da * inv;
        dst[c] = ((n + an / 2) / an) as u8
    }
    dst[3] = ((an + 127) / 255) as u8
}
fn image_sample(
    view: SceneView<'_>,
    shape: &vexel_scene::Shape,
    x: f64,
    y: f64,
) -> Option<[u8; 4]> {
    let image = view.images.get(shape.image?)?;
    let p = affine_map_point(shape.stroke_inverse, Point { x, y })?;
    if p.x < image.x
        || p.y < image.y
        || p.x >= image.x + image.width
        || p.y >= image.y + image.height
        || p.x < image.draw_x
        || p.y < image.draw_y
        || p.x >= image.draw_x + image.draw_width
        || p.y >= image.draw_y + image.draw_height
        || image.draw_width <= 0.0
        || image.draw_height <= 0.0
    {
        return None;
    }
    let u = (p.x - image.draw_x) / image.draw_width;
    let v = (p.y - image.draw_y) / image.draw_height;
    let mut sx = u * image.pixels.width as f64 - 0.5;
    let mut sy = v * image.pixels.height as f64 - 0.5;
    sx = sx.clamp(0.0, (image.pixels.width - 1) as f64);
    sy = sy.clamp(0.0, (image.pixels.height - 1) as f64);
    let x0 = sx.floor() as usize;
    let y0 = sy.floor() as usize;
    let x1 = (x0 + 1).min(image.pixels.width as usize - 1);
    let y1 = (y0 + 1).min(image.pixels.height as usize - 1);
    let fx = sx - x0 as f64;
    let fy = sy - y0 as f64;
    let pix = |xx: usize, yy: usize| -> Option<[u8; 4]> {
        let o = yy
            .checked_mul(image.pixels.row_stride)?
            .checked_add(xx.checked_mul(4)?)?;
        Some(*<&[u8; 4]>::try_from(image.pixels.rgba.get(o..o + 4)?).ok()?)
    };
    let a = pix(x0, y0)?;
    let b = pix(x1, y0)?;
    let c = pix(x0, y1)?;
    let d = pix(x1, y1)?;
    let mut out = [0; 4];
    for k in 0..4 {
        let top = a[k] as f64 * (1.0 - fx) + b[k] as f64 * fx;
        let bot = c[k] as f64 * (1.0 - fx) + d[k] as f64 * fx;
        out[k] = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8
    }
    let alpha = out[3];
    if alpha == 0 {
        return Some([0; 4]);
    }
    for c in 0..3 {
        out[c] = ((out[c] as u32 * 255 + alpha as u32 / 2) / alpha as u32).min(255) as u8
    }
    Some(out)
}
fn pattern_sample(
    view: SceneView<'_>,
    shape: &vexel_scene::Shape,
    paint: &Paint,
    x: f64,
    y: f64,
) -> Option<[u8; 4]> {
    let pattern = view.pattern(paint.pattern?)?;
    let p = affine_map_point(shape.stroke_inverse, Point { x, y })?;
    let tile = affine_map_point(paint.pattern_inverse, p)?;
    if !tile.x.is_finite() || !tile.y.is_finite() || tile.x.abs() > 1e12 || tile.y.abs() > 1e12 {
        return None;
    }
    let w = pattern.tile.width();
    let h = pattern.tile.height();
    let mut sx = (tile.x - 0.5) % (w as f64);
    let mut sy = (tile.y - 0.5) % (h as f64);
    if sx < 0.0 {
        sx += w as f64
    }
    if sy < 0.0 {
        sy += h as f64
    }
    let x0 = sx.floor() as usize;
    let y0 = sy.floor() as usize;
    let x1 = wrap_index(x0 as isize + 1, w as usize)?;
    let y1 = wrap_index(y0 as isize + 1, h as usize)?;
    let fx = sx - sx.floor();
    let fy = sy - sy.floor();
    let rgba = pattern.tile.rgba();
    let sample = |xx: usize, yy: usize| {
        let i = (yy * w as usize + xx) * 4;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    };
    let p = [
        sample(x0, y0),
        sample(x1, y0),
        sample(x0, y1),
        sample(x1, y1),
    ];
    let weights = [
        (1.0 - fx) * (1.0 - fy),
        fx * (1.0 - fy),
        (1.0 - fx) * fy,
        fx * fy,
    ];
    let mut alpha = 0.0;
    let mut pm = [0.0; 3];
    for i in 0..4 {
        let a = p[i][3] as f64 / 255.0;
        alpha += a * weights[i];
        for c in 0..3 {
            pm[c] += p[i][c] as f64 / 255.0 * a * weights[i]
        }
    }
    if !paint.pattern_opacity.is_finite() || !(0.0..=1.0).contains(&paint.pattern_opacity) {
        return None;
    }
    let opacity = paint.pattern_opacity;
    alpha *= opacity;
    if alpha <= 0.0 {
        return Some([0; 4]);
    }
    let mut out = [0; 4];
    for c in 0..3 {
        out[c] = (pm[c] / (alpha / opacity) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    }
    out[3] = (alpha.min(1.0) * 255.0).round() as u8;
    Some(out)
}
fn allowed(
    view: SceneView<'_>,
    shape: &vexel_scene::Shape,
    input: &RasterSampleInput<'_>,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(clip) = shape.clip else {
        return Ok(true);
    };
    input
        .resources
        .clip_contains(view, clip, input.px, input.py, control)
}
fn stroke_paint(
    view: SceneView<'_>,
    id: ShapeId,
    input: &RasterSampleInput<'_>,
    allowed: bool,
    alpha: AlphaMode,
    dst: &mut [u8; 4],
    control: &mut dyn WorkControl,
) -> Result<()> {
    if !allowed || !stroke_contains(view, id, input, control)? {
        return Ok(());
    }
    let Some(shape) = view.shape(id) else {
        return Ok(());
    };
    let Some(paint_i) = shape.stroke_paint else {
        if alpha == AlphaMode::FrozenOracleOnly {
            *dst = shape.stroke.rgba
        } else {
            over(dst, shape.stroke.rgba)
        }
        return Ok(());
    };
    let Some(p) = view.stroke_paints.get(paint_i) else {
        return Ok(());
    };
    let color = if p.kind == PaintKind::Pattern {
        pattern_sample(view, shape, p, input.px, input.py)
    } else if p.kind == PaintKind::Linear || p.kind == PaintKind::Radial {
        let mut c = input.paint_sample(p, p.stops.slice(view.stops).unwrap_or(&[]), control)?;
        c[3] = ((c[3] as u32 * shape.stroke.rgba[3] as u32 + 127) / 255) as u8;
        Some(c)
    } else {
        Some(shape.stroke.rgba)
    };
    if let Some(c) = color {
        if alpha == AlphaMode::FrozenOracleOnly && p.kind == PaintKind::Solid {
            *dst = c
        } else {
            over(dst, c)
        }
    }
    Ok(())
}
fn paint_result(
    view: SceneView<'_>,
    id: ShapeId,
    input: &RasterSampleInput<'_>,
    alpha: AlphaMode,
    inside: bool,
    fill: [u8; 4],
    dst: &mut [u8; 4],
    control: &mut dyn WorkControl,
) -> Result<()> {
    let Some(shape) = view.shape(id) else {
        return Ok(());
    };
    let allow = allowed(view, shape, input, control)?;
    let stroke_first = shape.stroke.paint_order == 1;
    if stroke_first {
        stroke_paint(view, id, input, allow, alpha, dst, control)?
    }
    if inside && allow {
        if let Some(image) = image_sample(view, shape, input.px, input.py) {
            over(dst, image)
        } else if let Some(pi) = shape.fill_paint {
            if let Some(p) = view.fill_paints.get(pi) {
                let sampled = match p.kind {
                    PaintKind::Image => image_sample(view, shape, input.px, input.py),
                    PaintKind::Pattern => pattern_sample(view, shape, p, input.px, input.py),
                    PaintKind::Linear | PaintKind::Radial => Some(input.paint_sample(
                        p,
                        p.stops.slice(view.stops).unwrap_or(&[]),
                        control,
                    )?),
                    PaintKind::Solid => Some(fill),
                };
                if let Some(c) = sampled {
                    if alpha == AlphaMode::FrozenOracleOnly && p.kind == PaintKind::Solid {
                        *dst = c
                    } else {
                        over(dst, c)
                    }
                }
            }
        } else if fill[3] != 0 {
            if alpha == AlphaMode::FrozenOracleOnly {
                *dst = fill
            } else {
                over(dst, fill)
            }
        }
    }
    if !stroke_first {
        stroke_paint(view, id, input, allow, alpha, dst, control)?
    }
    Ok(())
}

pub fn paint_sample(
    view: SceneView<'_>,
    id: ShapeId,
    input: &RasterSampleInput<'_>,
    alpha: AlphaMode,
    control: &mut dyn WorkControl,
) -> Result<[u8; 4]> {
    let Some(shape) = view.shape(id) else {
        return Ok([0; 4]);
    };
    let mut winding = 0i32;
    let mut parity = 0u8;
    let mut fill = [0; 4];
    if input.active_edges.is_empty() {
        let indices = shape
            .edges
            .end()
            .filter(|end| *end <= input.local_edges.len())
            .ok_or(vexel_scene::Diagnostic {
                kind: vexel_scene::ErrorKind::InvalidArgument,
                offset: 0,
            })?;
        for i in shape.edges.first..indices {
            let edge = &input.local_edges[i];
            control
                .charge(vexel_scene::WorkKind::FillEdgeTest, 1)
                .map_err(crate::stopped)?;
            let up = edge.a.y <= input.py && edge.b.y > input.py;
            if edge_crosses_right(edge, input.px, input.py) {
                winding += if up { 1 } else { -1 };
                parity ^= 1;
                fill = *view.edge_colors.get(i).ok_or(vexel_scene::Diagnostic {
                    kind: vexel_scene::ErrorKind::InvalidArgument,
                    offset: 0,
                })?
            }
        }
    } else {
        for i in input.active_edges {
            let edge = input
                .local_edges
                .get(*i as usize)
                .ok_or(vexel_scene::Diagnostic {
                    kind: vexel_scene::ErrorKind::InvalidArgument,
                    offset: 0,
                })?;
            control
                .charge(vexel_scene::WorkKind::FillEdgeTest, 1)
                .map_err(crate::stopped)?;
            let up = edge.a.y <= input.py && edge.b.y > input.py;
            if edge_crosses_right(edge, input.px, input.py) {
                winding += if up { 1 } else { -1 };
                parity ^= 1;
                fill = *view
                    .edge_colors
                    .get(*i as usize)
                    .ok_or(vexel_scene::Diagnostic {
                        kind: vexel_scene::ErrorKind::InvalidArgument,
                        offset: 0,
                    })?
            }
        }
    }
    let inside = fill_rule_contains(shape.evenodd, winding, parity);
    let mut out = [0; 4];
    paint_result(view, id, input, alpha, inside, fill, &mut out, control)?;
    Ok(out)
}
pub fn paint_sample_inside(
    view: SceneView<'_>,
    id: ShapeId,
    fill: [u8; 4],
    input: &RasterSampleInput<'_>,
    alpha: AlphaMode,
    control: &mut dyn WorkControl,
) -> Result<[u8; 4]> {
    let mut out = [0; 4];
    paint_result(view, id, input, alpha, true, fill, &mut out, control)?;
    Ok(out)
}
