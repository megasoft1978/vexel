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
use super::diff::{Rect, device_rect};
use super::layers::{Geo, composite_tile, composite_tile_blend, composite_tile_in, mask_layer};
use super::shapes::{Entry, OwnedShape, Prepared, prepare};
use std::cell::{OnceCell, RefCell};
use vexel_raster::Tile;
use vexel_render::filter::apply_filter_chain;
use vexel_render::{
    GROUP_SCRATCH_LIMIT, MAX_GROUP_DEPTH, MAX_MASK_DEPTH, MAX_MASK_INSTANCES, PatternPaintShape,
    PreserveAspect, RenderClip, RenderError, RenderNode, UnsupportedKind, ViewBox, ViewMap,
    render_dashed_shapes_fresh, render_dashed_shapes_in, render_groups, render_pattern_shapes,
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
    let mut renderer = Renderer::new(view_box, aspect, tile, cancel)?;
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
pub(super) fn geo(view: ViewBox, aspect: Option<PreserveAspect>, tile: &Tile) -> Geo {
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
pub(super) enum Kind {
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

/// Splits the shapes of `range` into the batches the renderer paints.
pub(super) fn batch_runs<'e, 'v>(
    entries: &'e [Entry<'v>],
    range: std::ops::Range<usize>,
) -> Result<Vec<(Kind, Vec<&'e Prepared<'v>>)>, RenderError> {
    let mut batches: Vec<(Kind, Vec<&Prepared<'_>>)> = Vec::new();
    for entry in entries.get(range).ok_or(RenderError::InvalidGeometry)? {
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
    Ok(batches)
}

/// One prepared scene: its entries and clips.
struct Scene<'s, 'v, V, A> {
    view: &'s SceneView<'v, V, A>,
    entries: &'s [Entry<'v>],
    clips: &'s [OnceCell<RenderClip<'s>>],
    /// Conservative pixel bounds of each scene shape.
    bounds: &'s [Option<Rect>],
}

/// Union of the bounds of the scene shapes `indices`; `None` when none paints.
fn union_of(bounds: &[Option<Rect>], indices: impl Iterator<Item = usize>) -> Option<Rect> {
    indices
        .filter_map(|i| bounds.get(i).copied().flatten())
        .reduce(Rect::union)
}

/// The regions and shapes of a differential render.
#[derive(Clone, Copy)]
pub(super) struct Only<'c> {
    /// Pixel rectangles to produce, pairwise disjoint.
    pub rects: &'c [Rect],
    /// Conservative pixel bounds of each scene shape.
    pub bounds: &'c [Option<Rect>],
}

impl Only<'_> {
    fn hit(&self, scene_index: usize) -> bool {
        self.bounds
            .get(scene_index)
            .copied()
            .flatten()
            .is_some_and(|b| self.rects.iter().any(|r| r.meets(b)))
    }
}

pub(super) struct Renderer<'c> {
    pub(super) geo: Geo,
    map: ViewMap,
    cancel: &'c dyn Fn() -> bool,
    tile_bytes: usize,
    scratch: usize,
    mask_depth: usize,
    masks: usize,
    /// Differential render: only shapes flagged here are painted. Batching and
    /// layer decisions still follow the whole scene, so painted pixels equal
    /// a full render.
    pub(super) only: Option<Only<'c>>,
    /// Transparent layers left by scissored batches, reused instead of allocated.
    spare: RefCell<Vec<Tile>>,
}

impl<'c> Renderer<'c> {
    pub(super) fn new(
        view_box: ViewBox,
        aspect: Option<PreserveAspect>,
        tile: &Tile,
        cancel: &'c dyn Fn() -> bool,
    ) -> Result<Self, RenderError> {
        Ok(Self {
            geo: geo(view_box, aspect, tile),
            map: ViewMap::from_viewbox(view_box, tile.width(), tile.height(), aspect)?,
            cancel,
            tile_bytes: tile.byte_count(),
            scratch: 0,
            mask_depth: 0,
            masks: 0,
            only: None,
            spare: RefCell::new(Vec::new()),
        })
    }
}

