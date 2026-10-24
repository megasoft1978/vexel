//! Uniform candidate grid over stroke primitives (the culling idea of C
//! `shape_render_stroke_grid.c`). A cell lists every segment and join whose
//! hit box reaches it. The exact predicates still decide each sample, and a
//! stroke contains a sample when ANY primitive does, so testing only the
//! candidates of the sample's cell gives the same answer as testing all.
//!
//! Exactness: a primitive's box is built with the same arithmetic as its hit
//! test, and cells come from one monotone function of the coordinate, so any
//! point passing a primitive's box test falls in a cell that lists it. Joins
//! use `|p - v| <= reach`, which is widened by a relative slack well above
//! rounding error.

use crate::stroke::{Join, Segment};
use crate::stroke_join::JoinGeom;
use crate::{Linecap, RenderError};

/// Primitives below this count are tested linearly.
pub(crate) const MIN_PRIMITIVES: usize = 24;
const MAX_AXIS_CELLS: usize = 512;
const SLACK: f64 = 1e-9;
const JOIN_FLAG: u32 = 1 << 31;

pub(crate) struct Grid {
    min_x: f64,
    min_y: f64,
    cell_w: f64,
    cell_h: f64,
    columns: usize,
    rows: usize,
    /// `starts[c]..starts[c + 1]` indexes `items` for cell `c`.
    starts: Vec<u32>,
    /// Segment index, or join index with [`JOIN_FLAG`].
    items: Vec<u32>,
}

struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

fn segment_rect(segment: &Segment, radius: f64) -> Rect {
    let square = segment.start_cap == Linecap::Square || segment.end_cap == Linecap::Square;
    let margin = if square { radius * std::f64::consts::SQRT_2 } else { radius };
    let (a, b) = (segment.a, segment.b);
    Rect {
        x0: a.x.min(b.x) - margin,
        y0: a.y.min(b.y) - margin,
        x1: a.x.max(b.x) + margin,
        y1: a.y.max(b.y) + margin,
    }
}

/// Cells along one axis: `axis` cells, but never narrower than the mean
/// primitive extent `mean`. A primitive spanning many thin cells lists each of
/// them, and the reference budget would then refuse the whole grid.
fn cells_along(span: f64, axis: usize, mean: f64) -> usize {
    let cell = (span / axis as f64).max(mean);
    ((span / cell).ceil() as usize).clamp(1, axis)
}

fn join_rect(join: &Join, geom: &JoinGeom, reach: f64) -> Rect {
    if let Some([x0, y0, x1, y1]) = geom.tight {
        return Rect { x0, y0, x1, y1 };
    }
    let pad = reach + (reach + join.vertex.x.abs().max(join.vertex.y.abs())) * SLACK;
    Rect {
        x0: join.vertex.x - pad,
        y0: join.vertex.y - pad,
        x1: join.vertex.x + pad,
        y1: join.vertex.y + pad,
    }
}

