// Shared clip scene generator for the clip parity tests (included by
// tests/clip_parity.rs and tests/gpu_clip.rs after common/strokes.rs, whose
// imports and `SRng` / `StrokeCase` / `gpu_stroke` it uses).

use std::sync::Arc;
use vexel_gpu_scene::input::{GpuClipChild, GpuClipPath, GpuGroup};
use vexel_render::{ClipShape, RenderClip, RenderGroup, RenderNode, render_groups};

/// One child of a clip: rings in clip-local space, its own rule, filled flag.
pub struct ChildSpec {
    pub rings: Vec<Vec<(f64, f64)>>,
    pub evenodd: bool,
    pub filled: bool,
}

/// A clip path: union of children, `parent` chain, local -> device matrix and
/// optionally object-bounding-box units (then rings are in the unit box and
/// `bounds` is the clipped content's box; `to_device` is the identity).
pub struct ClipSpec {
    pub children: Vec<ChildSpec>,
    pub parent: Option<usize>,
    pub to_device: Affine,
    pub bounds: Option<[f64; 4]>,
}

pub enum CNode {
    Shape {
        case: StrokeCase,
        stroke: bool,
        clip: Option<usize>,
    },
    Group {
        opacity: f64,
        clip: Option<usize>,
        children: Vec<CNode>,
    },
}

pub struct ClipScene {
    pub specs: Vec<ClipSpec>,
    pub nodes: Vec<CNode>,
}

#[derive(Clone, Copy, Default)]
pub struct Features {
    pub transforms: bool,
    pub chains: bool,
    pub obb: bool,
    pub groups: bool,
    pub strokes: bool,
}

