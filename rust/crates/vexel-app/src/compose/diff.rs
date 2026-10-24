//! Differential playback rendering.
//!
//! Consecutive animation ticks differ in a handful of shapes. The previous
//! frame is kept; each tick finds the shapes (and the batching context they
//! are painted in) that changed, repaints only the shapes that touch the
//! union of old and new bounds of those, and copies everything else from the
//! previous frame. Pixels are byte-identical to a full render because the
//! rasterizer and the source-over compositor are per pixel, shapes outside the
//! dirty rectangle cannot reach it, and batching/layer decisions are taken
//! from the whole scene exactly as a full render takes them.
//!
//! Scenes with masks, filters, patterns or images are never diffed (their
//! pixels depend on other pixels or on tiles that are not compared).

use super::layers::Geo;
use super::render_scene::{Drawn, Only, Renderer, Skipped, batch_runs};
use super::shapes::{Entry, OwnedShape, prepare};
use vexel_raster::Tile;
use vexel_render::{PreserveAspect, RenderError, UnsupportedKind, ViewBox};
use vexel_scene::{SceneView, ViewportRead};

/// Per-revision state of the differential renderer.
#[derive(Default)]
pub struct DiffCache {
    disabled: bool,
    prev: Option<Prev>,
    partials: u32,
}

impl DiffCache {
    /// Frames so far that repainted only part of the scene.
    pub fn partial_frames(&self) -> u32 {
        self.partials
    }

    /// True once the scene proved undiffable (every frame is a full render).
    pub fn disabled(&self) -> bool {
        self.disabled
    }
}

struct Prev {
    width: u32,
    height: u32,
    view: [u64; 4],
    clips: u64,
    frame: Vec<u8>,
    keys: Vec<Key>,
    bounds: Vec<Option<Rect>>,
    ctx: Vec<u64>,
}

#[derive(PartialEq)]
enum Key {
    Skip(UnsupportedKind),
    Draw(Box<OwnedShape>, Option<usize>),
}

/// Pixel rectangle, `x1`/`y1` exclusive.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Rect {
    pub(super) x0: u32,
    pub(super) y0: u32,
    pub(super) x1: u32,
    pub(super) y1: u32,
}

impl Rect {
    pub(super) fn scissor(self) -> [u32; 4] {
        [self.x0, self.y0, self.x1, self.y1]
    }

    fn union(self, o: Self) -> Self {
        Self {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }

    pub(super) fn meets(self, o: Self) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }

    fn area(self) -> u64 {
        u64::from(self.x1 - self.x0) * u64::from(self.y1 - self.y0)
    }
}

fn mix(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(23)
}

fn mix_f(h: u64, v: f64) -> u64 {
    mix(h, v.to_bits())
}

fn clip_hash<V: ViewportRead, A: Copy>(view: &SceneView<'_, V, A>) -> u64 {
    let mut h = mix(0xcbf2_9ce4_8422_2325, view.clips.len() as u64);
    for clip in view.clips {
        for e in &clip.geometry.edges {
            h = mix_f(mix_f(mix_f(mix_f(h, e.a.x), e.a.y), e.b.x), e.b.y);
        }
        for byte in format!(
            "{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
            clip.geometry.shapes,
            clip.parent,
            clip.owner_node,
            clip.owner_before_suffix,
            clip.object_bounding_box,
            clip.bounds,
            clip.user_inverse
        )
        .bytes()
        {
            h = mix(h, u64::from(byte));
        }
    }
    h
}

/// Whether the scene can be diffed at all.
fn eligible<V: ViewportRead, A: Copy>(view: &SceneView<'_, V, A>, entries: &[Entry<'_>]) -> bool {
    view.masks.is_empty()
        && view.filters.is_empty()
        && view.patterns.is_empty()
        && view.images.is_empty()
        && view
            .groups
            .iter()
            .all(|g| g.filter.is_none() && g.mask.is_none() && g.blend_mode == 0)
        && entries.iter().all(|e| match e {
            Entry::Skip(_) => true,
            Entry::Draw(p) => {
                p.fill.is_none()
                    && p.stroke.is_none()
                    && p.image.is_none()
                    && !p.shape.fill_pattern
                    && !p.shape.stroke_pattern
            }
        })
}

