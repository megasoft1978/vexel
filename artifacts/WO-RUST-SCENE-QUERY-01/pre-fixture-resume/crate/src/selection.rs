use vexel_base::Span;
use vexel_geometry::{Affine, Point};
use vexel_scene::{
    AllocationTag, ChargeLedger, Diagnostic, ErrorKind, KernelResources, LimitKind, Rect, Result,
    SceneBuilder, SceneView, Selection, ShapeId, ShapeInput, TableRange, WorkControl,
};

fn invalid() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset: 0,
    }
}

fn mapping(view: SceneView<'_>) -> Option<(f64, f64, f64, f64)> {
    let v = view.viewport;
    if v.width == 0
        || v.height == 0
        || ![v.view_x, v.view_y, v.view_width, v.view_height]
            .into_iter()
            .all(f64::is_finite)
        || v.view_width <= 0.0
        || v.view_height <= 0.0
    {
        return None;
    }
    let (mut sx, mut sy) = (
        f64::from(v.width) / v.view_width,
        f64::from(v.height) / v.view_height,
    );
    let (mut ox, mut oy) = (0.0, 0.0);
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

fn bounds(view: SceneView<'_>, id: ShapeId) -> Option<Selection> {
    let shape = view.shape(id)?;
    let edges = shape.edges.slice(view.edges)?;
    let (min_x, min_y, max_x, max_y) = if let Some(first) = edges.first() {
        let mut b = (first.a.x, first.a.y, first.a.x, first.a.y);
        for edge in edges {
            for p in [edge.a, edge.b] {
                if !p.x.is_finite() || !p.y.is_finite() {
                    return None;
                }
                b.0 = b.0.min(p.x);
                b.1 = b.1.min(p.y);
                b.2 = b.2.max(p.x);
                b.3 = b.3.max(p.y);
            }
        }
        b
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };
    let (sx, sy, ox, oy) = mapping(view)?;
    let left = ox + (min_x - view.viewport.view_x) * sx;
    let top = oy + (min_y - view.viewport.view_y) * sy;
    let right = ox + (max_x - view.viewport.view_x) * sx;
    let bottom = oy + (max_y - view.viewport.view_y) * sy;
    if ![left, top, right, bottom].into_iter().all(f64::is_finite) {
        return None;
    }
    Some(Selection {
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
    })
}

/// Safe equivalent of `svg_shape_bounds_query`; stale IDs and malformed ranges return None.
pub fn bounds_query(view: SceneView<'_>, shape: ShapeId) -> Option<Selection> {
    bounds(view, shape)
}

fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let l = dx * dx + dy * dy;
    if l == 0.0 {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / l).clamp(0.0, 1.0);
    (p.x - (a.x + t * dx)).hypot(p.y - (a.y + t * dy))
}