fn ngon(cx: f64, cy: f64, r: f64, n: usize, phase: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|k| {
            let a = phase + k as f64 / n as f64 * std::f64::consts::TAU;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

fn pentagram(cx: f64, cy: f64, r: f64) -> Vec<(f64, f64)> {
    (0..5)
        .map(|k| {
            let a = (k * 2) as f64 / 5.0 * std::f64::consts::TAU;
            (cx + r * a.sin(), cy - r * a.cos())
        })
        .collect()
}

fn rect_ring(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(f64, f64)> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn inverse_affine(m: Affine) -> Affine {
    let det = m.a * m.d - m.b * m.c;
    Affine {
        a: m.d / det,
        b: -m.b / det,
        c: -m.c / det,
        d: m.a / det,
        e: (m.c * m.f - m.d * m.e) / det,
        f: (m.b * m.e - m.a * m.f) / det,
    }
}

fn map_point(m: Affine, p: (f64, f64)) -> (f64, f64) {
    (m.a * p.0 + m.c * p.1 + m.e, m.b * p.0 + m.d * p.1 + m.f)
}

/// Adds one clip (and maybe a parent) covering around `area`; returns its index.
fn make_clip(
    rng: &mut SRng,
    specs: &mut Vec<ClipSpec>,
    f: Features,
    area: [f64; 4],
    allow_parent: bool,
) -> usize {
    let parent = (f.chains && allow_parent && rng.next() < 0.5)
        .then(|| make_clip(rng, specs, f, area, false));
    let (cx, cy) = ((area[0] + area[2]) / 2.0, (area[1] + area[3]) / 2.0);
    let r = ((area[2] - area[0]).max(area[3] - area[1]) / 2.0 * rng.range(0.5, 1.1)).max(6.0);
    let obb = f.obb && rng.next() < 0.3;
    let transformed = f.transforms && !obb && rng.next() < 0.6;
    let to_device = if transformed {
        let th = rng.range(0.0, std::f64::consts::TAU);
        let (sx, sy) = (rng.range(0.6, 1.6), rng.range(0.6, 1.6) * if rng.next() < 0.3 { -1.0 } else { 1.0 });
        Affine {
            a: sx * th.cos(),
            b: sx * th.sin(),
            c: -sy * th.sin(),
            d: sy * th.cos(),
            e: cx,
            f: cy,
        }
    } else {
        identity_affine()
    };
    // Clip-local centre and size: the device centre for identity clips, the
    // local origin for transformed ones, the unit box for bounding-box clips.
    let (lx, ly, lr) = if obb {
        (0.5, 0.5, rng.range(0.2, 0.55))
    } else if transformed {
        (0.0, 0.0, r / 1.1)
    } else {
        (cx, cy, r)
    };
    let child = |rings: Vec<Vec<(f64, f64)>>, evenodd: bool, filled: bool| ChildSpec {
        rings,
        evenodd,
        filled,
    };
    let children = match rng.pick(7) {
        0 => vec![child(vec![rect_ring(lx - lr, ly - lr * 0.8, lx + lr, ly + lr * 0.8)], false, true)],
        1 => vec![child(vec![pentagram(lx, ly, lr)], false, true)],
        2 => vec![child(vec![pentagram(lx, ly, lr)], true, true)],
        3 => vec![
            child(
                vec![
                    rect_ring(lx - lr, ly - lr, lx + lr, ly + lr),
                    rect_ring(lx - lr * 0.4, ly - lr * 0.4, lx + lr * 0.4, ly + lr * 0.4),
                ],
                true,
                true,
            ),
            child(vec![ngon(lx + lr * 0.5, ly, lr * 0.6, 9, 0.3)], false, true),
            child(vec![rect_ring(lx - 1e3, ly - 1e3, lx + 1e3, ly + 1e3)], false, false),
        ],
        4 => vec![
            child(vec![ngon(lx - lr * 0.4, ly, lr * 0.7, 7, 0.0)], false, true),
            child(vec![ngon(lx + lr * 0.4, ly, lr * 0.7, 5, 0.2)], true, true),
        ],
        5 => Vec::new(),
        _ => vec![child(vec![rect_ring(lx - lr, ly - lr, lx + lr, ly + lr)], false, false)],
    };
    specs.push(ClipSpec {
        children,
        parent,
        to_device,
        bounds: obb.then_some(area),
    });
    specs.len() - 1
}

fn make_shape(rng: &mut SRng, w: f64, h: f64, strokes: bool) -> (StrokeCase, bool, [f64; 4]) {
    let (cx, cy) = (rng.range(0.0, w), rng.range(0.0, h));
    let r = rng.range(8.0, 70.0);
    let ring = ngon(cx, cy, r, 3 + rng.pick(8), rng.range(0.0, 1.0));
    let stroke = strokes && rng.next() < 0.7;
    let fill = (!stroke || rng.next() < 0.5)
        .then(|| [rng.pick(256) as u8, rng.pick(256) as u8, rng.pick(256) as u8, if rng.next() < 0.5 { 255 } else { 100 + rng.pick(150) as u8 }]);
    let case = StrokeCase {
        subpaths: vec![(ring, true)],
        style: StrokeStyle {
            rgba: [rng.pick(256) as u8, rng.pick(256) as u8, rng.pick(256) as u8, 255],
            width: rng.range(1.0, 8.0),
            miter_limit: 4.0,
            dash_offset: 0.0,
            cap: Linecap::Butt,
            join: [Linejoin::Miter, Linejoin::Round, Linejoin::Bevel][rng.pick(3)],
            crisp_edges: false,
        },
        transform: identity_affine(),
        dashes: Vec::new(),
        fill,
        evenodd: rng.next() < 0.3,
    };
    (case, stroke || fill.is_none(), [cx - r, cy - r, cx + r, cy + r])
}

pub fn clip_scene(seed: u64, count: usize, w: f64, h: f64, f: Features) -> ClipScene {
    let mut rng = SRng(seed);
    let mut specs: Vec<ClipSpec> = Vec::new();
    let mut shapes = Vec::new();
    for _ in 0..count {
        let (case, stroke, area) = make_shape(&mut rng, w, h, f.strokes);
        let clip = if rng.next() < 0.85 {
            let reuse = !specs.is_empty() && rng.next() < 0.25;
            Some(if reuse {
                // Share an earlier non-bounding-box clip.
                let i = rng.pick(specs.len());
                if specs[i].bounds.is_some() { make_clip(&mut rng, &mut specs, f, area, true) } else { i }
            } else {
                make_clip(&mut rng, &mut specs, f, area, true)
            })
        } else {
            None
        };
        shapes.push(CNode::Shape { case, stroke, clip });
    }
    let nodes = if f.groups { group_up(&mut rng, &mut specs, f, shapes, w, h, 0) } else { shapes };
    ClipScene { specs, nodes }
}

fn group_up(
    rng: &mut SRng,
    specs: &mut Vec<ClipSpec>,
    f: Features,
    shapes: Vec<CNode>,
    w: f64,
    h: f64,
    depth: u32,
) -> Vec<CNode> {
    let mut out = Vec::new();
    let mut it = shapes.into_iter().peekable();
    while it.peek().is_some() {
        if rng.next() < 0.4 {
            let take = 1 + rng.pick(4);
            let mut children: Vec<CNode> = it.by_ref().take(take).collect();
            if depth < 1 && children.len() > 1 && rng.next() < 0.4 {
                children = group_up(rng, specs, f, children, w, h, depth + 1);
            }
            let clip = (rng.next() < 0.75).then(|| {
                let g = Features { chains: false, obb: false, ..f };
                let (cx, cy) = (rng.range(0.0, w), rng.range(0.0, h));
                let half = rng.range(40.0, 160.0);
                make_clip(rng, specs, g, [cx - half, cy - half, cx + half, cy + half], false)
            });
            let opacity = [1.0, 0.5, 0.8, 0.25][rng.pick(4)];
            out.push(CNode::Group { opacity, clip, children });
        } else {
            out.push(it.next().unwrap());
        }
    }
    out
}

// ---- GPU input ----------------------------------------------------------

fn spec_gpu(spec: &ClipSpec) -> Arc<GpuClipPath> {
    let map = |p: (f64, f64)| {
        let p = match spec.bounds {
            Some(b) => (b[0] + p.0 * (b[2] - b[0]), b[1] + p.1 * (b[3] - b[1])),
            None => p,
        };
        map_point(spec.to_device, p)
    };
    Arc::new(GpuClipPath {
        children: spec
            .children
            .iter()
            .filter(|c| c.filled)
            .map(|c| GpuClipChild {
                edges: c
                    .rings
                    .iter()
                    .flat_map(|ring| {
                        (0..ring.len()).map(|k| {
                            let (a, b) = (map(ring[k]), map(ring[(k + 1) % ring.len()]));
                            GpuSegment { x0: a.0, y0: a.1, x1: b.0, y1: b.1 }
                        })
                    })
                    .collect(),
                evenodd: c.evenodd,
            })
            .collect(),
    })
}

fn chain_of(specs: &[ClipSpec], arcs: &[Arc<GpuClipPath>], clip: Option<usize>) -> Vec<Arc<GpuClipPath>> {
    let mut chain = Vec::new();
    let mut at = clip;
    while let Some(i) = at {
        chain.push(arcs[i].clone());
        at = specs[i].parent;
    }
    chain.reverse();
    chain
}

fn case_shape(case: &StrokeCase, stroke: bool) -> GpuShape {
    let (edges, ranges, closed) = case.edges();
    let rs = render_shape(case, &edges, &ranges, &closed);
    let export = stroke
        .then(|| export_stroke(&rs, &[], false).unwrap())
        .flatten();
    GpuShape {
        edges: edges
            .iter()
            .map(|e| GpuSegment { x0: e.x0, y0: e.y0, x1: e.x1, y1: e.y1 })
            .collect(),
        evenodd: case.evenodd,
        fill: case.fill.map(GpuPaint::Solid),
        stroke: export.map(|e| gpu_stroke(&e, case.style.rgba, identity_affine())),
        clips: Vec::new(),
    }
}

fn gpu_nodes_of(
    scene: &ClipScene,
    arcs: &[Arc<GpuClipPath>],
    nodes: &[CNode],
) -> Vec<GpuNode> {
    nodes
        .iter()
        .map(|n| match n {
            CNode::Shape { case, stroke, clip } => {
                let mut s = case_shape(case, *stroke);
                s.clips = chain_of(&scene.specs, arcs, *clip);
                GpuNode::Shape(s)
            }
            CNode::Group { opacity, clip, children } => GpuNode::Group(GpuGroup {
                opacity: *opacity,
                clips: chain_of(&scene.specs, arcs, *clip),
                children: gpu_nodes_of(scene, arcs, children),
            }),
        })
        .collect()
}

pub fn clip_gpu_scene(scene: &ClipScene, w: u32, h: u32) -> GpuScene {
    let arcs: Vec<_> = scene.specs.iter().map(spec_gpu).collect();
    let mut out = GpuScene::new(w, h);
    out.nodes = gpu_nodes_of(scene, &arcs, &scene.nodes);
    out
}

// ---- CPU render ---------------------------------------------------------

fn cpu_clips(specs: &[ClipSpec]) -> Vec<&'static RenderClip<'static>> {
    let mut out: Vec<&'static RenderClip<'static>> = Vec::new();
    for spec in specs {
        let mut edges = Vec::new();
        let mut shapes = Vec::new();
        for c in &spec.children {
            let start = edges.len();
            for ring in &c.rings {
                for k in 0..ring.len() {
                    let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
                    edges.push(FlatEdge { x0: a.0, y0: a.1, x1: b.0, y1: b.1 });
                }
            }
            shapes.push(ClipShape { edges: start..edges.len(), evenodd: c.evenodd, filled: c.filled });
        }
        let clip = RenderClip {
            edges: Box::leak(edges.into_boxed_slice()),
            shapes: Box::leak(shapes.into_boxed_slice()),
            parent: spec.parent.map(|p| out[p]),
            object_bounding_box: spec.bounds.is_some(),
            bounds: spec.bounds.unwrap_or([0.0; 4]),
            user_inverse: inverse_affine(spec.to_device),
        };
        out.push(Box::leak(Box::new(clip)));
    }
    out
}

fn cpu_nodes_of(
    specs: &[ClipSpec],
    clips: &[&'static RenderClip<'static>],
    nodes: &[CNode],
) -> Vec<RenderNode<'static>> {
    nodes
        .iter()
        .map(|n| match n {
            CNode::Shape { case, stroke, clip } => {
                let (edges, ranges, closed) = case.edges();
                let (edges, ranges, closed): (&'static [FlatEdge], &'static [std::ops::Range<usize>], &'static [bool]) = (
                    Box::leak(edges.into_boxed_slice()),
                    Box::leak(ranges.into_boxed_slice()),
                    Box::leak(closed.into_boxed_slice()),
                );
                let case: &'static StrokeCase = Box::leak(Box::new(StrokeCase {
                    subpaths: Vec::new(),
                    style: case.style,
                    transform: case.transform,
                    dashes: Vec::new(),
                    fill: case.fill,
                    evenodd: case.evenodd,
                }));
                let mut shape = render_shape(case, edges, ranges, closed);
                if !*stroke {
                    shape.stroke = None;
                }
                RenderNode::Shape {
                    shape,
                    clip: clip.map(|i| clips[i]),
                }
            }
            CNode::Group { opacity, clip, children } => RenderNode::Group(RenderGroup {
                opacity: *opacity,
                clip: clip.map(|i| clips[i]),
                mask: None,
                children: Box::leak(cpu_nodes_of(specs, clips, children).into_boxed_slice()),
            }),
        })
        .collect()
}