/// Conservative device rectangle every pixel of `shape` lies in.
fn device_rect(shape: &OwnedShape, geo: &Geo, width: u32, height: u32) -> Option<Rect> {
    if shape.edges.is_empty() {
        return None;
    }
    let (mut l, mut t, mut r, mut b) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for e in &shape.edges {
        for (x, y) in [(e.x0, e.y0), (e.x1, e.y1)] {
            let (dx, dy) = geo.to_device(x, y);
            l = l.min(dx);
            r = r.max(dx);
            t = t.min(dy);
            b = b.max(dy);
        }
    }
    let mut pad = 2.0;
    if let Some(style) = shape.stroke {
        let tr = shape.transform;
        let norm = (tr.a * tr.a + tr.b * tr.b + tr.c * tr.c + tr.d * tr.d).sqrt();
        let reach = style.width.abs() / 2.0 * style.miter_limit.max(2.0) * norm;
        pad += reach * geo.scale_x.abs().max(geo.scale_y.abs());
    }
    if !(l.is_finite() && t.is_finite() && r.is_finite() && b.is_finite() && pad.is_finite()) {
        return Some(Rect {
            x0: 0,
            y0: 0,
            x1: width,
            y1: height,
        });
    }
    let clamp = |v: f64, max: u32| v.clamp(0.0, f64::from(max));
    let rect = Rect {
        x0: clamp((l - pad).floor(), width) as u32,
        y0: clamp((t - pad).floor(), height) as u32,
        x1: clamp((r + pad).ceil(), width) as u32,
        y1: clamp((b + pad).ceil(), height) as u32,
    };
    (rect.x0 < rect.x1 && rect.y0 < rect.y1).then_some(rect)
}

