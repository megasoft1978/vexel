//! GPU kernel against the pure-Rust reference and the real CPU renderer.
//! Skips cleanly without a Metal device; `VEXEL_REQUIRE_GPU=1` makes a missing
//! device a failure. Thresholds for the CPU comparison are frozen in
//! `tests/parity.json` (they may only tighten).
#![allow(dead_code)]

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use vexel_gpu_scene::policy::{
    PARITY_MEAN_ABS_ERROR_MAX, PARITY_OVER_2_MAX_FRACTION, PARITY_OVER_16_MAX_FRACTION,
};
use vexel_gpu_scene::run::{
    ComputeBackend, ComputeRequest, ComputeStats, GpuRasterizer, Output, RenderError,
};
use vexel_gpu_scene::{GpuError, UnsupportedReason, encode, reference};
use vexel_platform_apple::gpu_compute::{Buffer, GpuDevice, GpuError as MetalError, Pipeline};

include!("common/gen.rs");

/// (fraction over 2/255, fraction over 16/255, mean absolute error).
type Bounds = (f64, f64, f64);

/// Test adapter from the Metal runner to the crate's backend trait.
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

/// Backend that must never be reached (budget tests).
struct Never;
impl ComputeBackend for Never {
    fn run(&mut self, _: &ComputeRequest<'_>, _: &mut [u8]) -> Result<ComputeStats, GpuError> {
        panic!("backend invoked for an unsupported scene");
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Fill,
    Linear,
    Radial,
    Mixed,
}

fn kind_specs(kind: Kind, seed: u64, w: u32, h: u32) -> Vec<Spec> {
    let count = (w as usize * h as usize) / 700;
    let mut s = specs(seed, count * 3, f64::from(w), f64::from(h));
    match kind {
        Kind::Fill => s.retain(|s| matches!(s.paint, SpecPaint::Solid(_))),
        Kind::Linear => s.retain(|s| matches!(s.paint, SpecPaint::Linear { .. })),
        Kind::Radial => s.retain(|s| matches!(s.paint, SpecPaint::Radial { .. })),
        Kind::Mixed => s.truncate(count),
    }
    s
}

fn unwrap_render(r: Result<Vec<u8>, RenderError>) -> Vec<u8> {
    r.unwrap_or_else(|e| panic!("render failed: {e:?}"))
}

/// Kernel output equals the pure-Rust reference byte for byte for solid
/// fills; gradients may differ by float evaluation, bounded by the ADR.
#[test]
fn gpu_matches_reference() {
    let Some(mut g) = gpu() else { return };
    for kind in [Kind::Fill, Kind::Linear, Kind::Radial, Kind::Mixed] {
        for k in 0..6u64 {
            let size = [256u32, 333, 512][(k % 3) as usize];
            let (w, h) = (size, size * 3 / 4);
            let s = kind_specs(kind, 200 + k, w, h);
            let scene = scene_for(&s, w, h);
            let enc = encode(&scene).unwrap();
            for (out, expected) in [
                (Output::Straight, reference::render_straight(&enc)),
                (Output::Premultiplied, reference::render(&enc)),
            ] {
                let got = unwrap_render(g.render(&scene, out));
                let m = compare(&got, &expected);
                let diffs = got.iter().zip(&expected).filter(|(a, b)| a != b).count();
                println!(
                    "ref {kind:?} {out:?} seed {} {w}x{h}: differing bytes {diffs} max {} >2={:.5}% mae={:.7}",
                    200 + k,
                    m.max_diff,
                    m.over_2 * 100.0,
                    m.mae
                );
                if kind == Kind::Fill {
                    assert_eq!(diffs, 0, "solid fills must be byte exact");
                }
                assert!(m.over_2 <= PARITY_OVER_2_MAX_FRACTION, "{m:?}");
                assert!(m.over_16 <= PARITY_OVER_16_MAX_FRACTION, "{m:?}");
                assert!(m.mae <= PARITY_MEAN_ABS_ERROR_MAX, "{m:?}");
            }
        }
    }
}

/// Reads `"<kind>": {"over_2": x, "over_16": y, "mae": z}` from parity.json.
fn frozen(kind: &str) -> Bounds {
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

#[test]
fn gpu_matches_cpu_within_frozen_bounds() {
    let Some(mut g) = gpu() else { return };
    let mut failures: Vec<String> = Vec::new();
    for (kind, name) in [
        (Kind::Fill, "fill"),
        (Kind::Linear, "linear"),
        (Kind::Radial, "radial"),
    ] {
        let bounds = frozen(name);
        let mut worst = (0.0f64, 0.0f64, 0.0f64);
        for k in 0..12u64 {
            let size = [256u32, 900, 1024][(k % 3) as usize];
            let (w, h) = (size, size * 3 / 4);
            let s = kind_specs(kind, 300 + k, w, h);
            let scene = scene_for(&s, w, h);
            let cpu = cpu_render(&s, w, h);
            let got = unwrap_render(g.render(&scene, Output::Straight));
            let m = compare(&got, &cpu);
            println!(
                "cpu {name} seed {} {w}x{h} shapes={}: >2/255={:.5}% >16/255={:.5}% mae={:.7} max={}",
                300 + k,
                s.len(),
                m.over_2 * 100.0,
                m.over_16 * 100.0,
                m.mae,
                m.max_diff
            );
            let over = m.over_2 > bounds.0 || m.over_16 > bounds.1 || m.mae > bounds.2;
            if over {
                // Documented exceptions only (tests/parity.json), each capped.
                let id = format!("\"{name}:{}\"", 300 + k);
                let cap = frozen("exception_cap");
                if !include_str!("parity.json").contains(&id)
                    || m.over_2 > bounds.0
                    || m.over_16 > cap.1
                    || m.mae > bounds.2
                {
                    failures.push(format!("{name} seed {}: {m:?}", 300 + k));
                }
                println!("exception {name} seed {}", 300 + k);
            }
            worst = (
                worst.0.max(m.over_2),
                worst.1.max(m.over_16),
                worst.2.max(m.mae),
            );
        }
        println!("worst {name}: {worst:?} frozen {bounds:?}");
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

const ADR: Bounds = (
    PARITY_OVER_2_MAX_FRACTION,
    PARITY_OVER_16_MAX_FRACTION,
    PARITY_MEAN_ABS_ERROR_MAX,
);

#[test]
fn frozen_thresholds_not_looser_than_adr() {
    for name in ["fill", "linear", "radial"] {
        let b = frozen(name);
        assert!(
            b.0 <= ADR.0 && b.1 <= ADR.1 && b.2 <= ADR.2,
            "{name}: {b:?}"
        );
    }
}

#[test]
fn deterministic_over_ten_runs() {
    let Some(mut g) = gpu() else { return };
    let s = kind_specs(Kind::Mixed, 77, 512, 384);
    let scene = scene_for(&s, 512, 384);
    let mut hashes = Vec::new();
    for _ in 0..10 {
        let px = unwrap_render(g.render(&scene, Output::Premultiplied));
        let mut h = std::collections::hash_map::DefaultHasher::new();
        px.hash(&mut h);
        hashes.push(h.finish());
    }
    assert!(hashes.windows(2).all(|p| p[0] == p[1]), "{hashes:?}");
}

#[test]
fn oversize_scenes_are_unsupported_without_touching_the_gpu() {
    let mut g = GpuRasterizer::new(Never);
    let huge = GpuScene::new(1_000_000, 1_000_000);
    assert!(matches!(
        g.render(&huge, Output::Straight),
        Err(RenderError::Unsupported(UnsupportedReason::CanvasTooLarge))
    ));
    let mut many = GpuScene::new(64, 64);
    for i in 0..(vexel_gpu_scene::layout::MAX_SHAPES as usize + 1) {
        let o = (i % 50) as f64;
        many.push_fill(
            closed(&[(o, o), (o + 3.0, o), (o + 3.0, o + 3.0), (o, o + 3.0)]),
            false,
            GpuPaint::Solid([1, 2, 3, 255]),
        );
    }
    assert!(matches!(
        g.render(&many, Output::Straight),
        Err(RenderError::Unsupported(_))
    ));
}

fn p50(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// `cargo test --release -p vexel-gpu-scene --test gpu_parity -- --ignored
/// --nocapture bench`; prints a markdown table (also appended to the file
/// named by `VEXEL_BENCH_OUT`).
#[test]
#[ignore]
fn bench_gpu_vs_cpu() {
    let Some(mut g) = gpu() else { return };
    let mut table = String::from(
        "| profile | px | shapes | CPU p50 ms | GPU end-to-end p50 ms | GPU kernel p50 ms | speedup (e2e) | speedup (kernel) |\n|---|---|---|---|---|---|---|---|\n",
    );
    let only = std::env::var("VEXEL_BENCH_ONLY").ok();
    for kind in [Kind::Fill, Kind::Linear, Kind::Radial] {
        if only.as_ref().is_some_and(|o| *o != format!("{kind:?}")) {
            continue;
        }
        for size in [900u32, 2048] {
            for n in [500usize, 2000, 5000, 20000] {
                let mut s = specs(9, n * 6, f64::from(size), f64::from(size));
                s.retain(|s| match kind {
                    Kind::Fill => matches!(s.paint, SpecPaint::Solid(_)),
                    Kind::Linear => matches!(s.paint, SpecPaint::Linear { .. }),
                    _ => matches!(s.paint, SpecPaint::Radial { .. }),
                });
                s.truncate(n);
                let scene = scene_for(&s, size, size);
                let runs = if n >= 20000 || size > 1024 { 3 } else { 5 };
                let mut cpu = Vec::new();
                for _ in 0..runs {
                    let t = std::time::Instant::now();
                    let _ = cpu_render(&s, size, size);
                    cpu.push(t.elapsed().as_secs_f64() * 1e3);
                }
                let (mut e2e, mut kern) = (Vec::new(), Vec::new());
                for _ in 0..runs + 2 {
                    let t = match g.render_timed(&scene, Output::Premultiplied) {
                        Ok((_, t)) => t,
                        Err(e) => {
                            let row = format!(
                                "| {kind:?} | {size} | {} | {:.1} | CPU fallback ({e:?}) | | | |\n",
                                s.len(),
                                p50(&mut cpu)
                            );
                            print!("{row}");
                            table.push_str(&row);
                            break;
                        }
                    };
                    e2e.push(t.total.as_secs_f64() * 1e3);
                    kern.push(t.backend.gpu.unwrap_or(Duration::ZERO).as_secs_f64() * 1e3);
                }
                if e2e.is_empty() {
                    continue;
                }
                let (c, e, k) = (p50(&mut cpu), p50(&mut e2e), p50(&mut kern));
                let row = format!(
                    "| {kind:?} | {size} | {} | {c:.1} | {e:.1} | {k:.2} | {:.1}x | {:.0}x |\n",
                    s.len(),
                    c / e,
                    c / k
                );
                print!("{row}");
                table.push_str(&row);
            }
        }
    }
    println!("{table}");
    if let Some(path) = std::env::var_os("VEXEL_BENCH_OUT") {
        std::fs::write(path, table).unwrap();
    }
}
