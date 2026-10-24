//! Seeded scene generator shared by the rasterizer differential test and the
//! release benchmark. Storage is leaked: these are short-lived test processes.
#![allow(dead_code)]

use vexel_geometry::{Affine, affine_identity};
use vexel_render::{
    ClipShape, DashedShape, FlatEdge, GradientRef, Linecap, Linejoin, RenderClip, RenderGroup,
    RenderNode, RenderShape, StrokeStyle, ViewBox, ViewMap,
};
use vexel_scene::{Paint, PaintKind, PaintStop, Spread, TableRange};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profile {
    Fill,
    Gradient,
    Stroke,
    Nest,
    Clip,
    Mix,
}

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn color(&mut self) -> [u8; 4] {
        let a = if self.below(4) == 0 { 255 } else { 40 + self.below(216) as u8 };
        [self.below(256) as u8, self.below(256) as u8, self.below(256) as u8, a]
    }
}

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

fn leak_slice<T>(value: Vec<T>) -> &'static [T] {
    Box::leak(value.into_boxed_slice())
}

pub static STOPS: [PaintStop; 4] = [
    PaintStop { offset: 0.0, rgba: [250, 30, 20, 255] },
    PaintStop { offset: 0.35, rgba: [240, 220, 40, 200] },
    PaintStop { offset: 0.7, rgba: [20, 160, 230, 255] },
    PaintStop { offset: 1.0, rgba: [90, 20, 160, 90] },
];

fn map(m: Affine, x: f64, y: f64) -> (f64, f64) {
    (m.a * x + m.c * y + m.e, m.b * x + m.d * y + m.f)
}

fn transform(rng: &mut Rng, cx: f64, cy: f64) -> Affine {
    let angle = rng.range(0.0, 6.28);
    let (s, c) = angle.sin_cos();
    let (sx, sy) = (rng.range(0.7, 1.4), rng.range(0.7, 1.4));
    Affine { a: c * sx, b: s * sx, c: -s * sy, d: c * sy, e: cx, f: cy }
}

/// Local-space closed blob around the origin.
fn blob(rng: &mut Rng, radius: f64) -> Vec<(f64, f64)> {
    let n = 3 + rng.below(10);
    (0..n)
        .map(|i| {
            let angle = (i as f64 + rng.range(0.0, 0.8)) / n as f64 * 6.2831853;
            let r = radius * rng.range(0.4, 1.0);
            (r * angle.cos(), r * angle.sin())
        })
        .collect()
}

fn to_edges(points: &[(f64, f64)], xf: Affine, closed: bool) -> &'static [FlatEdge] {
    let doc: Vec<(f64, f64)> = points.iter().map(|p| map(xf, p.0, p.1)).collect();
    let count = if closed { doc.len() } else { doc.len() - 1 };
    leak_slice(
        (0..count)
            .map(|i| {
                let (a, b) = (doc[i], doc[(i + 1) % doc.len()]);
                FlatEdge { x0: a.0, y0: a.1, x1: b.0, y1: b.1 }
            })
            .collect(),
    )
}

fn stroke_style(rng: &mut Rng) -> StrokeStyle {
    StrokeStyle {
        rgba: rng.color(),
        width: rng.range(0.6, 7.0),
        miter_limit: 4.0,
        dash_offset: rng.range(0.0, 6.0),
        cap: [Linecap::Butt, Linecap::Square, Linecap::Round][rng.below(3)],
        join: [Linejoin::Miter, Linejoin::Bevel, Linejoin::Round][rng.below(3)],
        crisp_edges: false,
    }
}

