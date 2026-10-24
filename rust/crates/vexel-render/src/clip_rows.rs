//! Row-based clip coverage: the exact answer of `RenderClip::contains` for the
//! 16 samples of a pixel, without rescanning the clip's edges per sample.
//!
//! A clip whose `user_inverse` has no shear or rotation maps a sample row to a
//! single local `y`, so the clip's crossings are gathered once per sample row
//! ([`Crossings`]) and every sample is a binary search. The arithmetic that
//! produces the local point and the crossing test is the same as in `clip.rs`,
//! so the answers are identical. Other clips use `local_contains` per sample.

use crate::SAMPLES_PER_AXIS;
use crate::clip::{ClipList, RenderClip};
use crate::fill::Crossings;
use crate::scan::AxisTables;
use std::ops::Range;
use vexel_geometry::affine_valid;

/// Bit `sy * 4 + sx` of a mask is sample `(sx, sy)`.
pub(crate) const FULL_MASK: u16 = 0xFFFF;

/// Largest mapped coordinate `affine_map_point` accepts.
const MAPPED_LIMIT: f64 = 1.0e6;

struct Slot<'a> {
    edges: &'a [crate::FlatEdge],
    evenodd: bool,
}

/// A clip whose local `y` depends only on the document `y`.
struct Level<'a> {
    a: f64,
    e: f64,
    d: f64,
    f: f64,
    bbox: Option<[f64; 4]>,
    /// Object-bounding-box clip with an empty box: contains nothing.
    never: bool,
    slots: Range<usize>,
    /// Local `x` never decreases with document `x`.
    monotone: bool,
    _clip: std::marker::PhantomData<&'a RenderClip<'a>>,
}

/// Every clip a shape must satisfy, with its parents flattened in.
pub(crate) struct ClipPlan<'a> {
    levels: Vec<Level<'a>>,
    slots: Vec<Slot<'a>>,
    slow: Vec<&'a RenderClip<'a>>,
}

/// Per-row scratch: the crossings of each slot at each sample row.
#[derive(Default)]
pub(crate) struct ClipRows {
    crossings: Vec<Crossings>,
    /// `(row valid, local y)` per level and sample row.
    rows: Vec<(bool, f64)>,
}

impl<'a> ClipPlan<'a> {
    pub(crate) fn new(list: ClipList<'a>) -> Self {
        let mut flat: Vec<&'a RenderClip<'a>> = Vec::new();
        for clip in list {
            let mut current = Some(*clip);
            while let Some(c) = current {
                if !flat.iter().any(|seen| std::ptr::eq(*seen, c)) {
                    flat.push(c);
                }
                current = c.parent;
            }
        }
        let (mut levels, mut slots, mut slow) = (Vec::new(), Vec::new(), Vec::new());
        for clip in flat {
            let m = clip.user_inverse;
            if m.b != 0.0 || m.c != 0.0 || !affine_valid(m) {
                slow.push(clip);
                continue;
            }
            let first = slots.len();
            for shape in clip.shapes.iter().filter(|s| s.filled) {
                slots.push(Slot { edges: &clip.edges[shape.edges.clone()], evenodd: shape.evenodd });
            }
            let never = clip.object_bounding_box
                && (clip.bounds[2] <= clip.bounds[0] || clip.bounds[3] <= clip.bounds[1]);
            levels.push(Level {
                a: m.a,
                e: m.e,
                d: m.d,
                f: m.f,
                bbox: clip.object_bounding_box.then_some(clip.bounds),
                never,
                slots: first..slots.len(),
                monotone: m.a > 0.0,
                _clip: std::marker::PhantomData,
            });
        }
        Self { levels, slots, slow }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.levels.is_empty() && self.slow.is_empty()
    }