impl Grid {
    /// Builds the grid, or `None` when the primitives are few, the geometry is
    /// degenerate or the reference count would exceed the budget (a stroke so
    /// wide that every cell lists most primitives gains nothing).
    pub(crate) fn build(
        segments: &[Segment],
        joins: &[Join],
        geoms: &[JoinGeom],
        radius: f64,
        join_reach: f64,
    ) -> Result<Option<Self>, RenderError> {
        let count = segments.len() + joins.len();
        if count < MIN_PRIMITIVES || count >= JOIN_FLAG as usize {
            return Ok(None);
        }
        let rects: Vec<Rect> = segments
            .iter()
            .map(|s| segment_rect(s, radius))
            .chain(joins.iter().zip(geoms).map(|(j, g)| join_rect(j, g, join_reach)))
            .collect();
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for r in &rects {
            min_x = min_x.min(r.x0);
            min_y = min_y.min(r.y0);
            max_x = max_x.max(r.x1);
            max_y = max_y.max(r.y1);
        }
        let (width, height) = (max_x - min_x, max_y - min_y);
        if ![min_x, min_y, width, height].iter().all(|v| v.is_finite()) {
            return Ok(None);
        }
        let axis = ((count as f64).sqrt().ceil() as usize).clamp(2, MAX_AXIS_CELLS);
        let (mut sum_w, mut sum_h) = (0.0, 0.0);
        for r in &rects {
            sum_w += r.x1 - r.x0;
            sum_h += r.y1 - r.y0;
        }
        let columns = if width > 0.0 { cells_along(width, axis, sum_w / count as f64) } else { 1 };
        let rows = if height > 0.0 { cells_along(height, axis, sum_h / count as f64) } else { 1 };
        let mut grid = Self {
            min_x,
            min_y,
            cell_w: width / columns as f64,
            cell_h: height / rows as f64,
            columns,
            rows,
            starts: Vec::new(),
            items: Vec::new(),
        };
        let budget = count.saturating_mul(8).saturating_add(1024);
        let cells = columns * rows;
        let mut counts: Vec<u32> = Vec::new();
        counts.try_reserve_exact(cells + 1).map_err(|_| RenderError::LimitExceeded)?;
        counts.resize(cells + 1, 0);
        let mut references = 0usize;
        for r in &rects {
            let (c0, c1, r0, r1) = grid.cell_range(r);
            references += (c1 - c0 + 1) * (r1 - r0 + 1);
            if references > budget {
                return Ok(None);
            }
            for row in r0..=r1 {
                for column in c0..=c1 {
                    counts[row * columns + column] += 1;
                }
            }
        }
        let mut total = 0u32;
        for slot in counts.iter_mut() {
            let n = *slot;
            *slot = total;
            total += n;
        }
        grid.items.try_reserve_exact(references).map_err(|_| RenderError::LimitExceeded)?;
        grid.items.resize(references, 0);
        grid.starts = counts.clone();
        for (i, r) in rects.iter().enumerate() {
            let tag =
                if i < segments.len() { i as u32 } else { (i - segments.len()) as u32 | JOIN_FLAG };
            let (c0, c1, r0, r1) = grid.cell_range(r);
            for row in r0..=r1 {
                for column in c0..=c1 {
                    let cell = row * columns + column;
                    grid.items[counts[cell] as usize] = tag;
                    counts[cell] += 1;
                }
            }
        }
        Ok(Some(grid))
    }

    fn column_of(&self, x: f64) -> usize {
        if self.cell_w > 0.0 {
            (((x - self.min_x) / self.cell_w).floor().max(0.0) as usize).min(self.columns - 1)
        } else {
            0
        }
    }

    fn row_of(&self, y: f64) -> usize {
        if self.cell_h > 0.0 {
            (((y - self.min_y) / self.cell_h).floor().max(0.0) as usize).min(self.rows - 1)
        } else {
            0
        }
    }

    fn cell_range(&self, r: &Rect) -> (usize, usize, usize, usize) {
        (self.column_of(r.x0), self.column_of(r.x1), self.row_of(r.y0), self.row_of(r.y1))
    }

    /// Calls `f` for every item of every cell the rectangle touches.
    pub(crate) fn for_each_in(&self, x0: f64, y0: f64, x1: f64, y1: f64, mut f: impl FnMut(u32)) {
        let (c0, c1) = (self.column_of(x0), self.column_of(x1));
        let (r0, r1) = (self.row_of(y0), self.row_of(y1));
        for row in r0..=r1 {
            for column in c0..=c1 {
                let cell = row * self.columns + column;
                for item in &self.items[self.starts[cell] as usize..self.starts[cell + 1] as usize] {
                    f(*item);
                }
            }
        }
    }

    /// Candidate primitives for a local-space sample.
    pub(crate) fn candidates(&self, px: f64, py: f64) -> &[u32] {
        let cell = self.row_of(py) * self.columns + self.column_of(px);
        &self.items[self.starts[cell] as usize..self.starts[cell + 1] as usize]
    }
}

/// Splits a candidate tag into a segment or join index.
pub(crate) fn decode(tag: u32) -> (bool, usize) {
    (tag & JOIN_FLAG != 0, (tag & !JOIN_FLAG) as usize)
}