fn gradient(rng: &mut Rng, edges: &[FlatEdge]) -> GradientRef<'static> {
    let (mut lo, mut hi) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
    for e in edges {
        lo = (lo.0.min(e.x0), lo.1.min(e.y0));
        hi = (hi.0.max(e.x0), hi.1.max(e.y0));
    }
    let bbox = rng.below(2) == 0;
    let spread = [Spread::Pad, Spread::Reflect, Spread::Repeat][rng.below(3)];
    let kind = if rng.below(2) == 0 { PaintKind::Linear } else { PaintKind::Radial };
    let (w, h) = ((hi.0 - lo.0).max(1.0), (hi.1 - lo.1).max(1.0));
    let (cx, cy) = ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0);
    let coordinates = match (kind, bbox) {
        (PaintKind::Linear, true) => [0.0, 0.0, rng.range(0.4, 1.0), rng.range(0.0, 1.0), 0.0, 0.0],
        (PaintKind::Linear, false) => [lo.0, lo.1, hi.0, cy + rng.range(-w, w) / 3.0, 0.0, 0.0],
        (_, true) => [0.5, 0.5, rng.range(0.3, 0.7), rng.range(0.3, 0.7), rng.range(0.3, 0.7), 0.0],
        (_, false) => {
            let r = w.max(h) * rng.range(0.3, 0.7);
            [cx, cy, r, cx + rng.range(-r, r) / 4.0, cy + rng.range(-r, r) / 4.0, 0.0]
        }
    };
    let gradient_inverse = if rng.below(3) == 0 {
        Affine { a: 1.0, b: 0.2, c: -0.1, d: 1.1, e: rng.range(-0.2, 0.2), f: 0.0 }
    } else {
        affine_identity()
    };
    let paint = leak(Paint {
        kind,
        spread,
        object_bounding_box: bbox,
        coordinates,
        bounds: [lo.0, lo.1, hi.0, hi.1],
        user_inverse: affine_identity(),
        gradient_inverse,
        stops: TableRange { first: 0, count: STOPS.len() },
        pattern: None,
        pattern_inverse: affine_identity(),
        pattern_opacity: 1.0,
    });
    GradientRef { paint, stops: &STOPS }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Fill,
    Gradient,
    Stroke,
    Open,
    Both,
}

fn shape(rng: &mut Rng, size: f64, kind: Kind) -> RenderShape<'static> {
    let radius = rng.range(3.0, (size / 10.0).clamp(6.0, 40.0));
    let (cx, cy) = (rng.range(-radius, size + radius), rng.range(-radius, size + radius));
    let xf = transform(rng, cx, cy);
    let open = kind == Kind::Open;
    let local = if open {
        let n = 3 + rng.below(6);
        (0..n).map(|_| (rng.range(-radius, radius), rng.range(-radius, radius))).collect()
    } else {
        blob(rng, radius)
    };
    let edges = to_edges(&local, xf, !open);
    let mut s = RenderShape {
        edges,
        subpaths: &[],
        subpath_closed: &[],
        stroke_closed: !open,
        evenodd: rng.below(3) == 0,
        filled: !open,
        fill_rgba: rng.color(),
        stroke: None,
        transform: xf,
        fill_gradient: None,
        stroke_gradient: None,
    };
    match kind {
        Kind::Fill => {}
        Kind::Gradient => s.fill_gradient = Some(gradient(rng, edges)),
        Kind::Stroke | Kind::Open => s.stroke = Some(stroke_style(rng)),
        Kind::Both => {
            s.stroke = Some(stroke_style(rng));
            if rng.below(2) == 0 {
                s.stroke_gradient = Some(gradient(rng, edges));
            }
        }
    }
    s
}

fn clip(rng: &mut Rng, size: f64, parent: Option<&'static RenderClip<'static>>) -> &'static RenderClip<'static> {
    let (cx, cy) = (rng.range(0.0, size), rng.range(0.0, size));
    let radius = rng.range(size * 0.15, size * 0.6);
    let local = blob(rng, radius);
    let edges = to_edges(&local, Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: cx, f: cy }, true);
    let shapes = leak_slice(vec![ClipShape { edges: 0..edges.len(), evenodd: false, filled: true }]);
    leak(RenderClip {
        edges,
        shapes,
        parent,
        object_bounding_box: false,
        bounds: [0.0; 4],
        user_inverse: affine_identity(),
    })
}

