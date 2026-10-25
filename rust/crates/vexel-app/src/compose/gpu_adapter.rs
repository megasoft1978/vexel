//! Adapters for the GPU display route (ADR-0035): the prepared render shapes
//! lowered to `vexel-gpu-scene` input structs, and the Metal compute backend.
//!
//! Only solid and gradient fills of unclipped, unstroked shapes lower; anything
//! else returns `None` and the frame renders on the CPU. Pure value
//! conversion, no GPU calls, so the lowering is testable without a device.

use super::layers::Geo;
use super::shapes::{Entry, OwnedShape};
use vexel_geometry::{Affine, affine_compose};
use vexel_gpu_scene::input::{
    GpuColorStop, GpuGradient, GpuPaint, GpuScene, GpuSegment, GpuShape, GpuSpread,
};
use vexel_gpu_scene::{GpuNode, MAX_GRADIENT_STOPS};
use vexel_render::{FlatEdge, GradientRef};
use vexel_scene::{PaintKind, PaintStop, Spread};

fn affine(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Affine {
    Affine { a, b, c, d, e, f }
}

/// Lowers prepared entries (document space) to a device-space GPU scene.
pub(super) fn lower(entries: &[Entry<'_>], geo: &Geo, width: u32, height: u32) -> Option<GpuScene> {
    let mut scene = GpuScene::new(width, height);
    let (cw, ch) = (geo.view.width * geo.scale_x, geo.view.height * geo.scale_y);
    scene.content_rect = [
        geo.offset_x.max(0.0),
        geo.offset_y.max(0.0),
        (geo.offset_x + cw).min(f64::from(width)),
        (geo.offset_y + ch).min(f64::from(height)),
    ];
    for entry in entries {
        let Entry::Draw(p) = entry else {
            return None;
        };
        if p.image.is_some() || p.fill.is_some() || p.stroke.is_some() || p.clip.is_some() {
            return None;
        }
        let s = &p.shape;
        if s.fill_pattern
            || s.stroke_pattern
            || s.stroke_gradient.is_some()
            || s.stroke.is_some_and(|st| st.rgba[3] != 0 && st.width > 0.0) {
            return None;
        }
        if !s.filled || s.edges.is_empty() {
            continue;
        }
        let paint = match &s.fill_gradient {
            Some(_) => gradient(s.as_render().fill_gradient?, geo)?,
            None if s.fill_rgba[3] == 0 => continue,
            None => GpuPaint::Solid(s.fill_rgba),
        };
        scene.nodes.push(GpuNode::Shape(GpuShape {
            edges: segments(s, geo),
            evenodd: s.evenodd,
            fill: Some(paint),
            stroke: None,
            clips: Vec::new(),
        }));
    }
    Some(scene)
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
