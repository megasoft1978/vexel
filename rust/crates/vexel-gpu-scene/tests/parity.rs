//! Reference rasterizer of the encoded scene against the real `vexel-render`
//! CPU output, on seeded fill and gradient scenes. Bounds are the ADR-0035
//! tolerances from `policy`. `-- --nocapture` prints the metrics.

use vexel_gpu_scene::input::{
    GpuColorStop, GpuGradient, GpuGroup, GpuNode, GpuPaint, GpuScene, GpuSegment, GpuShape,
    GpuSpread,
};
use vexel_gpu_scene::policy::{
    PARITY_MEAN_ABS_ERROR_MAX, PARITY_OVER_2_MAX_FRACTION, PARITY_OVER_16_MAX_FRACTION,
};
use vexel_gpu_scene::{encode, reference};
use vexel_raster::Tile;
use vexel_render::{
    Affine, FlatEdge, GradientRef, RenderGroup, RenderNode, RenderShape, ViewBox, ViewMap,
    render_groups, render_shapes,
};
use vexel_scene::{Paint, PaintKind, PaintStop, Spread, TableRange};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / f64::from(1u32 << 31)
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
    fn byte(&mut self) -> u8 {
        (self.next() * 256.0) as u8
    }
}

/// One generated shape, the common source of both renderers' inputs.
struct Spec {
    ring: Vec<(f64, f64)>,
    extra_ring: Option<Vec<(f64, f64)>>,
    evenodd: bool,
    paint: SpecPaint,
}

enum SpecPaint {
    Solid([u8; 4]),
    Linear {
        p: [f64; 4],
        spread: Spread,
        stops: Vec<PaintStop>,
    },
    Radial {
        c: [f64; 5],
        spread: Spread,
        stops: Vec<PaintStop>,
    },
}

fn polygon(rng: &mut Lcg, w: f64, h: f64) -> Vec<(f64, f64)> {
    let (cx, cy) = (rng.range(-10.0, w + 10.0), rng.range(-10.0, h + 10.0));
    let r = rng.range(3.0, 60.0);
    let n = 3 + (rng.next() * 9.0) as usize;
    (0..n)
        .map(|k| {
            let a = (k as f64 + rng.next() * 0.8) / n as f64 * std::f64::consts::TAU;
            let rr = r * rng.range(0.4, 1.0);
            (cx + rr * a.cos(), cy + rr * a.sin())
        })
        .collect()
}

fn star(rng: &mut Lcg, w: f64, h: f64) -> Vec<(f64, f64)> {
    let (cx, cy) = (rng.range(20.0, w - 20.0), rng.range(20.0, h - 20.0));
    let r = rng.range(10.0, 50.0);
    (0..5)
        .map(|k| {
            let a = (k * 2) as f64 / 5.0 * std::f64::consts::TAU;
            (cx + r * a.sin(), cy - r * a.cos())
        })
        .collect()
}

fn stops(rng: &mut Lcg) -> Vec<PaintStop> {
    let n = 2 + (rng.next() * 3.0) as usize;
    let mut offsets: Vec<f64> = (0..n).map(|_| rng.next()).collect();
    offsets.sort_by(f64::total_cmp);
    offsets[0] = rng.range(0.0, 0.3);
    offsets[n - 1] = rng.range(0.7, 1.0);
    offsets
        .into_iter()
        .map(|offset| PaintStop {
            offset,
            rgba: [rng.byte(), rng.byte(), rng.byte(), 128 + rng.byte() / 2],
        })
        .collect()
}

fn spread(rng: &mut Lcg) -> Spread {
    match (rng.next() * 3.0) as u32 {
        0 => Spread::Pad,
        1 => Spread::Reflect,
        _ => Spread::Repeat,
    }
}

