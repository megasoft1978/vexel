//! Scene encoder: [`GpuScene`] to the buffers of the frozen layout.
//!
//! Interpretation of the layout where it is silent:
//!
//! * Shape edges are quantised once to canvas-space fixed point; horizontal
//!   edges and edges spanning no sample row are dropped (they never count).
//!   [`crate::bin`] then clips them per tile into tile-local edge ranges
//!   (kept / dropped-left / collapsed-right-to-carry), so a shape's own
//!   `edge_start` is 0 and `edge_count` is informational.
//! * Coverage uses the CPU rule: a sample at `(px, py)` counts an edge with
//!   `y0 <= py < y1` (down) or `y1 <= py < y0` (up) whose crossing is strictly
//!   right of `px`; up adds +1, down -1 (the sign convention is irrelevant to
//!   non-zero and even-odd). Edges left of the sample never count.
//! * `GpuShape::z` equals the index; shapes that paint nothing (transparent
//!   solid, invalid gradient, no edges, off canvas) are dropped.
//! * Clips: a shape's chain is the clip arcs of its enclosing groups (every
//!   group, layered or not, as the CPU `group.rs`) followed by its own. Each
//!   distinct `Arc` becomes one link of child records whose edges (absolute
//!   canvas fixed point, spanning at least one sample row) are stored once
//!   after the tile pools; identical chains share their records.
//! * The header's content range is the CPU content rectangle in sample units.

use crate::bin::{Binned, Binner, spans_sample_row};
use crate::classify::{MAX_GRADIENT_STOPS, analyze, has_layer, is_layered};
use crate::error::{GpuError, UnsupportedReason};
use crate::input::{GpuClipPath, GpuGradient, GpuNode, GpuPaint, GpuScene, GpuShape, GpuSpread};
use crate::layout::{
    self, FILL_EVENODD, FILL_NONZERO, FRAME_RUN, GpuClip, GpuEdge, GpuSceneHeader, GpuStop,
    GpuTile, GpuTileEntry, LAYOUT_MAGIC, LAYOUT_VERSION, MAX_FRAMES, PAINT_LINEAR, PAINT_RADIAL,
    PAINT_SOLID, SHAPE_FILL, SHAPE_STROKE, SPREAD_PAD, SPREAD_REFLECT, SPREAD_REPEAT, TILE_SIZE,
    to_fixed,
};
use crate::strokes::StrokeBinner;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

const FIXED_ONE: f32 = (1u32 << layout::EDGE_FIXED_SHIFT) as f32;

/// All scene buffers, ready to upload.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Encoded {
    pub header: GpuSceneHeader,
    pub shapes: Vec<layout::GpuShape>,
    pub edges: Vec<GpuEdge>,
    pub tiles: Vec<GpuTile>,
    pub tile_list: Vec<GpuTileEntry>,
    pub gradients: Vec<layout::GpuGradient>,
    pub stops: Vec<GpuStop>,
    pub groups: Vec<layout::GpuGroup>,
    pub clips: Vec<GpuClip>,
    pub prims: Vec<layout::GpuStrokePrim>,
}

/// Packs straight-alpha RGBA8 little-endian (r in the low byte).
pub fn pack_rgba(rgba: [u8; 4]) -> u32 {
    u32::from_le_bytes(rgba)
}

/// Encodes `scene`. Fails with `GpuError::Encode(reason)` when the scene is
/// not `Supported` (checked here again) or binning exceeds a budget.
pub fn encode(scene: &GpuScene) -> Result<Encoded, GpuError> {
    let mut out = Encoded::default();
    Encoder::default().run(scene, true, None, &mut out)?;
    Ok(out)
}

/// Frame-to-frame encoder: [`Encoder::encode_into`] reuses the scratch
/// buffers and the buffers of the `Encoded` it is given, so a steady-state
/// frame allocates nothing (first-touch page faults are a large part of the
/// cost of a cold [`encode`]).
#[derive(Default)]
pub struct Encoder {
    binner: Binner,
    binned: Binned,
    src: Vec<GpuEdge>,
    strokes: StrokeBinner,
    clip_edges: Vec<GpuEdge>,
}

