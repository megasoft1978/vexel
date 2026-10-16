//! Group compositing, a port of C `src/raster/shape_render_group.c`.
//!
//! A group that needs a layer (opacity below 1 or a mask) is rendered into a
//! transparent full-size layer, masked, multiplied by its opacity and then
//! source-over composited into its parent. Runs of plain shapes between layers
//! are rendered into their own transparent layer and composited at opacity 1,
//! as C `render_flat_range` does. A group with opacity 1 and no mask is
//! transparent: its children join the surrounding run and only its clip is
//! added to theirs, so a tree without any layered group renders exactly like
//! [`render_shapes`](crate::render_shapes).
//!
//! Clips are per-sample predicates (see [`RenderClip`]) and apply to every
//! shape below the group that carries them. Blend modes, filters and
//! isolation are not modelled here.

use crate::clip::{ClipList, RenderClip};
use crate::composite::source_over;
use crate::mask::{self, RenderMask};
use crate::render::render_shapes_clipped;
use crate::{RenderError, RenderShape, ViewMap};
use std::ops::Range;
use vexel_raster::Tile;

/// Group scratch budget (C `GROUP_SCRATCH_LIMIT`): bytes of live layer tiles.
pub const GROUP_SCRATCH_LIMIT: usize = 64 * 1024 * 1024;
/// Most mask instances in one scene (C `SHAPE_MASK_MAX_INSTANCES`).
pub const MAX_MASK_INSTANCES: usize = 1024;
/// Deepest mask-in-mask nesting rendered (C `context->mask_depth >= 8`).
pub const MAX_MASK_DEPTH: usize = 8;
/// Deepest group nesting (C `render_group_range` recursion bound).
pub const MAX_GROUP_DEPTH: usize = 64;

/// One entry of the ordered scene tree.
#[derive(Debug, Clone, Copy)]
pub enum RenderNode<'a> {
    /// A shape, optionally clipped (C `doc->clips[shape]`).
    Shape {
        shape: RenderShape<'a>,
        clip: Option<&'a RenderClip<'a>>,
    },
    Group(RenderGroup<'a>),
}

/// A group of nodes composited together (C `svg_shape_opacity_group`).
///
/// `clip` limits every shape below the group. `opacity` is finite in `0..=1`.
#[derive(Debug, Clone, Copy)]
pub struct RenderGroup<'a> {
    pub opacity: f64,
    pub clip: Option<&'a RenderClip<'a>>,
    pub mask: Option<&'a RenderMask<'a>>,
    pub children: &'a [RenderNode<'a>],
}

impl RenderGroup<'_> {
    fn is_layered(&self) -> bool {
        self.opacity != 1.0 || self.mask.is_some()
    }
}

/// Tree facts gathered (and validated) before any pixel is written.
#[derive(Default)]
struct Survey {
    masks: usize,
}

fn survey(
    nodes: &[RenderNode<'_>],
    depth: usize,
    mask_depth: usize,
    found: &mut Survey,
) -> Result<(), RenderError> {
    for node in nodes {
        match node {
            RenderNode::Shape { clip, .. } => {
                if let Some(clip) = clip {
                    clip.validate()?;
                }
            }
            RenderNode::Group(group) => {
                if depth >= MAX_GROUP_DEPTH {
                    return Err(RenderError::LimitExceeded);
                }
                if !group.opacity.is_finite() || !(0.0..=1.0).contains(&group.opacity) {
                    return Err(RenderError::InvalidGeometry);
                }
                if let Some(clip) = group.clip {
                    clip.validate()?;
                }
                if let Some(mask) = group.mask {
                    found.masks += 1;
                    if found.masks > MAX_MASK_INSTANCES || mask_depth >= MAX_MASK_DEPTH {
                        return Err(RenderError::LimitExceeded);
                    }
                    mask.validate()?;
                    survey(mask.content, 0, mask_depth + 1, found)?;
                }
                survey(group.children, depth + 1, mask_depth, found)?;
            }
        }
    }
    Ok(())
}

fn has_layer(nodes: &[RenderNode<'_>]) -> bool {
    nodes.iter().any(|node| match node {
        RenderNode::Shape { .. } => false,
        RenderNode::Group(group) => group.is_layered() || has_layer(group.children),
    })
}

/// Consecutive plain shapes with the clip chain each must satisfy.
#[derive(Default)]
struct Run<'a> {
    shapes: Vec<RenderShape<'a>>,
    ranges: Vec<Range<usize>>,
    arena: Vec<&'a RenderClip<'a>>,
}

impl<'a> Run<'a> {
    fn push(
        &mut self,
        shape: RenderShape<'a>,
        clips: &[&'a RenderClip<'a>],
    ) -> Result<(), RenderError> {
        let start = self.arena.len();
        self.arena
            .try_reserve(clips.len())
            .map_err(|_| RenderError::LimitExceeded)?;
        self.arena.extend_from_slice(clips);
        self.ranges
            .try_reserve(1)
            .map_err(|_| RenderError::LimitExceeded)?;
        self.ranges.push(start..self.arena.len());
        self.shapes
            .try_reserve(1)
            .map_err(|_| RenderError::LimitExceeded)?;
        self.shapes.push(shape);
        Ok(())
    }

    fn clear(&mut self) {
        self.shapes.clear();
        self.ranges.clear();
        self.arena.clear();
    }
}

/// Render state shared by a whole call (C `group_render_context`).
pub(crate) struct Ctx<'c> {
    pub(crate) view: ViewMap,
    pub(crate) control: &'c dyn Fn() -> bool,
    pub(crate) mask_depth: usize,
    tile_bytes: usize,
    scratch_bytes: usize,
}

