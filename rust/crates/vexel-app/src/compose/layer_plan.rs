//! Run selection of the preview layer cache.
//!
//! Every frame the scene is walked group list by group list. Consecutive
//! shapes whose transforms all changed by the same affine map since the
//! previous frame (an animated ancestor) form a candidate run; whole nested
//! groups join a run when their own shapes agree and they carry no filter,
//! mask or blend mode. Runs with a stored layer are verified against it;
//! runs seen as candidates twice in a row and worth the memory are rebuilt.

use super::layer_cache::{Action, Chunk, Key, LayerCache, Plan};
use super::layer_geom::{
    IDENTITY, IRect, apply, device_motion, inverse, mul, near, shape_bounds, singular_values,
};
use super::layers::Geo;
use super::shapes::{Entry, OwnedShape};
use std::collections::HashSet;
use vexel_geometry::Affine;
use vexel_scene::{SceneView, ViewportRead};

/// Largest scale change a layer is resampled through (fraction of 1).
const SCALE_LIMIT: f64 = 0.06;
/// Largest rotation a layer is resampled through, radians.
const ROTATION_LIMIT: f64 = 0.26;
/// Edge endpoint tolerance when comparing against the reference, device pixels.
const EDGE_TOLERANCE: f64 = 0.05;
/// Fixed cost of rasterising one shape, in pixels of fill.
const SHAPE_COST: u64 = 3000;
/// Cost of one flattened edge, in pixels of fill.
const EDGE_COST: u64 = 30;
/// Extra cost of running a filter chain over a layer.
const FILTER_COST: u64 = 1_000_000;
/// A run must cost this many times its resampling to be cached.
const WORTH: u64 = 3;

enum Verdict {
    Use { device: Affine, user: Affine },
    Rebuild,
    Stale,
}

/// Group lists, motion segments and per-shape facts of one frame.
struct Walker<'a, V, A> {
    view: &'a SceneView<'a, V, A>,
    entries: &'a [Entry<'a>],
    geo: &'a Geo,
    size: (u32, u32),
    motion: Vec<Option<Affine>>,
    seg: Vec<u32>,
    bad: Vec<u32>,
    dirty_group: Vec<bool>,
    inner_dirty: Vec<bool>,
    children: Vec<Vec<usize>>,
    by_first: Vec<usize>,
    cache: &'a mut LayerCache,
    plan: Plan,
    seen: HashSet<Key>,
    run: Option<(usize, usize, u32)>,
}