impl Encoder {
    /// Same result as [`encode`], written into `out` (its old contents are
    /// replaced; on error `out` is unspecified).
    pub fn encode_into(&mut self, scene: &GpuScene, out: &mut Encoded) -> Result<(), GpuError> {
        self.run(scene, true, None, out)
    }
}

/// Reference variant of [`encode`] where every tile entry lists ALL edges of
/// its shape (translated to the tile, no carry). Renders identically to
/// [`encode`]; exists to prove that and to measure the work clipping saves.
pub fn encode_unclipped(scene: &GpuScene) -> Result<Encoded, GpuError> {
    let mut out = Encoded::default();
    Encoder::default().run(scene, false, None, &mut out)?;
    Ok(out)
}

/// [`encode`] that also reports the time spent in tile binning.
#[cfg(test)]
pub(crate) fn encode_timed(scene: &GpuScene) -> Result<(Encoded, Duration), GpuError> {
    let mut bin_time = Duration::ZERO;
    let mut out = Encoded::default();
    Encoder::default().run(scene, true, Some(&mut bin_time), &mut out)?;
    Ok((out, bin_time))
}

impl Encoder {
    fn run(
        &mut self,
        scene: &GpuScene,
        clip: bool,
        mut bin_time: Option<&mut Duration>,
        out: &mut Encoded,
    ) -> Result<(), GpuError> {
        let stats = analyze(scene).map_err(GpuError::Encode)?;
        let tiles_x = scene.width.div_ceil(TILE_SIZE);
        let tiles_y = scene.height.div_ceil(TILE_SIZE);
        let [sx0, sx1] = sample_range(scene.content_rect[0], scene.content_rect[2], scene.width);
        let [sy0, sy1] = sample_range(scene.content_rect[1], scene.content_rect[3], scene.height);
        out.shapes.clear();
        out.gradients.clear();
        out.stops.clear();
        out.clips.clear();
        out.groups.clear();
        out.prims.clear();
        let Self {
            binner,
            binned,
            src,
            strokes,
            clip_edges,
        } = self;
        clip_edges.clear();
        binner.reset(tiles_x, tiles_y, clip);
        binner.reserve(stats.tile_entries as usize, stats.tile_entries as usize * 3);
        let mut ctx = Walk {
            binner,
            src,
            strokes,
            clips: ClipBuilder {
                edges: clip_edges,
                links: HashMap::new(),
                chains: HashMap::new(),
                groups: Vec::new(),
            },
            out,
            tiles_x,
            tiles_y,
            bin_time: bin_time.as_deref_mut(),
            layered: has_layer(&scene.nodes),
            groups_open: 0,
            frames: Vec::new(),
            run: None,
            next_run: 0,
        };
        ctx.nodes(&scene.nodes)?;
        let started = bin_time.is_some().then(Instant::now);
        // `binned` holds the previous frame's buffers, `out` takes this frame's.
        std::mem::swap(&mut binned.edges, &mut out.edges);
        std::mem::swap(&mut binned.tiles, &mut out.tiles);
        std::mem::swap(&mut binned.entries, &mut out.tile_list);
        binner.finish_into(binned);
        if let (Some(t), Some(started)) = (bin_time.as_deref_mut(), started) {
            *t += started.elapsed();
        }
        out.header = GpuSceneHeader {
            magic: LAYOUT_MAGIC,
            version: LAYOUT_VERSION,
            width: scene.width,
            height: scene.height,
            tiles_x,
            tiles_y,
            shape_count: out.shapes.len() as u32,
            edge_count: binned.edges.len() as u32,
            tile_entry_count: binned.entries.len() as u32,
            gradient_count: out.gradients.len() as u32,
            stop_count: out.stops.len() as u32,
            clip_count: 0,
            flags: 0,
            background_rgba: pack_rgba(scene.background),
            group_count: out.groups.len() as u32,
            reserved1: 0,
            content_x0: sx0,
            content_y0: sy0,
            content_x1: sx1,
            content_y1: sy1,
        };
        std::mem::swap(&mut binned.edges, &mut out.edges);
        std::mem::swap(&mut binned.tiles, &mut out.tiles);
        std::mem::swap(&mut binned.entries, &mut out.tile_list);
        // Clip child edges follow the tile pools.
        let offset = out.edges.len() as u32;
        for c in &mut out.clips {
            c.edge_start += offset;
        }
        out.edges.extend_from_slice(clip_edges);
        if out.edges.len() > layout::MAX_EDGES as usize {
            return Err(GpuError::Encode(UnsupportedReason::TooManyEdges));
        }
        out.header.edge_count = out.edges.len() as u32;
        out.header.clip_count = out.clips.len() as u32;
        Ok(())
    }
}

