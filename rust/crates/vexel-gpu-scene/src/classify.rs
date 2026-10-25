//! Capability classification: can the GPU route draw this scene, and within
//! the layout budgets? The supported set grows lane by lane; today it is solid,
//! linear and radial fills (any content rectangle, as letterboxed frames), no strokes,
//! groups or clips.

use crate::error::{Capability, UnsupportedReason as Reason};
use crate::input::{GpuNode, GpuPaint, GpuScene, GpuSegment, GpuShape};
use crate::layout::{MAX_EDGES, MAX_GRADIENTS, MAX_SHAPES, MAX_STOPS, MAX_TILE_LIST, TILE_SIZE};

/// Largest canvas axis in pixels. Fixed-point edge coordinates and the i64
/// crossing products stay far inside range below it.
pub const MAX_CANVAS_AXIS: u32 = 16_384;
/// Largest accepted absolute coordinate, as `vexel_render::MAX_COORDINATE`.
pub const MAX_COORDINATE: f64 = 1.0e6;
/// Budget for the sum over shapes of `edges * covered tiles`: the number of
/// edge visits a kernel makes per frame. Not part of the frozen layout.
pub const MAX_EDGE_TILE_WORK: u64 = 1 << 29;
/// Longest gradient stop list the CPU sampler accepts.
pub const MAX_GRADIENT_STOPS: usize = 256;

/// Everything `encode` needs to know after one walk.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Stats {
    pub shapes: u64,
    pub edges: u64,
    pub gradients: u64,
    pub stops: u64,
    pub tile_entries: u64,
    pub edge_tile_work: u64,
}

/// Classifies `scene`; `Supported` scenes are guaranteed to encode.
pub fn classify(scene: &GpuScene) -> Capability {
    match analyze(scene) {
        Ok(_) => Capability::Supported,
        Err(reason) => Capability::Unsupported(reason),
    }
}

pub(crate) fn analyze(scene: &GpuScene) -> Result<Stats, Reason> {
    check_canvas(scene)?;
    let tiles_x = u64::from(scene.width.div_ceil(TILE_SIZE));
    let tiles_y = u64::from(scene.height.div_ceil(TILE_SIZE));
    let mut stats = Stats::default();
    for node in &scene.nodes {
        let shape = match node {
            GpuNode::Group(_) => return Err(Reason::Groups),
            GpuNode::Shape(shape) => shape,
        };
        if shape.stroke.is_some() {
            return Err(Reason::Strokes);
        }
        if !shape.clips.is_empty() {
            return Err(Reason::Clips);
        }
        let Some(paint) = &shape.fill else { continue };
        stats.shapes += 1;
        if stats.shapes > u64::from(MAX_SHAPES) {
            return Err(Reason::TooManyShapes);
        }
        if let GpuPaint::Linear(g) | GpuPaint::Radial(g) = paint {
            stats.gradients += 1;
            stats.stops += g.stops.len() as u64;
            if stats.gradients > u64::from(MAX_GRADIENTS) {
                return Err(Reason::TooManyGradientStops);
            }
            if stats.stops > u64::from(MAX_STOPS) {
                return Err(Reason::TooManyGradientStops);
            }
        }
        stats.edges += shape.edges.len() as u64;
        if stats.edges > u64::from(MAX_EDGES) {
            return Err(Reason::TooManyEdges);
        }
        let Some(bounds) = shape_bounds(shape)? else {
            continue;
        };
        let tiles = tile_count(bounds, tiles_x, tiles_y);
        stats.tile_entries += tiles;
        stats.edge_tile_work += tiles * shape.edges.len() as u64;
        if stats.tile_entries > u64::from(MAX_TILE_LIST) {
            return Err(Reason::TooManyTileEntries);
        }
        if stats.edge_tile_work > MAX_EDGE_TILE_WORK {
            return Err(Reason::TooManyEdges);
        }
    }
    Ok(stats)
}

