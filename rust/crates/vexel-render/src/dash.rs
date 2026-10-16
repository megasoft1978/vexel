//! Dash expansion into stroke primitives.
//!
//! Ports `dash_cache_build_range` and `dash_range_joins_seam` from
//! `shape_render_dash_cache.c`. Mirrored C constants: at most
//! [`MAX_DASH_VALUES`] pattern values (`SVG_SHAPE_MAX_DASH_VALUES`), step
//! epsilon [`DASH_EPSILON`] (`RASTER_STROKE_DASH_EPSILON`), a document-wide
//! step budget of [`crate::MAX_STROKE_STEPS`] (`RASTER_DASH_CACHE_MAX_STEPS`)
//! and a byte budget of [`crate::MAX_STROKE_BYTES`]
//! (`RASTER_DASH_CACHE_BYTE_LIMIT`).
//!
//! Extension over C (whose parser rejects zero entries): zero-length "on"
//! intervals are dots, drawn with the stroke cap (round and square draw, butt
//! draws nothing). Patterns that are empty, contain a negative or non-finite
//! entry, or sum to zero render solid.

use crate::stroke::{JOIN_BYTES, Join, SEGMENT_BYTES, Segment, reserve};
use crate::{Linecap, MAX_STROKE_BYTES, MAX_STROKE_STEPS, RenderError};
use vexel_geometry::Point;

/// C `SVG_SHAPE_MAX_DASH_VALUES`.
pub const MAX_DASH_VALUES: usize = 16;
/// C `RASTER_STROKE_DASH_EPSILON`.
pub(crate) const DASH_EPSILON: f64 = 1e-12;

/// Document-wide stroke build budget (C `build_steps` and the cache byte limit).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Budget {
    pub steps: usize,
    pub bytes: usize,
}

impl Budget {
    pub fn step(&mut self, amount: usize) -> Result<(), RenderError> {
        if amount > MAX_STROKE_STEPS - self.steps.min(MAX_STROKE_STEPS) {
            return Err(RenderError::LimitExceeded);
        }
        self.steps += amount;
        Ok(())
    }

    pub fn primitives(&mut self, segments: usize, joins: usize) -> Result<(), RenderError> {
        let bytes = segments
            .checked_mul(SEGMENT_BYTES)
            .and_then(|s| joins.checked_mul(JOIN_BYTES).and_then(|j| s.checked_add(j)))
            .and_then(|add| self.bytes.checked_add(add))
            .ok_or(RenderError::LimitExceeded)?;
        if bytes > MAX_STROKE_BYTES {
            return Err(RenderError::LimitExceeded);
        }
        self.bytes = bytes;
        Ok(())
    }
}

/// A validated dash pattern: the user values plus the start offset.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pattern {
    values: Vec<f64>,
    offset: f64,
    period: f64,
}

impl Pattern {
    /// `Ok(None)` means the stroke is solid. More than [`MAX_DASH_VALUES`]
    /// entries is a limit error.
    pub fn new(dashes: &[f64], offset: f64) -> Result<Option<Self>, RenderError> {
        if dashes.is_empty() {
            return Ok(None);
        }
        if dashes.len() > MAX_DASH_VALUES {
            return Err(RenderError::LimitExceeded);
        }
        if dashes.iter().any(|d| !d.is_finite() || *d < 0.0) || !offset.is_finite() {
            return Ok(None);
        }
        let mut values = Vec::new();
        reserve(&mut values, dashes.len())?;
        values.extend_from_slice(dashes);
        let mut pattern = Self {
            values,
            offset,
            period: 0.0,
        };
        let mut period = 0.0;
        for index in 0..pattern.expanded() {
            period += pattern.value(index);
        }
        if !period.is_finite() || period <= 0.0 {
            return Ok(None);
        }
        pattern.period = period;
        Ok(Some(pattern))
    }

