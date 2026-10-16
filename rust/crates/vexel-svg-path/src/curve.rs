use crate::{failure, Diagnostic, DiagnosticKind};
use vexel_base::{ControlledError, WorkKind};
use vexel_geometry::{Affine, Edge, Point};

#[derive(Debug, Clone, Copy)]
struct Node { points: [Point; 4], depth: u8 }

/// The maximum live stack is seventeen nodes at the frozen depth-sixteen
/// limit. Inline storage replaces recursion without a new allocation or cap.
#[derive(Debug)]
pub(crate) struct CurveCursor {
    stack: [Node; 17],
    len: usize,
    pending_edge: Option<[Point; 2]>,
}
impl CurveCursor {
    pub fn new(points: [Point; 4]) -> Self {
        let mut stack = [Node { points: [Point { x: 0.0, y: 0.0 }; 4], depth: 0 }; 17];
        stack[0].points = points;
        Self { stack, len: 1, pending_edge: None }
    }
    pub fn kind(&self) -> WorkKind {
        if self.pending_edge.is_some() { WorkKind::Edge } else { WorkKind::Expansion }
    }
    pub fn advance(&mut self, out: &mut Vec<Edge>, transform: Affine, flatness: f64, max_edges: usize, offset: usize)
        -> Result<bool, ControlledError<Diagnostic>> {
        if let Some([from, to]) = self.pending_edge {
            crate::flatten::edge(out, transform, from, to, max_edges, offset)?;
            self.pending_edge = None;
            return Ok(self.len == 0);
        }
        self.len = self.len.checked_sub(1).ok_or(ControlledError::Overflow)?;
        let Node { points: p, depth } = self.stack[self.len];
        if point_segment_distance(p[1], p[0], p[3]).max(point_segment_distance(p[2], p[0], p[3])) <= flatness {
            self.pending_edge = Some([p[0], p[3]]);
            return Ok(false);
        }
        if depth >= 16 { return Err(ControlledError::Original(failure(DiagnosticKind::Limit, offset))); }
        let a = midpoint(p[0], p[1]);
        let c = midpoint(p[1], p[2]);
        let d = midpoint(p[2], p[3]);
        let e = midpoint(a, c);
        let f = midpoint(c, d);
        let middle = midpoint(e, f);
        let depth = depth.checked_add(1).ok_or(ControlledError::Overflow)?;
        // Right first on the stack, left first in traversal. Keep each original
        // midpoint operation and depth test in the accepted order.
        self.push(Node { points: [middle, f, d, p[3]], depth })?;
        self.push(Node { points: [p[0], a, e, middle], depth })?;
        Ok(false)
    }
    fn push(&mut self, node: Node) -> Result<(), ControlledError<Diagnostic>> {
        *self.stack.get_mut(self.len).ok_or(ControlledError::Overflow)? = node;
        self.len = self.len.checked_add(1).ok_or(ControlledError::Overflow)?;
        Ok(())
    }
}

fn midpoint(a: Point, b: Point) -> Point { Point { x: (a.x + b.x) * 0.5, y: (a.y + b.y) * 0.5 } }
fn point_segment_distance(point: Point, from: Point, to: Point) -> f64 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared == 0.0 { return (point.x - from.x).hypot(point.y - from.y); }
    let projection = (((point.x - from.x) * dx + (point.y - from.y) * dy) / length_squared).clamp(0.0, 1.0);
    let nearest = Point { x: from.x + projection * dx, y: from.y + projection * dy };
    (point.x - nearest.x).hypot(point.y - nearest.y)
}
