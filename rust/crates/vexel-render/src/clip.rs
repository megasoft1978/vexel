//! Clip coverage (C `svg_clip_contains`, `src/svg/clip/clip.c`).
//!
//! A clip is a per-sample predicate: a sample may paint only when it lies in
//! the union of the clip's filled child shapes and in the parent chain.

use crate::{FlatEdge, MAX_COORDINATE, RenderError};
use std::ops::Range;
use vexel_geometry::{Affine, Point, affine_map_point, affine_valid};

/// Clip chain a shape must satisfy at every sample (all entries must contain).
pub(crate) type ClipList<'a> = &'a [&'a RenderClip<'a>];

/// Largest accepted clip nesting (parent chain length). C bounds clip depth
/// through `CLIP_MAX_DEPTH`.
pub const MAX_CLIP_CHAIN: usize = 256;
/// Largest accepted edge count of one clip (C `CLIP_MAX_EDGES`).
pub const MAX_CLIP_EDGES: usize = 65_536;

/// One child shape of a clip: an edge range into [`RenderClip::edges`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipShape {
    pub edges: Range<usize>,
    pub evenodd: bool,
    /// Open geometry (`false`) never contributes coverage, as in C.
    pub filled: bool,
}

/// A resolved clip path.
///
/// `edges` are in the clip's own coordinate space: element-local user space,
/// or the unit box when `object_bounding_box` is set (then `bounds` is
/// `[min_x, min_y, max_x, max_y]` of the clipped content in local space).
/// `user_inverse` maps document user space to that local space. `parent`
/// mirrors the C chain: a sample must also lie in every ancestor, tested at
/// the original document position.
#[derive(Debug, Clone, Copy)]
pub struct RenderClip<'a> {
    pub edges: &'a [FlatEdge],
    pub shapes: &'a [ClipShape],
    pub parent: Option<&'a RenderClip<'a>>,
    pub object_bounding_box: bool,
    pub bounds: [f64; 4],
    pub user_inverse: Affine,
}

/// C `point_inside` over one clip child (same crossing rules as shape fill).
fn point_inside(edges: &[FlatEdge], evenodd: bool, x: f64, y: f64) -> bool {
    let mut winding = 0i32;
    let mut parity = false;
    for edge in edges {
        let up = edge.y0 <= y && edge.y1 > y;
        let down = edge.y1 <= y && edge.y0 > y;
        if !up && !down {
            continue;
        }
        let cross_x = edge.x0 + (y - edge.y0) * (edge.x1 - edge.x0) / (edge.y1 - edge.y0);
        if cross_x > x {
            winding += if up { 1 } else { -1 };
            parity = !parity;
        }
    }
    if evenodd { parity } else { winding != 0 }
}

impl RenderClip<'_> {
    /// C `svg_clip_contains` for a document user-space position.
    pub(crate) fn contains(&self, x: f64, y: f64) -> bool {
        self.local_contains(x, y)
            && self.parent.is_none_or(|parent| parent.contains(x, y))
    }

    /// The part of [`Self::contains`] that does not look at the parent chain.
    pub(crate) fn local_contains(&self, x: f64, y: f64) -> bool {
        let Some(local) = affine_map_point(self.user_inverse, Point { x, y }) else {
            return false;
        };
        let (mut x, mut y) = (local.x, local.y);
        if self.object_bounding_box {
            if self.bounds[2] <= self.bounds[0] || self.bounds[3] <= self.bounds[1] {
                return false;
            }
            x = (x - self.bounds[0]) / (self.bounds[2] - self.bounds[0]);
            y = (y - self.bounds[1]) / (self.bounds[3] - self.bounds[1]);
        }
        self.shapes.iter().any(|shape| {
            shape.filled && point_inside(&self.edges[shape.edges.clone()], shape.evenodd, x, y)
        })
    }

    /// Checks geometry and bounds up front so sampling cannot misbehave.
    pub(crate) fn validate(&self) -> Result<(), RenderError> {
        let mut current = Some(self);
        let mut links = 0usize;
        while let Some(clip) = current {
            links += 1;
            if links > MAX_CLIP_CHAIN {
                return Err(RenderError::LimitExceeded);
            }
            if clip.edges.len() > MAX_CLIP_EDGES {
                return Err(RenderError::LimitExceeded);
            }
            let finite = |v: f64| v.is_finite() && v.abs() <= MAX_COORDINATE;
            if !affine_valid(clip.user_inverse)
                || clip.bounds.iter().any(|v| !v.is_finite())
                || clip
                    .edges
                    .iter()
                    .any(|e| !(finite(e.x0) && finite(e.y0) && finite(e.x1) && finite(e.y1)))
                || clip
                    .shapes
                    .iter()
                    .any(|s| s.edges.start > s.edges.end || s.edges.end > clip.edges.len())
            {
                return Err(RenderError::InvalidGeometry);
            }
            current = clip.parent;
        }
        Ok(())
    }
}
