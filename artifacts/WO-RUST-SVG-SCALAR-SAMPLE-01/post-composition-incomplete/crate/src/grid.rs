use vexel_scene::{GridCell, JobDashRange, ValidatedDashView};

fn axis(value: f64, origin: f64, size: f64, count: u16) -> Option<usize> {
    let mut cell = (value - origin) / size;
    if !cell.is_finite() || cell < -1.0 || count == 0 {
        return None;
    }
    if cell < 0.0 {
        cell = 0.0;
    }
    let cell = cell.floor().min(count as f64 - 1.0);
    (cell >= 0.0).then_some(cell as usize)
}

pub(crate) fn job_grid_lookup<'a>(
    range: &JobDashRange,
    cells: &'a [GridCell],
    x: f64,
    y: f64,
) -> Option<&'a GridCell> {
    let col = axis(x, range.grid_min_x, range.grid_cell_size, range.grid_columns)?;
    let row = axis(y, range.grid_min_y, range.grid_cell_size, range.grid_rows)?;
    cells.get(row.checked_mul(range.grid_columns as usize)?.checked_add(col)?)
}

/// Look up a validated job-owned stroke-grid cell by shape and local point.
pub fn grid_lookup(
    cache: ValidatedDashView<'_>,
    shape: usize,
    x: f64,
    y: f64,
) -> Option<GridCell> {
    let (range, cells, _) = cache.grid_at(shape).ok()?;
    job_grid_lookup(range, cells, x, y).copied()
}