fn leaf(rng: &mut Rng, size: f64, profile: Profile, clips: &[&'static RenderClip<'static>]) -> RenderNode<'static> {
    let kind = match profile {
        Profile::Fill | Profile::Clip | Profile::Nest => Kind::Fill,
        Profile::Gradient => Kind::Gradient,
        Profile::Stroke => if rng.below(5) == 0 { Kind::Open } else { Kind::Stroke },
        Profile::Mix => [Kind::Fill, Kind::Gradient, Kind::Stroke, Kind::Open, Kind::Both][rng.below(5)],
    };
    let clip = if clips.is_empty() || rng.below(3) == 0 { None } else { Some(clips[rng.below(clips.len())]) };
    RenderNode::Shape { shape: shape(rng, size, kind), clip }
}

/// Seeded scene of about `count` shapes.
pub fn scene(seed: u64, size: u32, count: usize, profile: Profile) -> &'static [RenderNode<'static>] {
    let mut rng = Rng(seed ^ 0x9E37_79B9_7F4A_7C15);
    let fsize = f64::from(size);
    let clips: Vec<&'static RenderClip<'static>> = if matches!(profile, Profile::Clip | Profile::Mix) {
        let mut v: Vec<&'static RenderClip<'static>> = Vec::new();
        for i in 0..6 {
            let parent = if i % 3 == 2 { v.last().copied() } else { None };
            v.push(clip(&mut rng, fsize, parent));
        }
        v
    } else {
        Vec::new()
    };
    let nested = matches!(profile, Profile::Nest | Profile::Mix);
    let mut top = Vec::new();
    let mut made = 0;
    while made < count {
        if nested && rng.below(8) == 0 {
            let (group, n) = group(&mut rng, fsize, profile, &clips, 0);
            top.push(RenderNode::Group(group));
            made += n;
        } else {
            top.push(leaf(&mut rng, fsize, profile, &clips));
            made += 1;
        }
    }
    leak_slice(top)
}

fn group(
    rng: &mut Rng,
    size: f64,
    profile: Profile,
    clips: &[&'static RenderClip<'static>],
    depth: usize,
) -> (RenderGroup<'static>, usize) {
    let mut children = Vec::new();
    let mut made = 0;
    for _ in 0..3 + rng.below(8) {
        if depth < 3 && rng.below(5) == 0 {
            let (inner, n) = group(rng, size, profile, clips, depth + 1);
            children.push(RenderNode::Group(inner));
            made += n;
        } else {
            children.push(leaf(rng, size, profile, clips));
            made += 1;
        }
    }
    let opacity = [1.0, 0.8, 0.5, 0.25][rng.below(4)];
    let clip = if clips.is_empty() || rng.below(3) != 0 { None } else { Some(clips[rng.below(clips.len())]) };
    (RenderGroup { opacity, clip, mask: None, children: leak_slice(children) }, made)
}

/// Seeded dashed strokes (the dashed entry point has no groups).
pub fn dashed(seed: u64, size: u32, count: usize) -> Vec<DashedShape<'static>> {
    let mut rng = Rng(seed ^ 0xD1B5_4A32_D192_ED03);
    (0..count)
        .map(|_| {
            let kind = if rng.below(3) == 0 { Kind::Open } else { Kind::Stroke };
            let s = shape(&mut rng, f64::from(size), kind);
            let pattern = leak_slice(vec![rng.range(1.5, 9.0), rng.range(1.0, 6.0), rng.range(1.0, 4.0)]);
            DashedShape { shape: s, dashes: pattern }
        })
        .collect()
}

pub fn view(size: u32) -> ViewMap {
    ViewMap::from_viewbox(
        ViewBox { x: 0.0, y: 0.0, width: f64::from(size), height: f64::from(size) },
        size,
        size,
        None,
    )
    .unwrap()
}
