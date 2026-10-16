use crate::{Diagnostic, DiagnosticKind, MAX_COORDINATE, failure};
use vexel_base::{ControlledError, WorkKind};
use vexel_geometry::{Affine, Edge, Point};

#[derive(Debug)]
pub(crate) enum ArcPlan {
    Empty,
    Line,
    Samples(ArcCursor),
}

#[derive(Debug)]
pub(crate) struct ArcCursor {
    rx: f64,
    ry: f64,
    cp: f64,
    sp: f64,
    cx: f64,
    cy: f64,
    start: f64,
    delta: f64,
    steps: usize,
    index: usize,
    previous: Point,
    to: Point,
    pending: Option<Point>,
}
impl ArcCursor {
    /// Constant-size endpoint-to-center setup, followed by the original
    /// upfront sample-cap check and fallible reservation. Samples themselves
    /// are generated only by advance(), one at a time.
    pub fn prepare(
        edges: &mut Vec<Edge>,
        from: Point,
        rx: f64,
        ry: f64,
        rotation: f64,
        large: bool,
        sweep: bool,
        to: Point,
        tolerance: f64,
        max_edges: usize,
        offset: usize,
    ) -> Result<ArcPlan, ControlledError<Diagnostic>> {
        let mut rx = rx.abs();
        let mut ry = ry.abs();
        if rx == 0.0 || ry == 0.0 {
            return Ok(ArcPlan::Line);
        }
        if from == to {
            return Ok(ArcPlan::Empty);
        }
        let phi = rotation * 0.017453292519943295;
        let (cp, sp) = (phi.cos(), phi.sin());
        let (dx, dy) = ((from.x - to.x) * 0.5, (from.y - to.y) * 0.5);
        let (xp, yp) = (cp * dx + sp * dy, -sp * dx + cp * dy);
        let scale = xp * xp / (rx * rx) + yp * yp / (ry * ry);
        if !scale.is_finite() {
            return Err(ControlledError::Original(failure(
                DiagnosticKind::Number,
                offset,
            )));
        }
        if scale > 1.0 {
            let factor = scale.sqrt();
            rx *= factor;
            ry *= factor;
        }
        let numerator = rx * rx * ry * ry - rx * rx * yp * yp - ry * ry * xp * xp;
        let denominator = rx * rx * yp * yp + ry * ry * xp * xp;
        let ratio = if denominator == 0.0 {
            0.0
        } else {
            (numerator / denominator).max(0.0)
        };
        let sign = if large == sweep { -1.0 } else { 1.0 };
        let coefficient = sign * ratio.sqrt();
        let cxp = coefficient * (rx * yp / ry);
        let cyp = coefficient * (-ry * xp / rx);
        let cx = cp * cxp - sp * cyp + (from.x + to.x) * 0.5;
        let cy = sp * cxp + cp * cyp + (from.y + to.y) * 0.5;
        let (ux, uy) = ((xp - cxp) / rx, (yp - cyp) / ry);
        let (vx, vy) = ((-xp - cxp) / rx, (-yp - cyp) / ry);
        let start = uy.atan2(ux);
        let mut delta = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
        if !sweep && delta > 0.0 {
            delta -= std::f64::consts::TAU;
        }
        if sweep && delta < 0.0 {
            delta += std::f64::consts::TAU;
        }
        let radius = rx.max(ry);
        let max_angle = if radius <= tolerance {
            std::f64::consts::TAU
        } else {
            2.0 * (1.0 - tolerance / radius).max(-1.0).acos()
        };
        let raw = (delta.abs() / max_angle).ceil();
        if !raw.is_finite() || raw > MAX_COORDINATE * 4.0 || raw > max_edges as f64 {
            return Err(ControlledError::Original(failure(
                DiagnosticKind::Limit,
                offset,
            )));
        }
        let steps = (raw as usize).max(1);
        if steps > max_edges.saturating_sub(edges.len()) {
            return Err(ControlledError::Original(failure(
                DiagnosticKind::Limit,
                offset,
            )));
        }
        crate::control::reserve(edges, steps, max_edges, offset)?;
        Ok(ArcPlan::Samples(Self {
            rx,
            ry,
            cp,
            sp,
            cx,
            cy,
            start,
            delta,
            steps,
            index: 1,
            previous: from,
            to,
            pending: None,
        }))
    }
    pub fn kind(&self) -> WorkKind {
        if self.pending.is_some() {
            WorkKind::Edge
        } else {
            WorkKind::Expansion
        }
    }
    pub fn advance(
        &mut self,
        edges: &mut Vec<Edge>,
        transform: Affine,
        max_edges: usize,
        offset: usize,
    ) -> Result<bool, ControlledError<Diagnostic>> {
        if let Some(point) = self.pending {
            crate::flatten::edge(edges, transform, self.previous, point, max_edges, offset)?;
            self.previous = point;
            self.pending = None;
            if self.index == self.steps {
                return Ok(true);
            }
            self.index = self.index.checked_add(1).ok_or(ControlledError::Overflow)?;
        } else {
            let angle = self.start + self.delta * self.index as f64 / self.steps as f64;
            let mut point = Point {
                x: self.cx + self.cp * self.rx * angle.cos() - self.sp * self.ry * angle.sin(),
                y: self.cy + self.sp * self.rx * angle.cos() + self.cp * self.ry * angle.sin(),
            };
            if self.index == self.steps {
                point = self.to;
            }
            if !crate::flatten::bounded(point) {
                return Err(ControlledError::Original(failure(
                    DiagnosticKind::Number,
                    offset,
                )));
            }
            self.pending = Some(point);
        }
        Ok(false)
    }
}
