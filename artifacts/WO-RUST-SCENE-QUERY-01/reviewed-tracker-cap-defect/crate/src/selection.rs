use vexel_base::{Span, Stop, WorkControl, WorkKind};
use vexel_geometry::Point;
use vexel_scene::{
    Diagnostic, ErrorKind, KernelResources, LimitKind, PaintKind, Rect, Result, SceneView,
    Selection, ShapeId, ShapeInput,
};

fn invalid(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset,
    }
}
fn stopped(stop: Stop) -> Diagnostic {
    Diagnostic {
        kind: match stop {
            Stop::Canceled => ErrorKind::Canceled,
            Stop::Limit => ErrorKind::Limit(LimitKind::Work),
            Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
        },
        offset: 0,
    }
}
struct Unlimited;
impl WorkControl for Unlimited {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}

fn mapping(view: SceneView<'_>) -> Option<(f64, f64, f64, f64)> {
    let v = view.viewport;
    if v.width == 0
        || v.height == 0
        || ![v.view_x, v.view_y, v.view_width, v.view_height]
            .into_iter()
            .all(f64::is_finite)
        || v.view_width <= 0.
        || v.view_height <= 0.
    {
        return None;
    }
    let (mut sx, mut sy) = (
        f64::from(v.width) / v.view_width,
        f64::from(v.height) / v.view_height,
    );
    let (mut ox, mut oy) = (0., 0.);
    let mut a = v.aspect;
    if !a.set {
        a.align_x = 0.5;
        a.align_y = 0.5;
    }
    if !a.none {
        let s = if a.slice { sx.max(sy) } else { sx.min(sy) };
        sx = s;
        sy = s;
        ox = (f64::from(v.width) - v.view_width * s) * a.align_x;
        oy = (f64::from(v.height) - v.view_height * s) * a.align_y;
    }
    [sx, sy, ox, oy]
        .into_iter()
        .all(f64::is_finite)
        .then_some((sx, sy, ox, oy))
}

fn bounds_controlled(
    view: SceneView<'_>,
    id: ShapeId,
    control: &mut dyn WorkControl,
) -> Result<Option<Selection>> {
    let Some(shape) = view.shape(id) else {
        return Ok(None);
    };
    let Some(edges) = shape.edges.slice(view.edges) else {
        return Ok(None);
    };
    let (min_x, min_y, max_x, max_y) = if let Some(first) = edges.first() {
        let mut b = (first.a.x, first.a.y, first.a.x, first.a.y);
        for edge in edges {
            control.charge(WorkKind::Edge, 1).map_err(stopped)?;
            for p in [edge.a, edge.b] {
                if !p.x.is_finite() || !p.y.is_finite() {
                    return Ok(None);
                }
                b.0 = b.0.min(p.x);
                b.1 = b.1.min(p.y);
                b.2 = b.2.max(p.x);
                b.3 = b.3.max(p.y);
            }
        }
        b
    } else {
        (0., 0., 0., 0.)
    };
    let Some((sx, sy, ox, oy)) = mapping(view) else {
        return Ok(None);
    };
    let left = ox + (min_x - view.viewport.view_x) * sx;
    let top = oy + (min_y - view.viewport.view_y) * sy;
    let right = ox + (max_x - view.viewport.view_x) * sx;
    let bottom = oy + (max_y - view.viewport.view_y) * sy;
    if ![left, top, right, bottom].into_iter().all(f64::is_finite) {
        return Ok(None);
    }
    Ok(Some(Selection {
        shape: id,
        source: shape.source.element,
        user_bounds: Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        },
        device_bounds: Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        },
        transform: shape.source.transform,
    }))
}

/// Safe bounds query. This convenience call uses an unlimited local controller.
pub fn bounds_query(view: SceneView<'_>, shape: ShapeId) -> Option<Selection> {
    bounds_controlled(view, shape, &mut Unlimited)
        .ok()
        .flatten()
}
/// Bounds query that charges each visited edge and observes cancellation.
pub fn bounds_query_controlled(
    view: SceneView<'_>,
    shape: ShapeId,
    control: &mut dyn WorkControl,
) -> Result<Option<Selection>> {
    bounds_controlled(view, shape, control)
}

fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let length2 = dx * dx + dy * dy;
    if length2 == 0. {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / length2).clamp(0., 1.);
    (p.x - (a.x + t * dx)).hypot(p.y - (a.y + t * dy))
}

fn fill_contains(
    view: SceneView<'_>,
    id: ShapeId,
    x: f64,
    y: f64,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(shape) = view.shape(id) else {
        return Ok(false);
    };
    let Some(edges) = shape.edges.slice(view.edges) else {
        return Ok(false);
    };
    let mut winding = 0i32;
    let mut parity = false;
    let mut visible = view
        .fill_paint(id)
        .is_some_and(|paint| paint.kind == PaintKind::Image);
    for (i, edge) in edges.iter().enumerate() {
        control.charge(WorkKind::FillEdgeTest, 1).map_err(stopped)?;
        visible |= view
            .edge_colors
            .get(shape.edges.first + i)
            .is_some_and(|c| c[3] != 0);
        let up = edge.a.y <= y && edge.b.y > y;
        let down = edge.b.y <= y && edge.a.y > y;
        if !up && !down {
            continue;
        }
        let crossing = edge.a.x + (y - edge.a.y) * (edge.b.x - edge.a.x) / (edge.b.y - edge.a.y);
        if crossing > x {
            winding += if up { 1 } else { -1 };
            parity = !parity;
        }
    }
    Ok(visible && shape.filled && if shape.evenodd { parity } else { winding != 0 })
}

