//! Adapters for the GPU display route (ADR-0035): the prepared render shapes
//! lowered to `vexel-gpu-scene` input structs, and the Metal compute backend.
//!
//! Solid and gradient fills and strokes (caps, joins, dashes, as the CPU
//! exports them) lower, with their clip chains (inside opacity groups too;
//! filters, masks, patterns and blend modes do not); anything
//! else returns `None` and the frame renders on the CPU. Pure value
//! conversion, no GPU calls, so the lowering is testable without a device.

use super::layers::Geo;
use super::shapes::{Entry, OwnedShape};
use std::collections::HashMap;
use std::sync::Arc;
use vexel_geometry::{Affine, affine_compose, affine_valid};
use vexel_gpu_scene::input::{
    GpuCap, GpuClipChild, GpuClipPath, GpuColorStop, GpuGradient, GpuGroup, GpuJoinKind, GpuPaint,
    GpuScene, GpuSegment, GpuShape, GpuSpread, GpuStroke, GpuStrokeJoin, GpuStrokeSegment,
};
use vexel_gpu_scene::{GpuNode, MAX_GRADIENT_STOPS};
use vexel_render::export::{JoinKind, StrokeExport, export_stroke};
use vexel_render::{FlatEdge, GradientRef};
use vexel_scene::{Clip, Linecap, OpacityGroup, PaintKind, PaintStop, Spread};

