//! Owned render shapes extracted from a decoded scene, plus hit testing.

use std::ops::Range;
use std::sync::Arc;
use vexel_geometry::Affine;
use vexel_raster::Tile;
use vexel_render::{
    DashedShape, FlatEdge, GradientRef, PatternPaint, PatternPaintShape, RenderError, RenderShape,
    StrokeStyle, UnsupportedKind, from_scene_shape_with_gradients, from_scene_shape_with_pattern,
};
use vexel_scene::{Paint, PaintStop, SceneView, TableRange, ViewportRead};

/// A gradient paint owned by its shape. The stops are copied out of the scene
/// table and `paint.stops` is rebased to index that copy from zero.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedGradient {
    paint: Paint,
    stops: Vec<PaintStop>,
}

impl OwnedGradient {
    fn new(gradient: GradientRef<'_>) -> Result<Self, RenderError> {
        let section = gradient
            .paint
            .stops
            .slice(gradient.stops)
            .ok_or(RenderError::InvalidGeometry)?
            .to_vec();
        let mut paint = *gradient.paint;
        paint.stops = TableRange {
            first: 0,
            count: section.len(),
        };
        Ok(Self {
            paint,
            stops: section,
        })
    }

    fn borrow(&self) -> GradientRef<'_> {
        GradientRef {
            paint: &self.paint,
            stops: &self.stops,
        }
    }
}

#[cfg(test)]
impl OwnedShape {
    /// Gives the stroke a gradient paint, with the stops copied from `gradient`.
    pub(crate) fn set_stroke_gradient_for_test(
        &mut self,
        gradient: GradientRef<'_>,
    ) -> Result<(), RenderError> {
        self.stroke_gradient = Some(OwnedGradient::new(gradient)?);
        Ok(())
    }
}

/// A renderable shape that owns its geometry so it outlives the scene graph.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedShape {
    /// Index of the shape in scene (source) order.
    pub scene_index: usize,
    pub edges: Vec<FlatEdge>,
    pub subpaths: Vec<Range<usize>>,
    pub subpath_closed: Vec<bool>,
    pub stroke_closed: bool,
    pub evenodd: bool,
    pub filled: bool,
    pub fill_rgba: [u8; 4],
    pub stroke: Option<StrokeStyle>,
    pub transform: Affine,
    /// `stroke-dasharray` values; empty for a solid stroke.
    pub dashes: Vec<f64>,
    /// Fill / stroke paint is a pattern (the solid colour is then unused).
    pub fill_pattern: bool,
    pub stroke_pattern: bool,
    /// Fill / stroke paint is a linear or radial gradient.
    pub fill_gradient: Option<OwnedGradient>,
    pub stroke_gradient: Option<OwnedGradient>,
}