impl Ctx<'_> {
    pub(crate) fn cancelled(&self) -> bool {
        (self.control)()
    }

    pub(crate) fn reserve(&mut self) -> Result<(), RenderError> {
        if self.tile_bytes > GROUP_SCRATCH_LIMIT - self.scratch_bytes {
            return Err(RenderError::LimitExceeded);
        }
        self.scratch_bytes += self.tile_bytes;
        Ok(())
    }

    pub(crate) fn release(&mut self) {
        self.scratch_bytes = self.scratch_bytes.saturating_sub(self.tile_bytes);
    }

    /// A fresh transparent tile the size of the view; the caller holds a reservation.
    pub(crate) fn new_tile(&self) -> Result<Tile, RenderError> {
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(self.tile_bytes)
            .map_err(|_| RenderError::LimitExceeded)?;
        rgba.resize(self.tile_bytes, 0);
        Tile::from_owned_rgba(self.view.width(), self.view.height(), rgba)
            .map_err(|_| RenderError::LimitExceeded)
    }

    /// Renders the pending run into its own layer and composites it at opacity 1.
    fn flush(&mut self, run: &mut Run<'_>, destination: &mut Tile) -> Result<(), RenderError> {
        if run.shapes.is_empty() {
            return Ok(());
        }
        self.reserve()?;
        let result = self.flush_reserved(run, destination);
        self.release();
        run.clear();
        result
    }

    fn flush_reserved(&mut self, run: &Run<'_>, destination: &mut Tile) -> Result<(), RenderError> {
        let mut layer = self.new_tile()?;
        let lists: Vec<ClipList<'_>> = run.ranges.iter().map(|r| &run.arena[r.clone()]).collect();
        render_shapes_clipped(&run.shapes, &lists, self.view, &mut layer, self.control)?;
        composite_tile(destination, &layer, 1.0, self.control)
    }

    /// C `render_group_range` for one node sequence.
    fn walk<'a>(
        &mut self,
        nodes: &[RenderNode<'a>],
        clips: &mut Vec<&'a RenderClip<'a>>,
        run: &mut Run<'a>,
        destination: &mut Tile,
    ) -> Result<(), RenderError> {
        for node in nodes {
            if self.cancelled() {
                return Err(RenderError::Cancelled);
            }
            let outer = clips.len();
            match node {
                RenderNode::Shape { shape, clip } => {
                    clips.extend(*clip);
                    run.push(*shape, clips)?;
                }
                RenderNode::Group(group) if !group.is_layered() => {
                    clips.extend(group.clip);
                    self.walk(group.children, clips, run, destination)?;
                }
                RenderNode::Group(group) => {
                    if group.children.is_empty() {
                        continue;
                    }
                    self.flush(run, destination)?;
                    clips.extend(group.clip);
                    self.layered(group, clips, destination)?;
                }
            }
            clips.truncate(outer);
        }
        Ok(())
    }

    fn layered<'a>(
        &mut self,
        group: &RenderGroup<'a>,
        clips: &mut Vec<&'a RenderClip<'a>>,
        destination: &mut Tile,
    ) -> Result<(), RenderError> {
        self.reserve()?;
        let result = self.layered_reserved(group, clips, destination);
        self.release();
        result
    }

    fn layered_reserved<'a>(
        &mut self,
        group: &RenderGroup<'a>,
        clips: &mut Vec<&'a RenderClip<'a>>,
        destination: &mut Tile,
    ) -> Result<(), RenderError> {
        let mut layer = self.new_tile()?;
        let mut run = Run::default();
        self.walk(group.children, clips, &mut run, &mut layer)?;
        self.flush(&mut run, &mut layer)?;
        if let Some(mask) = group.mask {
            mask::apply(self, mask, &mut layer)?;
        }
        composite_tile(destination, &layer, group.opacity, self.control)
    }

    /// Renders `nodes` into the transparent `tile` (also used for mask content).
    pub(crate) fn render_tree(
        &mut self,
        nodes: &[RenderNode<'_>],
        tile: &mut Tile,
    ) -> Result<(), RenderError> {
        let mut clips = Vec::new();
        let mut run = Run::default();
        self.walk(nodes, &mut clips, &mut run, tile)?;
        if !run.shapes.is_empty() {
            if has_layer(nodes) {
                self.flush(&mut run, tile)?;
            } else {
                // Nothing layered: paint straight into the empty tile, exactly as `render_shapes`.
                let lists: Vec<ClipList<'_>> =
                    run.ranges.iter().map(|r| &run.arena[r.clone()]).collect();
                render_shapes_clipped(&run.shapes, &lists, self.view, tile, self.control)?;
            }
        }
        Ok(())
    }
}

