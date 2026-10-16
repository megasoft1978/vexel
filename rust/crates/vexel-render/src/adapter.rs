//! Adapter from `vexel_scene` records to [`RenderShape`], reporting every
//! deferred feature as a typed error instead of skipping it.

use crate::stroke::{invert, is_visible};
use crate::{FlatEdge, RenderError, RenderShape, StrokeStyle, UnsupportedKind};
use std::ops::Range;
use vexel_geometry::affine_identity;
use vexel_scene::{Paint, PaintKind, Shape};

fn classify(index: Option<usize>, paints: &[Paint]) -> Result<(), RenderError> {
    let Some(index) = index else {
        return Ok(());
    };
    let paint = paints.get(index).ok_or(RenderError::InvalidGeometry)?;
    match paint.kind {
        PaintKind::Solid => Ok(()),
        PaintKind::Linear | PaintKind::Radial => {
            Err(RenderError::Unsupported(UnsupportedKind::GradientPaint))
        }
        PaintKind::Pattern => Err(RenderError::Unsupported(UnsupportedKind::PatternPaint)),
        PaintKind::Image => Err(RenderError::Unsupported(UnsupportedKind::ImagePaint)),
    }
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
    if edges.len() != shape.edges.count || subpaths.len() != shape.subpaths.count {
        return Err(RenderError::InvalidGeometry);
    }
    if shape.clip.is_some() {
        return Err(RenderError::Unsupported(UnsupportedKind::Clip));
    }
    if shape.image.is_some() {
        return Err(RenderError::Unsupported(UnsupportedKind::ImagePaint));
    }
    classify(shape.fill_paint, fill_paints)?;
    classify(shape.stroke_paint, stroke_paints)?;
    let scene_stroke = &shape.stroke;
    if scene_stroke.paint_server && shape.stroke_paint.is_none() {
        return Err(RenderError::Unsupported(UnsupportedKind::GradientPaint));
    }
    if scene_stroke.dash_values.count != 0 {
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
        None if is_visible(&style) && !edges.is_empty() => return Err(RenderError::InvalidGeometry),
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