/// The CPU rasterizer (`render_groups`, identity view), straight RGBA8.
pub fn clip_cpu_render(scene: &ClipScene, w: u32, h: u32) -> Vec<u8> {
    let clips = cpu_clips(&scene.specs);
    let tree = cpu_nodes_of(&scene.specs, &clips, &scene.nodes);
    let view = ViewMap::from_viewbox(
        ViewBox { x: 0.0, y: 0.0, width: f64::from(w), height: f64::from(h) },
        w,
        h,
        None,
    )
    .unwrap();
    let mut tile = Tile::from_owned_rgba(w, h, vec![0; (w * h * 4) as usize]).unwrap();
    render_groups(&tree, view, &mut tile, &|| false).unwrap();
    tile.rgba().to_vec()
}

pub const CLIP_CONFIGS: [(&str, u64, usize, Features); 7] = [
    ("rects-paths", 0xC1, 60, Features { transforms: false, chains: false, obb: false, groups: false, strokes: false }),
    ("transforms", 0xC2, 60, Features { transforms: true, chains: false, obb: false, groups: false, strokes: false }),
    ("chains", 0xC3, 60, Features { transforms: true, chains: true, obb: false, groups: false, strokes: false }),
    ("obb", 0xC4, 60, Features { transforms: false, chains: true, obb: true, groups: false, strokes: false }),
    ("groups", 0xC5, 70, Features { transforms: true, chains: true, obb: false, groups: true, strokes: false }),
    ("strokes", 0xC6, 60, Features { transforms: true, chains: true, obb: true, groups: false, strokes: true }),
    ("everything", 0xC7, 90, Features { transforms: true, chains: true, obb: true, groups: true, strokes: true }),
];
