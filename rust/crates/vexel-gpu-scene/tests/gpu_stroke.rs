//! Metal stroke kernel against the pure-Rust reference (byte exact) and the
//! real CPU renderer (ADR-0035 bounds). Skips cleanly without a Metal device;
//! `VEXEL_REQUIRE_GPU=1` makes a missing device a failure.
#![allow(dead_code)]

use std::collections::HashMap;

use vexel_gpu_scene::policy::{
    PARITY_MEAN_ABS_ERROR_MAX, PARITY_OVER_2_MAX_FRACTION, PARITY_OVER_16_MAX_FRACTION,
};
use vexel_gpu_scene::run::{
    ComputeBackend, ComputeRequest, ComputeStats, GpuRasterizer, Output, RenderError,
};
use vexel_gpu_scene::{GpuError, encode, reference};
use vexel_platform_apple::gpu_compute::{Buffer, GpuDevice, GpuError as MetalError, Pipeline};

include!("common/strokes.rs");

struct Metal {
    device: GpuDevice,
    pipelines: HashMap<String, Pipeline>,
}

impl ComputeBackend for Metal {
    fn run(&mut self, r: &ComputeRequest<'_>, out: &mut [u8]) -> Result<ComputeStats, GpuError> {
        let key = format!("{}#{}", r.entry, r.source.len());
        if !self.pipelines.contains_key(&key) {
            let p = self.device.compile(r.source, r.entry).map_err(|e| {
                eprintln!("{e}");
                GpuError::ShaderCompile
            })?;
            self.pipelines.insert(key.clone(), p);
        }
        let map = |e: MetalError| match e {
            MetalError::Timeout(_) => GpuError::Timeout,
            MetalError::Alloc(_) | MetalError::Invalid(_) => GpuError::BufferAlloc,
            _ => GpuError::CommandFailed,
        };
        let mut buffers = Vec::new();
        for input in r.inputs {
            buffers.push(Buffer::from_slice(&self.device, input).map_err(map)?);
        }
        let output = Buffer::new(&self.device, out.len()).map_err(map)?;
        let mut refs: Vec<&Buffer> = buffers.iter().collect();
        refs.push(&output);
        let t = self
            .device
            .dispatch(&self.pipelines[&key], &refs, r.grid, r.threads_per_group)
            .map_err(map)?;
        output.read(0, out).map_err(map)?;
        Ok(ComputeStats { gpu: t.gpu, wall: t.wall })
    }
}

fn gpu() -> Option<GpuRasterizer<Metal>> {
    match GpuDevice::new() {
        Ok(device) => Some(GpuRasterizer::new(Metal { device, pipelines: HashMap::new() })),
        Err(e) => {
            assert!(std::env::var_os("VEXEL_REQUIRE_GPU").is_none_or(|v| v != "1"), "{e}");
            eprintln!("skipping: {e}");
            None
        }
    }
}

fn unwrap_render(r: Result<Vec<u8>, RenderError>) -> Vec<u8> {
    r.unwrap_or_else(|e| panic!("render failed: {e:?}"))
}

fn fractions(a: &[u8], b: &[u8]) -> (f64, f64, f64) {
    let pixels = a.len() / 4;
    let (mut o2, mut o16, mut sum) = (0usize, 0usize, 0u64);
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let worst = (0..4).map(|c| pa[c].abs_diff(pb[c])).max().unwrap();
        o2 += usize::from(worst > 2);
        o16 += usize::from(worst > 16);
        sum += (0..4).map(|c| u64::from(pa[c].abs_diff(pb[c]))).sum::<u64>();
    }
    (
        o2 as f64 / pixels as f64,
        o16 as f64 / pixels as f64,
        sum as f64 / (pixels * 4) as f64 / 255.0,
    )
}

#[test]
fn gpu_strokes_match_reference_byte_exact() {
    let Some(mut g) = gpu() else { return };
    for (k, size) in [(0u64, 256u32), (1, 333), (2, 480), (3, 512), (4, 900)] {
        let (min_w, max_w) = [(0.5, 3.0), (3.0, 14.0), (14.0, 40.0), (0.5, 40.0), (0.5, 40.0)][k as usize];
        let cases = stroke_cases(0x700 + k, 50, f64::from(size), f64::from(size), min_w, max_w);
        let (scene, _) = stroke_scene(&cases, size, size);
        let enc = encode(&scene).unwrap();
        for (out, expected) in [
            (Output::Straight, reference::render_straight(&enc)),
            (Output::Premultiplied, reference::render(&enc)),
        ] {
            let got = unwrap_render(g.render(&scene, out));
            let diffs = got.iter().zip(&expected).filter(|(a, b)| a != b).count();
            println!("stroke ref {out:?} seed {k} {size}px prims={}: differing bytes {diffs}", enc.prims.len());
            assert_eq!(diffs, 0, "stroke kernel must equal the reference byte for byte");
        }
    }
}

#[test]
fn gpu_strokes_match_cpu_within_adr_bounds() {
    let Some(mut g) = gpu() else { return };
    for (k, size) in [(0u64, 480u32), (1, 900), (2, 900), (3, 1024)] {
        let cases = stroke_cases(0x800 + k, 80, f64::from(size), f64::from(size), 0.5, 40.0);
        let (scene, cpu) = stroke_scene(&cases, size, size);
        let got = unwrap_render(g.render(&scene, Output::Straight));
        let (o2, o16, mae) = fractions(&got, &cpu);
        println!(
            "stroke cpu seed {k} {size}px: >2/255={:.5}% >16/255={:.5}% mae={mae:.7}",
            o2 * 100.0,
            o16 * 100.0
        );
        assert!(o2 <= PARITY_OVER_2_MAX_FRACTION && o16 <= PARITY_OVER_16_MAX_FRACTION);
        assert!(mae <= PARITY_MEAN_ABS_ERROR_MAX);
    }
}

#[test]
fn gpu_stroke_frame_time_report() {
    let Some(mut g) = gpu() else { return };
    let size = 900u32;
    for n in [50usize, 500] {
        let cases = stroke_cases(0x900, n, f64::from(size), f64::from(size), 0.5, 20.0);
        let (scene, _) = stroke_scene(&cases, size, size);
        let _ = unwrap_render(g.render(&scene, Output::Premultiplied));
        let mut times = Vec::new();
        for _ in 0..7 {
            let (_, t) = g.render_timed(&scene, Output::Premultiplied).unwrap();
            times.push((t.total.as_secs_f64() * 1e3, t.encode.as_secs_f64() * 1e3));
        }
        times.sort_by(|a, b| a.0.total_cmp(&b.0));
        println!("stroke frame {n} shapes {size}px: p50 total {:.2} ms (encode {:.2} ms)", times[3].0, times[3].1);
    }
}
