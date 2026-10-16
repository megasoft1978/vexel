use crate::{stopped, DASH_EPSILON};
use vexel_geometry::Edge;
use vexel_scene::{Result, SceneView, ShapeId, WorkControl, WorkKind};

pub fn dash_range_joins_seam(
    stroke: &vexel_scene::Stroke,
    dashes: &[f64],
    edges: &[Edge],
    first: usize,
    count: usize,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    if dashes.is_empty()
        || dashes.len() > 16
        || count == 0
        || dashes.iter().any(|d| !d.is_finite() || *d <= 0.0)
    {
        return Ok(false);
    }
    let exp = dashes.len() * (if dashes.len() % 2 == 1 { 2 } else { 1 });
    let mut period = 0.0;
    for i in 0..exp {
        control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
        period += dashes[i % dashes.len()]
    }
    if !period.is_finite() || period <= 0.0 {
        return Ok(false);
    }
    let mut phase = stroke.dash_offset % period;
    if !phase.is_finite() {
        return Ok(false);
    }
    if phase < 0.0 {
        phase += period
    }
    let mut pat = 0;
    while phase >= dashes[pat % dashes.len()] {
        phase -= dashes[pat % dashes.len()];
        pat = (pat + 1) % exp
    }
    let starts = pat % 2 == 0;
    let mut remain = dashes[pat % dashes.len()] - phase;
    if !remain.is_finite() || remain <= 0.0 {
        return Ok(false);
    }
    let mut ends = false;
    for i in 0..count {
        control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
        let Some(e) = edges.get(first + i) else {
            return Ok(false);
        };
        let mut len = (e.b.x - e.a.x).hypot(e.b.y - e.a.y);
        while len > 0.0 {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            let take = remain.min(len);
            ends = pat % 2 == 0;
            len -= take;
            remain -= take;
            if remain <= DASH_EPSILON {
                pat = (pat + 1) % exp;
                remain = dashes[pat % dashes.len()]
            }
        }
    }
    Ok(starts && ends)
}
pub fn subpath_closed_seam(
    view: SceneView<'_>,
    id: ShapeId,
    subpath: usize,
    edges: &[Edge],
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(shape) = view.shape(id) else {
        return Ok(false);
    };
    let Some(path) = shape
        .subpaths
        .slice(view.path_subpaths)
        .and_then(|v| v.get(subpath))
    else {
        return Ok(false);
    };
    let Some(dashes) = shape.stroke.dash_values.slice(view.dash_values) else {
        return Ok(false);
    };
    if !path.closed {
        return Ok(false);
    }
    dash_range_joins_seam(
        &shape.stroke,
        dashes,
        edges,
        path.first_edge,
        path.edge_count,
        control,
    )
}
pub fn shape_closed_seam(
    view: SceneView<'_>,
    id: ShapeId,
    edges: &[Edge],
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(shape) = view.shape(id) else {
        return Ok(false);
    };
    let Some(dashes) = shape.stroke.dash_values.slice(view.dash_values) else {
        return Ok(false);
    };
    if !shape
        .subpaths
        .slice(view.path_subpaths)
        .unwrap_or(&[])
        .is_empty()
    {
        for (i, subpath) in shape
            .subpaths
            .slice(view.path_subpaths)
            .unwrap()
            .iter()
            .enumerate()
        {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            if subpath.closed && subpath_closed_seam(view, id, i, edges, control)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    if !shape.stroke_closed {
        return Ok(false);
    }
    dash_range_joins_seam(
        &shape.stroke,
        dashes,
        edges,
        shape.edges.first,
        shape.edges.count,
        control,
    )
}

