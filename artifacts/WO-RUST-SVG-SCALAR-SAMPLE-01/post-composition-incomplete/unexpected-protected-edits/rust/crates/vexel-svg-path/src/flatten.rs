use crate::arc::{ArcCursor, ArcPlan};
use crate::control::{NoStop, legacy_error};
use crate::curve::CurveCursor;
use crate::{Diagnostic, DiagnosticKind, MAX_COORDINATE, failure};
use vexel_base::{ControlledError, WorkControl, WorkKind};
use vexel_geometry::{Affine, Edge, Point, affine_map_point};

pub(crate) fn tolerance(transform: Affine) -> f64 {
    let row_norm =
        (transform.a.abs() + transform.c.abs()).max(transform.b.abs() + transform.d.abs());
    let column_norm =
        (transform.a.abs() + transform.b.abs()).max(transform.c.abs() + transform.d.abs());
    let mut scale = (row_norm * column_norm).sqrt();
    if !scale.is_finite() || scale < 1.0 {
        scale = 1.0;
    }
    0.25 / scale
}

/// Returns the accepted flattening tolerance in transformed coordinates.
pub fn transformed_tolerance(transform: Affine) -> f64 {
    tolerance(transform)
}

/// Appends one transformed line edge using the parser's bounded edge engine.
pub fn append_edge(
    from: Point,
    to: Point,
    transform: Affine,
    max_edges: usize,
    source_offset: usize,
    out: &mut Vec<Edge>,
) -> Result<(), Diagnostic> {
    validate_wrapper_inputs(transform, &[from, to], source_offset)?;
    edge(
        out,
        transform,
        from,
        to,
        max_edges.min(crate::MAX_PATH_EDGES),
        source_offset,
    )
    .map_err(|error| legacy_error(error, source_offset))
}

/// Flattens a cubic into transformed edges using the parser's accepted curve engine.
///
/// Invalid transforms or points return before `out` is changed. Once curve
/// traversal starts, it may append edges before a later `Limit` or `OutOfMemory`
/// error; there is no automatic rollback. Callers that need rollback can record
/// `out.len()` first and truncate to that length after an error.
pub fn flatten_cubic(
    points: [Point; 4],
    transform: Affine,
    options: crate::ParseOptions,
    source_offset: usize,
    out: &mut Vec<Edge>,
) -> Result<(), Diagnostic> {
    flatten_cubic_controlled(points, transform, options, source_offset, out, &mut NoStop)
        .map_err(|error| legacy_error(error, source_offset))
}

/// The same cubic traversal with caller cancellation and work limits. Each
/// subdivision visit and each transformed append is charged separately in
/// batches of one. Validation precedes mutation. A stop or later geometry error
/// preserves earlier appends; callers may truncate to their recorded length.
pub fn flatten_cubic_controlled(
    points: [Point; 4],
    transform: Affine,
    options: crate::ParseOptions,
    source_offset: usize,
    out: &mut Vec<Edge>,
    control: &mut dyn WorkControl,
) -> Result<(), ControlledError<Diagnostic>> {
    validate_wrapper_inputs(transform, &points, source_offset)
        .map_err(ControlledError::Original)?;
    let mut cursor = GeometryCursor::Curve(CurveCursor::new(points));
    let flatness = tolerance(transform);
    let max_edges = options.max_edges.min(crate::MAX_PATH_EDGES);
    loop {
        control.checkpoint().map_err(ControlledError::from)?;
        control
            .charge(cursor.kind(), 1)
            .map_err(ControlledError::from)?;
        if cursor.advance(out, transform, flatness, max_edges, source_offset)? {
            return Ok(());
        }
    }
}

fn validate_wrapper_inputs(
    transform: Affine,
    points: &[Point],
    offset: usize,
) -> Result<(), Diagnostic> {
    if !vexel_geometry::affine_valid(transform) {
        return Err(failure(DiagnosticKind::InvalidTransform, offset));
    }
    if points.iter().any(|point| !bounded(*point)) {
        return Err(failure(DiagnosticKind::Number, offset));
    }
    Ok(())
}
pub(crate) fn bounded(point: Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x.abs() <= MAX_COORDINATE
        && point.y.abs() <= MAX_COORDINATE
}

pub(crate) fn edge(
    edges: &mut Vec<Edge>,
    transform: Affine,
    from: Point,
    to: Point,
    max_edges: usize,
    offset: usize,
) -> Result<(), ControlledError<Diagnostic>> {
    if edges.len() >= max_edges {
        return Err(ControlledError::Original(failure(
            DiagnosticKind::Limit,
            offset,
        )));
    }
    let a = affine_map_point(transform, from)
        .ok_or_else(|| ControlledError::Original(failure(DiagnosticKind::Number, offset)))?;
    let b = affine_map_point(transform, to)
        .ok_or_else(|| ControlledError::Original(failure(DiagnosticKind::Number, offset)))?;
    crate::control::reserve(edges, 1, max_edges, offset)?;
    edges.push(Edge { a, b });
    Ok(())
}

/// Owned pending traversal, shared by parser commands and public wrappers.
#[derive(Debug)]
pub(crate) enum GeometryCursor {
    Line(Point, Point),
    Curve(CurveCursor),
    Arc(ArcCursor),
}
impl GeometryCursor {
    pub fn kind(&self) -> WorkKind {
        match self {
            Self::Line(..) => WorkKind::Edge,
            Self::Curve(cursor) => cursor.kind(),
            Self::Arc(cursor) => cursor.kind(),
        }
    }
    pub fn advance(
        &mut self,
        edges: &mut Vec<Edge>,
        transform: Affine,
        flatness: f64,
        max_edges: usize,
        offset: usize,
    ) -> Result<bool, ControlledError<Diagnostic>> {
        match self {
            Self::Line(from, to) => {
                edge(edges, transform, *from, *to, max_edges, offset)?;
                Ok(true)
            }
            Self::Curve(cursor) => cursor.advance(edges, transform, flatness, max_edges, offset),
            Self::Arc(cursor) => cursor.advance(edges, transform, max_edges, offset),
        }
    }
    pub fn arc(
        edges: &mut Vec<Edge>,
        from: Point,
        values: [f64; 7],
        to: Point,
        flatness: f64,
        max_edges: usize,
        offset: usize,
    ) -> Result<Option<Self>, ControlledError<Diagnostic>> {
        Ok(
            match ArcCursor::prepare(
                edges,
                from,
                values[0],
                values[1],
                values[2],
                values[3] == 1.0,
                values[4] == 1.0,
                to,
                flatness,
                max_edges,
                offset,
            )? {
                ArcPlan::Empty => None,
                ArcPlan::Line => Some(Self::Line(from, to)),
                ArcPlan::Samples(cursor) => Some(Self::Arc(cursor)),
            },
        )
    }
}