/// Clip links and chains of one frame, deduplicated by `Arc` identity.
struct ClipBuilder<'a> {
    /// Child edges (absolute canvas fixed point), appended after the tile pools.
    edges: &'a mut Vec<GpuEdge>,
    /// Child records of each clip link (edge starts relative to `edges`).
    links: HashMap<*const GpuClipPath, Vec<GpuClip>>,
    /// `(clip_start, clip_count)` of each distinct chain.
    chains: HashMap<Vec<*const GpuClipPath>, (u32, u32)>,
    /// Clips of the groups enclosing the current node, outermost first.
    groups: Vec<Arc<GpuClipPath>>,
}

impl ClipBuilder<'_> {
    /// Child records of the chain `group_clips + shape_clips`: `(start,
    /// count)` in links, `(0, 0)` when unclipped.
    fn chain(&mut self, out: &mut Vec<GpuClip>, shape_clips: &[Arc<GpuClipPath>]) -> Result<(u32, u32), GpuError> {
        let links: Vec<Arc<GpuClipPath>> = self.groups.iter().chain(shape_clips).cloned().collect();
        if links.is_empty() {
            return Ok((0, 0));
        }
        let key: Vec<*const GpuClipPath> = links.iter().map(|c| Arc::as_ptr(c)).collect();
        if let Some(&hit) = self.chains.get(&key) {
            return Ok(hit);
        }
        let start = out.len() as u32;
        for link in &links {
            let ptr = Arc::as_ptr(link);
            if !self.links.contains_key(&ptr) {
                let records = self.build_link(link)?;
                self.links.insert(ptr, records);
            }
            out.extend_from_slice(&self.links[&ptr]);
        }
        if out.len() > layout::MAX_CLIP_RECORDS as usize {
            return Err(GpuError::Encode(UnsupportedReason::TooManyClips));
        }
        let hit = (start, links.len() as u32);
        self.chains.insert(key, hit);
        Ok(hit)
    }

    /// One link: the union of its children that can ever cover a sample; an
    /// empty union is one record without edges.
    fn build_link(&mut self, link: &GpuClipPath) -> Result<Vec<GpuClip>, GpuError> {
        let mut records = Vec::new();
        for child in &link.children {
            let start = self.edges.len() as u32;
            for seg in &child.edges {
                let fixed = [seg.x0, seg.y0, seg.x1, seg.y1].map(to_fixed);
                let [Some(x0), Some(y0), Some(x1), Some(y1)] = fixed else {
                    return Err(GpuError::Encode(UnsupportedReason::InvalidScene));
                };
                let edge = GpuEdge { x0, y0, x1, y1 };
                if spans_sample_row(&edge) {
                    self.edges.push(edge);
                }
            }
            let edge_count = self.edges.len() as u32 - start;
            if edge_count > 0 {
                records.push(GpuClip {
                    edge_start: start,
                    edge_count,
                    fill_rule: if child.evenodd {
                        FILL_EVENODD
                    } else {
                        FILL_NONZERO
                    },
                    union_size: 0,
                });
            }
        }
        if records.is_empty() {
            records.push(GpuClip::default());
        }
        records[0].union_size = records.len() as u32;
        Ok(records)
    }
}