impl LayerCache {
    /// Chooses the runs to draw through the layer path this frame; `None`
    /// when the scene's group structure is not one the renderer accepts.
    pub(super) fn plan<V: ViewportRead, A: Copy>(
        &mut self,
        view: &SceneView<'_, V, A>,
        entries: &[Entry<'_>],
        geo: &Geo,
        size: (u32, u32),
        view_key: [u64; 4],
    ) -> Option<Plan> {
        let key = (view_key, size.0, size.1);
        if self.geometry != Some(key) {
            self.clear();
            self.geometry = Some(key);
        }
        self.frame += 1;
        self.stats.frames += 1;
        let now: Vec<Option<Affine>> = entries
            .iter()
            .map(|e| match e {
                Entry::Draw(p) if finite(p.shape.transform) => Some(p.shape.transform),
                _ => None,
            })
            .collect();
        let motion = self.motion(&now);
        let mut seg = Vec::with_capacity(motion.len());
        let mut id = 0u32;
        for (i, d) in motion.iter().enumerate() {
            let same = i > 0 && matches!((d, &motion[i - 1]), (Some(a), Some(b)) if near(*a, *b));
            if !same {
                id += 1;
            }
            seg.push(id);
        }
        let mut bad = Vec::with_capacity(entries.len() + 1);
        bad.push(0);
        for e in entries {
            let last = bad.last().copied().unwrap_or(0);
            bad.push(last + u32::from(!cacheable(e)));
        }
        let groups = view.groups;
        let mut dirty_group = vec![false; groups.len()];
        let mut inner_dirty = vec![false; groups.len()];
        let mut children = vec![Vec::new(); groups.len() + 1];
        for (g, group) in groups.iter().enumerate() {
            let parent = group.parent.map_or(groups.len(), |p| p.index());
            children.get_mut(parent)?.push(g);
            if group.filter.is_some() || group.mask.is_some() || group.blend_mode != 0 {
                let mut up = groups[g].parent.map(|p| p.index());
                while let Some(c) = up {
                    if std::mem::replace(inner_dirty.get_mut(c)?, true) {
                        break;
                    }
                    up = groups.get(c)?.parent.map(|p| p.index());
                }
                let mut at = Some(g);
                while let Some(c) = at {
                    if std::mem::replace(dirty_group.get_mut(c)?, true) {
                        break;
                    }
                    at = groups.get(c)?.parent.map(|p| p.index());
                }
            }
        }
        let mut by_first: Vec<usize> = (0..groups.len()).collect();
        by_first.sort_by_key(|&g| {
            (
                groups[g].shapes.first,
                std::cmp::Reverse(groups[g].shapes.count),
            )
        });
        let mut walker = Walker {
            view,
            entries,
            geo,
            size,
            motion,
            seg,
            bad,
            dirty_group,
            inner_dirty,
            children,
            by_first,
            cache: &mut *self,
            plan: Plan::default(),
            seen: HashSet::new(),
            run: None,
        };
        walker.walk(None, 0..entries.len(), 0)?;
        let Walker { mut plan, seen, .. } = walker;
        plan.sort();
        self.candidates = seen;
        self.prev = now;
        Some(plan)
    }

    /// Motion of every shape since the previous frame, where defined.
    fn motion(&self, now: &[Option<Affine>]) -> Vec<Option<Affine>> {
        if self.prev.len() != now.len() {
            return vec![None; now.len()];
        }
        now.iter()
            .zip(&self.prev)
            .map(|(n, p)| Some(mul((*n)?, inverse((*p)?)?)))
            .collect()
    }
}

fn finite(m: Affine) -> bool {
    [m.a, m.b, m.c, m.d, m.e, m.f].iter().all(|v| v.is_finite())
}

/// Whether a shape's pixels depend on nothing but its own geometry and paint.
fn cacheable(entry: &Entry<'_>) -> bool {
    match entry {
        Entry::Draw(p) => {
            p.clip.is_none()
                && p.fill.is_none()
                && p.stroke.is_none()
                && p.image.is_none()
                && !p.shape.fill_pattern
                && !p.shape.stroke_pattern
        }
        Entry::Skip(_) => false,
    }
}

impl<V: ViewportRead, A: Copy> Walker<'_, V, A> {
    /// Walks the shapes `range` of one group list (`parent`), flushing runs.
    fn walk(
        &mut self,
        parent: Option<usize>,
        range: std::ops::Range<usize>,
        depth: usize,
    ) -> Option<()> {
        if depth > vexel_render::MAX_GROUP_DEPTH {
            return None;
        }
        let slot = parent.unwrap_or(self.view.groups.len());
        let mut cursor = range.start;
        for k in 0..self.children[slot].len() {
            let g = self.children[slot][k];
            let group = &self.view.groups[g];
            let (first, end) = (group.shapes.first, group.shapes.end()?);
            if first < cursor || end > range.end || first > end {
                return None;
            }
            for i in cursor..first {
                self.shape_unit(parent, i);
            }
            cursor = end;
            if first == end {
                continue;
            }
            let rigid = !self.dirty_group[g]
                && self.bad[end] == self.bad[first]
                && self.motion[first].is_some()
                && self.seg[first] == self.seg[end - 1];
            if rigid {
                self.extend(parent, first, end, self.seg[first]);
                continue;
            }
            self.flush(parent);
            // A filtered group that stands still: its filter output is reusable.
            let filtered = group.filter.is_some()
                && group.mask.is_none()
                && !self.inner_dirty[g]
                && self.bad[end] == self.bad[first]
                && self.motion[first].is_some_and(|d| near(d, IDENTITY))
                && self.seg[first] == self.seg[end - 1];
            let free = group.filter.is_some() && group.mask.is_none() && self.source_free(g);
            if !((free || filtered) && self.decide(parent, first, end, Some(g), free)) {
                self.walk(Some(g), first..end, depth + 1)?;
            }
        }
        for i in cursor..range.end {
            self.shape_unit(parent, i);
        }
        self.flush(parent);
        Some(())
    }

    fn shape_unit(&mut self, parent: Option<usize>, i: usize) {
        if self.bad[i + 1] == self.bad[i] && self.motion[i].is_some() {
            self.extend(parent, i, i + 1, self.seg[i]);
        } else {
            self.flush(parent);
        }
    }