    /// Gathers the crossings of the four sample rows `sample_y` (document y).
    pub(crate) fn prepare(&self, scratch: &mut ClipRows, sample_y: &[f64; SAMPLES_PER_AXIS]) {
        let need = self.slots.len() * SAMPLES_PER_AXIS;
        if scratch.crossings.len() < need {
            scratch.crossings.resize_with(need, Crossings::default);
        }
        scratch.rows.clear();
        for level in &self.levels {
            for (sy, &py) in sample_y.iter().enumerate() {
                let valid = !level.never && py.is_finite();
                // Same arithmetic as `affine_map_point` with b == c == 0.
                let mut y = level.d * py + level.f;
                let valid = valid && y.is_finite() && y.abs() <= MAPPED_LIMIT;
                if let Some(b) = level.bbox.filter(|_| valid) {
                    y = (y - b[1]) / (b[3] - b[1]);
                }
                scratch.rows.push((valid, y));
                if valid {
                    for slot in level.slots.clone() {
                        scratch.crossings[slot * SAMPLES_PER_AXIS + sy]
                            .collect_all(self.slots[slot].edges, y);
                    }
                }
            }
        }
    }

    /// Local x of document x `px`, or `None` where the clip contains nothing.
    fn local_x(level: &Level<'_>, px: f64) -> Option<f64> {
        if !px.is_finite() {
            return None;
        }
        let x = level.a * px + level.e;
        if !x.is_finite() || x.abs() > MAPPED_LIMIT {
            return None;
        }
        Some(match level.bbox {
            Some(b) => (x - b[0]) / (b[2] - b[0]),
            None => x,
        })
    }

    /// Exact clip membership of the 16 samples of pixel column `x`, for the
    /// rows prepared by [`Self::prepare`]. Samples outside the view are
    /// answered like any other sample.
    pub(crate) fn pixel_mask(
        &self,
        scratch: &ClipRows,
        tables: &AxisTables,
        x: u32,
        sample_y: &[f64; SAMPLES_PER_AXIS],
    ) -> u16 {
        let mut mask = FULL_MASK;
        let last = SAMPLES_PER_AXIS - 1;
        for (index, level) in self.levels.iter().enumerate() {
            let left = Self::local_x(level, tables.column(x, 0).0);
            let right = Self::local_x(level, tables.column(x, last).0);
            let ends = level.monotone.then_some(()).and(left).zip(right);
            for sy in 0..SAMPLES_PER_AXIS {
                let (valid, _) = scratch.rows[index * SAMPLES_PER_AXIS + sy];
                let row_bits = 0xFu16 << (sy * SAMPLES_PER_AXIS);
                if !valid {
                    mask &= !row_bits;
                    continue;
                }
                let at = |slot: usize| &scratch.crossings[slot * SAMPLES_PER_AXIS + sy];
                if let Some((lx0, lx1)) = ends {
                    // Local x never decreases across the pixel, so equal
                    // crossing counts at both ends mean one state for all four.
                    let same = level.slots.clone().all(|s| at(s).count_le(lx0) == at(s).count_le(lx1));
                    if same {
                        let inside = level.slots.clone().any(|s| {
                            let c = at(s);
                            c.inside_after(self.slots[s].evenodd, c.count_le(lx0))
                        });
                        if !inside {
                            mask &= !row_bits;
                        }
                        continue;
                    }
                }
                for sx in 0..SAMPLES_PER_AXIS {
                    let inside = Self::local_x(level, tables.column(x, sx).0).is_some_and(|lx| {
                        level.slots.clone().any(|s| at(s).inside(self.slots[s].evenodd, lx))
                    });
                    if !inside {
                        mask &= !(1 << (sy * SAMPLES_PER_AXIS + sx));
                    }
                }
            }
            if mask == 0 {
                return 0;
            }
        }
        for clip in &self.slow {
            for sy in 0..SAMPLES_PER_AXIS {
                for sx in 0..SAMPLES_PER_AXIS {
                    let bit = 1 << (sy * SAMPLES_PER_AXIS + sx);
                    if mask & bit != 0
                        && !clip.local_contains(tables.column(x, sx).0, sample_y[sy])
                    {
                        mask &= !bit;
                    }
                }
            }
        }
        mask
    }
}
