//! Preview layer cache: state, budget and LRU.
//!
//! During playback an animated camera (a transform on a parent group) moves
//! every shape of a scene although most shapes keep their own geometry, paint
//! and relative placement. Such a run of consecutive shapes that only moved
//! rigidly is rasterised once into a bitmap (a [`Layer`]) and later frames
//! draw the bitmap through the motion instead of rasterising paths again
//! (`layer_warp`). The CPU reference renderer is untouched: only display
//! caches in preview mode get a `LayerCache`;
//! exports, `render_png` and MCP never do.
//!
//! A run is reused only while it verifiably equals the pixels' source: same
//! shape count, paint and stroke style, nested group parameters, and every
//! edge equal to the reference edge moved by one common affine map. Anything
//! else (an edited path, a moved limb, a scale or rotation beyond the quality
//! window, content that would leave the bitmap) rebuilds or drops the layer.
//! See `layer_plan` for run selection and `layer_render` for drawing.

use super::layer_geom::IRect;
use super::layer_warp::Bitmap;
use super::shapes::OwnedShape;
use std::collections::{HashMap, HashSet};
use vexel_geometry::Affine;

/// Default cap on bitmap and reference-shape bytes held by one cache.
pub const DEFAULT_BUDGET: usize = 256 * 1024 * 1024;

/// Identity of a cached item: the group list it lives in, its shape range,
/// and whether it is a whole layered group (filter output) rather than a run.
pub(super) type Key = (Option<usize>, usize, usize, bool);

/// A cached rasterisation of one run of shapes at a reference pose.
pub(super) struct Layer {
    pub bitmap: Bitmap,
    /// Unclamped device bounds of everything the run paints at the reference pose.
    pub bounds: IRect,
    /// The shapes as rasterised; verification compares the current ones to these.
    pub shapes: Vec<OwnedShape>,
    pub ref_inverse: Vec<Affine>,
    /// Authored path signature of each shape (see `layer_plan::path_sig`).
    pub sigs: Vec<Option<u64>>,
    pub ctx: u64,
    pub built: u64,
    used: u64,
    bytes: usize,
}

impl Layer {
    pub fn new(
        bitmap: Bitmap,
        bounds: IRect,
        shapes: Vec<OwnedShape>,
        sigs: Vec<Option<u64>>,
        ctx: u64,
        built: u64,
    ) -> Option<Self> {
        let ref_inverse = shapes
            .iter()
            .map(|s| super::layer_geom::inverse(s.transform))
            .collect::<Option<Vec<_>>>()?;
        let edges: usize = shapes.iter().map(|s| s.edges.len()).sum();
        let bytes = bitmap.bytes()
            + edges * std::mem::size_of::<vexel_render::FlatEdge>()
            + shapes.len() * (std::mem::size_of::<OwnedShape>() + 48);
        Some(Self {
            bitmap,
            bounds,
            shapes,
            ref_inverse,
            sigs,
            ctx,
            built,
            used: built,
            bytes,
        })
    }
}

/// Counters for tests, benches and the profile report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayerStats {
    /// Frames drawn with the layer path.
    pub frames: u64,
    /// Layers rasterised (new or at a new pose).
    pub built: u64,
    /// Layers redrawn through a motion without rasterising.
    pub reused: u64,
    /// Layers dropped because their content changed.
    pub dropped: u64,
    /// Layers evicted by the byte budget.
    pub evicted: u64,
    /// Layers not stored because the budget could not fit them.
    pub rejected: u64,
    pub bytes: usize,
    pub peak_bytes: usize,
}

struct Backoff {
    until: u64,
    strikes: u32,
}

/// Layers of one playback session plus what is needed to pick new ones.
pub(super) struct LayerCache {
    pub(super) budget: usize,
    pub(super) frame: u64,
    entries: HashMap<Key, Layer>,
    /// Transform of every scene shape in the previous frame.
    pub(super) prev: Vec<Option<Affine>>,
    /// Runs that looked rigid in the previous frame: candidates this frame.
    pub(super) candidates: HashSet<Key>,
    volatile: HashMap<Key, Backoff>,
    /// Geometry (view, tile size) the layers were rasterised for.
    pub(super) geometry: Option<([u64; 4], u32, u32)>,
    pub(super) stats: LayerStats,
}