    fn extend(&mut self, parent: Option<usize>, start: usize, end: usize, seg: u32) {
        match &mut self.run {
            Some((_, e, s)) if *e == start && *s == seg => *e = end,
            _ => {
                self.flush(parent);
                self.run = Some((start, end, seg));
            }
        }
    }

    fn flush(&mut self, parent: Option<usize>) {
        if let Some((start, end, _)) = self.run.take() {
            self.decide(parent, start, end, None, false);
        }
    }

    /// Decides one finished run: use its layer, rebuild it, or leave it.
    fn decide(
        &mut self,
        parent: Option<usize>,
        start: usize,
        end: usize,
        group: Option<usize>,
        free: bool,
    ) -> bool {
        let key = (parent, start, end, group.is_some());
        let mut ctx = self.ctx(start, end, group);
        if let Some(g) = group {
            ctx ^= self.filter_sig(g).rotate_left(31);
            if free {
                // The output does not depend on the children at all.
                ctx = self.filter_sig(g);
            }
        }
        let moving = self.motion[start].is_some_and(|d| !near(d, IDENTITY));
        if self.cache.contains(&key) {
            match self.verify(&key, ctx, free) {
                Verdict::Use { device, user } => {
                    let bounds = self.cache.get(&key).map_or(IRect::EMPTY, |l| l.bounds);
                    self.cache.touch(&key);
                    self.plan.push(Chunk {
                        key,
                        action: Action::Use(device),
                        bounds,
                        group,
                        free,
                        ctx,
                        moving: !near(user, IDENTITY),
                    });
                    return true;
                }
                Verdict::Rebuild => {
                    let recent = self
                        .cache
                        .get(&key)
                        .is_some_and(|l| self.cache.frame.saturating_sub(l.built) <= 2);
                    if recent {
                        // Rebuilt right after being built: the motion is not rigid enough.
                        self.cache.drop_stale(&key);
                        return false;
                    }
                }
                Verdict::Stale => {
                    self.cache.drop_stale(&key);
                    return false;
                }
            }
        } else {
            if self.cache.is_volatile(&key) {
                return false;
            }
            if !self.cache.candidates.contains(&key) {
                self.seen.insert(key);
                return false;
            }
        }
        self.seen.insert(key);
        let found = if free {
            Some(IRect {
                x0: 0,
                y0: 0,
                x1: self.size.0 as i32,
                y1: self.size.1 as i32,
            })
        } else {
            self.worth(start, end, group.is_some())
        };
        match found {
            Some(bounds) => {
                self.plan.push(Chunk {
                    key,
                    action: Action::Build,
                    bounds,
                    group,
                    free,
                    ctx,
                    moving,
                });
                true
            }
            None => {
                self.cache.back_off(key);
                false
            }
        }
    }

    /// Whether a group's filter output cannot depend on the group's content:
    /// every primitive chain ends in generators (turbulence, flood) and the
    /// region is given in user space.
    fn source_free(&self, g: usize) -> bool {
        use vexel_scene::{FilterInput, FilterKind};
        let view = self.view;
        let Some(filter) = self.view.groups[g].filter.and_then(|id| view.filter(id)) else {
            return false;
        };
        if filter.filter_units_object_bounding_box
            || filter.primitive_units_object_bounding_box
            || !filter.custom_region
        {
            return false;
        }
        let Some(chain) = filter.primitives.slice(view.filter_primitives) else {
            return false;
        };
        let mut dependent: Vec<bool> = Vec::with_capacity(chain.len());
        for (i, p) in chain.iter().enumerate() {
            let of = |input: FilterInput, done: &[bool]| match input {
                FilterInput::Previous if i == 0 => true,
                FilterInput::Previous => done[i - 1],
                FilterInput::Result(j) => done.get(j).copied().unwrap_or(true),
                FilterInput::Graphic | FilterInput::Alpha => true,
            };
            let dep = match p.kind {
                FilterKind::Turbulence | FilterKind::Flood => false,
                FilterKind::Blend | FilterKind::Composite => {
                    of(p.input, &dependent) || of(p.input2, &dependent)
                }
                FilterKind::Merge => p.merge_inputs[..usize::from(p.merge_count).min(8)]
                    .iter()
                    .any(|input| of(*input, &dependent)),
                _ => of(p.input, &dependent),
            };
            dependent.push(dep);
        }
        dependent.last().is_some_and(|dep| !dep)
    }