fn check_canvas(scene: &GpuScene) -> Result<(), Reason> {
    if scene.width == 0 || scene.height == 0 {
        return Err(Reason::InvalidScene);
    }
    if scene.width > MAX_CANVAS_AXIS || scene.height > MAX_CANVAS_AXIS {
        return Err(Reason::CanvasTooLarge);
    }
    let r = scene.content_rect;
    if r.iter().any(|v| !v.is_finite()) {
        return Err(Reason::InvalidScene);
    }
    Ok(())
}

/// Device-space bounds `[min_x, min_y, max_x, max_y]` of a shape's edges, or
/// `None` for no edges. Rejects non-finite or out-of-range coordinates.
fn shape_bounds(shape: &GpuShape) -> Result<Option<[f64; 4]>, Reason> {
    let mut bounds = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for GpuSegment { x0, y0, x1, y1 } in &shape.edges {
        for (x, y) in [(*x0, *y0), (*x1, *y1)] {
            if !(x.is_finite() && y.is_finite())
                || x.abs() > MAX_COORDINATE
                || y.abs() > MAX_COORDINATE
            {
                return Err(Reason::InvalidScene);
            }
            bounds = [
                bounds[0].min(x),
                bounds[1].min(y),
                bounds[2].max(x),
                bounds[3].max(y),
            ];
        }
    }
    if shape.edges.is_empty() {
        return Ok(None);
    }
    Ok(Some(bounds))
}