fn affine(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Affine {
    Affine { a, b, c, d, e, f }
}

/// Lowers prepared entries (document space) to a device-space GPU scene.
/// A lowered scene and its estimated CPU raster work (weighted covered pixels).
pub(super) struct Lowered {
    pub scene: GpuScene,
    pub work: f64,
    /// Extra GPU cost of clipping in tile-entry equivalents
    /// ([`CLIP_VISIT_ENTRIES`] per clip edge visit of a tile thread group).
    pub extra_entries: f64,
}

pub(super) fn lower(
    entries: &[Entry<'_>],
    groups: &[OpacityGroup],
    clips: &[Clip],
    geo: &Geo,
    width: u32,
    height: u32,
) -> Option<Lowered> {
    let mut scene = GpuScene::new(width, height);
    let (cw, ch) = (geo.view.width * geo.scale_x, geo.view.height * geo.scale_y);
    scene.content_rect = [
        geo.offset_x.max(0.0),
        geo.offset_y.max(0.0),
        (geo.offset_x + cw).min(f64::from(width)),
        (geo.offset_y + ch).min(f64::from(height)),
    ];
    let mut lowering = Lowering {
        entries,
        groups,
        geo,
        size: (width, height),
        work: 0.0,
        clips,
        clip_cache: HashMap::new(),
        clip_visits: 0.0,
    };
    scene.nodes = lowering.range(0..entries.len(), None, 0)?;
    Some(Lowered {
        scene,
        work: lowering.work,
        extra_entries: lowering.clip_visits * CLIP_VISIT_ENTRIES,
    })
}

struct Lowering<'a, 'e> {
    entries: &'a [Entry<'e>],
    groups: &'a [OpacityGroup],
    geo: &'a Geo,
    size: (u32, u32),
    work: f64,
    clips: &'a [Clip],
    /// Lowered clip paths by scene clip index (shared by every shape using one).
    clip_cache: HashMap<usize, Arc<GpuClipPath>>,
    /// Sum over clipped shapes of (chain edges) x (tiles the shape covers).
    clip_visits: f64,
}

/// GPU cost of one clip edge visit by one tile thread group, in tile-entry
/// equivalents of the route policy model (calibrated in PERF.md, GPU clips).
pub(super) const CLIP_VISIT_ENTRIES: f64 = 0.008;
/// CPU work weight of a clipped pixel per clip link (axis-aligned clips are cheap on the CPU, rotated ones are not).
const CLIP_WEIGHT: f64 = 0.5;

impl Lowering<'_, '_> {
    /// The shapes `range` whose parent group is `parent`, in paint order
    /// (`render_scene` `range`). Opacity-1 groups are transparent and join the
    /// surrounding nodes; a group with a filter, mask or blend mode is not
    /// representable.
    fn range(
        &mut self,
        range: std::ops::Range<usize>,
        parent: Option<usize>,
        depth: usize,
    ) -> Option<Vec<GpuNode>> {
        if depth > 64 {
            return None;
        }
        let mut nodes = Vec::new();
        let mut cursor = range.start;
        for (index, group) in self.groups.iter().enumerate() {
            if group.parent.map(|g| g.index()) != parent {
                continue;
            }
            let (first, end) = (group.shapes.first, group.shapes.end()?);
            if first < cursor || end > range.end || first > end {
                return None;
            }
            self.flat(cursor..first, &mut nodes)?;
            cursor = end;
            if first == end {
                continue;
            }
            if group.filter.is_some()
                || group.mask.is_some()
                || group.blend_mode != 0
                || !group.opacity.is_finite()
                || !(0.0..=1.0).contains(&group.opacity)
            {
                return None;
            }
            let children = self.range(first..end, Some(index), depth + 1)?;
            if group.opacity == 1.0 {
                nodes.extend(children);
            } else {
                nodes.push(GpuNode::Group(GpuGroup {
                    opacity: group.opacity,
                    clips: Vec::new(),
                    children,
                }));
            }
        }
        self.flat(cursor..range.end, &mut nodes)?;
        Some(nodes)
    }

    fn flat(&mut self, range: std::ops::Range<usize>, nodes: &mut Vec<GpuNode>) -> Option<()> {
        let (geo, (width, height)) = (self.geo, self.size);
        for entry in self.entries.get(range)? {
            let Entry::Draw(p) = entry else {
                return None;
            };
            if p.image.is_some() || p.fill.is_some() || p.stroke.is_some() {
                return None;
            }
            let s = &p.shape;
            if s.fill_pattern || s.stroke_pattern || s.stroke.is_some_and(|st| st.crisp_edges) {
                return None;
            }
            let mut edges = Vec::new();
            let mut fill = None;
            if s.filled && !s.edges.is_empty() {
                fill = match &s.fill_gradient {
                    Some(_) => Some(gradient(s.as_render().fill_gradient?, geo)?),
                    None if s.fill_rgba[3] == 0 => None,
                    None => Some(GpuPaint::Solid(s.fill_rgba)),
                };
                if fill.is_some() {
                    edges = segments(s, geo);
                    self.work += bbox_area(&edges, width, height)
                        * match fill {
                            Some(GpuPaint::Solid(_)) => 1.0,
                            _ => GRADIENT_WEIGHT,
                        };
                }
            }
            let stroke = self.stroke(s)?;
            if fill.is_none() && stroke.is_none() {
                continue;
            }
            let clips = self.chain(p.clip)?;
            if !clips.is_empty() {
                self.clip_cost(&clips, &edges, stroke.as_ref());
            }
            nodes.push(GpuNode::Shape(GpuShape {
                edges,
                evenodd: s.evenodd,
                fill,
                stroke,
                clips,
            }));
        }
        Some(())
    }
}

impl Lowering<'_, '_> {
    /// The clip chain of a shape (outermost parent first) lowered to device
    /// space; `None` when a clip cannot lower (the frame renders on the CPU).
    fn chain(&mut self, clip: Option<usize>) -> Option<Vec<Arc<GpuClipPath>>> {
        let mut out = Vec::new();
        let mut at = clip;
        while let Some(index) = at {
            if out.len() > 64 {
                return None;
            }
            out.push(self.clip_path(index)?);
            at = self.clips.get(index)?.parent.map(|p| p.index());
        }
        out.reverse();
        Some(out)
    }

    /// One clip of the table: its filled children with their rules, in device
    /// pixels. The CPU maps a sample (document space) with `user_inverse` into
    /// the clip's local space (then, for bounding-box units, into the unit
    /// box) and tests the local edges; the edges are mapped the other way.
    fn clip_path(&mut self, index: usize) -> Option<Arc<GpuClipPath>> {
        if let Some(hit) = self.clip_cache.get(&index) {
            return Some(hit.clone());
        }
        let clip = self.clips.get(index)?;
        let b = clip.bounds;
        if b.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let nothing = !affine_valid(clip.user_inverse)
            || (clip.object_bounding_box && (b[2] <= b[0] || b[3] <= b[1]));
        let mut path = GpuClipPath::default();
        if !nothing {
            let forward = invert(clip.user_inverse)?;
            let geo = self.geo;
            let map = |x: f64, y: f64| {
                let (x, y) = if clip.object_bounding_box {
                    (b[0] + x * (b[2] - b[0]), b[1] + y * (b[3] - b[1]))
                } else {
                    (x, y)
                };
                geo.to_device(
                    forward.a * x + forward.c * y + forward.e,
                    forward.b * x + forward.d * y + forward.f,
                )
            };
            for child in clip.geometry.shapes.iter().filter(|c| c.filled) {
                let range = child.edges.first..child.edges.end()?;
                let edges = clip.geometry.edges.get(range)?;
                path.children.push(GpuClipChild {
                    edges: edges
                        .iter()
                        .map(|e| {
                            let e = FlatEdge::from(*e);
                            let (x0, y0) = map(e.x0, e.y0);
                            let (x1, y1) = map(e.x1, e.y1);
                            GpuSegment { x0, y0, x1, y1 }
                        })
                        .collect(),
                    evenodd: child.evenodd,
                });
            }
        }
        let path = Arc::new(path);
        self.clip_cache.insert(index, path.clone());
        Some(path)
    }

    /// Route cost of a clipped shape: CPU work for the clip tests and the GPU
    /// clip visits (chain edges x tiles the shape's box covers).
    fn clip_cost(
        &mut self,
        clips: &[Arc<GpuClipPath>],
        fill: &[GpuSegment],
        stroke: Option<&GpuStroke>,
    ) {
        let (width, height) = self.size;
        let area = if fill.is_empty() {
            // Stroke-only: the whole canvas is the safe over-estimate.
            let _ = stroke;
            f64::from(width) * f64::from(height)
        } else {
            bbox_area(fill, width, height)
        };
        let tiles = (area / 256.0).ceil().max(1.0);
        let edges: usize = clips
            .iter()
            .map(|c| c.children.iter().map(|k| k.edges.len()).sum::<usize>())
            .sum();
        self.work += area * CLIP_WEIGHT * clips.len() as f64;
        self.clip_visits += edges as f64 * tiles;
    }

    /// The stroke of `s` as the CPU builds it (`export_stroke`), lowered to
    /// device space; `Ok(None)`-like `Some(None)` when the shape paints none,
    /// `None` when the stroke cannot lower (the frame renders on the CPU).
    fn stroke(&mut self, s: &OwnedShape) -> Option<Option<GpuStroke>> {
        let geo = self.geo;
        let gradient_paint = s.stroke_gradient.is_some();
        let Ok(export) = export_stroke(&s.as_render(), &s.dashes, gradient_paint) else {
            return None;
        };
        let Some(export) = export else {
            return Some(None);
        };
        let style = s.stroke.as_ref()?;
        let paint = match &s.stroke_gradient {
            Some(_) => gradient(s.as_render().stroke_gradient?, geo)?,
            None => GpuPaint::Solid(style.rgba),
        };
        let stroke = lower_stroke(&export, paint, geo);
        let scale = geo.scale_x.abs().max(geo.scale_y.abs());
        self.work += stroke_area(&export, scale, self.size)
            * match stroke.paint {
                GpuPaint::Solid(_) => STROKE_WEIGHT,
                _ => STROKE_WEIGHT + GRADIENT_WEIGHT - 1.0,
            };
        Some(Some(stroke))
    }
}

/// Weight of a stroked pixel against a solid fill one in the CPU work estimate.
const STROKE_WEIGHT: f64 = 3.0;

/// Pixels the stroke's bounding box covers (centre-line box grown by the
/// radius, in device pixels).
fn stroke_area(e: &StrokeExport, scale: f64, size: (u32, u32)) -> f64 {
    let (mut l, mut t, mut r, mut b) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for s in &e.segments {
        for p in [s.a, s.b] {
            let (x, y) = (
                e.transform.a * p[0] + e.transform.c * p[1] + e.transform.e,
                e.transform.b * p[0] + e.transform.d * p[1] + e.transform.f,
            );
            l = l.min(x);
            r = r.max(x);
            t = t.min(y);
            b = b.max(y);
        }
    }
    let grow = e.radius * e.transform.a.abs().max(e.transform.d.abs()).max(1e-9) * scale;
    let w = (r.min(f64::from(size.0)) - l.max(0.0) + grow).max(0.0);
    let h = (b.min(f64::from(size.1)) - t.max(0.0) + grow).max(0.0);
    w * h
}

/// Inverse of an affine matrix; `None` when singular or not finite.
fn invert(m: Affine) -> Option<Affine> {
    let det = m.a * m.d - m.b * m.c;
    if !det.is_finite() || det.abs() < 1e-300 {
        return None;
    }
    let r = Affine {
        a: m.d / det,
        b: -m.b / det,
        c: -m.c / det,
        d: m.a / det,
        e: (m.c * m.f - m.d * m.e) / det,
        f: (m.b * m.e - m.a * m.f) / det,
    };
    [r.a, r.b, r.c, r.d, r.e, r.f]
        .iter()
        .all(|v| v.is_finite())
        .then_some(r)
}

fn cap(c: Linecap) -> GpuCap {
    match c {
        Linecap::Butt => GpuCap::Butt,
        Linecap::Round => GpuCap::Round,
        Linecap::Square => GpuCap::Square,
    }
}

/// Exporter output to the GPU input: local stroke space and a local -> device
/// transform (shape transform, then the document -> device mapping).
fn lower_stroke(e: &StrokeExport, paint: GpuPaint, geo: &Geo) -> GpuStroke {
    let (sx, sy) = (geo.scale_x, geo.scale_y);
    let t = e.transform;
    // device = (user - view.xy) * scale + offset, after the shape transform.
    let transform = [
        sx * t.a,
        sy * t.b,
        sx * t.c,
        sy * t.d,
        sx * t.e + geo.offset_x - geo.view.x * sx,
        sy * t.f + geo.offset_y - geo.view.y * sy,
    ];
    GpuStroke {
        paint,
        radius: e.radius,
        join_reach: e.join_reach,
        transform,
        segments: e
            .segments
            .iter()
            .map(|s| GpuStrokeSegment {
                a: s.a,
                b: s.b,
                start_cap: cap(s.start_cap),
                end_cap: cap(s.end_cap),
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

/// Weight of a gradient pixel against a solid one in the CPU work estimate.
pub(super) const GRADIENT_WEIGHT: f64 = 3.0;

/// Pixels of the canvas the shape's bounding box covers.
fn bbox_area(edges: &[GpuSegment], width: u32, height: u32) -> f64 {
    let (mut l, mut t, mut r, mut b) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for e in edges {
        l = l.min(e.x0).min(e.x1);
        r = r.max(e.x0).max(e.x1);
        t = t.min(e.y0).min(e.y1);
        b = b.max(e.y0).max(e.y1);
    }
    let w = (r.min(f64::from(width)) - l.max(0.0)).max(0.0);
    let h = (b.min(f64::from(height)) - t.max(0.0)).max(0.0);
    w * h
}

/// Device-space segments, every subpath closed explicitly.
fn segments(shape: &OwnedShape, geo: &Geo) -> Vec<GpuSegment> {
    let whole = [0..shape.edges.len()];
    let runs: &[std::ops::Range<usize>] = if shape.subpaths.is_empty() {
        &whole
    } else {
        &shape.subpaths
    };
    let mut out = Vec::with_capacity(shape.edges.len() + runs.len());
    for run in runs {
        let Some(edges) = shape.edges.get(run.clone()) else {
            continue;
        };
        let device = |e: &FlatEdge| {
            let (x0, y0) = geo.to_device(e.x0, e.y0);
            let (x1, y1) = geo.to_device(e.x1, e.y1);
            GpuSegment { x0, y0, x1, y1 }
        };
        let first_at = out.len();
        out.extend(edges.iter().map(device));
        if let (Some(first), Some(last)) = (out.get(first_at).copied(), out.last().copied())
            && (first.x0 != last.x1 || first.y0 != last.y1)
        {
            out.push(GpuSegment {
                x0: last.x1,
                y0: last.y1,
                x1: first.x0,
                y1: first.y0,
            });
        }
    }
    out
}

fn spread(s: Spread) -> GpuSpread {
    match s {
        Spread::Pad => GpuSpread::Pad,
        Spread::Reflect => GpuSpread::Reflect,
        Spread::Repeat => GpuSpread::Repeat,
    }
}

/// Stops the CPU sampler accepts: 2..=256, offsets in 0..=1, non-decreasing.
fn stops_ok(stops: &[PaintStop]) -> bool {
    (2..=MAX_GRADIENT_STOPS).contains(&stops.len())
        && stops.iter().all(|s| (0.0..=1.0).contains(&s.offset))
        && stops.windows(2).all(|w| w[0].offset <= w[1].offset)
}

/// One gradient channel as a device -> gradient-space affine (input.rs contract):
/// device -> user (inverse view map) -> `user_inverse` -> bounding-box
/// normalisation -> `gradient_inverse` -> paint-specific normalisation.
fn gradient(g: GradientRef<'_>, geo: &Geo) -> Option<GpuPaint> {
    let paint = g.paint;
    let stops = paint.stops.slice(g.stops)?;
    if !stops_ok(stops) {
        return None;
    }
    let (sx, sy) = (geo.scale_x, geo.scale_y);
    let view = affine(
        1.0 / sx,
        0.0,
        0.0,
        1.0 / sy,
        geo.view.x - geo.offset_x / sx,
        geo.view.y - geo.offset_y / sy,
    );
    let mut m = affine_compose(paint.user_inverse, view)?;
    if paint.object_bounding_box {
        let (w, h) = (
            paint.bounds[2] - paint.bounds[0],
            paint.bounds[3] - paint.bounds[1],
        );
        if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
            return None;
        }
        let bbox = affine(
            1.0 / w,
            0.0,
            0.0,
            1.0 / h,
            -paint.bounds[0] / w,
            -paint.bounds[1] / h,
        );
        m = affine_compose(bbox, m)?;
    }
    m = affine_compose(paint.gradient_inverse, m)?;
    let c = &paint.coordinates;
    let (norm, focal_ratio) = match paint.kind {
        PaintKind::Linear => {
            let (dx, dy) = (c[2] - c[0], c[3] - c[1]);
            let l = dx * dx + dy * dy;
            if !(l > 0.0 && l.is_finite()) {
                return None;
            }
            (
                affine(dx / l, 0.0, dy / l, 0.0, -(c[0] * dx + c[1] * dy) / l, 0.0),
                0.0,
            )
        }
        PaintKind::Radial => {
            if !(c[2] > 0.0 && c[2].is_finite()) {
                return None;
            }
            let (fx, fy) = ((c[3] - c[0]) / c[2], (c[4] - c[1]) / c[2]);
            let ratio = fx.hypot(fy);
            if !(ratio < 1.0) {
                return None;
            }
            let theta = fy.atan2(fx);
            let (s, co) = (theta.sin(), theta.cos());
            let (a, cc, b, d) = (co / c[2], s / c[2], -s / c[2], co / c[2]);
            (
                affine(a, b, cc, d, -(a * c[0] + cc * c[1]), -(b * c[0] + d * c[1])),
                ratio,
            )
        }
        _ => return None,
    };
    let m = affine_compose(norm, m)?;
    let gradient = GpuGradient {
        inverse: [m.a, m.b, m.c, m.d, m.e, m.f],
        spread: spread(paint.spread),
        stops: stops
            .iter()
            .map(|s| GpuColorStop {
                offset: s.offset,
                rgba: s.rgba,
            })
            .collect(),
        focal_ratio,
    };
    Some(if paint.kind == PaintKind::Linear {
        GpuPaint::Linear(gradient)
    } else {
        GpuPaint::Radial(gradient)
    })
}

#[cfg(target_os = "macos")]
pub(super) use metal::MetalBackend;

#[cfg(target_os = "macos")]
mod metal {
    use std::collections::HashMap;
    use std::time::Instant;
    use vexel_gpu_scene::GpuError;
    use vexel_gpu_scene::run::{ComputeBackend, ComputeRequest, ComputeStats};
    use vexel_platform_apple::gpu_compute::{
        BufferPool, GpuDevice, GpuError as MetalError, Pipeline,
    };

    /// `ComputeBackend` over the Metal runner. Lives on one worker thread:
    /// its buffers are not `Send`.
    pub(in crate::compose) struct MetalBackend {
        device: GpuDevice,
        pipelines: HashMap<String, Pipeline>,
        pool: BufferPool,
    }

    impl MetalBackend {
        pub(in crate::compose) fn new() -> Option<Self> {
            Some(Self {
                device: GpuDevice::new().ok()?,
                pipelines: HashMap::new(),
                pool: BufferPool::new(),
            })
        }
    }

    fn map(e: MetalError) -> GpuError {
        match e {
            MetalError::Timeout(_) => GpuError::Timeout,
            MetalError::Alloc(_) | MetalError::Invalid(_) => GpuError::BufferAlloc,
            MetalError::Compile(_) => GpuError::ShaderCompile,
            MetalError::Pipeline(_) => GpuError::PipelineCreate,
            _ => GpuError::CommandFailed,
        }
    }

    impl ComputeBackend for MetalBackend {
        fn run(
            &mut self,
            r: &ComputeRequest<'_>,
            out: &mut [u8],
        ) -> Result<ComputeStats, GpuError> {
            let start = Instant::now();
            let key = format!("{}#{}", r.entry, r.source.len());
            if !self.pipelines.contains_key(&key) {
                let p = self.device.compile(r.source, r.entry).map_err(map)?;
                self.pipelines.insert(key.clone(), p);
            }
            let mut buffers = Vec::with_capacity(r.inputs.len() + 1);
            let result = (|| {
                for input in r.inputs {
                    let buffer = self.pool.acquire(&self.device, input.len()).map_err(map)?;
                    buffer.write(0, input).map_err(map)?;
                    buffers.push(buffer);
                }
                buffers.push(self.pool.acquire(&self.device, out.len()).map_err(map)?);
                let refs: Vec<_> = buffers.iter().collect();
                let timing = self
                    .device
                    .dispatch(&self.pipelines[&key], &refs, r.grid, r.threads_per_group)
                    .map_err(map)?;
                if let Some(gpu) = timing.gpu {
                    crate::profile::record("gpu.kernel", gpu.as_secs_f64() * 1000.0);
                }
                crate::profile::record("gpu.run", start.elapsed().as_secs_f64() * 1000.0);
                let read = Instant::now();
                let output = buffers.last().ok_or(GpuError::BufferAlloc)?;
                output.read(0, out).map_err(map)?;
                crate::profile::record("gpu.readback", read.elapsed().as_secs_f64() * 1000.0);
                Ok(ComputeStats {
                    gpu: timing.gpu,
                    wall: timing.wall,
                })
            })();
            for buffer in buffers {
                self.pool.release(buffer);
            }
            if result.is_err() {
                // A failed launch may leave pooled state suspect: start clean.
                self.pool = BufferPool::new();
                self.pipelines.clear();
            }
            result
        }
    }
}