/// State of the recursive scene walk.
struct Walk<'a> {
    binner: &'a mut Binner,
    src: &'a mut Vec<GpuEdge>,
    strokes: &'a mut StrokeBinner,
    clips: ClipBuilder<'a>,
    out: &'a mut Encoded,
    tiles_x: u32,
    tiles_y: u32,
    bin_time: Option<&'a mut Duration>,
    /// Some layered group exists: shape runs go through run frames (CPU
    /// `render_tree`), else shapes paint straight onto the canvas.
    layered: bool,
    groups_open: u32,
    /// Open frame ids, outermost first (groups, then the current run).
    frames: Vec<u32>,
    /// The run frame currently open, if any.
    run: Option<u32>,
    next_run: u32,
}

impl Walk<'_> {
    /// Mirrors vexel-render `group.rs` `walk`: opacity-1 groups are
    /// transparent, a layered group closes the current run and opens a frame.
    fn nodes(&mut self, nodes: &[GpuNode]) -> Result<(), GpuError> {
        for node in nodes {
            match node {
                GpuNode::Shape(shape) => self.shape(shape)?,
                GpuNode::Group(g) if !is_layered(g) => {
                    let outer = self.clips.groups.len();
                    self.clips.groups.extend(g.clips.iter().cloned());
                    let result = self.nodes(&g.children);
                    self.clips.groups.truncate(outer);
                    result?;
                }
                GpuNode::Group(g) => {
                    if g.children.is_empty() {
                        continue;
                    }
                    let outer = self.clips.groups.len();
                    self.clips.groups.extend(g.clips.iter().cloned());
                    self.close_run();
                    let id = self.out.groups.len() as u32;
                    self.out.groups.push(opacity_table(g.opacity));
                    self.frames.push(id);
                    self.groups_open += 1;
                    let result = self.nodes(&g.children);
                    self.clips.groups.truncate(outer);
                    result?;
                    self.close_run();
                    self.groups_open -= 1;
                    self.frames.pop();
                }
            }
        }
        Ok(())
    }

    fn close_run(&mut self) {
        if self.run.take().is_some() {
            self.frames.pop();
        }
    }

    fn shape(&mut self, shape: &GpuShape) -> Result<(), GpuError> {
        if shape.fill.is_none() && shape.stroke.is_none() {
            return Ok(());
        }
        if self.layered && self.run.is_none() {
            let id = FRAME_RUN | self.next_run;
            self.next_run += 1;
            self.run = Some(id);
            self.frames.push(id);
        }
        self.add(shape)
    }

    fn frames_array(&self) -> [u32; MAX_FRAMES as usize] {
        let mut a = [0u32; MAX_FRAMES as usize];
        a[..self.frames.len()].copy_from_slice(&self.frames);
        a
    }

    fn add(&mut self, shape: &GpuShape) -> Result<(), GpuError> {
        let (frames, frame_count, groups_open) = (
            self.frames_array(),
            self.frames.len() as u32,
            self.groups_open,
        );
        let ctx = self;
        let (src, out) = (&mut *ctx.src, &mut *ctx.out);
        // Fill: quantised edges and the tile range they cover.
        let mut bounds = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
        let mut fill: Option<(&GpuPaint, [u32; 4])> = None;
        src.clear();
        if let Some(paint) = shape.fill.as_ref().filter(|p| paint_valid(p)) {
            for seg in &shape.edges {
                let fixed = [seg.x0, seg.y0, seg.x1, seg.y1].map(to_fixed);
                let [Some(x0), Some(y0), Some(x1), Some(y1)] = fixed else {
                    return Err(GpuError::Encode(UnsupportedReason::InvalidScene));
                };
                let edge = GpuEdge { x0, y0, x1, y1 };
                if !spans_sample_row(&edge) {
                    continue;
                }
                bounds = [
                    bounds[0].min(x0.min(x1)),
                    bounds[1].min(y0.min(y1)),
                    bounds[2].max(x0.max(x1)),
                    bounds[3].max(y0.max(y1)),
                ];
                src.push(edge);
            }
            if !src.is_empty()
                && let Some(range) = crate::bin::tile_range(bounds, ctx.tiles_x, ctx.tiles_y)
            {
                fill = Some((paint, range));
            }
        }
        // Stroke: primitives binned per tile.
        let mut stroke = None;
        if let Some(st) = shape.stroke.as_ref().filter(|s| paint_valid(&s.paint)) {
            if let Some(p) = ctx
                .strokes
                .prepare(st, ctx.tiles_x, ctx.tiles_y)
                .map_err(GpuError::Encode)?
            {
                stroke = Some((st, p));
            }
        }
        if fill.is_none() && stroke.is_none() {
            return Ok(());
        }
        let (clip_start, clip_count) = ctx.clips.chain(&mut out.clips, &shape.clips)?;
        let mut flags = 0;
        let (mut paint_kind, mut color_rgba, mut gradient_index) = (PAINT_SOLID, 0, u32::MAX);
        if let Some((paint, _)) = fill {
            flags |= SHAPE_FILL;
            (paint_kind, color_rgba, gradient_index) = encode_paint(out, paint);
        }
        let (mut sp_kind, mut sp_color, mut sp_gradient, mut sp_m) =
            (PAINT_SOLID, 0, u32::MAX, [0.0f32; 4]);
        let mut shape_box = bounds.map(|v| v as f32 / FIXED_ONE);
        let mut radius = 0.0f32;
        if let Some((st, p)) = &stroke {
            flags |= SHAPE_STROKE;
            (sp_kind, sp_color, sp_gradient) = encode_paint(out, &st.paint);
            sp_m = p.m;
            radius = st.radius as f32;
            let b = p.bounds.map(|v| v as f32);
            if fill.is_none() {
                shape_box = b;
            } else {
                shape_box = [
                    shape_box[0].min(b[0]),
                    shape_box[1].min(b[1]),
                    shape_box[2].max(b[2]),
                    shape_box[3].max(b[3]),
                ];
            }
        }
        let shape_index = out.shapes.len() as u32;
        out.shapes.push(layout::GpuShape {
            bbox: shape_box,
            z: shape_index,
            paint_kind,
            color_rgba,
            opacity: 1.0,
            fill_rule: if shape.evenodd {
                FILL_EVENODD
            } else {
                FILL_NONZERO
            },
            stroke_width: radius,
            edge_start: 0,
            edge_count: if fill.is_some() { src.len() as u32 } else { 0 },
            gradient_index,
            clip_start,
            clip_count,
            group_depth: groups_open,
            frame_count,
            frames,
            flags,
            stroke_paint_kind: sp_kind,
            stroke_color_rgba: sp_color,
            stroke_gradient_index: sp_gradient,
            stroke_m: sp_m,
        });
        let started = ctx.bin_time.is_some().then(Instant::now);
        let first = ctx.binner.pending_len();
        if let Some((_, range)) = fill {
            ctx.binner
                .add_shape(shape_index, src, range)
                .map_err(GpuError::Encode)?;
        }
        let fill_entries = first..ctx.binner.pending_len();
        if let Some((st, p)) = &stroke {
            let binner = &mut *ctx.binner;
            ctx.strokes
                .emit(st, p, ctx.tiles_x, &mut out.prims, |tile, start, count| {
                    binner.add_prims(shape_index, fill_entries.clone(), tile, start, count)
                })
                .map_err(GpuError::Encode)?;
        }
        if let (Some(t), Some(started)) = (ctx.bin_time.as_deref_mut(), started) {
            *t += started.elapsed();
        }
        Ok(())
    }
}

