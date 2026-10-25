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
//! * The header's content range is the CPU content rectangle in sample units.

use crate::bin::{Binned, Binner, spans_sample_row};
use crate::classify::{MAX_GRADIENT_STOPS, analyze};
use crate::error::{GpuError, UnsupportedReason};
use crate::input::{GpuGradient, GpuNode, GpuPaint, GpuScene, GpuSpread};
use crate::layout::{
    self, FILL_EVENODD, FILL_NONZERO, GpuClip, GpuEdge, GpuSceneHeader, GpuStop, GpuTile,
    GpuTileEntry, LAYOUT_MAGIC, LAYOUT_VERSION, PAINT_LINEAR, PAINT_RADIAL, PAINT_SOLID,
    SPREAD_PAD, SPREAD_REFLECT, SPREAD_REPEAT, TILE_SIZE, to_fixed,
};
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
    pub clips: Vec<GpuClip>,
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
        let Self {
            binner,
            binned,
            src,
        } = self;
        binner.reset(tiles_x, tiles_y, clip);
        binner.reserve(stats.tile_entries as usize, stats.tile_entries as usize * 3);
        for node in &scene.nodes {
            let GpuNode::Shape(shape) = node else {
                return Err(GpuError::Encode(UnsupportedReason::Groups));
            };
            let Some(paint) = &shape.fill else { continue };
            let paint_ok = match paint {
                GpuPaint::Solid(rgba) => rgba[3] != 0,
                GpuPaint::Linear(g) | GpuPaint::Radial(g) => gradient_valid(g),
            };
            if !paint_ok {
                continue;
            }
            src.clear();
            let mut bounds = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
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
            let range = crate::bin::tile_range(bounds, tiles_x, tiles_y);
            let Some(range) = range.filter(|_| !src.is_empty()) else {
                continue;
            };
            let (paint_kind, color_rgba, gradient_index) = match paint {
                GpuPaint::Solid(rgba) => (PAINT_SOLID, pack_rgba(*rgba), u32::MAX),
                GpuPaint::Linear(g) => (PAINT_LINEAR, 0, push_gradient(out, g)),
                GpuPaint::Radial(g) => (PAINT_RADIAL, 0, push_gradient(out, g)),
            };
            out.shapes.push(layout::GpuShape {
                bbox: [
                    bounds[0] as f32 / FIXED_ONE,
                    bounds[1] as f32 / FIXED_ONE,
                    bounds[2] as f32 / FIXED_ONE,
                    bounds[3] as f32 / FIXED_ONE,
                ],
                z: out.shapes.len() as u32,
                paint_kind,
                color_rgba,
                opacity: 1.0,
                fill_rule: if shape.evenodd {
                    FILL_EVENODD
                } else {
                    FILL_NONZERO
                },
                stroke_width: 0.0,
                edge_start: 0,
                edge_count: src.len() as u32,
                gradient_index,
                clip_start: 0,
                clip_count: 0,
                group_depth: 0,
            });
            let started = bin_time.is_some().then(Instant::now);
            let shape_index = out.shapes.len() as u32 - 1;
            binner
                .add_shape(shape_index, src, range)
                .map_err(GpuError::Encode)?;
            if let (Some(t), Some(started)) = (bin_time.as_deref_mut(), started) {
                *t += started.elapsed();
            }
        }
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
            reserved0: 0,
            reserved1: 0,
            content_x0: sx0,
            content_y0: sy0,
            content_x1: sx1,
            content_y1: sy1,
        };
        std::mem::swap(&mut binned.edges, &mut out.edges);
        std::mem::swap(&mut binned.tiles, &mut out.tiles);
        std::mem::swap(&mut binned.entries, &mut out.tile_list);
        Ok(())
    }
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

    #[test]
    fn unsupported_scene_does_not_encode() {
        let mut scene = GpuScene::new(16, 16);
        scene.nodes.push(GpuNode::Group(crate::input::GpuGroup {
            opacity: 0.5,
            clips: Vec::new(),
            children: Vec::new(),
        }));
        assert_eq!(
            encode(&scene).unwrap_err(),
            GpuError::Encode(UnsupportedReason::Groups)
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
