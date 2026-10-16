//! Owned clip storage and the `RenderClip` table built over it.
//!
//! `RenderClip` borrows its edges and its parent clip, so the geometry lives in
//! a [`ClipStore`] and the clips themselves in write-once cells that may point
//! at each other without any unsafe code.

use std::cell::OnceCell;
use vexel_render::{ClipShape, FlatEdge, MAX_CLIP_CHAIN, RenderClip, RenderError};
use vexel_scene::Clip;

/// Flattened clip geometry, parallel to the scene clip table.
pub(crate) struct ClipStore {
    edges: Vec<Vec<FlatEdge>>,
    shapes: Vec<Vec<ClipShape>>,
}

pub(crate) type ClipCells<'a> = Vec<OnceCell<RenderClip<'a>>>;

impl ClipStore {
    pub(crate) fn new(clips: &[Clip]) -> Result<Self, RenderError> {
        let mut edges = Vec::new();
        let mut shapes = Vec::new();
        edges
            .try_reserve_exact(clips.len())
            .map_err(|_| RenderError::LimitExceeded)?;
        shapes
            .try_reserve_exact(clips.len())
            .map_err(|_| RenderError::LimitExceeded)?;
        for clip in clips {
            edges.push(
                clip.geometry
                    .edges
                    .iter()
                    .copied()
                    .map(FlatEdge::from)
                    .collect(),
            );
            let mut children = Vec::with_capacity(clip.geometry.shapes.len());
            for child in &clip.geometry.shapes {
                let end = child.edges.end().ok_or(RenderError::InvalidGeometry)?;
                children.push(ClipShape {
                    edges: child.edges.first..end,
                    evenodd: child.evenodd,
                    filled: child.filled,
                });
            }
            shapes.push(children);
        }
        Ok(Self { edges, shapes })
    }

    /// Empty cells, one per clip; fill them with [`ClipStore::fill`].
    pub(crate) fn cells<'a>(&self) -> ClipCells<'a> {
        (0..self.edges.len()).map(|_| OnceCell::new()).collect()
    }

    /// Builds every clip in `cells` (parents first, as needed).
    pub(crate) fn fill<'a>(
        &'a self,
        clips: &[Clip],
        cells: &'a [OnceCell<RenderClip<'a>>],
    ) -> Result<(), RenderError> {
        for index in 0..clips.len() {
            self.build(index, clips, cells, 0)?;
        }
        Ok(())
    }

    fn build<'a>(
        &'a self,
        index: usize,
        clips: &[Clip],
        cells: &'a [OnceCell<RenderClip<'a>>],
        depth: usize,
    ) -> Result<&'a RenderClip<'a>, RenderError> {
        let cell = cells.get(index).ok_or(RenderError::InvalidGeometry)?;
        if let Some(done) = cell.get() {
            return Ok(done);
        }
        if depth > MAX_CLIP_CHAIN {
            return Err(RenderError::LimitExceeded);
        }
        let clip = clips.get(index).ok_or(RenderError::InvalidGeometry)?;
        let parent = match clip.parent {
            Some(parent) => Some(self.build(parent.index(), clips, cells, depth + 1)?),
            None => None,
        };
        let built = RenderClip {
            edges: &self.edges[index],
            shapes: &self.shapes[index],
            parent,
            object_bounding_box: clip.object_bounding_box,
            bounds: clip.bounds,
            user_inverse: clip.user_inverse,
        };
        Ok(cell.get_or_init(|| built))
    }
}