impl LayerCache {
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            frame: 0,
            entries: HashMap::new(),
            prev: Vec::new(),
            candidates: HashSet::new(),
            volatile: HashMap::new(),
            geometry: None,
            stats: LayerStats::default(),
        }
    }

    pub fn stats(&self) -> LayerStats {
        self.stats
    }

    /// Forgets the previous frame's motion (the next frame is not its successor).
    pub fn reset_motion(&mut self) {
        self.prev.clear();
        self.candidates.clear();
    }

    /// Drops every layer; the geometry changed.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.volatile.clear();
        self.reset_motion();
        self.stats.bytes = 0;
    }

    pub(super) fn get(&self, key: &Key) -> Option<&Layer> {
        self.entries.get(key)
    }

    pub(super) fn contains(&self, key: &Key) -> bool {
        self.entries.contains_key(key)
    }

    /// Marks a layer as used by the current frame.
    pub(super) fn touch(&mut self, key: &Key) {
        let frame = self.frame;
        if let Some(layer) = self.entries.get_mut(key) {
            layer.used = frame;
        }
    }

    /// Counts one layer redrawn through a motion.
    pub(super) fn note_reuse(&mut self) {
        self.stats.reused += 1;
    }

    /// Removes a layer whose content no longer matches and backs the run off.
    pub(super) fn drop_stale(&mut self, key: &Key) {
        if let Some(layer) = self.entries.remove(key) {
            self.stats.bytes -= layer.bytes;
            self.stats.dropped += 1;
        }
        self.back_off(*key);
    }

    pub(super) fn back_off(&mut self, key: Key) {
        let frame = self.frame;
        let entry = self.volatile.entry(key).or_insert(Backoff {
            until: 0,
            strikes: 0,
        });
        entry.strikes = (entry.strikes + 1).min(5);
        entry.until = frame + (2u64 << entry.strikes);
        // Bound the table: old backoffs are worth nothing.
        if self.volatile.len() > 4096 {
            self.volatile.retain(|_, b| b.until > frame);
        }
    }

    pub(super) fn is_volatile(&self, key: &Key) -> bool {
        self.volatile.get(key).is_some_and(|b| b.until > self.frame)
    }

    /// Stores a freshly rasterised layer, evicting least recently used ones to
    /// stay inside the budget. A layer that cannot fit is not stored.
    pub(super) fn insert(&mut self, key: Key, layer: Layer) {
        if let Some(old) = self.entries.remove(&key) {
            self.stats.bytes -= old.bytes;
        }
        self.stats.built += 1;
        if layer.bytes > self.budget {
            self.stats.rejected += 1;
            return;
        }
        while self.stats.bytes + layer.bytes > self.budget {
            let frame = self.frame;
            let oldest = self
                .entries
                .iter()
                .filter(|(_, l)| l.used < frame)
                .min_by_key(|(_, l)| l.used)
                .map(|(k, _)| *k);
            let Some(oldest) = oldest else {
                self.stats.rejected += 1;
                return;
            };
            if let Some(gone) = self.entries.remove(&oldest) {
                self.stats.bytes -= gone.bytes;
                self.stats.evicted += 1;
            }
        }
        self.stats.bytes += layer.bytes;
        self.stats.peak_bytes = self.stats.peak_bytes.max(self.stats.bytes);
        self.entries.insert(key, layer);
    }
}

/// How a planned run is drawn this frame.
#[derive(Clone, Copy, Debug)]
pub(super) enum Action {
    /// The stored layer through this device-space motion.
    Use(Affine),
    /// Rasterise the run now (new layer or new reference pose).
    Build,
}

/// One run chosen for the layer path this frame.
#[derive(Clone, Copy, Debug)]
pub(super) struct Chunk {
    pub key: Key,
    pub action: Action,
    /// Unclamped device bounds of the run's shapes now.
    pub bounds: IRect,
    /// The layered group this chunk stands for, if it is a group chunk.
    pub group: Option<usize>,
    /// A group chunk whose filter output ignores the group's content.
    pub free: bool,
    /// Hash of the nested group parameters of the run (and filter of a group).
    pub ctx: u64,
    /// The run moved (as opposed to staying put).
    pub moving: bool,
}

/// The runs of one frame, indexed for the renderer's walk.
#[derive(Default)]
pub(super) struct Plan {
    pub chunks: Vec<Chunk>,
    by_parent: HashMap<Option<usize>, Vec<usize>>,
}

impl Plan {
    pub fn push(&mut self, chunk: Chunk) {
        self.by_parent
            .entry(chunk.key.0)
            .or_default()
            .push(self.chunks.len());
        self.chunks.push(chunk);
    }

    pub fn sort(&mut self) {
        let chunks = &self.chunks;
        for list in self.by_parent.values_mut() {
            list.sort_by_key(|&i| chunks[i].key.1);
        }
    }

    /// The run of `parent` starting exactly at shape `pos`.
    pub fn starting_at(&self, parent: Option<usize>, pos: usize) -> Option<Chunk> {
        let list = self.by_parent.get(&parent)?;
        let at = list.partition_point(|&i| self.chunks[i].key.1 < pos);
        let chunk = self.chunks[*list.get(at)?];
        (chunk.key.1 == pos).then_some(chunk)
    }

    /// Start of the first run of `parent` beginning in `from..to`.
    pub fn next_start(&self, parent: Option<usize>, from: usize, to: usize) -> Option<usize> {
        let list = self.by_parent.get(&parent)?;
        let at = list.partition_point(|&i| self.chunks[i].key.1 < from);
        let start = self.chunks[*list.get(at)?].key.1;
        (start < to).then_some(start)
    }

    /// Whether the layer path should draw this frame. Static runs alone gain
    /// nothing over the differential renderer when it can handle the scene.
    pub fn engage(&self, diff_eligible: bool) -> bool {
        !self.chunks.is_empty() && (!diff_eligible || self.chunks.iter().any(|c| c.moving))
    }
}