    /// Entry count after repeating an odd-length pattern.
    pub fn expanded(&self) -> usize {
        let count = self.values.len();
        count + if count % 2 == 1 { count } else { 0 }
    }

    pub fn value(&self, index: usize) -> f64 {
        self.values[index % self.values.len()]
    }

    fn has_zero(&self) -> bool {
        self.values.contains(&0.0)
    }

    /// Phase within the period, in `[0, period]` (C `fmod` plus wrap).
    fn phase(&self) -> Result<f64, RenderError> {
        let phase = self.offset % self.period;
        if !phase.is_finite() {
            return Err(RenderError::LimitExceeded);
        }
        Ok(if phase < 0.0 {
            phase + self.period
        } else {
            phase
        })
    }
}

/// Position inside the expanded pattern.
struct Walker<'a> {
    pattern: &'a Pattern,
    index: usize,
    remain: f64,
}

impl<'a> Walker<'a> {
    /// Applies the offset (C phase loop). A zero entry at phase 0 is kept so
    /// a leading dot is drawn.
    fn start(pattern: &'a Pattern, budget: &mut Budget) -> Result<Self, RenderError> {
        let expanded = pattern.expanded();
        let mut phase = pattern.phase()?;
        let mut index = 0;
        let mut advances = 0;
        while phase >= pattern.value(index) && (pattern.value(index) > 0.0 || phase > 0.0) {
            if advances >= expanded {
                return Err(RenderError::LimitExceeded);
            }
            advances += 1;
            budget.step(1)?;
            phase -= pattern.value(index);
            index = (index + 1) % expanded;
        }
        let remain = pattern.value(index) - phase;
        if !remain.is_finite() || remain < 0.0 {
            return Err(RenderError::LimitExceeded);
        }
        Ok(Self {
            pattern,
            index,
            remain,
        })
    }

    fn on(&self) -> bool {
        self.index % 2 == 0
    }

    fn advance(&mut self) {
        self.index = (self.index + 1) % self.pattern.expanded();
        self.remain = self.pattern.value(self.index);
    }

    /// Passes zero-length intervals at the current position; true when one of
    /// them was an "on" interval (a dot).
    fn settle(&mut self) -> bool {
        let mut dot = false;
        for _ in 0..=self.pattern.expanded() {
            if self.remain != 0.0 {
                break;
            }
            dot |= self.on();
            self.advance();
        }
        dot
    }
}

fn point_at(edge: (Point, Point), dx: f64, dy: f64, along: f64) -> Point {
    Point {
        x: edge.0.x + dx * along,
        y: edge.0.y + dy * along,
    }
}

/// C `dash_range_joins_seam`: the pattern is "on" at both ends of a closed
/// range, so the dash across the seam needs a join instead of two caps.
/// Patterns with zero entries never join.
pub(crate) fn joins_seam(pattern: &Pattern, edges: &[(Point, Point)]) -> bool {
    if edges.is_empty() || pattern.has_zero() {
        return false;
    }
    let expanded = pattern.expanded();
    let Ok(mut phase) = pattern.phase() else {
        return false;
    };
    let mut index = 0;
    let mut advances = 0;
    while phase >= pattern.value(index) {
        if advances > expanded {
            return false;
        }
        advances += 1;
        phase -= pattern.value(index);
        index = (index + 1) % expanded;
    }
    let starts_on = index % 2 == 0;
    let mut remain = pattern.value(index) - phase;
    let mut ends_on = false;
    let mut steps = 0usize;
    for edge in edges {
        let mut length = (edge.1.x - edge.0.x).hypot(edge.1.y - edge.0.y);
        while length > 0.0 {
            steps += 1;
            if steps > MAX_STROKE_STEPS {
                return false;
            }
            let step = remain.min(length);
            ends_on = index % 2 == 0;
            length -= step;
            remain -= step;
            if remain <= DASH_EPSILON {
                index = (index + 1) % expanded;
                remain = pattern.value(index);
            }
        }
    }
    starts_on && ends_on
}

