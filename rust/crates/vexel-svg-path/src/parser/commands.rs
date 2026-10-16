//! Bounded command evaluation. All long geometry traversal remains pending in
//! the parent session; this module only computes one group's control points.
use super::{AfterGeometry, PathParseSession, Phase};
use crate::curve::CurveCursor;
use crate::flatten::GeometryCursor;
use crate::{failure, Diagnostic, DiagnosticKind};
use vexel_base::ControlledError;
use vexel_geometry::Point;

impl PathParseSession<'_> {
    pub(super) fn apply_group(&mut self) -> Result<(), ControlledError<Diagnostic>> {
        let values = self.group.numbers.map(|number| number.value);
        let relative = self.group.relative;
        let mut point = self.current;
        match self.group.upper {
            b'H' => point.x = if relative { self.current.x + values[0] } else { values[0] },
            b'V' => point.y = if relative { self.current.y + values[0] } else { values[0] },
            b'C' => {
                let mut c1 = Point { x: values[0], y: values[1] };
                let mut c2 = Point { x: values[2], y: values[3] };
                point = Point { x: values[4], y: values[5] };
                if relative { c1 = add(c1, self.current); c2 = add(c2, self.current); point = add(point, self.current); }
                self.check_points(&[c1, c2, point])?;
                self.cubic_control = c2; self.cubic_reflect = true; self.quad_reflect = false;
                self.curve([self.current, c1, c2, point], point);
                return Ok(());
            }
            b'S' => {
                let c1 = if self.cubic_reflect { reflect(self.current, self.cubic_control) } else { self.current };
                let mut c2 = Point { x: values[0], y: values[1] };
                point = Point { x: values[2], y: values[3] };
                if relative { c2 = add(c2, self.current); point = add(point, self.current); }
                self.check_points(&[c1, c2, point])?;
                self.cubic_control = c2; self.cubic_reflect = true; self.quad_reflect = false;
                self.curve([self.current, c1, c2, point], point);
                return Ok(());
            }
            b'Q' => {
                let mut control = Point { x: values[0], y: values[1] };
                point = Point { x: values[2], y: values[3] };
                if relative { control = add(control, self.current); point = add(point, self.current); }
                self.check_points(&[control, point])?;
                self.quad_control = control; self.quad_reflect = true; self.cubic_reflect = false;
                self.quad(control, point);
                return Ok(());
            }
            b'T' => {
                let control = if self.quad_reflect { reflect(self.current, self.quad_control) } else { self.current };
                point = Point { x: values[0], y: values[1] };
                if relative { point = add(point, self.current); }
                self.check_points(&[control, point])?;
                self.quad_control = control; self.quad_reflect = true; self.cubic_reflect = false;
                self.quad(control, point);
                return Ok(());
            }
            b'A' => {
                if (values[3] != 0.0 && values[3] != 1.0) || (values[4] != 0.0 && values[4] != 1.0) || values[0] < 0.0 || values[1] < 0.0 {
                    return Err(ControlledError::Original(failure(DiagnosticKind::Path, self.group_offset())));
                }
                point = Point { x: values[5], y: values[6] };
                if relative { point = add(point, self.current); }
                self.check_points(&[point])?;
                let offset = self.group_offset();
                let cursor = GeometryCursor::arc(&mut self.document.edges, self.current, values, point, self.flatness, self.max_edges, offset)?;
                self.group.point = point;
                self.cubic_reflect = false; self.quad_reflect = false;
                if let Some(cursor) = cursor { self.start_geometry(cursor, self.group_offset(), AfterGeometry::Group); }
                else { self.phase = Phase::CompleteGroup; }
                return Ok(());
            }
            _ => {
                point = Point { x: values[0], y: values[1] };
                if relative { point = add(point, self.current); }
                if self.group.upper == b'M' {
                    self.check_points(&[point])?;
                    self.group.point = point;
                    if self.open {
                        self.start_geometry(GeometryCursor::Line(self.current, self.first), self.group_offset(), AfterGeometry::Move);
                    } else { self.phase = Phase::FinishMove; }
                    return Ok(());
                }
            }
        }
        self.cubic_reflect = false; self.quad_reflect = false;
        self.check_points(&[point])?;
        self.group.point = point;
        self.start_geometry(GeometryCursor::Line(self.current, point), self.group_offset(), AfterGeometry::Group);
        Ok(())
    }
    fn curve(&mut self, points: [Point; 4], to: Point) {
        self.group.point = to;
        self.start_geometry(GeometryCursor::Curve(CurveCursor::new(points)), self.group_offset(), AfterGeometry::Group);
    }
    fn quad(&mut self, control: Point, to: Point) {
        let from = self.current;
        let c1 = Point { x: from.x + (control.x - from.x) * (2.0 / 3.0), y: from.y + (control.y - from.y) * (2.0 / 3.0) };
        let c2 = Point { x: to.x + (control.x - to.x) * (2.0 / 3.0), y: to.y + (control.y - to.y) * (2.0 / 3.0) };
        self.curve([from, c1, c2, to], to);
    }
}
fn add(a: Point, b: Point) -> Point { Point { x: a.x + b.x, y: a.y + b.y } }
fn reflect(origin: Point, control: Point) -> Point { Point { x: 2.0 * origin.x - control.x, y: 2.0 * origin.y - control.y } }