fn fill_contains(view: SceneView<'_>, id: ShapeId, x: f64, y: f64) -> bool {
    let Some(shape) = view.shape(id) else {
        return false;
    };
    let Some(edges) = shape.edges.slice(view.edges) else {
        return false;
    };
    let mut winding = 0i32;
    let mut parity = false;
    let visible = shape.image.is_some()
        || edges.iter().enumerate().any(|(i, _)| {
            view.edge_colors
                .get(shape.edges.first + i)
                .is_some_and(|c| c[3] != 0)
        });
    for edge in edges {
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
    visible && shape.filled && if shape.evenodd { parity } else { winding != 0 }
}

fn stroke_contains(view: SceneView<'_>, id: ShapeId, p: Point, tolerance: f64) -> bool {
    let Some(shape) = view.shape(id) else {
        return false;
    };
    if shape.stroke.rgba[3] == 0 || shape.stroke.width <= 0.0 {
        return false;
    }
    let inv = shape.stroke_inverse;
    let Some(local) = vexel_geometry::affine_map_point(inv, p) else {
        return false;
    };
    let norm = (inv.a * inv.a + inv.b * inv.b + inv.c * inv.c + inv.d * inv.d).sqrt();
    let radius = shape.stroke.width * 0.5 + tolerance * norm;
    let Some(subpaths) = shape.subpaths.slice(view.path_subpaths) else {
        return false;
    };
    let Some(edges) = shape.edges.slice(view.edges) else {
        return false;
    };
    if !subpaths.is_empty() {
        for path in subpaths {
            let Some(end) = path.first_edge.checked_add(path.edge_count) else {
                return false;
            };
            let Some(path_edges) = view.edges.get(path.first_edge..end) else {
                return false;
            };
            let n = path_edges.len().saturating_sub(usize::from(!path.closed));
            for edge in &path_edges[..n] {
                let (Some(a), Some(b)) = (
                    vexel_geometry::affine_map_point(inv, edge.a),
                    vexel_geometry::affine_map_point(inv, edge.b),
                ) else {
                    return false;
                };
                if segment_distance(local, a, b) <= radius {
                    return true;
                }
            }
        }
        return false;
    }
    let n = edges
        .len()
        .saturating_sub(usize::from(!shape.stroke_closed));
    edges[..n].iter().any(|e| {
        match (
            vexel_geometry::affine_map_point(inv, e.a),
            vexel_geometry::affine_map_point(inv, e.b),
        ) {
            (Some(a), Some(b)) => segment_distance(local, a, b) <= radius,
            _ => false,
        }
    })
}

/// Reverse painter-order selection with the concrete clip provider and caller work control.
pub fn hit_test(
    view: SceneView<'_>,
    x: f64,
    y: f64,
    tolerance: f64,
    resources: &dyn KernelResources,
    control: &mut dyn WorkControl,
) -> Result<Option<Selection>> {
    if ![x, y, tolerance].into_iter().all(f64::is_finite)
        || tolerance < 0.0
        || tolerance > 1_000_000.0
    {
        return Err(invalid());
    }
    for i in (0..view.shapes.len()).rev() {
        control.checkpoint().map_err(|s| Diagnostic {
            kind: match s {
                vexel_base::Stop::Canceled => ErrorKind::Canceled,
                vexel_base::Stop::Limit => ErrorKind::Limit(LimitKind::Work),
                vexel_base::Stop::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
            },
            offset: 0,
        })?;
        let Some(shape_data) = view.shapes.get(i) else {
            continue;
        };
        let Some(id) = shape_data
            .edges
            .count
            .checked_sub(1)
            .and_then(|last| shape_data.edges.first.checked_add(last))
            .and_then(|edge| view.edge_shape(edge))
        else {
            continue;
        };
        if id.index() != i {
            continue;
        }
        let shape = view.shape(id).ok_or_else(invalid)?;
        if let Some(clip) = shape.clip {
            if !resources.clip_contains(view, clip, x, y, control)? {
                continue;
            }
        }
        if !fill_contains(view, id, x, y) && !stroke_contains(view, id, Point { x, y }, tolerance) {
            continue;
        }
        return Ok(bounds(view, id));
    }
    Ok(None)
}

/// Returns the narrowest source range; ties select the later painter-order shape.
pub fn element_at_offset(view: SceneView<'_>, offset: usize) -> Option<Selection> {
    view.shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let span = s.source.element;
            (span.start <= offset && offset < span.end && span.start < span.end)
                .then_some((i, span))
        })
        .min_by(|(ia, a), (ib, b)| {
            a.end
                .saturating_sub(a.start)
                .cmp(&b.end.saturating_sub(b.start))
                .then_with(|| ib.cmp(ia))
        })
        .and_then(|(i, _)| {
            view.edge_shape(view.shapes[i].edges.first)
                .filter(|id| id.index() == i)
                .and_then(|id| bounds(view, id))
        })
}

/// Assigns a source span before `append_shape`; published scene spans stay immutable.
pub fn source_assign(input: &mut ShapeInput, source: &[u8], span: Span) -> Result<()> {
    if span.slice(source).is_none() {
        return Err(invalid());
    }
    input.shape.source.element = span;
    Ok(())
}

/// Range form retained for API discoverability: validates a prospective builder range only.
/// Actual assignment is append-time through `source_assign` because SceneBuilder publishes immutable shapes.
pub fn validate_source_range(
    builder: &SceneBuilder,
    scene: vexel_scene::SceneId,
    shapes: TableRange,
    source: Span,
) -> Result<()> {
    if source.slice(builder.source_owner()).is_none() {
        return Err(invalid());
    }
    let view = builder.scene(scene)?;
    if shapes.end().is_none_or(|end| end > view.shapes.len()) {
        return Err(invalid());
    }
    Ok(())
}

#[allow(dead_code)]
fn _accepted_contract(_: &mut ChargeLedger, _: AllocationTag, _: Affine) {}