fn push_segment(
    segments: &mut Vec<Segment>,
    budget: &mut Budget,
    segment: Segment,
) -> Result<(), RenderError> {
    budget.primitives(1, 0)?;
    reserve(segments, 1)?;
    segments.push(segment);
    Ok(())
}

fn push_join(joins: &mut Vec<Join>, budget: &mut Budget, join: Join) -> Result<(), RenderError> {
    budget.primitives(0, 1)?;
    reserve(joins, 1)?;
    joins.push(join);
    Ok(())
}

fn push_dot(
    segments: &mut Vec<Segment>,
    budget: &mut Budget,
    cap: Linecap,
    at: Point,
) -> Result<(), RenderError> {
    if cap == Linecap::Butt {
        return Ok(());
    }
    push_segment(
        segments,
        budget,
        Segment {
            a: at,
            b: at,
            start_cap: cap,
            end_cap: cap,
        },
    )
}

/// C `dash_cache_build_range` for one stroked range of local-space `edges`
/// (the implicit fill-closing edge already dropped). `seam` is true when a
/// closed range joins across its seam.
pub(crate) fn build_range(
    segments: &mut Vec<Segment>,
    joins: &mut Vec<Join>,
    budget: &mut Budget,
    pattern: &Pattern,
    cap: Linecap,
    edges: &[(Point, Point)],
    seam: bool,
) -> Result<(), RenderError> {
    if edges.is_empty() {
        return Ok(());
    }
    budget.step(pattern.expanded())?;
    let mut walker = Walker::start(pattern, budget)?;
    if walker.settle() {
        push_dot(segments, budget, cap, edges[0].0)?;
    }
    let count = edges.len();
    let mut joins_from_previous = false;
    for (edge_index, edge) in edges.iter().enumerate() {
        budget.step(1)?;
        let dx = edge.1.x - edge.0.x;
        let dy = edge.1.y - edge.0.y;
        let length = dx.hypot(dy);
        if !length.is_finite() {
            return Err(RenderError::LimitExceeded);
        }
        if length == 0.0 {
            continue;
        }
        let mut at = 0.0;
        let mut continues_at_vertex = false;
        while at < length {
            budget.step(1)?;
            let step = walker.remain.min(length - at);
            let end = at + step;
            if !(step > 0.0) || !end.is_finite() {
                return Err(RenderError::LimitExceeded);
            }
            let on = walker.on();
            let continues = on && end == length && walker.remain - step > DASH_EPSILON;
            let crosses_vertex = continues && edge_index + 1 < count;
            if on {
                let starts_at_join =
                    at == 0.0 && (joins_from_previous || (seam && edge_index == 0));
                let ends_at_seam = end == length && seam && edge_index + 1 == count;
                let segment = Segment {
                    a: point_at(*edge, dx, dy, at / length),
                    b: point_at(*edge, dx, dy, end / length),
                    start_cap: if starts_at_join { Linecap::Butt } else { cap },
                    end_cap: if crosses_vertex || ends_at_seam {
                        Linecap::Butt
                    } else {
                        cap
                    },
                };
                push_segment(segments, budget, segment)?;
            }
            continues_at_vertex |= crosses_vertex;
            at = end;
            walker.remain -= step;
            if walker.remain <= DASH_EPSILON {
                walker.advance();
                if walker.settle() {
                    push_dot(segments, budget, cap, point_at(*edge, dx, dy, at / length))?;
                }
            }
        }
        if continues_at_vertex && edge_index + 1 < count {
            let next = edges[edge_index + 1];
            push_join(
                joins,
                budget,
                Join {
                    previous: edge.0,
                    vertex: edge.1,
                    next: next.1,
                },
            )?;
        }
        joins_from_previous = continues_at_vertex;
    }
    if seam {
        push_join(
            joins,
            budget,
            Join {
                previous: edges[count - 1].0,
                vertex: edges[count - 1].1,
                next: edges[0].1,
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
