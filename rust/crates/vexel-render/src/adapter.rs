//! Adapter from `vexel_scene` records to [`RenderShape`], reporting every
//! deferred feature as a typed error instead of skipping it.

use crate::stroke::{invert, is_painted};
use crate::{
    DashedShape, FlatEdge, GradientRef, RenderError, RenderShape, StrokeStyle, UnsupportedKind,
};
use std::ops::Range;
use vexel_geometry::affine_identity;
use vexel_scene::{Paint, PaintKind, PaintStop, Shape};

/// Whether a paint channel is a gradient. Gradients are `Unsupported` unless
/// `gradients` is set, which only [`from_scene_shape_with_gradients`] does.
fn classify(index: Option<usize>, paints: &[Paint], gradients: bool) -> Result<bool, RenderError> {
    let Some(index) = index else {
        return Ok(false);
    };
    let paint = paints.get(index).ok_or(RenderError::InvalidGeometry)?;
    match paint.kind {
        PaintKind::Solid => Ok(false),
        PaintKind::Linear | PaintKind::Radial if gradients => Ok(true),
        PaintKind::Linear | PaintKind::Radial => {
            Err(RenderError::Unsupported(UnsupportedKind::GradientPaint))
        }
        PaintKind::Pattern => Err(RenderError::Unsupported(UnsupportedKind::PatternPaint)),
        PaintKind::Image => Err(RenderError::Unsupported(UnsupportedKind::ImagePaint)),
    }
}

/// The gradient of a channel that [`classify`] accepted, with the scene stop
/// table its paint indexes. Solid or absent paint gives `None`.
fn gradient_ref<'a>(
    index: Option<usize>,
    paints: &'a [Paint],
    stops: &'a [PaintStop],
) -> Result<Option<GradientRef<'a>>, RenderError> {
    let Some(index) = index else {
        return Ok(None);
    };
    let paint = paints.get(index).ok_or(RenderError::InvalidGeometry)?;
    if !matches!(paint.kind, PaintKind::Linear | PaintKind::Radial) {
        return Ok(None);
    }
    if paint.stops.slice(stops).is_none() {
        return Err(RenderError::InvalidGeometry);
    }
    Ok(Some(GradientRef { paint, stops }))
}

/// Converts one scene shape. `edges`, `subpaths` (ranges relative to `edges`)
/// and `subpath_closed` are the shape's own geometry, and `fill_rgba` is the
/// colour its edges carry. Returns `Unsupported` for non-solid fill or stroke
/// paint, dashes, clips, images and non-default paint order. Crisp edges are
/// carried in [`StrokeStyle`] and rejected by `render_shapes`.
pub fn from_scene_shape<'a>(
    shape: &Shape,
    fill_rgba: [u8; 4],
    fill_paints: &[Paint],
    stroke_paints: &[Paint],
    edges: &'a [FlatEdge],
    subpaths: &'a [Range<usize>],
    subpath_closed: &'a [bool],
) -> Result<RenderShape<'a>, RenderError> {
    convert(
        shape,
        fill_rgba,
        fill_paints,
        stroke_paints,
        edges,
        subpaths,
        subpath_closed,
        false,
        false,
    )
}

/// [`from_scene_shape`] that also accepts linear and radial gradient paint on
/// fill and stroke. `stops` is the scene's whole stop table, which the paints
/// index. Every other deferral is reported exactly as by `from_scene_shape`.
pub fn from_scene_shape_with_gradients<'a>(
    shape: &Shape,
    fill_rgba: [u8; 4],
    fill_paints: &'a [Paint],
    stroke_paints: &'a [Paint],
    stops: &'a [PaintStop],
    edges: &'a [FlatEdge],
    subpaths: &'a [Range<usize>],
    subpath_closed: &'a [bool],
) -> Result<RenderShape<'a>, RenderError> {
    let mut render = convert(
        shape,
        fill_rgba,
        fill_paints,
        stroke_paints,
        edges,
        subpaths,
        subpath_closed,
        false,
        true,
    )?;
    render.fill_gradient = gradient_ref(shape.fill_paint, fill_paints, stops)?;
    render.stroke_gradient = gradient_ref(shape.stroke_paint, stroke_paints, stops)?;
    Ok(render)
}

