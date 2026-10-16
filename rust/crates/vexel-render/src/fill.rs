//! Point-in-shape test (C `raster_shape_paint_sample`, crossing loop).

use crate::FlatEdge;

/// Ray cast to +x over the edges named by `active`. An edge counts when it
/// spans `py` (half-open, upward or downward) and crosses right of `px`;
/// evenodd uses parity, nonzero uses the signed winding.
pub(crate) fn inside(edges: &[FlatEdge], active: &[usize], evenodd: bool, px: f64, py: f64) -> bool {
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
