//! Capability classification: can the GPU route draw this scene, and within
//! the layout budgets? The supported set grows lane by lane; today it is solid,
//! linear and radial fills (any content rectangle, as letterboxed frames) and
//! opacity groups up to `MAX_GROUP_DEPTH` and strokes (segments, caps, joins,
//! dashes as exported by the CPU) within the primitive budget; no clips.

use crate::error::{Capability, UnsupportedReason as Reason};
use crate::input::{GpuNode, GpuPaint, GpuScene, GpuSegment, GpuShape, GpuStroke};
use crate::strokes;
use crate::layout::{
    MAX_CLIP_CHAIN, MAX_CLIP_RECORDS, MAX_EDGES, MAX_GRADIENTS, MAX_GROUP_DEPTH, MAX_SHAPES,
    MAX_STOPS, MAX_STROKE_PRIMS, MAX_TILE_LIST, TILE_SIZE,
};

/// Largest canvas axis in pixels. Fixed-point edge coordinates and the i64
/// crossing products stay far inside range below it.
pub const MAX_CANVAS_AXIS: u32 = 16_384;
/// Largest accepted absolute coordinate in pixels: the fixed-point limit of
/// the layout (the CPU accepts 1e6). Larger coordinates classify as
/// `Unsupported(InvalidScene)` and the scene renders on the CPU.
pub const MAX_COORDINATE: f64 = crate::layout::MAX_COORDINATE as f64;
/// Budget for the estimated per-tile edge pool: for every edge the tiles its
/// own bounding box covers (the copies a tile entry keeps verbatim), plus one
/// tile entry's worth for the collapsed right-hand carry. The encoder enforces
/// the hard limit ([`MAX_EDGES`]) and reports `Encode(TooManyEdges)` if the
/// real pool still overflows. Not part of the frozen layout.
pub const MAX_EDGE_TILE_WORK: u64 = MAX_EDGES as u64;
/// Budget for clip work: the sum over clipped shapes of (clip child edges in
/// its chain) x (tiles the shape covers), the edge visits a kernel makes for
/// clipping per frame (clip edges are not clipped per tile). Not part of the
/// frozen layout.
pub const MAX_CLIP_EDGE_WORK: u64 = 1 << 27;
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
    pub groups: u64,
    pub clip_records: u64,
    pub clip_edge_work: u64,
    pub stroke_work: u64,
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
    let mut clipped = false;
    walk(
        &scene.nodes,
        0,
        [tiles_x, tiles_y],
        &mut stats,
        &mut clipped,
    )?;
    if clipped {
        // Within every budget, but the clip lane has not landed: CPU renders it.
        return Err(Reason::Clips);
    }
    Ok(stats)
}

/// True when the group needs its own layer (CPU `RenderGroup::is_layered`
/// without masks): opacity other than exactly 1.
pub(crate) fn is_layered(g: &crate::input::GpuGroup) -> bool {
    g.opacity != 1.0
}

/// True when any layered group (even an empty one) exists in `nodes`: the CPU
/// then routes shape runs through layers (`group.rs` `has_layer`).
pub(crate) fn has_layer(nodes: &[GpuNode]) -> bool {
    nodes.iter().any(|n| match n {
        GpuNode::Shape(_) => false,
        GpuNode::Group(g) => is_layered(g) || has_layer(&g.children),
    })
}

