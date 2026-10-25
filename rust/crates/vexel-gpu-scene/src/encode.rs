//! Scene encoder: [`GpuScene`] to the buffers of the frozen layout.
//!
//! Interpretation of the layout where it is silent (see also
//! `docs/gpu-interface-requests.md`):
//!
//! * `GpuShape::edge_start/edge_count` is one range per shape, so edges are
//!   NOT rebased per tile. They are stored once, in canvas coordinates (tile
//!   (0,0) origin), fixed point, and a kernel subtracts its tile
//!   origin. Horizontal edges (`y0 == y1` after quantisation) are dropped.
//! * Coverage uses the CPU rule: a sample at `(px, py)` counts an edge with
//!   `y0 <= py < y1` (down) or `y1 <= py < y0` (up) whose crossing is strictly
//!   right of `px`; up adds +1, down -1 (the sign convention is irrelevant to
//!   non-zero and even-odd). Edges left of the sample never count.
//! * `GpuShape::z` equals the index; shapes that paint nothing (transparent
//!   solid, invalid gradient, no edges, off canvas) are dropped.

use crate::bin::{FixedBox, bin_tiles};
use crate::classify::{MAX_GRADIENT_STOPS, analyze};
use crate::error::{GpuError, UnsupportedReason};
use crate::input::{GpuGradient, GpuNode, GpuPaint, GpuScene, GpuSpread};
use crate::layout::{
    self, FILL_EVENODD, FILL_NONZERO, GpuClip, GpuEdge, GpuSceneHeader, GpuStop, GpuTile,
    LAYOUT_MAGIC, LAYOUT_VERSION, PAINT_LINEAR, PAINT_RADIAL, PAINT_SOLID, SPREAD_PAD,
    SPREAD_REFLECT, SPREAD_REPEAT, TILE_SIZE, to_fixed,
};

const FIXED_ONE: f32 = (1u32 << layout::EDGE_FIXED_SHIFT) as f32;

/// All scene buffers, ready to upload.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Encoded {
    pub header: GpuSceneHeader,
    pub shapes: Vec<layout::GpuShape>,
    pub edges: Vec<GpuEdge>,
    pub tiles: Vec<GpuTile>,
    pub tile_list: Vec<u32>,
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
    analyze(scene).map_err(GpuError::Encode)?;
    let tiles_x = scene.width.div_ceil(TILE_SIZE);
    let tiles_y = scene.height.div_ceil(TILE_SIZE);
    let [sx0, sx1] = sample_range(scene.content_rect[0], scene.content_rect[2], scene.width);
    let [sy0, sy1] = sample_range(scene.content_rect[1], scene.content_rect[3], scene.height);
    let mut out = Encoded::default();
    let mut boxes: Vec<FixedBox> = Vec::new();
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
        let edge_start = out.edges.len();
        let mut bounds = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
        for seg in &shape.edges {
            let fixed = [seg.x0, seg.y0, seg.x1, seg.y1].map(to_fixed);
            let [Some(x0), Some(y0), Some(x1), Some(y1)] = fixed else {
                return Err(GpuError::Encode(UnsupportedReason::InvalidScene));
            };
            if y0 == y1 {
                continue;
            }
            bounds = [
                bounds[0].min(x0.min(x1)),
                bounds[1].min(y0.min(y1)),
                bounds[2].max(x0.max(x1)),
                bounds[3].max(y0.max(y1)),
            ];
            out.edges.push(GpuEdge { x0, y0, x1, y1 });
        }
        let culled = out.edges.len() == edge_start
            || crate::bin::tile_range(bounds, tiles_x, tiles_y).is_none();
        if culled {
            out.edges.truncate(edge_start);
            continue;
        }
        let (paint_kind, color_rgba, gradient_index) = match paint {
            GpuPaint::Solid(rgba) => (PAINT_SOLID, pack_rgba(*rgba), u32::MAX),
            GpuPaint::Linear(g) => (PAINT_LINEAR, 0, push_gradient(&mut out, g)),
            GpuPaint::Radial(g) => (PAINT_RADIAL, 0, push_gradient(&mut out, g)),
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
            edge_start: edge_start as u32,
            edge_count: (out.edges.len() - edge_start) as u32,
            gradient_index,
            clip_start: 0,
            clip_count: 0,
            group_depth: 0,
        });
        boxes.push(bounds);
    }
    let (tiles, tile_list) = bin_tiles(&boxes, tiles_x, tiles_y).map_err(GpuError::Encode)?;
    out.header = GpuSceneHeader {
        magic: LAYOUT_MAGIC,
        version: LAYOUT_VERSION,
        width: scene.width,
        height: scene.height,
        tiles_x,
        tiles_y,
        shape_count: out.shapes.len() as u32,
        edge_count: out.edges.len() as u32,
        tile_entry_count: tile_list.len() as u32,
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
    out.tiles = tiles;
    out.tile_list = tile_list;
    Ok(out)
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
        assert_eq!(e.edges.len(), 4);
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
        assert_eq!(
            e.edges[0],
            GpuEdge {
                x0: 20 << layout::EDGE_FIXED_SHIFT,
                y0: 2 << layout::EDGE_FIXED_SHIFT,
                x1: 20 << layout::EDGE_FIXED_SHIFT,
                y1: 10 << layout::EDGE_FIXED_SHIFT
            }
        );
        // 3 x 2 tiles; shape 0 covers tiles 0,1 of row 0; shape 1 tiles 1,2 of row 0.
        let h = e.header;
        assert_eq!((h.tiles_x, h.tiles_y, h.shape_count), (3, 2, 2));
        assert_eq!(h.background_rgba, u32::from_le_bytes([1, 2, 3, 4]));
        assert_eq!(h.tile_entry_count as usize, e.tile_list.len());
        let list = |t: usize| {
            &e.tile_list[e.tiles[t].list_start as usize..][..e.tiles[t].list_count as usize]
        };
        assert_eq!(list(0), &[0]);
        assert_eq!(list(1), &[0, 1]);
        assert_eq!(list(2), &[1]);
        assert_eq!(list(3), &[] as &[u32]);
        assert_eq!(list(5), &[] as &[u32]);
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