    /// Hash of a group's filter definition (source offsets excluded).
    fn filter_sig(&self, g: usize) -> u64 {
        let view = self.view;
        let Some(id) = view.groups[g].filter else {
            return 0;
        };
        let Some(filter) = view.filter(id) else {
            return 1;
        };
        let mut text = format!("{filter:?}");
        for primitive in filter
            .primitives
            .slice(view.filter_primitives)
            .unwrap_or(&[])
        {
            let mut primitive = *primitive;
            primitive.source_offset = 0;
            text.push_str(&format!("{primitive:?}"));
        }
        text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }

    /// Hash of the nested group parameters inside the shapes `start..end`.
    fn ctx(&self, start: usize, end: usize, skip: Option<usize>) -> u64 {
        let groups = self.view.groups;
        let at = self
            .by_first
            .partition_point(|&g| groups[g].shapes.first < start);
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        let mut mix = |v: u64| h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(23);
        for &g in &self.by_first[at..] {
            let group = &groups[g];
            if group.shapes.first >= end {
                break;
            }
            if skip != Some(g) && group.shapes.end().is_some_and(|e| e <= end) {
                mix((group.shapes.first - start) as u64);
                mix(group.shapes.count as u64);
                mix(group.opacity.to_bits());
            }
        }
        h
    }

    /// Device bounds of the run when caching it pays; `None` otherwise.
    fn worth(&self, start: usize, end: usize, filtered: bool) -> Option<IRect> {
        let canvas = IRect {
            x0: 0,
            y0: 0,
            x1: self.size.0 as i32,
            y1: self.size.1 as i32,
        };
        let reach = canvas.grow(self.size.0 as i32 / 4, self.size.1 as i32 / 4);
        let mut bounds = IRect::EMPTY;
        let (mut raster, mut edges) = (0u64, 0u64);
        for entry in &self.entries[start..end] {
            let Entry::Draw(p) = entry else { return None };
            let Some(b) = shape_bounds(&p.shape, self.geo) else {
                continue;
            };
            bounds = bounds.union(b);
            let seen = b.intersect(reach);
            raster += SHAPE_COST + (seen.width() * seen.height()) as u64;
            edges += p.shape.edges.len() as u64;
        }
        raster += edges * EDGE_COST;
        if filtered {
            raster += FILTER_COST;
        }
        let region = bounds.intersect(reach);
        if region.is_empty() {
            return None;
        }
        let shown = bounds.intersect(canvas);
        let resample = (shown.width() * shown.height()) as u64 + 5000;
        (raster >= WORTH * resample).then_some(bounds)
    }

    /// Compares the run to its stored layer and works out the motion.
    fn verify(&self, key: &Key, ctx: u64, free: bool) -> Verdict {
        let Some(layer) = self.cache.get(key) else {
            return Verdict::Stale;
        };
        let (start, end) = (key.1, key.2);
        if free {
            return if layer.shapes.is_empty() && layer.ctx == ctx {
                Verdict::Use {
                    device: IDENTITY,
                    user: IDENTITY,
                }
            } else {
                Verdict::Stale
            };
        }
        if layer.shapes.len() != end - start || layer.ctx != ctx {
            return Verdict::Stale;
        }
        let mut user: Option<Affine> = None;
        let tolerance =
            EDGE_TOLERANCE / self.geo.scale_x.abs().max(self.geo.scale_y.abs()).max(1e-9);
        for (j, entry) in self.entries[start..end].iter().enumerate() {
            let (Entry::Draw(now), Some(inv)) = (entry, layer.ref_inverse.get(j)) else {
                return Verdict::Stale;
            };
            let (now, was) = (&now.shape, &layer.shapes[j]);
            let d = mul(now.transform, *inv);
            match user {
                None => user = Some(d),
                Some(first) if !near(first, d) => {
                    dbg_stale(key, j, "motion", &format!("{first:?} vs {d:?}"));
                    return Verdict::Stale;
                }
                Some(_) => {}
            }
            if !same_paint(now, was) {
                dbg_stale(key, j, "paint", "");
                return Verdict::Stale;
            }
            let signed = layer.sigs.get(j).copied().flatten();
            let same = match signed {
                // Same authored path numbers: only the flattening may differ
                // (it depends on the zoom), so compare the ends coarsely.
                Some(sig) => {
                    path_sig(self.view, now.scene_index) == Some(sig)
                        && same_ends(now, was, d, tolerance * 10.0)
                }
                None => same_edges(now, was, d, tolerance),
            };
            if !same {
                dbg_stale(
                    key,
                    j,
                    "edges",
                    &format!("{} vs {}", now.edges.len(), was.edges.len()),
                );
                return Verdict::Stale;
            }
        }
        let Some(user) = user else {
            return Verdict::Stale;
        };
        let Some(device) = device_motion(self.geo, user) else {
            return Verdict::Stale;
        };
        if key.3 {
            // A filtered group is reused only where it stands: its output
            // is then the pixels a fresh render would produce.
            return if near(user, IDENTITY) {
                Verdict::Use {
                    device: IDENTITY,
                    user,
                }
            } else {
                Verdict::Stale
            };
        }
        let (hi, lo) = singular_values(device);
        let angle = device.b.atan2(device.a).abs();
        if hi > 1.0 + SCALE_LIMIT || lo < 1.0 - SCALE_LIMIT || angle > ROTATION_LIMIT {
            return Verdict::Rebuild;
        }
        let canvas = IRect {
            x0: 0,
            y0: 0,
            x1: self.size.0 as i32,
            y1: self.size.1 as i32,
        };
        let Some(back) = inverse(device).and_then(|i| canvas.map(i)) else {
            return Verdict::Stale;
        };
        if !layer
            .bitmap
            .region
            .contains(back.grow(1, 1).intersect(layer.bounds))
        {
            return Verdict::Rebuild;
        }
        Verdict::Use { device, user }
    }
}