/// True when a paint can colour a sample (not fully transparent, valid gradient).
fn paint_valid(paint: &GpuPaint) -> bool {
    match paint {
        GpuPaint::Solid(rgba) => rgba[3] != 0,
        GpuPaint::Linear(g) | GpuPaint::Radial(g) => gradient_valid(g),
    }
}

/// (paint kind, packed colour, gradient index) of a valid paint.
fn encode_paint(out: &mut Encoded, paint: &GpuPaint) -> (u32, u32, u32) {
    match paint {
        GpuPaint::Solid(rgba) => (PAINT_SOLID, pack_rgba(*rgba), u32::MAX),
        GpuPaint::Linear(g) => (PAINT_LINEAR, 0, push_gradient(out, g)),
        GpuPaint::Radial(g) => (PAINT_RADIAL, 0, push_gradient(out, g)),
    }
}

/// The CPU's `composite_layer` `alpha_lut`, packed four bytes to a word.
fn opacity_table(opacity: f64) -> layout::GpuGroup {
    let mut g = layout::GpuGroup::default();
    for alpha in 0..256usize {
        let scaled = (alpha as f64 / 255.0 * opacity).min(1.0);
        let v = if scaled > 0.0 {
            (scaled * 255.0).round() as u32
        } else {
            0
        };
        g.lut[alpha / 4] |= v << ((alpha % 4) * 8);
    }
    g
}

