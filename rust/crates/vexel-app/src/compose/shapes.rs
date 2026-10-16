//! Owned render shapes extracted from a decoded scene, plus hit testing.

use super::error::{ComposeError, Disclosure, element_at, feature_name};
use std::ops::Range;
use vexel_geometry::Affine;
use vexel_render::{FlatEdge, RenderError, RenderShape, StrokeStyle, from_scene_shape};
use vexel_scene::SceneView;

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
        if self.filled && self.fill_rgba[3] > 0 && self.fill_hit(&runs, x, y) {
            return true;
        }
        let Some(style) = self.stroke.filter(|s| s.rgba[3] > 0 && s.width > 0.0) else {
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

/// Extracts every shape the V1 renderer supports. A shape with a deferred
/// feature is skipped and returned as a disclosure; any other adapter failure
/// is an error.
pub(crate) fn extract(
    scene: &SceneView<'_>,
    source: &[u8],
) -> Result<(Vec<OwnedShape>, Vec<Disclosure>), ComposeError> {
    let mut shapes = Vec::new();
    let mut disclosures = Vec::new();
    for (scene_index, shape) in scene.shapes.iter().enumerate() {
        let geometry = || ComposeError::Malformed {
            detail: "shape geometry",
            offset: 0,
        };
        let range = shape.edges.first
            ..shape
                .edges
                .first
                .checked_add(shape.edges.count)
                .ok_or_else(geometry)?;
        let edges: Vec<FlatEdge> = scene
            .edges
            .get(range.clone())
            .ok_or_else(geometry)?
            .iter()
            .map(|e| FlatEdge::from(*e))
            .collect();
        let fill_rgba = scene
            .edge_colors
            .get(range.start)
            .copied()
            .filter(|_| !edges.is_empty());
        let fill_rgba = fill_rgba.unwrap_or([0; 4]);
        let sub_end = shape
            .subpaths
            .first
            .checked_add(shape.subpaths.count)
            .ok_or_else(geometry)?;
        let subs = scene
            .path_subpaths
            .get(shape.subpaths.first..sub_end)
            .ok_or_else(geometry)?;
        // Scene subpaths index the scene-wide edge table; the renderer wants
        // ranges relative to this shape's own edges.
        let subpaths = subs
            .iter()
            .map(|s| {
                let first = s.first_edge.checked_sub(range.start)?;
                Some(first..first.checked_add(s.edge_count)?)
            })
            .collect::<Option<Vec<Range<usize>>>>()
            .ok_or_else(geometry)?;
        let subpath_closed: Vec<bool> = subs.iter().map(|s| s.closed).collect();
        let offset = shape.source.element.start;
        let disclose = |kind| Disclosure {
            feature: feature_name(kind),
            offset: Some(offset),
            element: element_at(source, offset),
        };
        let converted = from_scene_shape(
            shape,
            fill_rgba,
            scene.fill_paints,
            scene.stroke_paints,
            &edges,
            &subpaths,
            &subpath_closed,
        );
        let render = match converted {
            Ok(render) => render,
            Err(RenderError::Unsupported(kind)) => {
                disclosures.push(disclose(kind));
                continue;
            }
            Err(_) => return Err(geometry()),
        };
        if render.stroke.is_some_and(|s| s.crisp_edges) {
            disclosures.push(disclose(vexel_render::UnsupportedKind::CrispEdges));
            continue;
        }
        shapes.push(OwnedShape {
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
        });
    }
    Ok((shapes, disclosures))
}