fn walk(
    nodes: &[GpuNode],
    depth: u32,
    tiles_xy: [u64; 2],
    stats: &mut Stats,
    clipped: &mut bool,
) -> Result<(), Reason> {
    let [tiles_x, tiles_y] = tiles_xy;
    for node in nodes {
        let shape = match node {
            GpuNode::Group(g) => {
                if !g.opacity.is_finite() || !(0.0..=1.0).contains(&g.opacity) {
                    return Err(Reason::InvalidScene);
                }
                if g.clips.len() > MAX_CLIP_CHAIN as usize {
                    return Err(Reason::ClipChainTooLong);
                }
                if !g.clips.is_empty() {
                    *clipped = true;
                }
                if g.children.is_empty() {
                    continue;
                }
                let layered = is_layered(g);
                let inner = depth + u32::from(layered);
                if inner > MAX_GROUP_DEPTH {
                    return Err(Reason::GroupTooDeep);
                }
                if layered {
                    stats.groups += 1;
                }
                walk(&g.children, inner, tiles_xy, stats, clipped)?;
                continue;
            }
            GpuNode::Shape(shape) => shape,
        };
        if shape.clips.len() > MAX_CLIP_CHAIN as usize {
            return Err(Reason::ClipChainTooLong);
        }
        if let Some(stroke) = &shape.stroke {
            stroke_budget(stats, stroke, tiles_x, tiles_y)?;
            if shape.fill.is_none() && !shape.clips.is_empty() {
                *clipped = true;
            }
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
        // Cheap bound first; per-edge precision only once it would not fit.
        let bound = shape.edges.len() as u64 * tiles + tiles;
        stats.edge_tile_work += if stats.edge_tile_work + bound <= MAX_EDGE_TILE_WORK {
            bound
        } else {
            edge_tile_copies(shape, tiles_x, tiles_y) + tiles
        };
        if stats.tile_entries > u64::from(MAX_TILE_LIST) {
            return Err(Reason::TooManyTileEntries);
        }
        if stats.edge_tile_work > MAX_EDGE_TILE_WORK {
            return Err(Reason::TooManyEdges);
        }
        if !shape.clips.is_empty() {
            clip_budget(stats, shape, tiles)?;
            *clipped = true;
        }
    }
    Ok(())
}

/// Stroke budget: primitives in the pool, and the (tile, primitive) records
/// the primitives' device boxes bound (each costs one pool record and one
/// primitive visit per tile thread, the kernel's work).
fn stroke_budget(
    stats: &mut Stats,
    stroke: &GpuStroke,
    tiles_x: u64,
    tiles_y: u64,
) -> Result<(), Reason> {
    if !strokes::stroke_valid(stroke) || strokes::invert(&stroke.transform).is_none() {
        return Err(Reason::InvalidScene);
    }
    if stroke.segments.len() as u64 + stroke.joins.len() as u64 > u64::from(MAX_STROKE_PRIMS) {
        return Err(Reason::TooManyStrokePrims);
    }
    let Some((_, work, bounds)) = strokes::estimate(stroke, tiles_x as u32, tiles_y as u32) else {
        return Ok(());
    };
    stats.stroke_work += work;
    if stats.stroke_work > u64::from(MAX_STROKE_PRIMS) {
        return Err(Reason::TooManyStrokePrims);
    }
    stats.tile_entries += tile_count(bounds, tiles_x, tiles_y);
    if stats.tile_entries > u64::from(MAX_TILE_LIST) {
        return Err(Reason::TooManyTileEntries);
    }
    Ok(())
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

/// Clip budget math of one shape covering `tiles` tiles: one child record per
/// chain link until the input can express unions (`GpuClipPath` is a single
/// filled child today), clip edges count toward [`MAX_EDGES`], and the clip
/// work is `clip edges * tiles`.
fn clip_budget(stats: &mut Stats, shape: &GpuShape, tiles: u64) -> Result<(), Reason> {
    let mut chain_edges = 0u64;
    for clip in &shape.clips {
        check_points(&clip.edges)?;
        chain_edges += clip.edges.len() as u64;
    }
    stats.clip_records += shape.clips.len() as u64;
    if stats.clip_records > u64::from(MAX_CLIP_RECORDS) {
        return Err(Reason::TooManyClips);
    }
    stats.edges += chain_edges;
    if stats.edges > u64::from(MAX_EDGES) {
        return Err(Reason::TooManyEdges);
    }
    stats.clip_edge_work += chain_edges * tiles;
    if stats.clip_edge_work > MAX_CLIP_EDGE_WORK {
        return Err(Reason::TooManyEdges);
    }
    Ok(())
}

/// Rejects non-finite or out-of-range coordinates.
fn check_points(edges: &[GpuSegment]) -> Result<(), Reason> {
    for GpuSegment { x0, y0, x1, y1 } in edges {
        for v in [x0, y0, x1, y1] {
            if !v.is_finite() || v.abs() > MAX_COORDINATE {
                return Err(Reason::InvalidScene);
            }
        }
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

/// Sum over the edges of the tiles each edge's own box touches.
fn edge_tile_copies(shape: &GpuShape, tiles_x: u64, tiles_y: u64) -> u64 {
    shape
        .edges
        .iter()
        .filter(|e| e.y0 != e.y1)
        .map(|e| {
            tile_count(
                [
                    e.x0.min(e.x1),
                    e.y0.min(e.y1),
                    e.x0.max(e.x1),
                    e.y0.max(e.y1),
                ],
                tiles_x,
                tiles_y,
            )
        })
        .sum()
}

/// Tiles of the canvas grid touched by device-space `bounds`.
fn tile_count(bounds: [f64; 4], tiles_x: u64, tiles_y: u64) -> u64 {
    // Multiplying by 1/16 is exact (power of two) and cheaper than dividing.
    const INV_TILE: f64 = 1.0 / TILE_SIZE as f64;
    let span = |lo: f64, hi: f64, count: u64| -> u64 {
        let first = (lo * INV_TILE).floor().max(0.0);
        let last = (hi * INV_TILE).floor().min(count as f64 - 1.0);
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
        GpuCap, GpuClipPath, GpuColorStop, GpuGradient, GpuGroup, GpuSpread, GpuStroke,
        GpuStrokeSegment,
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
        let stroke = |segments: usize| GpuStroke {
            paint: GpuPaint::Solid([0, 0, 0, 255]),
            radius: 0.5,
            join_reach: 2.0,
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            segments: vec![
                GpuStrokeSegment {
                    a: [1.0, 1.0],
                    b: [20.0, 5.0],
                    start_cap: GpuCap::Butt,
                    end_cap: GpuCap::Round,
                };
                segments
            ],
            joins: Vec::new(),
        };
        let mut stroked = fill(square(0.0, 0.0, 4.0));
        stroked.stroke = Some(stroke(3));
        assert_eq!(classify(&scene_with(stroked)), Capability::Supported);
        // Singular transform and non-finite radius are invalid scenes.
        let mut bad = fill(Vec::new());
        let mut st = stroke(1);
        st.transform = [1.0, 2.0, 2.0, 4.0, 0.0, 0.0];
        bad.stroke = Some(st);
        assert_eq!(
            classify(&scene_with(bad)),
            Capability::Unsupported(Reason::InvalidScene)
        );
        // More tile visits than the pool holds: falls back to the CPU.
        let mut huge = fill(Vec::new());
        let mut st = stroke(1);
        st.segments = vec![
            GpuStrokeSegment {
                a: [0.0, 0.0],
                b: [63.0, 63.0],
                start_cap: GpuCap::Butt,
                end_cap: GpuCap::Butt,
            };
            600_000
        ];
        huge.stroke = Some(st);
        assert_eq!(
            classify(&scene_with(huge)),
            Capability::Unsupported(Reason::TooManyStrokePrims)
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
            children: vec![GpuNode::Shape(fill(square(0.0, 0.0, 4.0)))],
        }));
        assert_eq!(classify(&grouped), Capability::Supported);
    }

    fn clipped_scene(
        canvas: u32,
        chain: usize,
        shapes: usize,
        clip_edges: Vec<GpuSegment>,
    ) -> GpuScene {
        let mut scene = GpuScene::new(canvas, canvas);
        for _ in 0..shapes {
            let mut s = fill(square(0.0, 0.0, f64::from(canvas)));
            for _ in 0..chain {
                s.clips.push(GpuClipPath {
                    edges: clip_edges.clone(),
                    evenodd: false,
                });
            }
            scene.nodes.push(GpuNode::Shape(s));
        }
        scene
    }

    #[test]
    fn clip_budgets() {
        let clip = square(2.0, 2.0, 30.0);
        // Within budget: only the missing clip lane makes it unsupported.
        for chain in 1..=MAX_CLIP_CHAIN as usize {
            assert_eq!(
                classify(&clipped_scene(64, chain, 1, clip.clone())),
                Capability::Unsupported(Reason::Clips)
            );
        }
        assert_eq!(
            classify(&clipped_scene(64, 5, 1, clip.clone())),
            Capability::Unsupported(Reason::ClipChainTooLong)
        );
        // 4 links x 16385 shapes = 65540 records > MAX_CLIP_RECORDS.
        assert_eq!(
            classify(&clipped_scene(64, 4, 16_385, clip.clone())),
            Capability::Unsupported(Reason::TooManyClips)
        );
        // Clip edges count toward MAX_EDGES.
        let many = (0..4_200_000)
            .map(|i| GpuSegment {
                x0: 1.0,
                y0: f64::from(i % 50),
                x1: 2.0,
                y1: 3.0,
            })
            .collect();
        assert_eq!(
            classify(&clipped_scene(64, 1, 1, many)),
            Capability::Unsupported(Reason::TooManyEdges)
        );
        // Clip work on a 1024 x 1024 canvas (4096 tiles per full-canvas
        // shape): 30,000 clip edges = 123M visits fit, 40,000 = 164M do not.
        let busy = |n: usize| -> Vec<GpuSegment> {
            (0..n)
                .map(|i| GpuSegment {
                    x0: 1.0,
                    y0: (i % 900) as f64,
                    x1: 2.0,
                    y1: 3.0,
                })
                .collect()
        };
        assert_eq!(
            classify(&clipped_scene(1024, 1, 1, busy(30_000))),
            Capability::Unsupported(Reason::Clips)
        );
        assert_eq!(
            classify(&clipped_scene(1024, 1, 1, busy(40_000))),
            Capability::Unsupported(Reason::TooManyEdges)
        );
        let bad = vec![GpuSegment {
            x0: f64::NAN,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        }];
        assert_eq!(
            classify(&clipped_scene(64, 1, 1, bad)),
            Capability::Unsupported(Reason::InvalidScene)
        );
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
        let at_limit = fill(vec![GpuSegment {
            x0: MAX_COORDINATE,
            y0: -MAX_COORDINATE,
            x1: 1.0,
            y1: 1.0,
        }]);
        assert_eq!(classify(&scene_with(at_limit)), Capability::Supported);
        let past_limit = fill(vec![GpuSegment {
            x0: MAX_COORDINATE + 1.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        }]);
        assert_eq!(
            classify(&scene_with(past_limit)),
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
        // 1024 x 1024 tiles = 1,048,576 entries per shape, each needing at
        // least one edge: the 4,194,304 edge pool is exceeded by the 5th shape
        // (a tile entry always holds an edge, so it binds before the list).
        assert_eq!(
            classify(&scene),
            Capability::Unsupported(Reason::TooManyEdges)
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