/// Batch and group context of every drawn shape, mirroring `Renderer::range`.
fn plan<V: ViewportRead, A: Copy>(
    view: &SceneView<'_, V, A>,
    entries: &[Entry<'_>],
) -> Result<Vec<u64>, RenderError> {
    let mut out = vec![0; entries.len()];
    plan_range(
        view,
        entries,
        0..entries.len(),
        None,
        0,
        view.groups.is_empty(),
        &mut out,
    )?;
    Ok(out)
}

fn plan_range<V: ViewportRead, A: Copy>(
    view: &SceneView<'_, V, A>,
    entries: &[Entry<'_>],
    range: std::ops::Range<usize>,
    parent: Option<usize>,
    chain: u64,
    direct: bool,
    out: &mut [u64],
) -> Result<(), RenderError> {
    let mut cursor = range.start;
    for (index, group) in view.groups.iter().enumerate() {
        if group.parent.map(|g| g.index()) != parent {
            continue;
        }
        let first = group.shapes.first;
        let end = group.shapes.end().ok_or(RenderError::InvalidGeometry)?;
        if first < cursor || end > range.end || first > end {
            return Err(RenderError::InvalidGeometry);
        }
        plan_flat(entries, cursor..first, chain, direct, out)?;
        cursor = end;
        if first != end {
            let mut inner = mix(chain, first as u64);
            inner = mix(mix(inner, end as u64), 0x6772_6f75_70);
            inner = mix_f(inner, group.opacity);
            plan_range(view, entries, first..end, Some(index), inner, false, out)?;
        }
    }
    plan_flat(entries, cursor..range.end, chain, direct, out)
}

fn plan_flat(
    entries: &[Entry<'_>],
    range: std::ops::Range<usize>,
    chain: u64,
    direct: bool,
    out: &mut [u64],
) -> Result<(), RenderError> {
    let batches = batch_runs(entries, range)?;
    let layered = !(direct && batches.len() == 1);
    for (kind, items) in &batches {
        let (Some(first), Some(last)) = (items.first(), items.last()) else {
            continue;
        };
        let mut h = mix(chain, first.shape.scene_index as u64);
        h = mix(h, last.shape.scene_index as u64);
        h = mix(h, items.len() as u64);
        h = mix(h, *kind as u64 + 1);
        h = mix(h, u64::from(layered));
        for item in items {
            out[item.shape.scene_index] = h;
        }
    }
    Ok(())
}

/// [`super::render_scene::render_scene`] for playback frames (never strict),
/// repainting only what changed since the previous call on `cache`.
pub(crate) fn render_scene_diff<V: ViewportRead, A: Copy>(
    view: &SceneView<'_, V, A>,
    view_box: ViewBox,
    aspect: Option<PreserveAspect>,
    tile: &mut Tile,
    cancel: &dyn Fn() -> bool,
    cache: &mut DiffCache,
) -> Result<Drawn, RenderError> {
    let (width, height) = (tile.width(), tile.height());
    let mut renderer = Renderer::new(view_box, aspect, tile, cancel)?;
    let entries = prepare(view)?;
    if cache.disabled || !eligible(view, &entries) {
        cache.disabled = true;
        cache.prev = None;
        renderer.draw_entries(view, &entries, tile)?;
        return Ok(drawn_of(entries));
    }
    let ctx = plan(view, &entries)?;
    let bounds: Vec<Option<Rect>> = entries
        .iter()
        .map(|e| match e {
            Entry::Draw(p) => device_rect(&p.shape, &renderer.geo, width, height),
            Entry::Skip(_) => None,
        })
        .collect();
    let view_key = [
        view_box.x.to_bits(),
        view_box.y.to_bits(),
        view_box.width.to_bits() ^ view_box.height.to_bits().rotate_left(17),
        aspect.map_or(0, |a| {
            mix_f(
                mix_f(mix(u64::from(a.none), u64::from(a.slice)), a.align_x),
                a.align_y,
            )
        }),
    ];
    let clips = clip_hash(view);
    let usable = cache.prev.as_ref().filter(|p| {
        p.width == width
            && p.height == height
            && p.view == view_key
            && p.clips == clips
            && p.keys.len() == entries.len()
    });
    let mut dirty: Vec<Rect> = Vec::new();
    if let Some(prev) = usable {
        for (i, entry) in entries.iter().enumerate() {
            let same = prev.ctx[i] == ctx[i]
                && match (&prev.keys[i], entry) {
                    (Key::Skip(a), Entry::Skip(b)) => a == b,
                    (Key::Draw(a, ac), Entry::Draw(p)) => **a == p.shape && *ac == p.clip,
                    _ => false,
                };
            if !same {
                dirty.extend([prev.bounds[i], bounds[i]].into_iter().flatten());
            }
        }
    }
    let dirty = merge(dirty);
    match usable {
        Some(prev) if dirty.is_empty() => tile.rgba_mut().copy_from_slice(&prev.frame),
        Some(prev) if partial_pays(&dirty, &bounds) => {
            cache.partials += 1;
            renderer.only = Some(Only {
                rects: &dirty,
                bounds: &bounds,
            });
            renderer.draw_entries(view, &entries, tile)?;
            splice(tile, &prev.frame, &dirty);
        }
        _ => renderer.draw_entries(view, &entries, tile)?,
    }

    let frame = tile.rgba().to_vec();
    let keys = entries
        .iter()
        .map(|e| match e {
            Entry::Skip(kind) => Key::Skip(*kind),
            Entry::Draw(p) => Key::Draw(Box::new(p.shape.clone()), p.clip),
        })
        .collect();
    cache.prev = Some(Prev {
        width,
        height,
        view: view_key,
        clips,
        frame,
        keys,
        bounds,
        ctx,
    });
    Ok(drawn_of(entries))
}

fn drawn_of(entries: Vec<Entry<'_>>) -> Drawn {
    let mut drawn = Drawn::default();
    for (scene_index, entry) in entries.into_iter().enumerate() {
        if let Entry::Skip(kind) = entry {
            drawn.skipped.push(Skipped { scene_index, kind });
        }
    }
    drawn
}

/// Whether repainting the shapes that reach `dirty` costs clearly less than
/// repainting all of them: per-shape setup plus the pixels inside the rectangles.
fn partial_pays(dirty: &[Rect], bounds: &[Option<Rect>]) -> bool {
    const SETUP: u64 = 2000;
    let cost = |b: Rect, within: Option<Rect>| {
        let area = within.map_or(b.area(), |r| {
            let (x0, y0) = (b.x0.max(r.x0), b.y0.max(r.y0));
            let (x1, y1) = (b.x1.min(r.x1), b.y1.min(r.y1));
            u64::from(x1.saturating_sub(x0)) * u64::from(y1.saturating_sub(y0))
        });
        SETUP + area
    };
    let all: u64 = bounds.iter().flatten().map(|b| cost(*b, None)).sum();
    let part: u64 = dirty
        .iter()
        .map(|r| {
            bounds
                .iter()
                .flatten()
                .filter(|b| r.meets(**b))
                .map(|b| cost(*b, Some(*r)))
                .sum::<u64>()
        })
        .sum();
    part * 5 <= all * 4
}

/// Merges overlapping rectangles until pairwise disjoint; past
/// [`MAX_RECTS`] they collapse into their union.
fn merge(mut rects: Vec<Rect>) -> Vec<Rect> {
    const MAX_RECTS: usize = 48;
    let mut out: Vec<Rect> = Vec::new();
    while let Some(mut r) = rects.pop() {
        let mut i = 0;
        while i < out.len() {
            if r.meets(out[i]) {
                r = r.union(out.swap_remove(i));
                i = 0;
            } else {
                i += 1;
            }
        }
        out.push(r);
        if out.len() > MAX_RECTS {
            let all = out.iter().chain(&rects).copied().reduce(Rect::union);
            return all.into_iter().collect();
        }
    }
    out
}

/// Keeps the painted pixels of `tile` inside `rects` and the previous frame
/// everywhere else.
fn splice(tile: &mut Tile, prev: &[u8], rects: &[Rect]) {
    let row = tile.width() as usize * 4;
    let mut out = prev.to_vec();
    for rect in rects {
        let (x0, x1) = (rect.x0 as usize * 4, rect.x1 as usize * 4);
        for y in rect.y0 as usize..rect.y1 as usize {
            out[y * row + x0..y * row + x1]
                .copy_from_slice(&tile.rgba()[y * row + x0..y * row + x1]);
        }
    }
    tile.rgba_mut().copy_from_slice(&out);
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