impl OwnedShape {
    pub fn as_render(&self) -> RenderShape<'_> {
        RenderShape {
            fill_gradient: self.fill_gradient.as_ref().map(OwnedGradient::borrow),
            stroke_gradient: self.stroke_gradient.as_ref().map(OwnedGradient::borrow),
            edges: &self.edges,
            subpaths: &self.subpaths,
            subpath_closed: &self.subpath_closed,
            stroke_closed: self.stroke_closed,
            evenodd: self.evenodd,
            filled: self.filled,
            fill_rgba: self.fill_rgba,
            stroke: self.stroke,
            transform: self.transform,
        }
    }

    pub fn as_dashed(&self) -> DashedShape<'_> {
        DashedShape {
            shape: self.as_render(),
            dashes: &self.dashes,
        }
    }

    /// Edge runs (subpaths, or the whole list) with their closed flag.
    fn runs(&self) -> Vec<(&[FlatEdge], bool)> {
        if self.subpaths.is_empty() {
            return vec![(&self.edges[..], self.stroke_closed)];
        }
        self.subpaths
            .iter()
            .zip(&self.subpath_closed)
            .filter_map(|(r, c)| self.edges.get(r.clone()).map(|e| (e, *c)))
            .collect()
    }

    /// Whether a document-space point lies on this shape's visible paint.
    /// Fill uses the shape's fill rule with each run implicitly closed; stroke
    /// is the distance to the edges within half the transformed width (caps
    /// and joins are not modelled).
    pub fn hit(&self, x: f64, y: f64) -> bool {
        let runs = self.runs();
        let fill_paint = self.fill_pattern || self.fill_gradient.is_some() || self.fill_rgba[3] > 0;
        if self.filled && fill_paint && self.fill_hit(&runs, x, y) {
            return true;
        }
        let stroke_paint = self.stroke_pattern || self.stroke_gradient.is_some();
        let Some(style) = self
            .stroke
            .filter(|s| (stroke_paint || s.rgba[3] > 0) && s.width > 0.0)
        else {
            return false;
        };
        let det = (self.transform.a * self.transform.d - self.transform.b * self.transform.c).abs();
        let reach = style.width * det.sqrt() / 2.0;
        runs.iter().any(|(edges, closed)| {
            let close = closing(edges, *closed);
            edges
                .iter()
                .chain(close.iter())
                .any(|e| distance(e, x, y) <= reach)
        })
    }

    fn fill_hit(&self, runs: &[(&[FlatEdge], bool)], x: f64, y: f64) -> bool {
        let mut winding = 0i32;
        let mut crossings = 0u32;
        for (edges, _) in runs {
            let close = closing(edges, true);
            for e in edges.iter().chain(close.iter()) {
                if (e.y0 <= y) != (e.y1 <= y) {
                    let t = (y - e.y0) / (e.y1 - e.y0);
                    if x < e.x0 + t * (e.x1 - e.x0) {
                        crossings += 1;
                        winding += if e.y1 > e.y0 { 1 } else { -1 };
                    }
                }
            }
        }
        if self.evenodd {
            crossings % 2 == 1
        } else {
            winding != 0
        }
    }
}

fn closing(edges: &[FlatEdge], closed: bool) -> Option<FlatEdge> {
    let (first, last) = (edges.first()?, edges.last()?);
    (closed && (first.x0 != last.x1 || first.y0 != last.y1)).then_some(FlatEdge {
        x0: last.x1,
        y0: last.y1,
        x1: first.x0,
        y1: first.y0,
    })
}

fn distance(e: &FlatEdge, x: f64, y: f64) -> f64 {
    let (dx, dy) = (e.x1 - e.x0, e.y1 - e.y0);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((x - e.x0) * dx + (y - e.y0) * dy) / len2).clamp(0.0, 1.0)
    };
    (x - (e.x0 + t * dx)).hypot(y - (e.y0 + t * dy))
}

/// One scene shape ready to render: owned geometry plus the borrowed pattern
/// tiles and the clip it is limited to (an index into the scene clip table).
pub(crate) struct Prepared<'a> {
    pub shape: OwnedShape,
    pub fill: Option<PatternPaint<'a>>,
    pub stroke: Option<PatternPaint<'a>>,
    pub clip: Option<usize>,
    /// Image fill: a padded copy of the pixels and the local-to-tile mapping.
    pub image: Option<(Arc<Tile>, Affine)>,
}

impl Prepared<'_> {
    /// The fill paint: its pattern, or the image drawn as a pattern tile.
    pub fn fill_paint(&self) -> Option<PatternPaint<'_>> {
        self.fill.or_else(|| {
            self.image.as_ref().map(|(tile, inverse)| PatternPaint {
                tile: Some(&**tile),
                inverse: *inverse,
                opacity: 1.0,
            })
        })
    }
}

