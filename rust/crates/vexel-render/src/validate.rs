//! Input validation and work bounds, mirroring the entry checks of
//! `raster_shape_render_flat` and the dash-cache limits.

use crate::dash::{Budget, Pattern};
use crate::stroke::{self, JOIN_BYTES, RANGE_BYTES, SEGMENT_BYTES};
use crate::{
    MAX_COORDINATE, MAX_EDGES, MAX_SHAPES, MAX_STROKE_BYTES, MAX_STROKE_STEPS, MAX_SUBPATHS,
    RenderError, RenderShape, UnsupportedKind, ViewMap,
};
use vexel_geometry::affine_valid;
use vexel_raster::Tile;

fn add(left: usize, right: usize) -> Result<usize, RenderError> {
    left.checked_add(right).ok_or(RenderError::LimitExceeded)
}

fn mul(left: usize, right: usize) -> Result<usize, RenderError> {
    left.checked_mul(right).ok_or(RenderError::LimitExceeded)
}

fn validate_shape(shape: &RenderShape<'_>) -> Result<(), RenderError> {
    if shape.subpaths.len() != shape.subpath_closed.len() {
        return Err(RenderError::InvalidGeometry);
    }
    if shape.subpaths.iter().any(|r| r.start > r.end || r.end > shape.edges.len()) {
        return Err(RenderError::InvalidGeometry);
    }
    let finite = |v: f64| v.is_finite() && v.abs() <= MAX_COORDINATE;
    if shape.edges.iter().any(|e| !(finite(e.x0) && finite(e.y0) && finite(e.x1) && finite(e.y1))) {
        return Err(RenderError::InvalidGeometry);
    }
    let Some(stroke) = shape.stroke else {
        return Ok(());
    };
    if stroke.crisp_edges {
        return Err(RenderError::Unsupported(UnsupportedKind::CrispEdges));
    }
    if !stroke.width.is_finite()
        || stroke.width < 0.0
        || !stroke.miter_limit.is_finite()
        || stroke.miter_limit < 1.0
        || !stroke.dash_offset.is_finite()
    {
        return Err(RenderError::InvalidGeometry);
    }
    if stroke::is_visible(&stroke) && !shape.edges.is_empty() {
        if !affine_valid(shape.transform) || stroke::invert(shape.transform).is_none() {
            return Err(RenderError::InvalidGeometry);
        }
    }
    Ok(())
}

/// Validates everything before any pixel is written. `patterns` is empty or
/// parallel to `shapes`; dashed shapes are charged while their primitives are
/// built, so only solid shapes are counted here. Returns the budget spent.
pub(crate) fn validate(
    shapes: &[RenderShape<'_>],
    patterns: &[Option<Pattern>],
    view: &ViewMap,
    tile: &Tile,
) -> Result<Budget, RenderError> {
    if tile.width() != view.width() || tile.height() != view.height() {
        return Err(RenderError::InvalidGeometry);
    }
    if shapes.len() > MAX_SHAPES {
        return Err(RenderError::LimitExceeded);
    }
    let (mut edges, mut subpaths, mut steps, mut bytes) = (0usize, 0usize, 0usize, 0usize);
    for (index, shape) in shapes.iter().enumerate() {
        validate_shape(shape)?;
        edges = add(edges, shape.edges.len())?;
        subpaths = add(subpaths, shape.subpaths.len())?;
        if edges > MAX_EDGES || subpaths > MAX_SUBPATHS {
            return Err(RenderError::LimitExceeded);
        }
        let dashed = patterns.get(index).is_some_and(Option::is_some);
        if !dashed && shape.stroke.as_ref().is_some_and(stroke::is_visible) {
            for index in 0..stroke::range_count(shape) {
                let (segments, joins) = stroke::range_at(shape, index).primitive_counts();
                steps = add(steps, add(segments, joins)?)?;
                bytes = add(bytes, add(mul(segments, SEGMENT_BYTES)?, mul(joins, JOIN_BYTES)?)?)?;
            }
        }
    }
    bytes = add(bytes, mul(shapes.len(), RANGE_BYTES)?)?;
    if steps > MAX_STROKE_STEPS || bytes > MAX_STROKE_BYTES {
        return Err(RenderError::LimitExceeded);
    }
    if tile.rgba().iter().any(|byte| *byte != 0) {
        return Err(RenderError::TileNotEmpty);
    }
    Ok(Budget { steps, bytes })
}

#[cfg(test)]
mod limit03_tests {
    use super::validate;
    use crate::{
        FlatEdge, Linecap, Linejoin, RenderError, RenderShape, StrokeStyle, ViewBox, ViewMap,
    };
    use vexel_geometry::affine_identity;
    use vexel_raster::Tile;

    /// Steps `validate` charges for one closed solid ring of `n` identical edges.
    fn ring_steps(n: usize) -> Result<usize, RenderError> {
        let unit = FlatEdge {
            x0: 1.0,
            y0: 1.0,
            x1: 2.0,
            y1: 1.0,
        };
        let edges = vec![unit; n];
        let stroke = StrokeStyle {
            rgba: [0, 0, 0, 255],
            width: 1.0,
            miter_limit: 4.0,
            dash_offset: 0.0,
            cap: Linecap::Butt,
            join: Linejoin::Miter,
            crisp_edges: false,
        };
        let shape = RenderShape {
            edges: &edges,
            subpaths: &[],
            subpath_closed: &[],
            stroke_closed: true,
            evenodd: false,
            filled: false,
            fill_rgba: [0; 4],
            stroke: Some(stroke),
            transform: affine_identity(),
        };
        let vb = ViewBox {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 4.0,
        };
        let view = ViewMap::from_viewbox(vb, 4, 4, None).unwrap();
        let tile = Tile::from_owned_rgba(4, 4, vec![0; 64]).unwrap();
        validate(&[shape], &[], &view, &tile).map(|budget| budget.steps)
    }

    #[test]
    fn limit03_solid_ring_steps_admit_4m_and_refuse_the_next() {
        // A closed ring of E edges costs E segments plus E joins: 2E steps.
        assert_eq!(ring_steps(2_000_000), Ok(4_000_000));
        assert_eq!(ring_steps(2_000_001), Err(RenderError::LimitExceeded));
    }
}
