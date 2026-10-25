//! Drawing side of the preview layer cache: the planned walk of a group list
//! (cached runs are resampled, everything else is painted as usual) and the
//! rasterisation of a run into a layer.

use super::diff::{DiffCache, device_rect};
use super::layer_cache::{Action, Chunk, Layer, LayerCache, Plan};
use super::layer_geom::{IDENTITY, IRect};
use super::layer_plan::path_sig;
use super::layer_warp::{Bitmap, blend_over, warp_over};
use super::render_scene::{Renderer, Scene};
use super::shapes::{Entry, OwnedShape};
use std::ops::Range;
use vexel_raster::Tile;
use vexel_render::{MAX_GROUP_DEPTH, PreserveAspect, RenderError, ViewBox};
use vexel_scene::{SceneView, ViewportRead};

/// The cache and this frame's plan, carried by a [`Renderer`].
pub(super) struct LayerRun<'c> {
    pub cache: &'c mut LayerCache,
    pub plan: Plan,
}

/// One step of a group list: a stretch of loose shapes or a whole group.
struct Item {
    start: usize,
    end: usize,
    group: Option<usize>,
}

/// Renders `entries` of `view` through the layer cache when the frame has
/// runs worth resampling. `Ok(false)` leaves the tile untouched: the caller
/// renders the exact way.
pub(super) fn try_layers<V: ViewportRead, A: Copy>(
    cache: &mut DiffCache,
    view: &SceneView<'_, V, A>,
    entries: &[Entry<'_>],
    view_box: ViewBox,
    aspect: Option<PreserveAspect>,
    view_key: [u64; 4],
    diff_eligible: bool,
    tile: &mut Tile,
    cancel: &dyn Fn() -> bool,
) -> Result<bool, RenderError> {
    if !cache.preview {
        return Ok(false);
    }
    let Some(layers) = cache.layers.as_mut() else {
        return Ok(false);
    };
    let mut renderer = Renderer::new(view_box, aspect, tile, cancel)?;
    let size = (tile.width(), tile.height());
    let plan = {
        let _plan = crate::profile::phase("lc.plan");
        layers.plan(view, entries, &renderer.geo, size, view_key)
    };
    let Some(plan) = plan else {
        return Ok(false);
    };
    if std::env::var_os("VEXEL_LAYER_DEBUG").is_some() {
        let covered: usize = plan.chunks.iter().map(|c| c.key.2 - c.key.1).sum();
        let list: Vec<String> = plan
            .chunks
            .iter()
            .map(|c| {
                format!(
                    "{}-{}{}{}",
                    c.key.1,
                    c.key.2,
                    if matches!(c.action, Action::Build) {
                        "B"
                    } else {
                        "U"
                    },
                    if c.moving { "m" } else { "" }
                )
            })
            .collect();
        eprintln!(
            "LAYERS shapes {} covered {} engage {} stats {:?} chunks {}",
            entries.len(),
            covered,
            plan.engage(diff_eligible),
            layers.stats(),
            list.join(",")
        );
    }
    if !plan.engage(diff_eligible) {
        return Ok(false);
    }
    renderer.layers = Some(LayerRun {
        cache: layers,
        plan,
    });
    renderer.draw_entries(view, entries, tile)?;
    drop(renderer);
    // The frame holds resampled pixels: it must not seed the exact differential path.
    cache.forget_exact_frame();
    Ok(true)
}

impl Renderer<'_> {
    /// The shapes `range` of group list `parent` as ordered items. With `sub`
    /// the range is a run inside the list: other groups of the list are skipped.
    fn items<V: ViewportRead, A: Copy>(
        scene: &Scene<'_, '_, V, A>,
        parent: Option<usize>,
        range: &Range<usize>,
        sub: bool,
    ) -> Result<Vec<Item>, RenderError> {
        let mut items = Vec::new();
        let mut cursor = range.start;
        for (index, group) in scene.view.groups.iter().enumerate() {
            if group.parent.map(|g| g.index()) != parent {
                continue;
            }
            let first = group.shapes.first;
            let end = group.shapes.end().ok_or(RenderError::InvalidGeometry)?;
            if sub && (first == end || end <= range.start || first >= range.end) {
                continue;
            }
            if first < cursor || end > range.end || first > end {
                return Err(RenderError::InvalidGeometry);
            }
            if first > cursor {
                items.push(Item {
                    start: cursor,
                    end: first,
                    group: None,
                });
            }
            if first != end {
                items.push(Item {
                    start: first,
                    end,
                    group: Some(index),
                });
            }
            cursor = end;
        }
        if cursor < range.end {
            items.push(Item {
                start: cursor,
                end: range.end,
                group: None,
            });
        }
        Ok(items)
    }

    /// [`Renderer::range`] with this frame's runs: cached runs are resampled
    /// into `dest` at their place in the paint order, the rest is painted.
    pub(super) fn range_planned<V: ViewportRead, A: Copy>(
        &mut self,
        scene: &Scene<'_, '_, V, A>,
        range: Range<usize>,
        parent: Option<usize>,
        dest: &mut Tile,
        depth: usize,
    ) -> Result<(), RenderError> {
        if depth > MAX_GROUP_DEPTH {
            return Err(RenderError::LimitExceeded);
        }
        let items = Self::items(scene, parent, &range, false)?;
        let (mut k, mut pos) = (0, range.start);
        while pos < range.end {
            let chunk = self
                .layers
                .as_ref()
                .and_then(|run| run.plan.starting_at(parent, pos));
            if let Some(chunk) = chunk {
                self.chunk(scene, chunk, dest, depth)?;
                pos = chunk.key.2;
                while items.get(k).is_some_and(|item| item.end <= pos) {
                    k += 1;
                }
                continue;
            }
            let Some(item) = items.get(k) else { break };
            match item.group {
                Some(group) => {
                    self.group(scene, group, dest, depth)?;
                    pos = item.end;
                    k += 1;
                }
                None => {
                    let stop = self
                        .layers
                        .as_ref()
                        .and_then(|run| run.plan.next_start(parent, pos + 1, item.end))
                        .unwrap_or(item.end);
                    // Not `direct`: the destination may already hold resampled runs.
                    self.flat(scene, pos..stop, dest, false)?;
                    pos = stop;
                    if pos >= item.end {
                        k += 1;
                    }
                }
            }
        }
        Ok(())
    }

    /// Paints the run `range` of group list `parent` into the blank `dest`.
    fn draw_run<V: ViewportRead, A: Copy>(
        &mut self,
        scene: &Scene<'_, '_, V, A>,
        range: Range<usize>,
        parent: Option<usize>,
        dest: &mut Tile,
        depth: usize,
    ) -> Result<(), RenderError> {
        for item in Self::items(scene, parent, &range, true)? {
            match item.group {
                Some(group) => self.group(scene, group, dest, depth)?,
                None => self.flat(scene, item.start..item.end, dest, false)?,
            }
        }
        Ok(())
    }

    fn chunk<V: ViewportRead, A: Copy>(
        &mut self,
        scene: &Scene<'_, '_, V, A>,
        chunk: Chunk,
        dest: &mut Tile,
        depth: usize,
    ) -> Result<(), RenderError> {
        let cancel = self.cancel;
        if let Some(group) = chunk.group {
            return self.group_chunk(scene, chunk, group, dest, depth);
        }
        match chunk.action {
            Action::Use(motion) => {
                let _use = crate::profile::phase("lc.use");
                let run = self.layers.as_mut().ok_or(RenderError::InvalidGeometry)?;
                run.cache.note_reuse();
                let layer = run
                    .cache
                    .get(&chunk.key)
                    .ok_or(RenderError::InvalidGeometry)?;
                warp_over(dest, &layer.bitmap, motion, 1.0, cancel)
            }
            Action::Build => {
                let layer = {
                    let _build = crate::profile::phase("lc.build");
                    self.build(scene, &chunk, depth)?
                };
                let _use = crate::profile::phase("lc.use");
                warp_over(dest, &layer.bitmap, IDENTITY, 1.0, cancel)?;
                let run = self.layers.as_mut().ok_or(RenderError::InvalidGeometry)?;
                run.cache.insert(chunk.key, layer);
                Ok(())
            }
        }
    }

    /// A filtered group that stands still: its layer after the filter is built
    /// once, then composited with the group's current opacity and blend mode.
    fn group_chunk<V: ViewportRead, A: Copy>(
        &mut self,
        scene: &Scene<'_, '_, V, A>,
        chunk: Chunk,
        group: usize,
        dest: &mut Tile,
        depth: usize,
    ) -> Result<(), RenderError> {
        let params = &scene.view.groups[group];
        if !params.opacity.is_finite() || !(0.0..=1.0).contains(&params.opacity) {
            return Err(RenderError::InvalidGeometry);
        }
        let (opacity, mode) = (params.opacity, params.blend_mode);
        if mode > vexel_scene::MAX_BLEND_MODE {
            return Err(RenderError::InvalidGeometry);
        }
        let cancel = self.cancel;
        let canvas = IRect {
            x0: 0,
            y0: 0,
            x1: self.map.width() as i32,
            y1: self.map.height() as i32,
        };
        if matches!(chunk.action, Action::Build) {
            let _build = crate::profile::phase("lc.build");
            self.reserve()?;
            let tile = self.layer_pixels(scene, group, chunk.key.1..chunk.key.2, depth);
            self.release();
            let tile = tile?;
            let occupied = occupied_rect(&tile);
            let (shapes, sigs) = if chunk.free {
                (Vec::new(), Vec::new())
            } else {
                reference_shapes(scene, chunk.key.1..chunk.key.2)?
            };
            let frame = self.layers.as_ref().map_or(0, |run| run.cache.frame);
            let layer = Layer::new(
                Bitmap {
                    region: canvas,
                    rgba: tile.into_rgba(),
                },
                occupied,
                shapes,
                sigs,
                chunk.ctx,
                frame,
            )
            .ok_or(RenderError::InvalidGeometry)?;
            let run = self.layers.as_mut().ok_or(RenderError::InvalidGeometry)?;
            run.cache.insert(chunk.key, layer);
        } else if let Some(run) = self.layers.as_mut() {
            run.cache.note_reuse();
        }
        let run = self.layers.as_mut().ok_or(RenderError::InvalidGeometry)?;
        let Some(layer) = run.cache.get(&chunk.key) else {
            // Over the byte budget: the layer was not stored. Draw the group exactly.
            return self.group(scene, group, dest, depth);
        };
        let _use = crate::profile::phase("lc.use");
        blend_over(
            dest,
            &layer.bitmap,
            opacity,
            mode,
            layer.bounds.intersect(canvas),
            cancel,
        )
    }

    /// Rasterises a run at the current pose into a layer. The bitmap covers
    /// the run's bounds, limited to the canvas plus a margin for camera moves.
    fn build<V: ViewportRead, A: Copy>(
        &mut self,
        scene: &Scene<'_, '_, V, A>,
        chunk: &Chunk,
        depth: usize,
    ) -> Result<Layer, RenderError> {
        let (width, height) = (self.map.width() as i32, self.map.height() as i32);
        let reach = IRect {
            x0: 0,
            y0: 0,
            x1: width,
            y1: height,
        }
        .grow(width / 4, height / 4);
        let region = chunk.bounds.intersect(reach);
        if region.is_empty() {
            return Err(RenderError::InvalidGeometry);
        }
        let (rw, rh) = (region.width() as u32, region.height() as u32);
        let mut tile = super::blank_tile(rw, rh).map_err(|_| RenderError::LimitExceeded)?;
        // A view that maps user space onto `tile` with the canvas' own scale,
        // shifted so the tile's pixel (0, 0) is the canvas pixel `region.{x0,y0}`.
        let g = self.geo;
        let view = ViewBox {
            x: g.view.x - (g.offset_x - f64::from(region.x0)) / g.scale_x,
            y: g.view.y - (g.offset_y - f64::from(region.y0)) / g.scale_y,
            width: f64::from(rw) / g.scale_x,
            height: f64::from(rh) / g.scale_y,
        };
        let aspect = PreserveAspect {
            align_x: 0.0,
            align_y: 0.0,
            none: true,
            slice: false,
        };
        let mut inner = Renderer::new(view, Some(aspect), &tile, self.cancel)?;
        let (start, end) = (chunk.key.1, chunk.key.2);
        let mut bounds = vec![None; scene.entries.len()];
        for (offset, entry) in scene.entries[start..end].iter().enumerate() {
            let Entry::Draw(p) = entry else {
                return Err(RenderError::InvalidGeometry);
            };
            bounds[start + offset] = device_rect(&p.shape, &inner.geo, rw, rh);
        }
        let (shapes, sigs) = reference_shapes(scene, start..end)?;
        let inner_scene = Scene {
            view: scene.view,
            entries: scene.entries,
            // Cacheable shapes carry no clip.
            clips: &[],
            bounds: &bounds,
        };
        inner.draw_run(&inner_scene, start..end, chunk.key.0, &mut tile, depth)?;
        let frame = self.layers.as_ref().map_or(0, |run| run.cache.frame);
        Layer::new(
            Bitmap {
                region,
                rgba: tile.into_rgba(),
            },
            chunk.bounds,
            shapes,
            sigs,
            chunk.ctx,
            frame,
        )
        .ok_or(RenderError::InvalidGeometry)
    }
}