fn specs(seed: u64, count: usize, w: f64, h: f64) -> Vec<Spec> {
    let mut rng = Lcg(seed);
    (0..count)
        .map(|i| {
            let kind = i % 6;
            let (ring, extra_ring, evenodd) = match kind {
                4 => (star(&mut rng, w, h), None, i % 12 == 4),
                5 => {
                    let outer = polygon(&mut rng, w, h);
                    let inner: Vec<_> = outer
                        .iter()
                        .map(|p| (p.0 * 0.9 + 3.0, p.1 * 0.95 + 1.0))
                        .collect();
                    (outer, Some(inner), true)
                }
                _ => (polygon(&mut rng, w, h), None, i % 2 == 0),
            };
            let paint = match kind {
                0 | 1 => SpecPaint::Solid([
                    rng.byte(),
                    rng.byte(),
                    rng.byte(),
                    if i % 4 == 0 { 255 } else { rng.byte() },
                ]),
                2 | 4 => SpecPaint::Linear {
                    p: [
                        rng.range(0.0, w),
                        rng.range(0.0, h),
                        rng.range(0.0, w),
                        rng.range(0.0, h),
                    ],
                    spread: spread(&mut rng),
                    stops: stops(&mut rng),
                },
                _ => {
                    let (cx, cy, r) = (rng.range(0.0, w), rng.range(0.0, h), rng.range(8.0, 90.0));
                    // Focal point: the centre for some, 0.6 r off-centre otherwise.
                    let focal = if i % 4 == 1 { 0.0 } else { 0.6 };
                    let a = rng.range(0.0, std::f64::consts::TAU);
                    SpecPaint::Radial {
                        c: [
                            cx,
                            cy,
                            r,
                            cx + focal * r * a.cos(),
                            cy + focal * r * a.sin(),
                        ],
                        spread: spread(&mut rng),
                        stops: stops(&mut rng),
                    }
                }
            };
            Spec {
                ring,
                extra_ring,
                evenodd,
                paint,
            }
        })
        .collect()
}

fn closed(ring: &[(f64, f64)]) -> Vec<GpuSegment> {
    (0..ring.len())
        .map(|k| {
            let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
            GpuSegment {
                x0: a.0,
                y0: a.1,
                x1: b.0,
                y1: b.1,
            }
        })
        .collect()
}

fn spec_edges(spec: &Spec) -> Vec<GpuSegment> {
    let mut edges = closed(&spec.ring);
    if let Some(extra) = &spec.extra_ring {
        edges.extend(closed(extra));
    }
    edges
}

/// Rounds every vertex to the layout's fixed-point grid, so the encoder's
/// quantisation is lossless and only the sampling arithmetic is compared.
fn grid(v: f64) -> f64 {
    let unit = f64::from(1u32 << vexel_gpu_scene::layout::EDGE_FIXED_SHIFT);
    (v * unit).round() / unit
}

fn snapped(mut specs: Vec<Spec>) -> Vec<Spec> {
    let snap = |p: &mut (f64, f64)| *p = (grid(p.0), grid(p.1));
    for spec in &mut specs {
        spec.ring.iter_mut().for_each(snap);
        spec.extra_ring.iter_mut().flatten().for_each(snap);
    }
    specs
}

fn gpu_spread(s: Spread) -> GpuSpread {
    match s {
        Spread::Pad => GpuSpread::Pad,
        Spread::Reflect => GpuSpread::Reflect,
        Spread::Repeat => GpuSpread::Repeat,
    }
}

fn gpu_stops(stops: &[PaintStop]) -> Vec<GpuColorStop> {
    stops
        .iter()
        .map(|s| GpuColorStop {
            offset: s.offset,
            rgba: s.rgba,
        })
        .collect()
}

/// The adapter the app lane writes, for the paint kinds used here.
fn gpu_paint(spec: &Spec) -> GpuPaint {
    match &spec.paint {
        SpecPaint::Solid(rgba) => GpuPaint::Solid(*rgba),
        SpecPaint::Linear { p, spread, stops } => {
            let (dx, dy) = (p[2] - p[0], p[3] - p[1]);
            let l = dx * dx + dy * dy;
            GpuPaint::Linear(GpuGradient {
                inverse: [dx / l, 0.0, dy / l, 0.0, -(p[0] * dx + p[1] * dy) / l, 0.0],
                spread: gpu_spread(*spread),
                stops: gpu_stops(stops),
                focal_ratio: 0.0,
            })
        }
        SpecPaint::Radial { c, spread, stops } => {
            let (fx, fy) = ((c[3] - c[0]) / c[2], (c[4] - c[1]) / c[2]);
            let theta = fy.atan2(fx);
            let (s, co) = (theta.sin(), theta.cos());
            // p'' = R(-theta) * (p - centre) / r
            let (a, cc) = (co / c[2], s / c[2]);
            let (b, d) = (-s / c[2], co / c[2]);
            GpuPaint::Radial(GpuGradient {
                inverse: [a, b, cc, d, -(a * c[0] + cc * c[1]), -(b * c[0] + d * c[1])],
                spread: gpu_spread(*spread),
                stops: gpu_stops(stops),
                focal_ratio: fx.hypot(fy),
            })
        }
    }
}