/// An `<image>` as a pattern tile: the pixels with a one pixel border of
/// replicated edge pixels, so the pattern sampler's wrapping bilinear lookup
/// equals the clamped lookup of the image sampler. `inverse` maps the shape's
/// local space to tile pixels over the draw rectangle.
fn image_tile(
    image: &vexel_scene::Image,
    shared: &mut Vec<Arc<Tile>>,
) -> Result<(Arc<Tile>, Affine), RenderError> {
    let pixels = &image.pixels;
    pixels
        .validate(0)
        .map_err(|_| RenderError::InvalidGeometry)?;
    let (w, h) = (pixels.width as usize, pixels.height as usize);
    let mut rgba = Vec::new();
    rgba.try_reserve_exact((w + 2) * (h + 2) * 4)
        .map_err(|_| RenderError::LimitExceeded)?;
    for y in 0..h + 2 {
        let row = y.saturating_sub(1).min(h - 1) * pixels.row_stride;
        for x in 0..w + 2 {
            let at = row + x.saturating_sub(1).min(w - 1) * 4;
            rgba.extend_from_slice(&pixels.rgba[at..at + 4]);
        }
    }
    let tile = Tile::from_owned_rgba(w as u32 + 2, h as u32 + 2, rgba)
        .map_err(|_| RenderError::LimitExceeded)?;
    // Images with the same pixels share one tile: the renderer bounds the number
    // of distinct tiles per batch, and a page of repeated icons is common.
    let tile = match shared.iter().find(|t| {
        t.width() == tile.width() && t.height() == tile.height() && t.rgba() == tile.rgba()
    }) {
        Some(existing) => existing.clone(),
        None => {
            let new = Arc::new(tile);
            shared.push(new.clone());
            new
        }
    };
    let (sx, sy) = (w as f64 / image.draw_width, h as f64 / image.draw_height);
    let inverse = Affine {
        a: sx,
        b: 0.0,
        c: 0.0,
        d: sy,
        e: 1.0 - image.draw_x * sx,
        f: 1.0 - image.draw_y * sy,
    };
    Ok((tile, inverse))
}

/// A scene shape is either drawn or skipped with the deferred feature named.
pub(crate) enum Entry<'a> {
    Draw(Prepared<'a>),
    Skip(UnsupportedKind),
}

/// Re-resolves a pattern paint against the scene so its tile borrows the scene
/// (the adapter ties it to the shape's local geometry).
fn lift<'a, V: ViewportRead, A: Copy>(
    scene: &SceneView<'a, V, A>,
    paint: Option<PatternPaint<'_>>,
    index: Option<usize>,
    paints: &[vexel_scene::Paint],
) -> Option<PatternPaint<'a>> {
    let paint = paint?;
    let pattern = scene.pattern(paints.get(index?)?.pattern?)?;
    Some(PatternPaint {
        tile: Some(&pattern.tile),
        inverse: paint.inverse,
        opacity: paint.opacity,
    })
}

