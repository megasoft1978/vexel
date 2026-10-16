//! Owned render shapes extracted from a decoded scene, plus hit testing.

use std::ops::Range;
use vexel_geometry::Affine;
use vexel_render::{
    DashedShape, FlatEdge, PatternPaint, RenderError, RenderShape, StrokeStyle, UnsupportedKind,
    from_scene_shape_with_pattern,
};
use vexel_scene::{SceneView, TableRange, ViewportRead};

/// A renderable shape that owns its geometry so it outlives the scene graph.
#[derive(Debug, Clone)]
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
}

impl OwnedShape {
    pub fn as_render(&self) -> RenderShape<'_> {
        RenderShape {
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
        if self.filled && (self.fill_pattern || self.fill_rgba[3] > 0) && self.fill_hit(&runs, x, y)
        {
            return true;
        }
        let Some(style) = self
            .stroke
            .filter(|s| (self.stroke_pattern || s.rgba[3] > 0) && s.width > 0.0)
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
/// image, crisp edges, paint order, or a combination the renderer cannot
/// express such as a clipped or patterned dashed shape) is returned as
/// [`Entry::Skip`], never dropped silently.
pub(crate) fn prepare<'a, V: ViewportRead, A: Copy>(
    scene: &SceneView<'a, V, A>,
) -> Result<Vec<Entry<'a>>, RenderError> {
    let bad = RenderError::InvalidGeometry;
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
            Err(RenderError::Unsupported(kind)) => {
                entries.push(Entry::Skip(kind));
                continue;
            }
            Err(error) => return Err(error),
        };
        let patterned = paint.fill.is_some() || paint.stroke.is_some();
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
                fill_pattern: paint.fill.is_some(),
                stroke_pattern: paint.stroke.is_some(),
            },
            fill: lift(scene, paint.fill, shape.fill_paint, scene.fill_paints),
            stroke: lift(scene, paint.stroke, shape.stroke_paint, scene.stroke_paints),
            clip: shape.clip.map(|clip| clip.index()),
        }));
    }
    Ok(entries)
}
