use std::time::Instant;

use super::*;

/// Returns a device, or `None` (skip) when no GPU exists. With
/// `VEXEL_REQUIRE_GPU=1` a missing GPU fails the test instead.
fn gpu() -> Option<GpuDevice> {
    match GpuDevice::new() {
        Ok(d) => Some(d),
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

const FILL: &str = r#"
#include <metal_stdlib>
using namespace metal;
kernel void fill(device uint* out [[buffer(0)]], uint i [[thread_position_in_grid]]) {
    out[i] = 0xDEADBEEFu;
}
kernel void add_one(device uint* out [[buffer(0)]], uint i [[thread_position_in_grid]]) {
    out[i] = out[i] + 1u;
}
kernel void times_two(device uint* out [[buffer(0)]], uint i [[thread_position_in_grid]]) {
    out[i] = out[i] * 2u;
}
kernel void empty_kernel() {}
"#;

#[test]
fn constant_fill() {
    let Some(gpu) = gpu() else { return };
    let p = gpu.compile(FILL, "fill").unwrap();
    let out = Buffer::new(&gpu, 1024 * 4).unwrap();
    let t = gpu.dispatch(&p, &[&out], (1024, 1, 1), (64, 1, 1)).unwrap();
    let mut v = vec![0u32; 1024];
    out.read_typed(0, &mut v).unwrap();
    assert!(v.iter().all(|&x| x == 0xDEAD_BEEF));
    assert!(t.wall > std::time::Duration::ZERO);
}

#[test]
fn buffer_round_trip_and_bounds() {
    let Some(gpu) = gpu() else { return };
    let data: Vec<u8> = (0..=255).collect();
    let b = Buffer::from_slice(&gpu, &data).unwrap();
    let mut back = vec![0u8; 256];
    b.read(0, &mut back).unwrap();
    assert_eq!(back, data);
    b.write(10, &[9, 9]).unwrap();
    let mut two = [0u8; 2];
    b.read(10, &mut two).unwrap();
    assert_eq!(two, [9, 9]);
    assert!(matches!(b.write(255, &[0, 0]), Err(GpuError::Invalid(_))));
    assert!(matches!(
        b.read(usize::MAX, &mut two),
        Err(GpuError::Invalid(_))
    ));
    assert!(matches!(Buffer::new(&gpu, 0), Err(GpuError::Invalid(_))));
    assert!(matches!(
        Buffer::new(&gpu, MAX_BUFFER_BYTES + 1),
        Err(GpuError::Invalid(_))
    ));
    let f = Buffer::from_typed(&gpu, &[1.5f32, -2.0]).unwrap();
    let mut fv = [0f32; 2];
    f.read_typed(0, &mut fv).unwrap();
    assert_eq!(fv, [1.5, -2.0]);
}

#[test]
fn invalid_msl_reports_log() {
    let Some(gpu) = gpu() else { return };
    match gpu.compile("kernel void k( { nonsense", "k") {
        Err(GpuError::Compile(m)) => assert!(m.contains("error"), "{m}"),
        other => panic!("expected Compile error, got {:?}", other.map(|_| ())),
    }
    assert!(matches!(
        gpu.compile(FILL, "missing"),
        Err(GpuError::Pipeline(_))
    ));
}

#[test]
fn sequence_runs_in_order() {
    let Some(gpu) = gpu() else { return };
    let add = gpu.compile(FILL, "add_one").unwrap();
    let dbl = gpu.compile(FILL, "times_two").unwrap();
    let b = Buffer::from_typed(&gpu, &[3u32; 256]).unwrap();
    let bound = [&b];
    let step = |p| Dispatch {
        pipeline: p,
        buffers: &bound,
        grid: (256, 1, 1),
        threads_per_group: (32, 1, 1),
    };
    // (3 + 1) * 2 = 8, whereas (3 * 2) + 1 = 7.
    gpu.dispatch_sequence(&[step(&add), step(&dbl)]).unwrap();
    let mut v = [0u32; 256];
    b.read_typed(0, &mut v).unwrap();
    assert!(v.iter().all(|&x| x == 8));
    // Separate command buffers keep ordering too: (8 * 2) + 1 = 17.
    gpu.dispatch(&dbl, &[&b], (256, 1, 1), (32, 1, 1)).unwrap();
    gpu.dispatch(&add, &[&b], (256, 1, 1), (32, 1, 1)).unwrap();
    b.read_typed(0, &mut v).unwrap();
    assert!(v.iter().all(|&x| x == 17));
}

#[test]
fn dispatch_validation() {
    let Some(gpu) = gpu() else { return };
    let p = gpu.compile(FILL, "fill").unwrap();
    let b = Buffer::new(&gpu, 64).unwrap();
    assert!(matches!(
        gpu.dispatch(&p, &[&b], (0, 1, 1), (1, 1, 1)),
        Err(GpuError::Invalid(_))
    ));
    assert!(matches!(
        gpu.dispatch(&p, &[&b], (16, 1, 1), (1 << 20, 1, 1)),
        Err(GpuError::Invalid(_))
    ));
    assert!(matches!(
        gpu.dispatch_sequence(&[]),
        Err(GpuError::Invalid(_))
    ));
}

#[test]
fn pool_reuses_buffers() {
    let Some(gpu) = gpu() else { return };
    let mut pool = BufferPool::new();
    let a = pool.acquire(&gpu, 1000).unwrap();
    assert_eq!(a.len(), 1024);
    a.write(0, &[7]).unwrap();
    pool.release(a);
    assert_eq!(pool.cached_bytes(), 1024);
    let b = pool.acquire(&gpu, 600).unwrap();
    assert_eq!(pool.cached_bytes(), 0);
    let mut one = [1u8];
    b.read(0, &mut one).unwrap();
    assert_eq!(one, [0], "reused buffer must be zeroed");
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

/// `cargo test -p vexel-platform-apple --release gpu_compute::tests::benchmark -- --ignored --nocapture`
#[test]
#[ignore]
fn benchmark() {
    let Some(gpu) = gpu() else { return };
    println!("device: {}", gpu.name());
    let p = gpu.compile(FILL, "empty_kernel").unwrap();
    for _ in 0..50 {
        gpu.dispatch(&p, &[], (1, 1, 1), (1, 1, 1)).unwrap();
    }
    let mut us: Vec<f64> = (0..1000)
        .map(|_| {
            let t = Instant::now();
            gpu.dispatch(&p, &[], (1, 1, 1), (1, 1, 1)).unwrap();
            t.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    us.sort_by(f64::total_cmp);
    println!(
        "empty dispatch latency: p50 {:.1} us, p95 {:.1} us",
        pct(&us, 0.5),
        pct(&us, 0.95)
    );
    for mb in [4usize, 16] {
        let n = mb << 20;
        let src = vec![0xA5u8; n];
        let mut dst = vec![0u8; n];
        let b = Buffer::new(&gpu, n).unwrap();
        let t = Instant::now();
        for _ in 0..20 {
            b.write(0, std::hint::black_box(&src)).unwrap();
        }
        let up = t.elapsed().as_secs_f64() / 20.0;
        let t = Instant::now();
        for _ in 0..20 {
            b.read(0, &mut dst).unwrap();
            std::hint::black_box(&dst);
        }
        let down = t.elapsed().as_secs_f64() / 20.0;
        println!(
            "{mb} MB: upload {:.1} GB/s ({:.2} ms), readback {:.1} GB/s ({:.2} ms)",
            n as f64 / up / 1e9,
            up * 1e3,
            n as f64 / down / 1e9,
            down * 1e3
        );
    }
}
