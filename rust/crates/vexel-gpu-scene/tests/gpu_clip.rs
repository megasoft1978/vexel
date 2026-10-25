//! Metal clip kernel against the pure-Rust reference (byte exact) and the
//! real CPU renderer (ADR-0035 bounds), on the seeded clip scenes. Skips cleanly without a Metal device;
//! `VEXEL_REQUIRE_GPU=1` makes a missing device a failure.
#![allow(dead_code)]

use std::collections::HashMap;

use vexel_gpu_scene::run::{
    ComputeBackend, ComputeRequest, ComputeStats, GpuRasterizer, Output, RenderError,
};
use vexel_gpu_scene::{GpuError, encode, reference};
use vexel_platform_apple::gpu_compute::{Buffer, GpuDevice, GpuError as MetalError, Pipeline};

include!("common/strokes.rs");
include!("common/clips.rs");

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
        Ok(ComputeStats {
            gpu: t.gpu,
            wall: t.wall,
        })
    }
}

fn gpu() -> Option<GpuRasterizer<Metal>> {
    match GpuDevice::new() {
        Ok(device) => Some(GpuRasterizer::new(Metal {
            device,
            pipelines: HashMap::new(),
        })),
        Err(e) => {
            assert!(
                std::env::var_os("VEXEL_REQUIRE_GPU").is_none_or(|v| v != "1"),
                "{e}"
            );
            eprintln!("skipping: {e}");
            None
        }
    }
}

fn unwrap_render(r: Result<Vec<u8>, RenderError>) -> Vec<u8> {
    r.unwrap_or_else(|e| panic!("render failed: {e:?}"))
}

/// Reads `"<kind>": {"over_2": x, "over_16": y, "mae": z}` from parity.json.
fn frozen(kind: &str) -> (f64, f64, f64) {
    let text = include_str!("parity.json");
    let at = text
        .find(&format!("\"{kind}\""))
        .expect("kind in parity.json");
    let rest = &text[at..];
    let num = |key: &str| -> f64 {
        let i = rest.find(&format!("\"{key}\"")).unwrap() + key.len() + 2;
        let tail = rest[i..].trim_start_matches([':', ' ']);
        let end = tail.find([',', '}', '\n']).unwrap();
        tail[..end].trim().parse().unwrap()
    };
    (num("over_2"), num("over_16"), num("mae"))
}

fn fractions(a: &[u8], b: &[u8]) -> (f64, f64, f64) {
    let pixels = a.len() / 4;
    let (mut o2, mut o16, mut sum) = (0usize, 0usize, 0u64);
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let worst = (0..4).map(|c| pa[c].abs_diff(pb[c])).max().unwrap();
        o2 += usize::from(worst > 2);
        o16 += usize::from(worst > 16);
        sum += (0..4)
            .map(|c| u64::from(pa[c].abs_diff(pb[c])))
            .sum::<u64>();
    }
    (
        o2 as f64 / pixels as f64,
        o16 as f64 / pixels as f64,
        sum as f64 / (pixels * 4) as f64 / 255.0,
    )
}

#[test]
fn gpu_clips_match_reference_byte_exact_and_cpu_within_adr() {
    let Some(mut g) = gpu() else { return };
    for (name, seed, count, features) in CLIP_CONFIGS {
        for (k, (w, h)) in [(333u32, 301u32), (640, 480)].into_iter().enumerate() {
            let scene = clip_scene(
                seed + 32 * k as u64,
                count,
                f64::from(w),
                f64::from(h),
                features,
            );
            let gs = clip_gpu_scene(&scene, w, h);
            let enc = encode(&gs).unwrap();
            for (out, expected) in [
                (Output::Straight, reference::render_straight(&enc)),
                (Output::Premultiplied, reference::render(&enc)),
            ] {
                let got = unwrap_render(g.render(&gs, out));
                let diffs = got.iter().zip(&expected).filter(|(a, b)| a != b).count();
                println!(
                    "clip ref {name} {w}x{h} {out:?} records={}: differing bytes {diffs}",
                    enc.clips.len()
                );
                assert_eq!(diffs, 0, "{name}: clip kernel must equal the reference");
            }
            let cpu = clip_cpu_render(&scene, w, h);
            let got = unwrap_render(g.render(&gs, Output::Straight));
            let (o2, o16, mae) = fractions(&got, &cpu);
            println!(
                "clip cpu {name} {w}x{h}: >2/255={:.5}% >16/255={:.5}% mae={mae:.7}",
                o2 * 100.0,
                o16 * 100.0
            );
            let (b2, b16, bmae) = frozen("clips");
            assert!(
                o2 <= b2 && o16 <= b16 && mae <= bmae,
                "{name}: outside frozen bounds"
            );
        }
    }
}

/// Edge visits of the clip chains: every clip edge of the chain, once per
/// tile entry (what the kernel does per threadgroup).
fn clip_visits(enc: &vexel_gpu_scene::Encoded) -> u64 {
    let chain_edges = |s: &vexel_gpu_scene::layout::GpuShape| -> u64 {
        let mut at = s.clip_start as usize;
        let mut total = 0u64;
        for _ in 0..s.clip_count {
            let size = enc.clips[at].union_size as usize;
            total += enc.clips[at..at + size]
                .iter()
                .map(|c| u64::from(c.edge_count))
                .sum::<u64>();
            at += size;
        }
        total
    };
    enc.tile_list
        .iter()
        .map(|t| chain_edges(&enc.shapes[t.shape as usize]))
        .sum()
}

/// Clipped frame speed against the CPU on synthetic clip scenes (N clipped
/// shapes at 900 px, release numbers matter). Prints only.
#[test]
fn gpu_clip_frame_time_report() {
    let Some(mut g) = gpu() else { return };
    let size = 900u32;
    let profiles = [
        ("simple", Features::default()),
        (
            "transforms+chains",
            Features {
                transforms: true,
                chains: true,
                obb: true,
                groups: false,
                strokes: false,
            },
        ),
    ];
    for (name, f) in profiles {
        for n in [500usize, 2000, 5000] {
            let scene = clip_scene(0xB0 + n as u64, n, f64::from(size), f64::from(size), f);
            let gs = clip_gpu_scene(&scene, size, size);
            let mut cpu = (0..5)
                .map(|_| clip_cpu_render_timed(&scene, size, size).1)
                .collect::<Vec<_>>();
            cpu.sort_by(f64::total_cmp);
            if let Err(e) = g.render(&gs, Output::Premultiplied) {
                println!(
                    "clip {name} {n} shapes {size}px: GPU route declines ({e:?}); cpu p50 {:.1} ms",
                    cpu[2]
                );
                continue;
            }
            let enc = g.encoded().clone();
            let mut times = Vec::new();
            for _ in 0..7 {
                let (_, t) = g.render_timed(&gs, Output::Premultiplied).unwrap();
                times.push((
                    t.total.as_secs_f64() * 1e3,
                    t.encode.as_secs_f64() * 1e3,
                    t.backend.gpu.map_or(f64::NAN, |d| d.as_secs_f64() * 1e3),
                ));
            }
            times.sort_by(|a, b| a.0.total_cmp(&b.0));
            let visits = clip_visits(&enc);
            println!(
                "clip {name} {n} shapes {size}px: gpu p50 total {:.2} ms (encode {:.2}, kernel {:.2}); cpu p50 {:.2} ms; entries={} clip visits={visits}",
                times[3].0,
                times[3].1,
                times[3].2,
                cpu[2],
                enc.tile_list.len()
            );
        }
    }
}
