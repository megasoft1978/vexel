use crate::types::{invalid, limit, stopped};
use crate::*;
/// Preserved C stroke-grid encoding: high bit tags a global join index.
pub const STROKE_GRID_JOIN_BIT: u32 = 0x8000_0000;
impl DashCache {
    pub fn validate(&self, control: &mut dyn WorkControl) -> Result<()> {
        control.checkpoint().map_err(stopped)?;
        if self.shapes.len() > MAX_SHAPES {
            return Err(limit(LimitKind::Shapes));
        }
        for segment in &self.segments {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            if ![
                segment.edge.a.x,
                segment.edge.a.y,
                segment.edge.b.x,
                segment.edge.b.y,
            ]
            .into_iter()
            .all(f64::is_finite)
            {
                return Err(invalid(0));
            }
        }
        for join in &self.joins {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            if ![join.previous, join.vertex, join.next]
                .into_iter()
                .all(vexel_geometry::finite_point)
            {
                return Err(invalid(0));
            }
        }
        for range in &self.shapes {
            control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
            range.segments.require(self.segments.len(), 0)?;
            range.joins.require(self.joins.len(), 0)?;
            if ![range.grid_min_x, range.grid_min_y, range.grid_cell_size]
                .into_iter()
                .all(f64::is_finite)
            {
                return Err(invalid(0));
            }
            let cells = (range.grid_columns as usize)
                .checked_mul(range.grid_rows as usize)
                .ok_or_else(crate::memory::arithmetic)?;
            if cells != range.grid_cells.len()
                || ((cells == 0) != (range.grid_columns == 0 && range.grid_rows == 0))
                || (cells != 0 && range.grid_cell_size <= 0.0)
            {
                return Err(invalid(0));
            }
            for cell in &range.grid_cells {
                control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                TableRange {
                    first: cell.first,
                    count: cell.count,
                }
                .require(range.grid_refs.len(), 0)?;
            }
            for reference in &range.grid_refs {
                control.charge(WorkKind::StrokeTest, 1).map_err(stopped)?;
                let index = (*reference & !STROKE_GRID_JOIN_BIT) as usize;
                let allowed = if *reference & STROKE_GRID_JOIN_BIT != 0 {
                    range.joins
                } else {
                    range.segments
                };
                if index < allowed.first
                    || index >= allowed.end().ok_or_else(crate::memory::arithmetic)?
                {
                    return Err(invalid(0));
                }
            }
        }
        Ok(())
    }
}