/// Sample-index range `[lo, hi)` along one axis of `size` pixels that lies in
/// the content interval `[start, end)`: the CPU keeps a sample at
/// `(index + 0.5) / 4` when `start <= sample < end`. Exact in `f64`.
fn sample_range(start: f64, end: f64, size: u32) -> [u32; 2] {
    let max = f64::from(size * layout::SAMPLES_PER_AXIS);
    let index = |v: f64| {
        (v * f64::from(layout::SAMPLES_PER_AXIS) - 0.5)
            .ceil()
            .clamp(0.0, max) as u32
    };
    let lo = index(start);
    [lo, index(end).max(lo)]
}

/// The CPU sampler paints nothing for fewer than 2 or more than 256 stops,
/// unsorted or out-of-range offsets, or an invalid matrix; such a shape is
/// dropped here too.
fn gradient_valid(g: &GpuGradient) -> bool {
    let n = g.stops.len();
    let m = g.inverse;
    n >= 2
        && n <= MAX_GRADIENT_STOPS
        && g.stops
            .iter()
            .all(|s| s.offset.is_finite() && (0.0..=1.0).contains(&s.offset))
        && g.stops.windows(2).all(|p| p[1].offset >= p[0].offset)
        && m.iter().all(|v| v.is_finite())
        && m[..4].iter().all(|v| v.abs() <= 1.0e12)
        && m[4..].iter().all(|v| v.abs() <= 1.0e6)
        && g.focal_ratio.is_finite()
}