fn stroke_contains(
    view: SceneView<'_>,
    id: ShapeId,
    p: Point,
    tolerance: f64,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(shape) = view.shape(id) else {
        return Ok(false);
    };
    if shape.stroke.rgba[3] == 0 || shape.stroke.width <= 0. {
        return Ok(false);
    }
    let inv = shape.stroke_inverse;
    let Some(local) = vexel_geometry::affine_map_point(inv, p) else {
        return Ok(false);
    };
    let norm = (inv.a * inv.a + inv.b * inv.b + inv.c * inv.c + inv.d * inv.d).sqrt();
    let radius = shape.stroke.width * 0.5 + tolerance * norm;
    let Some(subpaths) = shape.subpaths.slice(view.path_subpaths) else {
        return Ok(false);
    };
    let Some(edges) = shape.edges.slice(view.edges) else {
        return Ok(false);
    };
    if !subpaths.is_empty() {
        for path in subpaths {
            let Some(end) = path.first_edge.checked_add(path.edge_count) else {
                return Ok(false);
            };
            let Some(path_edges) = view.edges.get(path.first_edge..end) else {
                return Ok(false);
            };
            let count = path_edges.len().saturating_sub(usize::from(!path.closed));
            for edge in &path_edges[..count] {
                control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                let (Some(a), Some(b)) = (
                    vexel_geometry::affine_map_point(inv, edge.a),
                    vexel_geometry::affine_map_point(inv, edge.b),
                ) else {
                    return Ok(false);
                };
                if segment_distance(local, a, b) <= radius {
                    return Ok(true);
                }
            }
        }
        return Ok(false);
    }
    let count = edges
        .len()
        .saturating_sub(usize::from(!shape.stroke_closed));
    for edge in &edges[..count] {
        control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
        let (Some(a), Some(b)) = (
            vexel_geometry::affine_map_point(inv, edge.a),
            vexel_geometry::affine_map_point(inv, edge.b),
        ) else {
            return Ok(false);
        };
        if segment_distance(local, a, b) <= radius {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Reverse-painter selection using the caller's concrete clip provider and work controller.
pub fn hit_test(
    view: SceneView<'_>,
    x: f64,
    y: f64,
    tolerance: f64,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<Option<Selection>> {
    if ![x, y, tolerance].into_iter().all(f64::is_finite)
        || tolerance < 0.
        || tolerance > 1_000_000.
    {
        return Err(invalid(0));
    }
    for i in (0..view.shapes.len()).rev() {
        control.checkpoint().map_err(stopped)?;
        let Some(id) = view.shape_id(i) else { continue };
        let shape = view.shape(id).ok_or_else(|| invalid(0))?;
        if let Some(clip) = shape.clip {
            if !resources.clip_contains(view, clip, x, y, control)? {
                continue;
            }
        }
        if !fill_contains(view, id, x, y, control)?
            && !stroke_contains(view, id, Point { x, y }, tolerance, control)?
        {
            continue;
        }
        return bounds_controlled(view, id, control);
    }
    Ok(None)
}

/// Returns the narrowest half-open source span; equal ranges prefer later painter order.
pub fn element_at_offset(view: SceneView<'_>, offset: usize) -> Option<Selection> {
    element_at_offset_controlled(view, offset, &mut Unlimited)
        .ok()
        .flatten()
}
/// Controlled source lookup; each source record is charged before inspection.
pub fn element_at_offset_controlled(
    view: SceneView<'_>,
    offset: usize,
    control: &mut dyn WorkControl,
) -> Result<Option<Selection>> {
    let mut best: Option<(usize, usize)> = None;
    for (i, shape) in view.shapes.iter().enumerate() {
        control.charge(WorkKind::SourceRead, 1).map_err(stopped)?;
        let span = shape.source.element;
        if span.start > offset || offset >= span.end || span.start >= span.end {
            continue;
        }
        let length = span.end - span.start;
        if best.is_none_or(|(best_i, best_len)| {
            length < best_len || (length == best_len && i > best_i)
        }) {
            best = Some((i, length));
        }
    }
    match best {
        Some((i, _)) => match view.shape_id(i) {
            Some(id) => bounds_controlled(view, id, control),
            None => Ok(None),
        },
        None => Ok(None),
    }
}

/// Sets source ownership before append; published scene spans remain immutable.
pub fn source_assign(input: &mut ShapeInput, source: &[u8], span: Span) -> Result<()> {
    if span.slice(source).is_none() {
        return Err(invalid(span.start));
    }
    input.shape.source.element = span;
    Ok(())
}
