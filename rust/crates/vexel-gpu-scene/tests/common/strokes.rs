// Shared stroke scene generator and CPU renderer for the stroke parity tests
// (included by tests/stroke_parity.rs and tests/gpu_parity.rs).

use vexel_gpu_scene::input::{
    GpuCap, GpuJoinKind, GpuNode, GpuPaint, GpuScene, GpuSegment, GpuShape, GpuStroke,
    GpuStrokeJoin, GpuStrokeSegment,
};
use vexel_raster::Tile;
use vexel_render::export::{JoinKind, StrokeExport, export_stroke};
use vexel_render::{
    Affine, DashedShape, FlatEdge, Linecap, Linejoin, RenderShape, StrokeStyle, ViewBox, ViewMap,
    render_dashed_shapes,
};

pub struct SRng(pub u64);

impl SRng {
    pub fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / f64::from(1u32 << 31)
    }
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
    pub fn pick(&mut self, n: usize) -> usize {
        ((self.next() * n as f64) as usize).min(n - 1)
    }
}

/// One stroked (and maybe filled) shape: subpath point lists in DOCUMENT
/// space (device space: the view is the identity), each closed or open.
pub struct StrokeCase {
    pub subpaths: Vec<(Vec<(f64, f64)>, bool)>,
    pub style: StrokeStyle,
    pub transform: Affine,
    pub dashes: Vec<f64>,
    pub fill: Option<[u8; 4]>,
    pub evenodd: bool,
}

impl StrokeCase {
    /// Edges as the flattener emits them: every subpath carries its closing
    /// edge (fill), `closed` only decides whether it is stroked.
    pub fn edges(&self) -> (Vec<FlatEdge>, Vec<std::ops::Range<usize>>, Vec<bool>) {
        let mut edges = Vec::new();
        let (mut ranges, mut closed) = (Vec::new(), Vec::new());
        for (pts, is_closed) in &self.subpaths {
            let start = edges.len();
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                edges.push(FlatEdge { x0: a.0, y0: a.1, x1: b.0, y1: b.1 });
            }
            ranges.push(start..edges.len());
            closed.push(*is_closed);
        }
        (edges, ranges, closed)
    }
}

pub fn identity_affine() -> Affine {
    Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 }
}

/// Seeded stroke scene: `count` cases inside a `w * h` canvas.
pub fn stroke_cases(seed: u64, count: usize, w: f64, h: f64, min_w: f64, max_w: f64) -> Vec<StrokeCase> {
    let mut rng = SRng(seed);
    let caps = [Linecap::Butt, Linecap::Round, Linecap::Square];
    let joins = [Linejoin::Miter, Linejoin::Round, Linejoin::Bevel];
    (0..count)
        .map(|i| {
            let (cx, cy) = (rng.range(0.1 * w, 0.9 * w), rng.range(0.1 * h, 0.9 * h));
            let reach = rng.range(20.0, 0.25 * w);
            let kind = i % 5;
            let pts: Vec<(f64, f64)> = match kind {
                // Circle-ish with many short segments (nearly straight joins).
                3 => (0..60)
                    .map(|k| {
                        let a = k as f64 / 60.0 * std::f64::consts::TAU;
                        (cx + reach * 0.5 * a.cos(), cy + reach * 0.5 * a.sin())
                    })
                    .collect(),
                // Zig-zag with sharp angles (miter limit) and dots.
                4 => {
                    let mut v = vec![(cx, cy)];
                    for _ in 0..2 + rng.pick(4) {
                        let last = *v.last().unwrap();
                        v.push((last.0 + rng.range(-reach, reach), last.1 + rng.range(-reach, reach)));
                        if rng.pick(4) == 0 {
                            let l = *v.last().unwrap();
                            v.push(l);
                        }
                    }
                    v
                }
                _ => (0..2 + rng.pick(6))
                    .map(|_| (cx + rng.range(-reach, reach), cy + rng.range(-reach, reach)))
                    .collect(),
            };
            let closed = pts.len() > 2 && rng.pick(2) == 0;
            let width = rng.range(min_w, max_w);
            let dashes = if i % 4 == 1 {
                vec![rng.range(1.0, 12.0), rng.range(1.0, 8.0)]
            } else {
                Vec::new()
            };
            let transform = if i % 3 == 2 {
                Affine {
                    a: rng.range(0.6, 1.6),
                    b: rng.range(-0.5, 0.5),
                    c: rng.range(-0.5, 0.5),
                    d: rng.range(0.6, 1.6),
                    e: rng.range(-20.0, 20.0),
                    f: rng.range(-20.0, 20.0),
                }
            } else {
                identity_affine()
            };
            let filled = closed && i % 2 == 0;
            StrokeCase {
                subpaths: vec![(pts, closed)],
                style: StrokeStyle {
                    rgba: [rng.pick(256) as u8, rng.pick(256) as u8, rng.pick(256) as u8,
                        if i % 3 == 0 { 255 } else { 90 + rng.pick(160) as u8 }],
                    width,
                    miter_limit: rng.range(1.0, 6.0),
                    dash_offset: rng.range(0.0, 10.0),
                    cap: caps[rng.pick(3)],
                    join: joins[rng.pick(3)],
                    crisp_edges: false,
                },
                transform,
                dashes,
                fill: filled.then(|| [rng.pick(256) as u8, rng.pick(256) as u8, rng.pick(256) as u8, 200]),
                evenodd: i % 2 == 0,
            }
        })
        .collect()
}