/// C `composite_tile` with blend mode normal: layer alpha times `opacity`, source-over.
fn composite_tile(
    destination: &mut Tile,
    source: &Tile,
    opacity: f64,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    let row_bytes = source.width() as usize * 4;
    for (dst_row, src_row) in destination
        .rgba_mut()
        .chunks_exact_mut(row_bytes)
        .zip(source.rgba().chunks_exact(row_bytes))
    {
        if control() {
            return Err(RenderError::Cancelled);
        }
        for (dst, src) in dst_row.chunks_exact_mut(4).zip(src_row.chunks_exact(4)) {
            let source_alpha = (f64::from(src[3]) / 255.0 * opacity).min(1.0);
            if source_alpha <= 0.0 {
                continue;
            }
            let rgba = [src[0], src[1], src[2], (source_alpha * 255.0).round() as u8];
            let dst: &mut [u8; 4] = dst.try_into().map_err(|_| RenderError::InvalidGeometry)?;
            source_over(dst, rgba);
        }
    }
    Ok(())
}

/// Renders an ordered group/shape tree into the empty `tile`.
///
/// `tile` must be fully transparent black and match `view`. `control` is polled
/// per node, per shape and per pixel row; returning true aborts with
/// [`RenderError::Cancelled`] and leaves the tile unspecified. Layer memory is
/// bounded by [`GROUP_SCRATCH_LIMIT`]; mask count, mask depth and group depth
/// by [`MAX_MASK_INSTANCES`], [`MAX_MASK_DEPTH`] and [`MAX_GROUP_DEPTH`], all
/// reported as [`RenderError::LimitExceeded`]. Deferred features inside shapes
/// (dashes, non-solid paint) keep returning their typed `Unsupported` from the
/// shape path.
pub fn render_groups(
    nodes: &[RenderNode<'_>],
    view: ViewMap,
    tile: &mut Tile,
    control: &dyn Fn() -> bool,
) -> Result<(), RenderError> {
    if tile.width() != view.width() || tile.height() != view.height() {
        return Err(RenderError::InvalidGeometry);
    }
    survey(nodes, 0, 0, &mut Survey::default())?;
    if tile.rgba().iter().any(|byte| *byte != 0) {
        return Err(RenderError::TileNotEmpty);
    }
    let mut ctx = Ctx {
        view,
        control,
        mask_depth: 0,
        tile_bytes: tile.byte_count(),
        scratch_bytes: 0,
    };
    ctx.render_tree(nodes, tile)
}