/// Like [`from_scene_shape`] but accepts dashed strokes. `dashes` is the
/// shape's slice of the scene dash table (`SceneView::shape_dashes`); its
/// length must equal the scene stroke's `dash_values.count`.
pub fn from_scene_shape_with_dashes<'a>(
    shape: &Shape,
    fill_rgba: [u8; 4],
    fill_paints: &[Paint],
    stroke_paints: &[Paint],
    edges: &'a [FlatEdge],
    subpaths: &'a [Range<usize>],
    subpath_closed: &'a [bool],
    dashes: &'a [f64],
) -> Result<DashedShape<'a>, RenderError> {
    if dashes.len() != shape.stroke.dash_values.count {
        return Err(RenderError::InvalidGeometry);
    }
    let shape = convert(
        shape,
        fill_rgba,
        fill_paints,
        stroke_paints,
        edges,
        subpaths,
        subpath_closed,
        true,
        false,
    )?;
    Ok(DashedShape { shape, dashes })
}

#[allow(clippy::too_many_arguments)]
fn convert<'a>(
    shape: &Shape,
    fill_rgba: [u8; 4],
    fill_paints: &[Paint],
    stroke_paints: &[Paint],
    edges: &'a [FlatEdge],
    subpaths: &'a [Range<usize>],
    subpath_closed: &'a [bool],
    allow_dashes: bool,
    gradients: bool,
) -> Result<RenderShape<'a>, RenderError> {
    if edges.len() != shape.edges.count || subpaths.len() != shape.subpaths.count {
        return Err(RenderError::InvalidGeometry);
    }
    if shape.clip.is_some() {
        return Err(RenderError::Unsupported(UnsupportedKind::Clip));
    }
    if shape.image.is_some() {
        return Err(RenderError::Unsupported(UnsupportedKind::ImagePaint));
    }
    classify(shape.fill_paint, fill_paints, gradients)?;
    let stroke_gradient = classify(shape.stroke_paint, stroke_paints, gradients)?;
    let scene_stroke = &shape.stroke;
    if scene_stroke.paint_server && shape.stroke_paint.is_none() {
        return Err(RenderError::Unsupported(UnsupportedKind::GradientPaint));
    }
    if !allow_dashes && scene_stroke.dash_values.count != 0 {
        return Err(RenderError::Unsupported(UnsupportedKind::Dash));
    }
    if scene_stroke.paint_order != 0 {
        return Err(RenderError::Unsupported(UnsupportedKind::PaintOrder));
    }
    let style = StrokeStyle {
        rgba: scene_stroke.rgba,
        width: scene_stroke.width,
        miter_limit: scene_stroke.miter_limit,
        dash_offset: scene_stroke.dash_offset,
        cap: scene_stroke.cap,
        join: scene_stroke.join,
        crisp_edges: scene_stroke.crisp_edges,
    };
    let transform = match invert(shape.stroke_inverse) {
        Some(forward) => forward,
        None if is_painted(&style, stroke_gradient) && !edges.is_empty() => {
            return Err(RenderError::InvalidGeometry);
        }
        None => affine_identity(),
    };
    Ok(RenderShape {
        edges,
        subpaths,
        subpath_closed,
        stroke_closed: shape.stroke_closed,
        evenodd: shape.evenodd,
        filled: shape.filled,
        fill_rgba,
        stroke: Some(style),
        transform,
        fill_gradient: None,
        stroke_gradient: None,
    })
}

/// Scene-wide features the V1 renderer defers. Callers fill the counts from
/// the scene tables and call [`SceneFeatures::require_supported`] before
/// rendering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SceneFeatures {
    pub clips: usize,
    pub opacity_groups: usize,
    pub masks: usize,
    pub filters: usize,
    pub text_runs: usize,
    pub animation_overlay: bool,
    pub smil: bool,
}

impl SceneFeatures {
    /// First deferred feature present, as a typed error.
    pub fn require_supported(&self) -> Result<(), RenderError> {
        let kind = if self.clips != 0 {
            UnsupportedKind::Clip
        } else if self.opacity_groups != 0 {
            UnsupportedKind::OpacityGroup
        } else if self.masks != 0 {
            UnsupportedKind::Mask
        } else if self.filters != 0 {
            UnsupportedKind::Filter
        } else if self.text_runs != 0 {
            UnsupportedKind::Text
        } else if self.animation_overlay {
            UnsupportedKind::AnimationOverlay
        } else if self.smil {
            UnsupportedKind::Smil
        } else {
            return Ok(());
        };
        Err(RenderError::Unsupported(kind))
    }
}