fn scene_for(specs: &[Spec], w: u32, h: u32) -> GpuScene {
    let mut scene = GpuScene::new(w, h);
    for spec in specs {
        scene.push_fill(spec_edges(spec), spec.evenodd, gpu_paint(spec));
    }
    scene
}

fn identity() -> Affine {
    Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    }
}

/// The CPU rasterizer through its public API, identity view.
fn cpu_render(specs: &[Spec], w: u32, h: u32) -> Vec<u8> {
    let vb = ViewBox {
        x: 0.0,
        y: 0.0,
        width: f64::from(w),
        height: f64::from(h),
    };
    cpu_render_view(specs, w, h, vb, (0.0, 0.0), 1.0)
}

/// The CPU rasterizer with an explicit view box. `specs` are in device space;
/// they are lowered to user space with the (known, power-of-two) `offset` and
/// `scale` of `ViewMap::from_viewbox(vb, w, h, None)` so the CPU maps them back
/// exactly.
fn cpu_render_view(
    specs: &[Spec],
    w: u32,
    h: u32,
    vb: ViewBox,
    offset: (f64, f64),
    scale: f64,
) -> Vec<u8> {
    cpu_draw(specs, w, h, vb, offset, scale, &|shapes, view, tile| {
        render_shapes(shapes, view, tile, &|| false).unwrap()
    })
}