/// Clones of the shapes `range` and their authored path signatures.
fn reference_shapes<V: ViewportRead, A: Copy>(
    scene: &Scene<'_, '_, V, A>,
    range: Range<usize>,
) -> Result<(Vec<OwnedShape>, Vec<Option<u64>>), RenderError> {
    let mut shapes = Vec::with_capacity(range.len());
    let mut sigs = Vec::with_capacity(range.len());
    for entry in &scene.entries[range] {
        let Entry::Draw(p) = entry else {
            return Err(RenderError::InvalidGeometry);
        };
        shapes.push(p.shape.clone());
        sigs.push(path_sig(scene.view, p.shape.scene_index));
    }
    Ok((shapes, sigs))
}

/// Pixel rectangle covering every pixel of `tile` with alpha.
fn occupied_rect(tile: &Tile) -> IRect {
    let (w, h) = (tile.width() as usize, tile.height() as usize);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (y, row) in tile.rgba().chunks_exact(w * 4).enumerate() {
        let Some(first) = row.chunks_exact(4).position(|p| p[3] != 0) else {
            continue;
        };
        let last = row
            .chunks_exact(4)
            .rposition(|p| p[3] != 0)
            .unwrap_or(first);
        x0 = x0.min(first);
        x1 = x1.max(last + 1);
        y0 = y0.min(y);
        y1 = y + 1;
    }
    if x0 >= x1 || y0 >= y1 {
        return IRect::EMPTY;
    }
    IRect {
        x0: x0 as i32,
        y0: y0 as i32,
        x1: x1 as i32,
        y1: y1 as i32,
    }
}