fn gpu_cap(c: Linecap) -> GpuCap {
    match c {
        Linecap::Butt => GpuCap::Butt,
        Linecap::Round => GpuCap::Round,
        Linecap::Square => GpuCap::Square,
    }
}

/// Lowers the exporter output; `to_device` maps document to device space.
pub fn gpu_stroke(e: &StrokeExport, rgba: [u8; 4], to_device: Affine) -> GpuStroke {
    // to_device after e.transform.
    let (m, n) = (to_device, e.transform);
    let t = Affine {
        a: m.a * n.a + m.c * n.b,
        b: m.b * n.a + m.d * n.b,
        c: m.a * n.c + m.c * n.d,
        d: m.b * n.c + m.d * n.d,
        e: m.a * n.e + m.c * n.f + m.e,
        f: m.b * n.e + m.d * n.f + m.f,
    };
    GpuStroke {
        paint: GpuPaint::Solid(rgba),
        radius: e.radius,
        join_reach: e.join_reach,
        transform: [t.a, t.b, t.c, t.d, t.e, t.f],
        segments: e
            .segments
            .iter()
            .map(|s| GpuStrokeSegment {
                a: s.a,
                b: s.b,
                start_cap: gpu_cap(s.start_cap),
                end_cap: gpu_cap(s.end_cap),
            })
            .collect(),
        joins: e
            .joins
            .iter()
            .map(|j| GpuStrokeJoin {
                vertex: j.vertex,
                kind: match j.kind {
                    JoinKind::Never => GpuJoinKind::Never,
                    JoinKind::Round => GpuJoinKind::Round,
                    JoinKind::Triangle => GpuJoinKind::Triangle,
                    JoinKind::Miter => GpuJoinKind::Miter,
                },
                p1: j.p1,
                miter: j.miter,
                p2: j.p2,
                tight: j.tight,
            })
            .collect(),
    }
}

fn render_shape<'a>(
    case: &'a StrokeCase,
    edges: &'a [FlatEdge],
    ranges: &'a [std::ops::Range<usize>],
    closed: &'a [bool],
) -> RenderShape<'a> {
    RenderShape {
        edges,
        subpaths: ranges,
        subpath_closed: closed,
        stroke_closed: false,
        evenodd: case.evenodd,
        filled: case.fill.is_some(),
        fill_rgba: case.fill.unwrap_or([0; 4]),
        stroke: Some(case.style),
        transform: case.transform,
        fill_gradient: None,
        stroke_gradient: None,
    }
}

/// CPU render (straight RGBA8, identity view) and the GPU scene of the same cases.
pub fn stroke_scene(cases: &[StrokeCase], w: u32, h: u32) -> (GpuScene, Vec<u8>) {
    let view = ViewMap::from_viewbox(
        ViewBox { x: 0.0, y: 0.0, width: f64::from(w), height: f64::from(h) },
        w,
        h,
        None,
    )
    .unwrap();
    let stores: Vec<_> = cases.iter().map(StrokeCase::edges).collect();
    let dashed: Vec<DashedShape<'_>> = cases
        .iter()
        .zip(&stores)
        .map(|(c, (edges, ranges, closed))| DashedShape {
            shape: render_shape(c, edges, ranges, closed),
            dashes: &c.dashes,
        })
        .collect();
    let mut tile = Tile::from_owned_rgba(w, h, vec![0; (w * h * 4) as usize]).unwrap();
    render_dashed_shapes(&dashed, view, &mut tile, &|| false).unwrap();
    let cpu = tile.rgba().to_vec();

    let mut scene = GpuScene::new(w, h);
    for (c, d) in cases.iter().zip(&dashed) {
        let export = export_stroke(&d.shape, d.dashes, false).unwrap();
        let (edges, _, _) = c.edges();
        scene.nodes.push(GpuNode::Shape(GpuShape {
            edges: edges
                .iter()
                .map(|e| GpuSegment { x0: e.x0, y0: e.y0, x1: e.x1, y1: e.y1 })
                .collect(),
            evenodd: c.evenodd,
            fill: c.fill.map(GpuPaint::Solid),
            stroke: export.map(|e| gpu_stroke(&e, c.style.rgba, identity_affine())),
            clips: Vec::new(),
        }));
    }
    (scene, cpu)
}
