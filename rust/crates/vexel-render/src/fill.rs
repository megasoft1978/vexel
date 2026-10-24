//! Point-in-shape test (C `raster_shape_paint_sample`, crossing loop).

use crate::FlatEdge;

/// Ray cast to +x over the edges named by `active`. An edge counts when it
/// spans `py` (half-open, upward or downward) and crosses right of `px`;
/// evenodd uses parity, nonzero uses the signed winding.
pub(crate) fn inside(
    edges: &[FlatEdge],
    active: &[usize],
    evenodd: bool,
    px: f64,
    py: f64,
) -> bool {
    let mut winding = 0i32;
    let mut parity = false;
    for &index in active {
        let edge = &edges[index];
        let upward = edge.y0 <= py && edge.y1 > py;
        let downward = edge.y1 <= py && edge.y0 > py;
        if !upward && !downward {
            continue;
        }
        let xcross = edge.x0 + (py - edge.y0) * (edge.x1 - edge.x0) / (edge.y1 - edge.y0);
        if xcross > px {
            winding += if upward { 1 } else { -1 };
            parity = !parity;
        }
    }
    if evenodd { parity } else { winding != 0 }
}

/// Crossings of one sample row, sorted by x, with suffix windings. Gives the
/// same answer as [`inside`] for every `px`: the edges that span the row are
/// gathered once instead of once per sample.
#[derive(Default)]
pub(crate) struct Crossings {
    xs: Vec<f64>,
    /// `winding_from[i]` is the signed winding of crossings `i..`.
    winding_from: Vec<i32>,
    scratch: Vec<(f64, i32)>,
}

impl Crossings {
    /// Collects the crossings of `py` over the edges named by `active`.
    pub(crate) fn collect(&mut self, edges: &[FlatEdge], active: &[usize], py: f64) {
        self.scratch.clear();
        for &index in active {
            let edge = &edges[index];
            let upward = edge.y0 <= py && edge.y1 > py;
            let downward = edge.y1 <= py && edge.y0 > py;
            if !upward && !downward {
                continue;
            }
            let xcross = edge.x0 + (py - edge.y0) * (edge.x1 - edge.x0) / (edge.y1 - edge.y0);
            self.scratch.push((xcross, if upward { 1 } else { -1 }));
        }
        self.finish();
    }

    /// Sorts the gathered crossings and builds the suffix windings.
    fn finish(&mut self) {
        self.scratch.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        self.xs.clear();
        self.winding_from.clear();
        self.winding_from.resize(self.scratch.len() + 1, 0);
        for (i, &(_, direction)) in self.scratch.iter().enumerate().rev() {
            self.winding_from[i] = self.winding_from[i + 1] + direction;
        }
        self.xs.extend(self.scratch.iter().map(|c| c.0));
    }

    /// [`Self::collect`] over every edge of `edges`.
    pub(crate) fn collect_all(&mut self, edges: &[FlatEdge], py: f64) {
        self.scratch.clear();
        for edge in edges {
            let upward = edge.y0 <= py && edge.y1 > py;
            let downward = edge.y1 <= py && edge.y0 > py;
            if !upward && !downward {
                continue;
            }
            let xcross = edge.x0 + (py - edge.y0) * (edge.x1 - edge.x0) / (edge.y1 - edge.y0);
            self.scratch.push((xcross, if upward { 1 } else { -1 }));
        }
        self.finish();
    }

    pub(crate) fn first_x(&self) -> Option<f64> {
        self.xs.first().copied()
    }

    pub(crate) fn last_x(&self) -> Option<f64> {
        self.xs.last().copied()
    }

    /// The winding left of every crossing is the total; when it is empty
    /// (zero / even) samples left of the first crossing are outside.
    pub(crate) fn outside_on_left(&self, evenodd: bool) -> bool {
        if evenodd { self.xs.len() % 2 == 0 } else { self.winding_from[0] == 0 }
    }

    /// Same result as [`inside`] for the row this was collected for.
    pub(crate) fn inside(&self, evenodd: bool, px: f64) -> bool {
        self.inside_after(evenodd, self.xs.partition_point(|x| *x <= px))
    }

    /// Number of crossings at or left of `px`, continuing from `cursor`, a
    /// count already known to be at most the answer (`px` only increases).
    pub(crate) fn advance(&self, mut cursor: usize, px: f64) -> usize {
        while cursor < self.xs.len() && self.xs[cursor] <= px {
            cursor += 1;
        }
        cursor
    }

    /// Number of crossings at or left of `px`.
    pub(crate) fn count_le(&self, px: f64) -> usize {
        self.xs.partition_point(|x| *x <= px)
    }

    /// [`Self::inside`] given `first_right`, the count of crossings `<= px`.
    pub(crate) fn inside_after(&self, evenodd: bool, first_right: usize) -> bool {
        if evenodd {
            (self.xs.len() - first_right) % 2 == 1
        } else {
            self.winding_from[first_right] != 0
        }
    }
}