/// Prepares every shape of `scene` for the renderer. Solid, dashed, pattern
/// and clipped shapes are drawn; a shape with a deferred feature (gradient,
/// crisp edges, paint order, or a combination the renderer cannot
/// express such as a clipped or patterned dashed shape) is returned as
/// [`Entry::Skip`], never dropped silently.
pub(crate) fn prepare<'a, V: ViewportRead, A: Copy>(
    scene: &SceneView<'a, V, A>,
) -> Result<Vec<Entry<'a>>, RenderError> {
    let bad = RenderError::InvalidGeometry;
    let mut shared_tiles: Vec<Arc<Tile>> = Vec::new();
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(scene.shapes.len())
        .map_err(|_| RenderError::LimitExceeded)?;
    for (scene_index, shape) in scene.shapes.iter().enumerate() {
        let range = shape.edges.first
            ..shape
                .edges
                .first
                .checked_add(shape.edges.count)
                .ok_or(bad)?;
        let edges: Vec<FlatEdge> = scene
            .edges
            .get(range.clone())
            .ok_or(bad)?
            .iter()
            .map(|e| FlatEdge::from(*e))
            .collect();
        let fill_rgba = scene
            .edge_colors
            .get(range.start)
            .copied()
            .filter(|_| !edges.is_empty())
            .unwrap_or([0; 4]);
        let subs = shape.subpaths.slice(scene.path_subpaths).ok_or(bad)?;
        // Scene subpaths index the scene-wide edge table; the renderer wants
        // ranges relative to this shape's own edges.
        let subpaths = subs
            .iter()
            .map(|s| {
                let first = s.first_edge.checked_sub(range.start)?;
                Some(first..first.checked_add(s.edge_count)?)
            })
            .collect::<Option<Vec<Range<usize>>>>()
            .ok_or(bad)?;
        let subpath_closed: Vec<bool> = subs.iter().map(|s| s.closed).collect();
        let dashes = shape
            .stroke
            .dash_values
            .slice(scene.dash_values)
            .ok_or(bad)?;
        // Clips and dashes are routed by the scene renderer, not the solid adapter.
        let mut solid = *shape;
        solid.clip = None;
        solid.stroke.dash_values = TableRange::EMPTY;
        let image = match shape.image {
            Some(index) => {
                solid.image = None;
                solid.fill_paint = None;
                Some(image_tile(
                    scene.images.get(index).ok_or(bad)?,
                    &mut shared_tiles,
                )?)
            }
            None => None,
        };
        let resolve = |id| scene.pattern(id);
        let converted = from_scene_shape_with_pattern(
            &solid,
            fill_rgba,
            scene.fill_paints,
            scene.stroke_paints,
            &resolve,
            &edges,
            &subpaths,
            &subpath_closed,
        );
        let paint = match converted {
            Ok(paint) => paint,
            // Gradient paint is drawn per sample by the renderer. The pattern
            // entry point stays solid-only (it has no scene stop table), so a
            // gradient shape is retried here. A shape with a pattern channel and
            // a gradient channel fails this retry with `PatternPaint` and is
            // skipped. That is deliberate: the renderer's centre-only pattern
            // sampling does not mark gradient stroke edges, so such a shape
            // would anti-alias wrongly. Not fixed here.
            Err(RenderError::Unsupported(UnsupportedKind::GradientPaint)) => {
                let gradient = from_scene_shape_with_gradients(
                    &solid,
                    fill_rgba,
                    scene.fill_paints,
                    scene.stroke_paints,
                    scene.stops,
                    &edges,
                    &subpaths,
                    &subpath_closed,
                );
                match gradient {
                    Ok(render) => PatternPaintShape {
                        shape: render,
                        fill: None,
                        stroke: None,
                    },
                    Err(RenderError::Unsupported(kind)) => {
                        entries.push(Entry::Skip(kind));
                        continue;
                    }
                    Err(error) => return Err(error),
                }
            }
            Err(RenderError::Unsupported(kind)) => {
                entries.push(Entry::Skip(kind));
                continue;
            }
            Err(error) => return Err(error),
        };
        let patterned = paint.fill.is_some() || paint.stroke.is_some() || image.is_some();
        let render = paint.shape;
        if render.stroke.is_some_and(|s| s.crisp_edges) {
            entries.push(Entry::Skip(UnsupportedKind::CrispEdges));
            continue;
        }
        if shape.clip.is_some() && (patterned || !dashes.is_empty()) {
            entries.push(Entry::Skip(UnsupportedKind::Clip));
            continue;
        }
        if patterned && !dashes.is_empty() {
            entries.push(Entry::Skip(UnsupportedKind::Dash));
            continue;
        }
        entries.push(Entry::Draw(Prepared {
            shape: OwnedShape {
                scene_index,
                edges: edges.clone(),
                subpaths: subpaths.clone(),
                subpath_closed: subpath_closed.clone(),
                stroke_closed: render.stroke_closed,
                evenodd: render.evenodd,
                filled: render.filled,
                fill_rgba: render.fill_rgba,
                stroke: render.stroke,
                transform: render.transform,
                dashes: dashes.to_vec(),
                fill_pattern: paint.fill.is_some() || image.is_some(),
                stroke_pattern: paint.stroke.is_some(),
                fill_gradient: render.fill_gradient.map(OwnedGradient::new).transpose()?,
                stroke_gradient: render.stroke_gradient.map(OwnedGradient::new).transpose()?,
            },
            fill: lift(scene, paint.fill, shape.fill_paint, scene.fill_paints),
            stroke: lift(scene, paint.stroke, shape.stroke_paint, scene.stroke_paints),
            clip: shape.clip.map(|clip| clip.index()),
            image,
        }));
    }
    Ok(entries)
}
