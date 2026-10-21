//! Scene renderer for compose and animation frames: routes a decoded scene to
//! the `vexel-render` entry points and composes the layers.
//!
//! The walk is C `render_group_range`: shapes outside any layered group are
//! painted in runs, a group that needs a layer (opacity below 1, a mask or a
//! filter) is rendered into a transparent layer, filtered, masked and then
//! composited into its parent. A run is split into batches by the entry point
//! that can draw it: `render_pattern_shapes` for pattern paint,
//! `render_dashed_shapes` for dashes, `render_groups` for clipped shapes and
//! plain shapes otherwise. Layer memory is bounded by `GROUP_SCRATCH_LIMIT`.
//! No new rasterization happens here.

use super::clips::ClipStore;
use super::layers::{Geo, composite_tile, mask_layer};
use super::shapes::{Entry, OwnedShape, Prepared, prepare};
use std::cell::OnceCell;
use vexel_raster::Tile;
use vexel_render::filter::apply_filter_chain;
use vexel_render::{
    GROUP_SCRATCH_LIMIT, MAX_GROUP_DEPTH, MAX_MASK_DEPTH, MAX_MASK_INSTANCES, PatternPaintShape,
    PreserveAspect, RenderClip, RenderError, RenderNode, UnsupportedKind, ViewBox, ViewMap,
    render_dashed_shapes, render_groups, render_pattern_shapes,
};
use vexel_scene::{Mask, SceneView, ViewportRead};

/// A shape the renderer skipped because of a deferred feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Skipped {
    pub scene_index: usize,
    pub kind: UnsupportedKind,
}

/// What a render drew and skipped, in scene order.
#[derive(Debug, Default)]
pub(crate) struct Drawn {
    pub shapes: Vec<OwnedShape>,
    pub skipped: Vec<Skipped>,
}

/// Renders the root `view` into the transparent `tile`. With `strict` a
/// skipped shape is an error; otherwise it is reported in [`Drawn::skipped`].
pub(crate) fn render_scene<V: ViewportRead, A: Copy>(
    view: &SceneView<'_, V, A>,
    view_box: ViewBox,
    aspect: Option<PreserveAspect>,
    tile: &mut Tile,
    strict: bool,
    cancel: &dyn Fn() -> bool,
) -> Result<Drawn, RenderError> {
    let map = ViewMap::from_viewbox(view_box, tile.width(), tile.height(), aspect)?;
    let mut renderer = Renderer {
        geo: geo(view_box, aspect, tile),
        map,
        cancel,
        tile_bytes: tile.byte_count(),
        scratch: 0,
        mask_depth: 0,
        masks: 0,
    };
    let entries = renderer.draw(view, tile, strict)?;
    let mut drawn = Drawn::default();
    for (scene_index, entry) in entries.into_iter().enumerate() {
        match entry {
            Entry::Draw(prepared) => drawn.shapes.push(prepared.shape),
            Entry::Skip(kind) => drawn.skipped.push(Skipped { scene_index, kind }),
        }
    }
    Ok(drawn)
}

/// C `map_viewport`, the same mapping `ViewMap::from_viewbox` applies.
fn geo(view: ViewBox, aspect: Option<PreserveAspect>, tile: &Tile) -> Geo {
    let aspect = aspect.unwrap_or(PreserveAspect::MID_MEET);
    let (width, height) = (f64::from(tile.width()), f64::from(tile.height()));
    let mut scale_x = width / view.width;
    let mut scale_y = height / view.height;
    let (mut offset_x, mut offset_y) = (0.0, 0.0);
    if !aspect.none {
        let scale = if aspect.slice {
            scale_x.max(scale_y)
        } else {
            scale_x.min(scale_y)
        };
        scale_x = scale;
        scale_y = scale;
        offset_x = (width - view.width * scale) * aspect.align_x;
        offset_y = (height - view.height * scale) * aspect.align_y;
    }
    Geo {
        view,
        scale_x,
        scale_y,
        offset_x,
        offset_y,
    }
}

/// How a batch of shapes is handed to the renderer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Plain,
    Dashed,
    Pattern,
    Clipped,
}

fn kind_of(prepared: &Prepared<'_>) -> Kind {
    if prepared.clip.is_some() {
        Kind::Clipped
    } else if prepared.fill_paint().is_some() || prepared.stroke.is_some() {
        Kind::Pattern
    } else if !prepared.shape.dashes.is_empty() {
        Kind::Dashed
    } else {
        Kind::Plain
    }
}

/// One prepared scene: its entries and clips.
struct Scene<'s, 'v, V, A> {
    view: &'s SceneView<'v, V, A>,
    entries: &'s [Entry<'v>],
    clips: &'s [OnceCell<RenderClip<'s>>],
}