fn push_gradient(out: &mut Encoded, g: &GpuGradient) -> u32 {
    let index = out.gradients.len() as u32;
    let stop_start = out.stops.len() as u32;
    for s in &g.stops {
        out.stops.push(GpuStop {
            offset: s.offset as f32,
            rgba: s.rgba.map(f32::from),
        });
    }
    out.gradients.push(layout::GpuGradient {
        inverse: g.inverse.map(|v| v as f32),
        spread: match g.spread {
            GpuSpread::Pad => SPREAD_PAD,
            GpuSpread::Reflect => SPREAD_REFLECT,
            GpuSpread::Repeat => SPREAD_REPEAT,
        },
        stop_start,
        stop_count: g.stops.len() as u32,
        focal_ratio: g.focal_ratio as f32,
        reserved0: 0,
        reserved1: 0,
    });
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{GpuColorStop, GpuSegment};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<GpuSegment> {
        let s = |a, b, c, d| GpuSegment {
            x0: a,
            y0: b,
            x1: c,
            y1: d,
        };
        vec![
            s(x0, y0, x1, y0),
            s(x1, y0, x1, y1),
            s(x1, y1, x0, y1),
            s(x0, y1, x0, y0),
        ]
    }

    fn stops() -> Vec<GpuColorStop> {
        vec![
            GpuColorStop {
                offset: 0.0,
                rgba: [255, 0, 0, 255],
            },
            GpuColorStop {
                offset: 1.0,
                rgba: [0, 0, 255, 255],
            },
        ]
    }

    #[test]
    fn encodes_shapes_edges_tiles_and_gradients() {
        let mut scene = GpuScene::new(40, 20);
        scene.background = [1, 2, 3, 4];
        scene.push_fill(
            rect(2.0, 2.0, 20.0, 10.0),
            false,
            GpuPaint::Solid([10, 20, 30, 255]),
        );
        scene.push_fill(
            rect(0.0, 0.0, 5.0, 5.0),
            true,
            GpuPaint::Solid([0, 0, 0, 0]),
        );
        let g = GpuGradient {
            inverse: [0.1, 0.0, 0.0, 0.0, 0.0, 0.0],
            spread: GpuSpread::Repeat,
            stops: stops(),
            focal_ratio: 0.0,
        };
        scene.push_fill(rect(30.0, 5.0, 38.0, 15.0), true, GpuPaint::Linear(g));
        scene.push_fill(
            rect(100.0, 100.0, 110.0, 110.0),
            false,
            GpuPaint::Solid([1, 1, 1, 255]),
        );
        let e = encode(&scene).unwrap();
        // Transparent and off-canvas shapes are dropped; horizontals too.
        assert_eq!(e.shapes.len(), 2);
        assert_eq!((e.shapes[0].edge_start, e.shapes[0].edge_count), (0, 2));
        assert_eq!(e.shapes[0].z, 0);
        assert_eq!(e.shapes[1].z, 1);
        assert_eq!(
            e.shapes[0].color_rgba,
            u32::from_le_bytes([10, 20, 30, 255])
        );
        assert_eq!(e.shapes[1].fill_rule, FILL_EVENODD);
        assert_eq!(e.shapes[1].gradient_index, 0);
        assert_eq!((e.stops.len(), e.gradients.len()), (2, 1));
        assert_eq!(e.gradients[0].spread, SPREAD_REPEAT);
        // 3 x 2 tiles; shape 0 covers tiles 0,1 of row 0; shape 1 tiles 1,2 of row 0.
        let h = e.header;
        assert_eq!((h.tiles_x, h.tiles_y, h.shape_count), (3, 2, 2));
        assert_eq!(h.background_rgba, u32::from_le_bytes([1, 2, 3, 4]));
        assert_eq!(h.tile_entry_count as usize, e.tile_list.len());
        assert_eq!(h.edge_count as usize, e.edges.len());
        let list = |t: usize| -> Vec<&GpuTileEntry> {
            e.tile_list[e.tiles[t].list_start as usize..][..e.tiles[t].list_count as usize]
                .iter()
                .collect()
        };
        let shapes = |t: usize| list(t).iter().map(|x| x.shape).collect::<Vec<_>>();
        assert_eq!(shapes(0), [0]);
        assert_eq!(shapes(1), [0, 1]);
        assert_eq!(shapes(2), [1]);
        assert!(shapes(3).is_empty() && shapes(5).is_empty());
        // Tile 0 of shape 0: left edge (x = 2, tile-local) kept, right edge
        // (x = 20) collapsed to a unit edge on the border x = 16.
        let first = list(0)[0];
        let edges = &e.edges[first.edge_start as usize..][..first.edge_count as usize];
        let fx = |v: i32| v << layout::EDGE_FIXED_SHIFT;
        assert_eq!(edges.len(), 2);
        assert!(edges.contains(&GpuEdge {
            x0: fx(2),
            y0: fx(10),
            x1: fx(2),
            y1: fx(2)
        }));
        assert!(edges.contains(&GpuEdge {
            x0: fx(16),
            y0: fx(2),
            x1: fx(16),
            y1: fx(10)
        }));
    }

    #[test]
    fn invalid_gradient_paints_nothing() {
        let mut scene = GpuScene::new(16, 16);
        let mut g = GpuGradient {
            inverse: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            spread: GpuSpread::Pad,
            stops: stops(),
            focal_ratio: 0.0,
        };
        g.stops.truncate(1);
        scene.push_fill(rect(1.0, 1.0, 9.0, 9.0), false, GpuPaint::Linear(g));
        assert_eq!(encode(&scene).unwrap().shapes.len(), 0);
    }

    fn leaf(x: f64) -> GpuNode {
        GpuNode::Shape(crate::input::GpuShape {
            edges: rect(x, 1.0, x + 4.0, 5.0),
            evenodd: false,
            fill: Some(GpuPaint::Solid([1, 2, 3, 255])),
            stroke: None,
            clips: Vec::new(),
        })
    }

    fn group(opacity: f64, children: Vec<GpuNode>) -> GpuNode {
        GpuNode::Group(crate::input::GpuGroup {
            opacity,
            clips: Vec::new(),
            children,
        })
    }

    #[test]
    fn groups_become_frame_chains() {
        let mut scene = GpuScene::new(32, 16);
        // run0 | group0 { run1 | group1 { run2 } run3 } | run4 (opacity-1 group joins it)
        scene.nodes = vec![
            leaf(1.0),
            group(
                0.5,
                vec![leaf(6.0), group(0.25, vec![leaf(11.0)]), leaf(16.0)],
            ),
            group(1.0, vec![leaf(21.0)]),
            leaf(26.0),
        ];
        let e = encode(&scene).unwrap();
        let run = |n: u32| FRAME_RUN | n;
        let chains: Vec<Vec<u32>> = e
            .shapes
            .iter()
            .map(|s| s.frames[..s.frame_count as usize].to_vec())
            .collect();
        assert_eq!(
            chains,
            vec![
                vec![run(0)],
                vec![0, run(1)],
                vec![0, 1, run(2)],
                vec![0, run(3)],
                vec![run(4)],
                vec![run(4)],
            ]
        );
        assert_eq!(e.header.group_count, 2);
        assert_eq!(e.shapes[2].group_depth, 2);
        // Opacity table: alpha 255 at 0.5 -> 128 (round(0.5 * 255)), 0 -> 0.
        let lut = |g: usize, a: usize| (e.groups[g].lut[a / 4] >> ((a % 4) * 8)) & 255;
        assert_eq!((lut(0, 255), lut(0, 0), lut(1, 255)), (128, 0, 64));
        // No layered group: straight painting, empty chains.
        scene.nodes = vec![leaf(1.0), group(1.0, vec![leaf(6.0)])];
        assert!(
            encode(&scene)
                .unwrap()
                .shapes
                .iter()
                .all(|s| s.frame_count == 0)
        );
    }

    #[test]
    fn content_rect_becomes_sample_range() {
        let mut scene = GpuScene::new(16, 16);
        let h = encode(&scene).unwrap().header;
        assert_eq!(
            (h.content_x0, h.content_y0, h.content_x1, h.content_y1),
            (0, 0, 64, 64)
        );
        // Sample k is at (k + 0.5) / 4: x >= 2.0 keeps k >= 8 (2.125) and
        // x < 10.1 keeps k < 40 (39 -> 9.875, 40 -> 10.125).
        scene.content_rect = [2.0, 1.0, 10.1, 100.0];
        let h = encode(&scene).unwrap().header;
        assert_eq!(
            (h.content_x0, h.content_y0, h.content_x1, h.content_y1),
            (8, 4, 40, 64)
        );
        // Exactly on a sample: 2.125 includes sample 8 as a start, excludes it as an end.
        scene.content_rect = [2.125, 0.0, 2.125, 16.0];
        let h = encode(&scene).unwrap().header;
        assert_eq!((h.content_x0, h.content_x1), (8, 8));
        scene.content_rect = [5.0, 0.0, 1.0, 16.0];
        let h = encode(&scene).unwrap().header;
        assert_eq!(h.content_x1, h.content_x0);
    }

    #[test]
    fn deterministic() {
        let mut scene = GpuScene::new(64, 64);
        for i in 0..20 {
            let o = f64::from(i) * 3.0;
            scene.push_fill(
                rect(o, o, o + 17.0, o + 9.5),
                i % 2 == 0,
                GpuPaint::Solid([i as u8, 0, 0, 200]),
            );
        }
        assert_eq!(encode(&scene).unwrap(), encode(&scene).unwrap());
    }
}