/// Tiles of the canvas grid touched by device-space `bounds`.
fn tile_count(bounds: [f64; 4], tiles_x: u64, tiles_y: u64) -> u64 {
    let tile = f64::from(TILE_SIZE);
    let span = |lo: f64, hi: f64, count: u64| -> u64 {
        let first = (lo / tile).floor().max(0.0);
        let last = (hi / tile).floor().min(count as f64 - 1.0);
        if last < first {
            0
        } else {
            (last - first) as u64 + 1
        }
    };
    span(bounds[0], bounds[2], tiles_x) * span(bounds[1], bounds[3], tiles_y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{
        GpuCap, GpuClipPath, GpuColorStop, GpuGradient, GpuGroup, GpuJoin, GpuSpread, GpuStroke,
    };

    fn square(x: f64, y: f64, s: f64) -> Vec<GpuSegment> {
        let seg = |x0, y0, x1, y1| GpuSegment { x0, y0, x1, y1 };
        vec![
            seg(x, y, x + s, y),
            seg(x + s, y, x + s, y + s),
            seg(x + s, y + s, x, y + s),
            seg(x, y + s, x, y),
        ]
    }

    fn scene_with(shape: GpuShape) -> GpuScene {
        let mut scene = GpuScene::new(64, 64);
        scene.nodes.push(GpuNode::Shape(shape));
        scene
    }

    fn fill(edges: Vec<GpuSegment>) -> GpuShape {
        GpuShape {
            edges,
            evenodd: false,
            fill: Some(GpuPaint::Solid([1, 2, 3, 255])),
            stroke: None,
            clips: Vec::new(),
        }
    }

    #[test]
    fn solid_and_gradients_supported() {
        let mut scene = GpuScene::new(64, 64);
        scene.push_fill(
            square(1.0, 1.0, 9.0),
            false,
            GpuPaint::Solid([9, 9, 9, 255]),
        );
        let g = GpuGradient {
            inverse: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            spread: GpuSpread::Pad,
            stops: vec![
                GpuColorStop {
                    offset: 0.0,
                    rgba: [0; 4],
                },
                GpuColorStop {
                    offset: 1.0,
                    rgba: [255; 4],
                },
            ],
            focal_ratio: 0.0,
        };
        scene.push_fill(square(2.0, 2.0, 9.0), true, GpuPaint::Linear(g.clone()));
        scene.push_fill(square(2.0, 2.0, 9.0), true, GpuPaint::Radial(g));
        assert_eq!(classify(&scene), Capability::Supported);
    }

    #[test]
    fn deferred_features_report_reason() {
        let mut stroked = fill(square(0.0, 0.0, 4.0));
        stroked.stroke = Some(GpuStroke {
            paint: GpuPaint::Solid([0, 0, 0, 255]),
            width: 1.0,
            miter_limit: 4.0,
            cap: GpuCap::Butt,
            join: GpuJoin::Miter,
            dashes: Vec::new(),
            dash_offset: 0.0,
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        });
        assert_eq!(
            classify(&scene_with(stroked)),
            Capability::Unsupported(Reason::Strokes)
        );

        let mut clipped = fill(square(0.0, 0.0, 4.0));
        clipped.clips.push(GpuClipPath {
            edges: square(0.0, 0.0, 2.0),
            evenodd: false,
        });
        assert_eq!(
            classify(&scene_with(clipped)),
            Capability::Unsupported(Reason::Clips)
        );

        let mut grouped = GpuScene::new(8, 8);
        grouped.nodes.push(GpuNode::Group(GpuGroup {
            opacity: 0.5,
            clips: Vec::new(),
            children: Vec::new(),
        }));
        assert_eq!(classify(&grouped), Capability::Unsupported(Reason::Groups));
    }

    #[test]
    fn canvas_and_coordinates_checked() {
        assert_eq!(
            classify(&GpuScene::new(MAX_CANVAS_AXIS + 1, 8)),
            Capability::Unsupported(Reason::CanvasTooLarge)
        );
        assert_eq!(
            classify(&GpuScene::new(0, 8)),
            Capability::Unsupported(Reason::InvalidScene)
        );
        let mut letterbox = GpuScene::new(64, 64);
        letterbox.content_rect = [8.0, 0.0, 56.0, 64.0];
        assert_eq!(classify(&letterbox), Capability::Supported);
        letterbox.content_rect[0] = f64::NAN;
        assert_eq!(
            classify(&letterbox),
            Capability::Unsupported(Reason::InvalidScene)
        );
        let nan = fill(vec![GpuSegment {
            x0: f64::NAN,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        }]);
        assert_eq!(
            classify(&scene_with(nan)),
            Capability::Unsupported(Reason::InvalidScene)
        );
        let far = fill(vec![GpuSegment {
            x0: 2.0e6,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        }]);
        assert_eq!(
            classify(&scene_with(far)),
            Capability::Unsupported(Reason::InvalidScene)
        );
    }

    #[test]
    fn budgets() {
        // Many tile entries: one canvas-sized shape repeated.
        let mut scene = GpuScene::new(MAX_CANVAS_AXIS, MAX_CANVAS_AXIS);
        let big = square(0.0, 0.0, f64::from(MAX_CANVAS_AXIS));
        for _ in 0..9 {
            scene.push_fill(big.clone(), false, GpuPaint::Solid([0, 0, 0, 255]));
        }
        // 1024 x 1024 tiles = 1,048,576 entries per shape; 9 > 8,388,608.
        assert_eq!(
            classify(&scene),
            Capability::Unsupported(Reason::TooManyTileEntries)
        );
        // Shape count.
        let mut many = GpuScene::new(16, 16);
        let tiny = square(0.0, 0.0, 1.0);
        for _ in 0..=MAX_SHAPES {
            many.push_fill(tiny.clone(), false, GpuPaint::Solid([0, 0, 0, 255]));
        }
        assert_eq!(
            classify(&many),
            Capability::Unsupported(Reason::TooManyShapes)
        );
    }

    #[test]
    fn shapes_off_canvas_cost_nothing() {
        let mut scene = GpuScene::new(32, 32);
        scene.push_fill(
            square(500.0, 500.0, 10.0),
            false,
            GpuPaint::Solid([0, 0, 0, 255]),
        );
        assert_eq!(analyze(&scene).unwrap().tile_entries, 0);
    }
}