struct Renderer<'c> {
    geo: Geo,
    map: ViewMap,
    cancel: &'c dyn Fn() -> bool,
    tile_bytes: usize,
    scratch: usize,
    mask_depth: usize,
    masks: usize,
}

impl Renderer<'_> {
    fn cancelled(&self) -> bool {
        (self.cancel)()
    }

    /// C `reserve_scratch`: bytes of live layer tiles.
    fn reserve(&mut self) -> Result<(), RenderError> {
        if self.tile_bytes > GROUP_SCRATCH_LIMIT - self.scratch {
            return Err(RenderError::LimitExceeded);
        }
        self.scratch += self.tile_bytes;
        Ok(())
    }

    fn release(&mut self) {
        self.scratch = self.scratch.saturating_sub(self.tile_bytes);
    }

    /// A fresh transparent tile the size of the view; the caller holds a reservation.
    fn new_tile(&self) -> Result<Tile, RenderError> {
        let _alloc = crate::profile::phase("r.alloc");
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(self.tile_bytes)
            .map_err(|_| RenderError::LimitExceeded)?;
        rgba.resize(self.tile_bytes, 0);
        Tile::from_owned_rgba(self.map.width(), self.map.height(), rgba)
            .map_err(|_| RenderError::LimitExceeded)
    }

    /// Prepares `view` and renders it into the empty `tile`; returns the entries.
    fn draw<'v, V: ViewportRead, A: Copy>(
        &mut self,
        view: &SceneView<'v, V, A>,
        tile: &mut Tile,
        strict: bool,
    ) -> Result<Vec<Entry<'v>>, RenderError> {
        let entries = prepare(view)?;
        if strict
            && let Some(Entry::Skip(kind)) = entries.iter().find(|e| matches!(e, Entry::Skip(_)))
        {
            return Err(RenderError::Unsupported(*kind));
        }
        let store = ClipStore::new(view.clips)?;
        let cells = store.cells();
        store.fill(view.clips, &cells)?;
        let scene = Scene {
            view,
            entries: &entries,
            clips: &cells,
        };
        let count = entries.len();
        // Without groups the tile is empty and a single batch paints straight into it.
        self.range(&scene, 0..count, None, tile, 0, view.groups.is_empty())?;
        Ok(entries)
    }

    /// C `render_group_range` over the shapes `range` whose parent group is `parent`.
    fn range<V: ViewportRead, A: Copy>(
        &mut self,
        scene: &Scene<'_, '_, V, A>,
        range: std::ops::Range<usize>,
        parent: Option<usize>,
        dest: &mut Tile,
        depth: usize,
        direct: bool,
    ) -> Result<(), RenderError> {
        if depth > MAX_GROUP_DEPTH {
            return Err(RenderError::LimitExceeded);
        }
        let mut cursor = range.start;
        for (index, group) in scene.view.groups.iter().enumerate() {
            if group.parent.map(|g| g.index()) != parent {
                continue;
            }
            let first = group.shapes.first;
            let end = group.shapes.end().ok_or(RenderError::InvalidGeometry)?;
            if first < cursor || end > range.end || first > end {
                return Err(RenderError::InvalidGeometry);
            }
            self.flat(scene, cursor..first, dest, direct)?;
            cursor = end;
            if first != end {
                self.group(scene, index, dest, depth)?;
            }
        }
        self.flat(scene, cursor..range.end, dest, direct)
    }

    fn group<V: ViewportRead, A: Copy>(
        &mut self,
        scene: &Scene<'_, '_, V, A>,
        index: usize,
        dest: &mut Tile,
        depth: usize,
    ) -> Result<(), RenderError> {
        let group = &scene.view.groups[index];
        if !group.opacity.is_finite() || !(0.0..=1.0).contains(&group.opacity) {
            return Err(RenderError::InvalidGeometry);
        }
        // Blend modes and isolation are not modelled by the renderer.
        if group.blend_mode != 0 {
            return Err(RenderError::Unsupported(UnsupportedKind::OpacityGroup));
        }
        let children =
            group.shapes.first..group.shapes.end().ok_or(RenderError::InvalidGeometry)?;
        let layered = group.opacity != 1.0 || group.filter.is_some() || group.mask.is_some();
        if !layered {
            return self.range(scene, children, Some(index), dest, depth + 1, false);
        }
        self.reserve()?;
        let result = self.layered(scene, index, children, dest, depth);
        self.release();
        result
    }

    /// Children into a layer, then filter, mask and composite (C order).
    fn layered<V: ViewportRead, A: Copy>(
        &mut self,
        scene: &Scene<'_, '_, V, A>,
        index: usize,
        children: std::ops::Range<usize>,
        dest: &mut Tile,
        depth: usize,
    ) -> Result<(), RenderError> {
        let group = &scene.view.groups[index];
        let mut layer = self.new_tile()?;
        self.range(scene, children, Some(index), &mut layer, depth + 1, false)?;
        if let Some(id) = group.filter {
            let filter = scene.view.filter(id).ok_or(RenderError::InvalidGeometry)?;
            let chain = filter
                .primitives
                .slice(scene.view.filter_primitives)
                .ok_or(RenderError::InvalidGeometry)?;
            let available = GROUP_SCRATCH_LIMIT - self.scratch;
            apply_filter_chain(
                layer.rgba_mut(),
                &self.map,
                filter,
                chain,
                available,
                self.cancel,
            )?;
        }
        if let Some(id) = group.mask {
            let mask = scene.view.mask(id).ok_or(RenderError::InvalidGeometry)?;
            self.mask(scene.view, mask, &mut layer)?;
        }
        let _composite = crate::profile::phase("r.composite");
        composite_tile(dest, &layer, group.opacity, self.cancel)
    }

    /// C `apply_mask`: renders the mask content scene and multiplies it into `layer`.
    fn mask<V: ViewportRead, A: Copy>(
        &mut self,
        view: &SceneView<'_, V, A>,
        mask: &Mask,
        layer: &mut Tile,
    ) -> Result<(), RenderError> {
        self.masks += 1;
        if self.mask_depth >= MAX_MASK_DEPTH || self.masks > MAX_MASK_INSTANCES {
            return Err(RenderError::LimitExceeded);
        }
        self.reserve()?;
        let result = (|| {
            let content_view = view
                .scene(mask.content)
                .ok_or(RenderError::InvalidGeometry)?;
            let mut content = self.new_tile()?;
            self.mask_depth += 1;
            let drawn = self.draw(&content_view, &mut content, true);
            self.mask_depth -= 1;
            drawn?;
            mask_layer(layer, &content, mask, &self.geo, self.cancel)
        })();
        self.release();
        result
    }

    /// Paints the plain run `range` (shapes of one parent) into `dest`.
    fn flat<V: ViewportRead, A: Copy>(
        &mut self,
        scene: &Scene<'_, '_, V, A>,
        range: std::ops::Range<usize>,
        dest: &mut Tile,
        direct: bool,
    ) -> Result<(), RenderError> {
        let mut batches: Vec<(Kind, Vec<&Prepared<'_>>)> = Vec::new();
        for entry in scene
            .entries
            .get(range)
            .ok_or(RenderError::InvalidGeometry)?
        {
            let Entry::Draw(prepared) = entry else {
                continue;
            };
            let kind = kind_of(prepared);
            match batches.last_mut() {
                Some((batch, items)) if kind == Kind::Plain || *batch == kind => {
                    items.push(prepared);
                }
                Some((batch, items)) if *batch == Kind::Plain => {
                    *batch = kind;
                    items.push(prepared);
                }
                _ => batches.push((kind, vec![prepared])),
            }
        }
        let single = batches.len() == 1;
        for (kind, items) in &batches {
            if self.cancelled() {
                return Err(RenderError::Cancelled);
            }
            if direct && single {
                self.paint(scene, *kind, items, dest)?;
                continue;
            }
            self.reserve()?;
            let result = (|| {
                let mut layer = self.new_tile()?;
                self.paint(scene, *kind, items, &mut layer)?;
                let _composite = crate::profile::phase("r.composite");
                composite_tile(dest, &layer, 1.0, self.cancel)
            })();
            self.release();
            result?;
        }
        Ok(())
    }

    /// Hands one batch to the matching `vexel-render` entry point.
    fn paint<V: ViewportRead, A: Copy>(
        &self,
        scene: &Scene<'_, '_, V, A>,
        kind: Kind,
        items: &[&Prepared<'_>],
        tile: &mut Tile,
    ) -> Result<(), RenderError> {
        let _paint = crate::profile::phase("r.paint");
        match kind {
            Kind::Plain | Kind::Dashed => {
                let shapes: Vec<_> = items.iter().map(|p| p.shape.as_dashed()).collect();
                render_dashed_shapes(&shapes, self.map, tile, self.cancel)
            }
            Kind::Pattern => {
                let shapes: Vec<_> = items
                    .iter()
                    .map(|p| PatternPaintShape {
                        shape: p.shape.as_render(),
                        fill: p.fill_paint(),
                        stroke: p.stroke,
                    })
                    .collect();
                render_pattern_shapes(&shapes, self.map, tile, self.cancel)
            }
            Kind::Clipped => {
                let mut nodes = Vec::new();
                for p in items {
                    let clip = match p.clip {
                        Some(index) => Some(
                            scene
                                .clips
                                .get(index)
                                .and_then(OnceCell::get)
                                .ok_or(RenderError::InvalidGeometry)?,
                        ),
                        None => None,
                    };
                    nodes.push(RenderNode::Shape {
                        shape: p.shape.as_render(),
                        clip,
                    });
                }
                render_groups(&nodes, self.map, tile, self.cancel)
            }
        }
    }
}
