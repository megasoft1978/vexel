//! Row index for the shape loop: sample positions precomputed per axis and an
//! active-edge sweep, so edges are bucketed once instead of rescanned per row.
//! Only skips work: every value equals what `ViewMap` computes per sample.

use crate::{FlatEdge, RenderError, SAMPLES_PER_AXIS, ViewMap};

pub(crate) const SAMPLE_LOCATIONS: [f64; SAMPLES_PER_AXIS] = [0.125, 0.375, 0.625, 0.875];

/// Per-render tables. `edge_rows[k]` is the user y of device row boundary `k`
/// (non-decreasing); `columns` holds user x and in-view flag per sample column.
pub(crate) struct AxisTables {
    boundary_y: Vec<f64>,
    column_x: Vec<f64>,
    column_ok: Vec<bool>,
}

fn reserved<T>(count: usize) -> Result<Vec<T>, RenderError> {
    let mut vec = Vec::new();
    vec.try_reserve_exact(count).map_err(|_| RenderError::LimitExceeded)?;
    Ok(vec)
}

impl AxisTables {
    pub(crate) fn new(view: &ViewMap) -> Result<Self, RenderError> {
        let width = view.width() as usize;
        let height = view.height() as usize;
        let mut boundary_y = reserved(height + 2)?;
        for k in 0..height + 2 {
            boundary_y.push(view.to_user(0.0, k as f64).1);
        }
        let mut column_x = reserved(width * SAMPLES_PER_AXIS)?;
        let mut column_ok = reserved(width * SAMPLES_PER_AXIS)?;
        for x in 0..width {
            for location in SAMPLE_LOCATIONS {
                let sample_x = x as f64 + location;
                column_ok.push(view.x_in_view(sample_x));
                column_x.push(view.to_user(sample_x, 0.0).0);
            }
        }
        Ok(Self { boundary_y, column_x, column_ok })
    }

    /// `(user x, in view)` of sample `sx` of pixel column `x`, equal to
    /// `to_user` and the x half of `sample_in_view`.
    pub(crate) fn column(&self, x: u32, sx: usize) -> (f64, bool) {
        let at = x as usize * SAMPLES_PER_AXIS + sx;
        (self.column_x[at], self.column_ok[at])
    }

    /// Pixel rows `first..=last` whose edge-activity predicate
    /// `min <= to_user(y + 1) && max >= to_user(y)` holds, or `None`.
    fn touched_rows(&self, low: f64, high: f64) -> Option<(usize, usize)> {
        let first = self.boundary_y[1..].partition_point(|v| *v < low);
        let after_last = self.boundary_y.partition_point(|v| *v <= high);
        (after_last > 0 && first < after_last).then(|| (first, after_last - 1))
    }
}

/// Active edges of the current pixel row, in sweep order.
#[derive(Default)]
pub(crate) struct Sweep {
    /// `(first row, last row, edge index)` sorted by first row.
    starts: Vec<(u32, u32, u32)>,
    next: usize,
    pub(crate) active: Vec<usize>,
    last_row: Vec<u32>,
}

impl Sweep {
    /// Bins the edges that touch some row of `y_first..=y_last`.
    pub(crate) fn reset(
        &mut self,
        edges: &[FlatEdge],
        tables: &AxisTables,
        y_first: u32,
        y_last: u32,
    ) -> Result<(), RenderError> {
        self.starts.clear();
        self.active.clear();
        self.last_row.clear();
        self.next = 0;
        self.starts.try_reserve(edges.len()).map_err(|_| RenderError::LimitExceeded)?;
        self.active.try_reserve(edges.len().min(4096)).map_err(|_| RenderError::LimitExceeded)?;
        for (index, edge) in edges.iter().enumerate() {
            let Some((first, last)) =
                tables.touched_rows(edge.y0.min(edge.y1), edge.y0.max(edge.y1))
            else {
                continue;
            };
            if last < y_first as usize || first > y_last as usize {
                continue;
            }
            let first = first.max(y_first as usize) as u32;
            let last = last.min(y_last as usize) as u32;
            self.starts.push((first, last, index as u32));
        }
        self.starts.sort_unstable();
        Ok(())
    }

    /// Makes `active` the edges touching pixel row `row` (rows ascend).
    pub(crate) fn advance(&mut self, row: u32) {
        let mut kept = 0;
        for i in 0..self.active.len() {
            if self.last_row[i] >= row {
                self.active[kept] = self.active[i];
                self.last_row[kept] = self.last_row[i];
                kept += 1;
            }
        }
        self.active.truncate(kept);
        self.last_row.truncate(kept);
        while let Some(&(first, last, index)) = self.starts.get(self.next) {
            if first > row {
                break;
            }
            self.active.push(index as usize);
            self.last_row.push(last);
            self.next += 1;
        }
    }
}