/// Everything that decides colour and stroking except the geometry.
fn same_paint(a: &OwnedShape, b: &OwnedShape) -> bool {
    a.subpath_closed == b.subpath_closed
        && a.stroke_closed == b.stroke_closed
        && a.evenodd == b.evenodd
        && a.filled == b.filled
        && a.fill_rgba == b.fill_rgba
        && a.stroke == b.stroke
        && a.dashes == b.dashes
        && a.fill_gradient == b.fill_gradient
        && a.stroke_gradient == b.stroke_gradient
        && a.subpaths.len() == b.subpaths.len()
}

/// Hash of the authored path commands and numbers of scene shape `index`:
/// what the shape's outline is made of, independent of transform and zoom.
pub(super) fn path_sig<V: ViewportRead, A: Copy>(
    view: &SceneView<'_, V, A>,
    index: usize,
) -> Option<u64> {
    let segments = view.path_segments;
    let first = segments.partition_point(|s| s.shape.index() < index);
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(23);
    let mut any = false;
    for s in &segments[first..] {
        if s.shape.index() != index {
            break;
        }
        any = true;
        mix(u64::from(s.path.command));
        let numbers = s.path.first_number..s.path.first_number.checked_add(s.path.number_count)?;
        for n in view.path_numbers.get(numbers)? {
            mix(n.value.to_bits());
        }
    }
    any.then_some(h)
}

/// Whether the first and last edge of `now` are those of `was` moved by `d`.
fn same_ends(now: &OwnedShape, was: &OwnedShape, d: Affine, tolerance: f64) -> bool {
    let (Some(n0), Some(w0), Some(n1), Some(w1)) = (
        now.edges.first(),
        was.edges.first(),
        now.edges.last(),
        was.edges.last(),
    ) else {
        return now.edges.is_empty() == was.edges.is_empty();
    };
    let close = |(x, y): (f64, f64), a: f64, b: f64| {
        (x - a).abs() <= tolerance && (y - b).abs() <= tolerance
    };
    close(apply(d, w0.x0, w0.y0), n0.x0, n0.y0) && close(apply(d, w1.x1, w1.y1), n1.x1, n1.y1)
}

/// Whether `now` is `was` moved by `d`, edge by edge.
fn same_edges(now: &OwnedShape, was: &OwnedShape, d: Affine, tolerance: f64) -> bool {
    if now.edges.len() != was.edges.len() || now.subpaths != was.subpaths {
        return false;
    }
    now.edges.iter().zip(&was.edges).all(|(n, w)| {
        let (x0, y0) = apply(d, w.x0, w.y0);
        let (x1, y1) = apply(d, w.x1, w.y1);
        (x0 - n.x0).abs() <= tolerance
            && (y0 - n.y0).abs() <= tolerance
            && (x1 - n.x1).abs() <= tolerance
            && (y1 - n.y1).abs() <= tolerance
    })
}

fn dbg_stale(key: &Key, j: usize, why: &str, extra: &str) {
    if std::env::var_os("VEXEL_LAYER_DEBUG").is_some() {
        eprintln!("STALE {key:?} shape +{j} {why} {extra}");
    }
}