/// [`cpu_render_view`] with the drawing step supplied (flat or group tree).
fn cpu_draw(
    specs: &[Spec],
    w: u32,
    h: u32,
    vb: ViewBox,
    offset: (f64, f64),
    scale: f64,
    draw: &dyn Fn(&[RenderShape<'_>], ViewMap, &mut Tile),
) -> Vec<u8> {
    let base = Paint {
        kind: PaintKind::Linear,
        spread: Spread::Pad,
        object_bounding_box: false,
        coordinates: [0.0; 6],
        bounds: [0.0; 4],
        user_inverse: identity(),
        gradient_inverse: identity(),
        stops: TableRange { first: 0, count: 0 },
        pattern: None,
        pattern_inverse: identity(),
        pattern_opacity: 1.0,
    };
    let mut edge_store: Vec<Vec<FlatEdge>> = Vec::new();
    let mut paint_store: Vec<Option<(Paint, Vec<PaintStop>)>> = Vec::new();
    for spec in specs {
        edge_store.push(
            spec_edges(spec)
                .iter()
                .map(|s| FlatEdge {
                    x0: (s.x0 - offset.0) / scale,
                    y0: (s.y0 - offset.1) / scale,
                    x1: (s.x1 - offset.0) / scale,
                    y1: (s.y1 - offset.1) / scale,
                })
                .collect(),
        );
        paint_store.push(match &spec.paint {
            SpecPaint::Solid(_) => None,
            SpecPaint::Linear { p, spread, stops } => Some((
                Paint {
                    kind: PaintKind::Linear,
                    spread: *spread,
                    coordinates: [p[0], p[1], p[2], p[3], 0.0, 0.0],
                    stops: TableRange {
                        first: 0,
                        count: stops.len(),
                    },
                    ..base
                },
                stops.clone(),
            )),
            SpecPaint::Radial { c, spread, stops } => Some((
                Paint {
                    kind: PaintKind::Radial,
                    spread: *spread,
                    coordinates: [c[0], c[1], c[2], c[3], c[4], 0.0],
                    stops: TableRange {
                        first: 0,
                        count: stops.len(),
                    },
                    ..base
                },
                stops.clone(),
            )),
        });
    }
    let shapes: Vec<RenderShape<'_>> = specs
        .iter()
        .enumerate()
        .map(|(i, spec)| RenderShape {
            edges: &edge_store[i],
            subpaths: &[],
            subpath_closed: &[],
            stroke_closed: false,
            evenodd: spec.evenodd,
            filled: true,
            fill_rgba: match spec.paint {
                SpecPaint::Solid(rgba) => rgba,
                _ => [0; 4],
            },
            stroke: None,
            transform: identity(),
            fill_gradient: paint_store[i]
                .as_ref()
                .map(|(paint, stops)| GradientRef { paint, stops }),
            stroke_gradient: None,
        })
        .collect();
    let view = ViewMap::from_viewbox(vb, w, h, None).unwrap();
    let mut tile = Tile::from_owned_rgba(w, h, vec![0; w as usize * h as usize * 4]).unwrap();
    draw(&shapes, view, &mut tile);
    tile.rgba().to_vec()
}

#[derive(Debug)]
struct Metrics {
    over_2: f64,
    over_16: f64,
    mae: f64,
    max_diff: u8,
}

fn compare(a: &[u8], b: &[u8]) -> Metrics {
    assert_eq!(a.len(), b.len());
    let pixels = a.len() / 4;
    let (mut over_2, mut over_16, mut sum, mut max_diff) = (0usize, 0usize, 0u64, 0u8);
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let worst = (0..4).map(|c| pa[c].abs_diff(pb[c])).max().unwrap();
        over_2 += usize::from(worst > 2);
        over_16 += usize::from(worst > 16);
        max_diff = max_diff.max(worst);
        sum += (0..4)
            .map(|c| u64::from(pa[c].abs_diff(pb[c])))
            .sum::<u64>();
    }
    Metrics {
        over_2: over_2 as f64 / pixels as f64,
        over_16: over_16 as f64 / pixels as f64,
        mae: sum as f64 / (pixels * 4) as f64 / 255.0,
        max_diff,
    }
}

/// (fraction over 2/255, fraction over 16/255, mean absolute error).
type Bounds = (f64, f64, f64);

/// The ADR-0035 tolerances.
const ADR: Bounds = (
    PARITY_OVER_2_MAX_FRACTION,
    PARITY_OVER_16_MAX_FRACTION,
    PARITY_MEAN_ABS_ERROR_MAX,
);

/// Small frames (under 100k pixels): the ADR bounds, except over 16/255 which
/// is granular at one pixel (0.002% of 50k pixels) and gets 0.01% headroom.
const SMALL_FRAME: Bounds = (
    PARITY_OVER_2_MAX_FRACTION,
    0.0001,
    PARITY_MEAN_ABS_ERROR_MAX,
);

fn check(name: &str, specs: &[Spec], w: u32, h: u32, bounds: Bounds) {
    let scene = scene_for(specs, w, h);
    let cpu = cpu_render(specs, w, h);
    check_against(name, specs, &scene, &cpu, bounds);
}

fn check_against(name: &str, specs: &[Spec], scene: &GpuScene, cpu: &[u8], bounds: Bounds) {
    let (w, h) = (scene.width, scene.height);
    assert!(vexel_gpu_scene::classify(scene).is_supported());
    let encoded = encode(scene).unwrap();
    let m = compare(&reference::render_straight(&encoded), cpu);
    println!(
        "{name}: {w}x{h} shapes={} kept={} >2/255={:.5}% >16/255={:.5}% mae={:.6} max={}",
        specs.len(),
        encoded.shapes.len(),
        m.over_2 * 100.0,
        m.over_16 * 100.0,
        m.mae,
        m.max_diff
    );
    assert!(m.over_2 <= bounds.0, "{m:?}");
    assert!(m.over_16 <= bounds.1, "{m:?}");
    assert!(m.mae <= bounds.2, "{m:?}");
}

/// Vertices on the 1/256 grid: the encoder's quantisation is lossless, so
/// this proves the encoding and the reference sampling. Meets the ADR bounds.
#[test]
fn grid_aligned_scenes_match_cpu_within_adr() {
    for (seed, count, w, h) in [
        (31, 100, 200, 136),
        (32, 200, 333, 250),
        (33, 300, 512, 384),
    ] {
        let s = snapped(specs(seed, count, f64::from(w), f64::from(h)));
        check(&format!("snapped seed {seed}"), &s, w, h, ADR);
    }
}

#[test]
fn solid_fills_match_cpu() {
    for seed in 1..=4 {
        let mut s = specs(seed, 80, 200.0, 136.0);
        s.retain(|s| matches!(s.paint, SpecPaint::Solid(_)));
        check(&format!("solid seed {seed}"), &s, 200, 136, SMALL_FRAME);
    }
}

#[test]
fn gradient_fills_match_cpu() {
    for seed in 21..=23 {
        let mut s = specs(seed, 120, 256.0, 192.0);
        s.retain(|s| !matches!(s.paint, SpecPaint::Solid(_)));
        check(&format!("gradient seed {seed}"), &s, 256, 192, SMALL_FRAME);
    }
}

#[test]
fn mixed_scenes_match_cpu() {
    for (seed, count, w, h) in [(11, 40, 200, 136), (12, 120, 333, 250), (13, 300, 512, 384)] {
        let s = specs(seed, count, f64::from(w), f64::from(h));
        check(&format!("mixed seed {seed}"), &s, w, h, SMALL_FRAME);
    }
}

/// Unsnapped (arbitrary f64) vertices at sizes 256 / 900 / 1024, 12 seeds:
/// every scene meets the ADR-0035 bounds with the layout's edge precision.
#[test]
fn unsnapped_scenes_meet_adr_bounds() {
    for k in 0..12u64 {
        let size = [256u32, 900, 1024][(k % 3) as usize];
        let (w, h) = (size, size * 3 / 4);
        let count = (w as usize * h as usize) / 700;
        let s = specs(100 + k, count, f64::from(w), f64::from(h));
        check(&format!("unsnapped seed {}", 100 + k), &s, w, h, ADR);
    }
}

/// Meet letterboxing: the CPU excludes samples outside the `ViewMap` content
/// rectangle; the header's content range must reproduce that exactly. Scale 2,
/// so the content rectangle is half the canvas in one axis, centred.
#[test]
fn letterboxed_frames_match_cpu() {
    for (seed, count, w, h, tall) in [
        (41, 120, 256, 192, false),
        (42, 160, 300, 200, true),
        (43, 200, 512, 384, false),
    ] {
        let (fw, fh) = (f64::from(w), f64::from(h));
        let (vb, offset) = if tall {
            // Pillarbox: content width fw / 2, centred horizontally.
            (
                ViewBox {
                    x: 0.0,
                    y: 0.0,
                    width: fw / 4.0,
                    height: fh / 2.0,
                },
                (fw / 4.0, 0.0),
            )
        } else {
            (
                ViewBox {
                    x: 0.0,
                    y: 0.0,
                    width: fw / 2.0,
                    height: fh / 4.0,
                },
                (0.0, fh / 4.0),
            )
        };
        // Solid only: gradient coordinates live in user space on the CPU.
        let mut s = snapped(specs(seed, count, fw, fh));
        s.retain(|s| matches!(s.paint, SpecPaint::Solid(_)));
        let mut scene = scene_for(&s, w, h);
        scene.content_rect = [
            offset.0,
            offset.1,
            offset.0 + vb.width * 2.0,
            offset.1 + vb.height * 2.0,
        ];
        let cpu = cpu_render_view(&s, w, h, vb, offset, 2.0);
        check_against(&format!("letterbox seed {seed}"), &s, &scene, &cpu, ADR);
    }
}

include!("common/groups.rs");

/// Opacity groups, 1 to 4 levels, opacity 0.1..1.0 (1.0 included: the CPU
/// flattens it), mixed solid and gradient fills, overlapping children: the
/// reference meets the ADR-0035 bounds against `render_groups`.
#[test]
fn group_scenes_match_cpu_within_adr() {
    for (k, (seed, count, w, h)) in [
        (51, 60, 200, 136),
        (52, 120, 256, 192),
        (53, 200, 333, 250),
        (54, 300, 512, 384),
        (55, 150, 256, 192),
        (56, 250, 400, 300),
    ]
    .into_iter()
    .enumerate()
    {
        let max_depth = 1 + (k as u32 % 4);
        let s = specs(seed, count, f64::from(w), f64::from(h));
        let nodes = group_tree(seed, count, max_depth);
        let scene = group_scene(&s, &nodes, w, h);
        let cpu = group_cpu(&s, &nodes, w, h);
        check_against(
            &format!("groups seed {seed} depth<={max_depth}"),
            &s,
            &scene,
            &cpu,
            ADR,
        );
    }
}

/// Nesting deeper than the layout's `MAX_GROUP_DEPTH` is a CPU fallback.
#[test]
fn deep_groups_fall_back() {
    let mut node = GpuNode::Shape(GpuShape {
        edges: closed(&[(1.0, 1.0), (9.0, 1.0), (9.0, 9.0)]),
        evenodd: false,
        fill: Some(GpuPaint::Solid([9, 9, 9, 255])),
        stroke: None,
        clips: Vec::new(),
    });
    for depth in 1..=5 {
        node = GpuNode::Group(GpuGroup {
            opacity: 0.5,
            clips: Vec::new(),
            children: vec![node],
        });
        let mut scene = GpuScene::new(16, 16);
        scene.nodes = vec![node.clone()];
        let want = if depth <= 4 {
            vexel_gpu_scene::Capability::Supported
        } else {
            vexel_gpu_scene::Capability::Unsupported(
                vexel_gpu_scene::UnsupportedReason::GroupTooDeep,
            )
        };
        assert_eq!(vexel_gpu_scene::classify(&scene), want, "depth {depth}");
    }
}