impl Renderer<'_> {
    /// The pixel rectangles a composite of a layer touches; `None` means the
    /// whole tile. `region` bounds every non-transparent pixel of the layer
    /// (`None`: the whole layer may hold pixels).
    fn composite_rects(&self, region: Option<Option<Rect>>) -> Option<Vec<Rect>> {
        match (self.only, region) {
            (Some(only), Some(region)) => Some(
                region
                    .into_iter()
                    .flat_map(|g| only.rects.iter().filter_map(move |r| r.intersect(g)))
                    .collect(),
            ),
            (Some(only), None) => Some(only.rects.to_vec()),
            (None, Some(region)) => Some(region.into_iter().collect()),
            (None, None) => None,
        }
    }

    /// A transparent layer: a recycled one when available.
    fn take_layer(&self) -> Result<Tile, RenderError> {
        match self.spare.borrow_mut().pop() {
            Some(layer) => Ok(layer),
            None => self.new_tile(),
        }
    }

    /// Clears `rects` of `layer`, which a scissored batch painted only inside,
    /// and keeps it for the next batch.
    fn recycle_layer(&self, mut layer: Tile, rects: &[Rect]) {
        let width = layer.width() as usize;
        for rect in rects {
            for y in rect.y0 as usize..rect.y1 as usize {
                layer.rgba_mut()
                    [(y * width + rect.x0 as usize) * 4..(y * width + rect.x1 as usize) * 4]
                    .fill(0);
            }
        }
        self.spare.borrow_mut().push(layer);
    }

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
        self.draw_entries(view, &entries, tile)?;
        Ok(entries)
    }

    /// Renders prepared `entries` of `view` into the empty `tile`.
    pub(super) fn draw_entries<V: ViewportRead, A: Copy>(
        &mut self,
        view: &SceneView<'_, V, A>,
        entries: &[Entry<'_>],
        tile: &mut Tile,
    ) -> Result<(), RenderError> {
        let store = ClipStore::new(view.clips)?;
        let cells = store.cells();
        store.fill(view.clips, &cells)?;
        let computed: Vec<Option<Rect>>;
        let bounds = match self.only {
            Some(only) => only.bounds,
            None => {
                let (w, h) = (self.map.width(), self.map.height());
                computed = entries
                    .iter()
                    .map(|e| match e {
                        Entry::Draw(p) => device_rect(&p.shape, &self.geo, w, h),
                        Entry::Skip(_) => None,
                    })
                    .collect();
                &computed
            }
        };
        let scene = Scene {
            view,
            entries,
            clips: &cells,
            bounds,
        };
        // Without groups the tile is empty and a single batch paints straight into it.
        self.range(
            &scene,
            0..entries.len(),
            None,
            tile,
            0,
            view.groups.is_empty(),
        )
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
        if group.blend_mode > vexel_scene::MAX_BLEND_MODE {
            return Err(RenderError::InvalidGeometry);
        }
        let children =
            group.shapes.first..group.shapes.end().ok_or(RenderError::InvalidGeometry)?;
        if let Some(only) = self.only
            && !children.clone().any(|i| only.hit(i))
        {
            return Ok(());
        }
        let layered = group.opacity != 1.0
            || group.filter.is_some()
            || group.mask.is_some()
            || group.blend_mode != 0;
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
        let mut layer = self.take_layer()?;
        let children_range = children.clone();
        self.range(scene, children, Some(index), &mut layer, depth + 1, false)?;
        if let Some(id) = group.filter {
            let filter = scene.view.filter(id).ok_or(RenderError::InvalidGeometry)?;
            let chain = filter
                .primitives
                .slice(scene.view.filter_primitives)
                .ok_or(RenderError::InvalidGeometry)?;
            // Filter scratch is capped by its own limit, on top of the live layers.
            let available = vexel_render::filter::FILTER_SCRATCH_LIMIT;
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
        // Filters spread pixels and masks reshape them; otherwise the layer
        // only holds what its shapes painted.
        let region = (group.filter.is_none() && group.mask.is_none())
            .then(|| union_of(scene.bounds, children_range));
        let rects = self.composite_rects(region);
        if group.blend_mode != 0 {
            composite_tile_blend(
                dest,
                &layer,
                group.opacity,
                group.blend_mode,
                rects.as_deref(),
                self.cancel,
            )?;
        } else {
            match &rects {
                Some(rects) => composite_tile_in(dest, &layer, group.opacity, rects, self.cancel)?,
                None => composite_tile(dest, &layer, group.opacity, self.cancel)?,
            }
        }
        // Without filter or mask the layer only received composites inside the
        // rectangles just composited.
        if let Some(rects) = rects {
            self.recycle_layer(layer, &rects);
        }
        Ok(())
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
        let batches = batch_runs(scene.entries, range)?;
        let single = batches.len() == 1;
        for (kind, items) in &batches {
            if self.cancelled() {
                return Err(RenderError::Cancelled);
            }
            let filtered: Vec<&Prepared<'_>>;
            let items = match self.only {
                Some(only) => {
                    filtered = items
                        .iter()
                        .copied()
                        .filter(|p| only.hit(p.shape.scene_index))
                        .collect();
                    if filtered.is_empty() {
                        continue;
                    }
                    &filtered
                }
                None => items,
            };
            if direct && single {
                self.paint(scene, *kind, items, dest)?;
                continue;
            }
            self.reserve()?;
            let result = (|| {
                let mut layer = self.take_layer()?;
                self.paint(scene, *kind, items, &mut layer)?;
                let _composite = crate::profile::phase("r.composite");
                let region = Some(union_of(
                    scene.bounds,
                    items.iter().map(|p| p.shape.scene_index),
                ));
                let rects = self.composite_rects(region);
                match &rects {
                    Some(rects) => composite_tile_in(dest, &layer, 1.0, rects, self.cancel)?,
                    None => composite_tile(dest, &layer, 1.0, self.cancel)?,
                }
                // A plain batch paints only inside its shapes' bounds (and the
                // scissor), which the composited rectangles cover: clearing
                // them restores a transparent layer.
                if matches!(kind, Kind::Plain | Kind::Dashed)
                    && let Some(rects) = rects
                {
                    self.recycle_layer(layer, &rects);
                }
                Ok(())
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
                if let Some(only) = self.only {
                    for rect in only.rects {
                        let shapes: Vec<_> = items
                            .iter()
                            .filter(|p| {
                                only.bounds
                                    .get(p.shape.scene_index)
                                    .copied()
                                    .flatten()
                                    .is_some_and(|b| rect.meets(b))
                            })
                            .map(|p| p.shape.as_dashed())
                            .collect();
                        if !shapes.is_empty() {
                            render_dashed_shapes_in(
                                &shapes,
                                self.map,
                                tile,
                                rect.scissor(),
                                self.cancel,
                            )?;
                        }
                    }
                    return Ok(());
                }
                let shapes: Vec<_> = items.iter().map(|p| p.shape.as_dashed()).collect();
                // The tile is the caller's blank tile or a layer allocated for this batch.
                render_dashed_shapes_fresh(&shapes, self.map, tile, self.cancel)
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
