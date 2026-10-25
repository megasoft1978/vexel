//! GPU display route: structure, fallback, diff-cache bypass and CPU parity.
//! Device-dependent checks skip cleanly without Metal; `VEXEL_REQUIRE_GPU=1`
//! turns a missing device into a failure.

use super::gpu_route::{Decision, Session, render_checked};
use super::*;
use std::sync::Mutex;
use vexel_gpu_scene::policy::{
    LATCH_ERROR_LIMIT, PARITY_MEAN_ABS_ERROR_MAX, PARITY_OVER_2_MAX_FRACTION,
    PARITY_OVER_16_MAX_FRACTION,
};
use vexel_gpu_scene::run::{ComputeBackend, ComputeRequest, ComputeStats, GpuRasterizer};
use vexel_gpu_scene::{GpuError, GpuPaint, GpuScene, GpuSegment};

/// The route switches are process globals: tests that flip them take turns.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as f64 / f64::from(1u32 << 31)
    }
}

/// Seeded scene: filled paths with solid, linear and radial paints.
fn scene_svg(seed: u64, count: usize, w: u32, h: u32, kinds: &str) -> Vec<u8> {
    scene_svg_scaled(seed, count, w, h, kinds, 1.0)
}

fn scene_svg_scaled(seed: u64, count: usize, w: u32, h: u32, kinds: &str, scale: f64) -> Vec<u8> {
    let mut rng = Lcg(seed);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><defs>"
    );
    for g in 0..8 {
        let units = if g % 2 == 0 {
            ""
        } else {
            " gradientUnits=\"userSpaceOnUse\""
        };
        let spread = ["pad", "reflect", "repeat", "pad"][g % 4];
        let (x1, y1) = (
            rng.next() * f64::from(w) * 0.5,
            rng.next() * f64::from(h) * 0.5,
        );
        let stops = "<stop offset=\"0\" stop-color=\"#e63946\"/><stop offset=\"0.5\" stop-color=\"#f1fa8c\" stop-opacity=\"0.8\"/><stop offset=\"1\" stop-color=\"#1d3557\"/>";
        if units.is_empty() {
            svg += &format!(
                "<linearGradient id=\"l{g}\" x1=\"0.1\" y1=\"0\" x2=\"0.9\" y2=\"1\" spreadMethod=\"{spread}\">{stops}</linearGradient><radialGradient id=\"r{g}\" cx=\"0.5\" cy=\"0.5\" r=\"0.6\" fx=\"0.4\" fy=\"0.45\" spreadMethod=\"{spread}\">{stops}</radialGradient>"
            );
        } else {
            svg += &format!(
                "<linearGradient id=\"l{g}\"{units} x1=\"{x1:.1}\" y1=\"{y1:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" spreadMethod=\"{spread}\">{stops}</linearGradient><radialGradient id=\"r{g}\"{units} cx=\"{x1:.1}\" cy=\"{y1:.1}\" r=\"{:.1}\" fx=\"{:.1}\" fy=\"{y1:.1}\" spreadMethod=\"{spread}\">{stops}</radialGradient>",
                x1 + f64::from(w) * 0.4,
                y1 + f64::from(h) * 0.3,
                f64::from(w) * 0.35,
                x1 + 10.0,
            );
        }
    }
    svg += "</defs>";
    for i in 0..count {
        let (cx, cy) = (rng.next() * f64::from(w), rng.next() * f64::from(h));
        let r = (6.0 + rng.next() * f64::from(w.min(h)) * 0.08) * scale;
        let n = 3 + (rng.next() * 5.0) as usize;
        let mut d = String::new();
        for k in 0..n {
            let a = (k as f64 + rng.next() * 0.5) / n as f64 * std::f64::consts::TAU;
            let rr = r * (0.5 + rng.next() * 0.5);
            d += &format!(
                "{}{:.2} {:.2}",
                if k == 0 { "M" } else { "L" },
                cx + rr * a.cos(),
                cy + rr * a.sin()
            );
        }
        let kind = match (kinds, i % 3) {
            ("solid", _) | ("mixed", 0) => format!(
                "fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{:.2}\"",
                (rng.next() * 255.0) as u8,
                (rng.next() * 255.0) as u8,
                (rng.next() * 255.0) as u8,
                0.4 + rng.next() * 0.6
            ),
            ("linear", _) | ("mixed", 1) => format!("fill=\"url(#l{})\"", i % 8),
            _ => format!("fill=\"url(#r{})\"", i % 8),
        };
        let rule = if i % 7 == 0 {
            " fill-rule=\"evenodd\""
        } else {
            ""
        };
        svg += &format!("<path d=\"{d}Z\" {kind}{rule}/>");
    }
    svg += "</svg>";
    svg.into_bytes()
}

fn frame(bytes: &[u8], w: u32, h: u32, cache: &mut DiffCache) -> Frame {
    compose_frame_cached(Arc::new(bytes.to_vec()), w, h, cache).unwrap()
}

fn cpu_frame(bytes: &[u8], w: u32, h: u32) -> Frame {
    compose_frame_owned(Arc::new(bytes.to_vec()), w, h).unwrap()
}

/// Run `f` with the route forced on and the global state restored after.
fn with_gpu<T>(on: Option<bool>, f: impl FnOnce() -> T) -> T {
    set_gpu_display(on);
    let out = f();
    set_gpu_display(None);
    out
}

fn require_gpu() -> bool {
    std::env::var_os("VEXEL_REQUIRE_GPU").is_some_and(|v| v == "1")
}

/// (fraction over 2/255, fraction over 16/255, mean abs error in 0..1)
fn metrics(a: &[u8], b: &[u8]) -> (f64, f64, f64) {
    assert_eq!(a.len(), b.len());
    let (mut o2, mut o16, mut sum) = (0usize, 0usize, 0u64);
    for (p, q) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let m = p.iter().zip(q).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
        o2 += usize::from(m > 2);
        o16 += usize::from(m > 16);
        sum += p
            .iter()
            .zip(q)
            .map(|(x, y)| u64::from(x.abs_diff(*y)))
            .sum::<u64>();
    }
    let px = (a.len() / 4) as f64;
    (
        o2 as f64 / px,
        o16 as f64 / px,
        sum as f64 / (a.len() as f64 * 255.0),
    )
}

#[test]
fn export_paths_do_not_reference_the_route() {
    // The route is reachable only from the display compose functions: files
    // naming it or the display cache constructor are exactly these.
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    walk(&root.join("src"), &mut files);
    walk(&root.join("tests"), &mut files);
    let rel = |p: &std::path::Path| p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
    let mut route = Vec::new();
    let mut display = Vec::new();
    let mut force = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        let name = rel(path);
        if name.ends_with("tests_gpu.rs") {
            continue;
        }
        if text.contains("gpu_route") && !name.starts_with("tests/gpu_route_report") {
            route.push(name.clone());
        }
        if text.contains("DiffCache::display") {
            display.push(name.clone());
        }
        if text.contains("set_gpu_display") {
            force.push(name);
        }
    }
    route.sort();
    display.sort();
    force.sort();
    assert_eq!(
        route,
        ["src/compose/diff.rs", "src/compose/mod.rs"],
        "gpu_route referenced outside the compose display path"
    );
    assert_eq!(
        display,
        [
            "src/compose/gpu_route.rs",
            "src/compose/worker.rs",
            "src/native_animation/prepared.rs"
        ],
        "display caches (the only GPU-capable kind) built outside the two workers"
    );
    // Forcing the route is for benches and tests only, never library code.
    for name in &force {
        assert!(
            name.starts_with("tests/")
                || name.ends_with("gpu_route.rs")
                || name.ends_with("mod.rs"),
            "{name} forces the GPU route"
        );
    }
}

#[test]
fn default_caches_and_plain_compose_never_use_the_gpu() {
    let _g = serial();
    let svg = scene_svg(1, 60, 256, 192, "solid");
    let before = gpu_frames();
    with_gpu(Some(true), || {
        let _ = cpu_frame(&svg, 256, 192);
        let mut cache = DiffCache::default();
        let _ = frame(&svg, 256, 192, &mut cache);
        let _ = compose_svg(&svg, 256, 192).unwrap();
        let _ = compose_document(&svg, 256, 192, &|| false).unwrap();
    });
    assert_eq!(gpu_frames(), before);
}

/// Backend whose every launch fails.
struct Failing(u32);

impl ComputeBackend for Failing {
    fn run(&mut self, _: &ComputeRequest<'_>, _: &mut [u8]) -> Result<ComputeStats, GpuError> {
        self.0 += 1;
        Err(GpuError::CommandFailed)
    }
}

fn tiny_scene() -> GpuScene {
    let mut scene = GpuScene::new(32, 32);
    let seg = |x0, y0, x1, y1| GpuSegment { x0, y0, x1, y1 };
    scene.push_fill(
        vec![
            seg(2.0, 2.0, 20.0, 2.0),
            seg(20.0, 2.0, 20.0, 20.0),
            seg(20.0, 20.0, 2.0, 20.0),
            seg(2.0, 20.0, 2.0, 2.0),
        ],
        false,
        GpuPaint::Solid([200, 10, 10, 255]),
    );
    scene
}

#[test]
fn runtime_errors_fall_back_and_latch_after_three() {
    let mut session = Session::new();
    let mut rasterizer = GpuRasterizer::new(Failing(0));
    let scene = tiny_scene();
    for n in 1..=LATCH_ERROR_LIMIT {
        assert!(!session.latch.is_off(), "latched early at {n}");
        assert!(render_checked(&mut rasterizer, &scene, &mut session).is_none());
        assert_eq!(super::gpu_route::last_decision(), Decision::Error);
    }
    assert!(session.latch.is_off());
    assert_eq!(rasterizer.backend_mut().0, LATCH_ERROR_LIMIT);
}

#[test]
fn latched_session_turns_the_route_off() {
    let _g = serial();
    let svg = scene_svg(2, 40, 200, 150, "solid");
    gpu_reset_latch();
    // Trip the process latch directly through the public session path.
    {
        let mut session = Session::new();
        for _ in 0..LATCH_ERROR_LIMIT {
            session.error(GpuError::CommandFailed);
        }
        assert!(session.latch.is_off());
    }
    // Three runtime errors recorded on the process session.
    for _ in 0..LATCH_ERROR_LIMIT {
        super::gpu_route::record_error_for_test();
    }
    let before = gpu_frames();
    let mut cache = DiffCache::display();
    let got = with_gpu(Some(true), || frame(&svg, 200, 150, &mut cache));
    assert_eq!(super::gpu_route::last_decision(), Decision::Off);
    assert_eq!(gpu_frames(), before);
    assert_eq!(got, cpu_frame(&svg, 200, 150));
    assert_eq!(gpu_status(), "VEXEL_GPU: latched off");
    gpu_reset_latch();
}

#[test]
fn ineligible_scenes_stay_on_the_cpu_with_identical_bytes() {
    let _g = serial();
    gpu_reset_latch();
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><rect x="4" y="4" width="40" height="40" fill="#336699" stroke="#000" stroke-width="3"/></svg>"##;
    let mut cache = DiffCache::display();
    let got = with_gpu(Some(true), || frame(svg, 64, 64, &mut cache));
    assert_eq!(super::gpu_route::last_decision(), Decision::Cpu("stroke"));
    assert_eq!(got, cpu_frame(svg, 64, 64));
}

#[test]
fn eligible_scene_is_lowered_and_classified() {
    let _g = serial();
    gpu_reset_latch();
    let svg = scene_svg(3, 200, 400, 300, "mixed");
    let mut cache = DiffCache::display();
    let got = with_gpu(Some(true), || frame(&svg, 400, 300, &mut cache));
    let decision = super::gpu_route::last_decision();
    assert!(
        matches!(decision, Decision::Gpu | Decision::NoDevice),
        "{decision:?}"
    );
    if decision == Decision::NoDevice {
        assert!(!require_gpu(), "VEXEL_REQUIRE_GPU=1 but no Metal device");
        assert_eq!(got, cpu_frame(&svg, 400, 300));
    }
}

#[test]
fn gpu_frames_match_cpu_within_adr_bounds() {
    let _g = serial();
    gpu_reset_latch();
    let mut checked = 0;
    for (seed, kind, w, h) in [
        (10, "solid", 400, 300),
        (11, "linear", 400, 300),
        (12, "radial", 640, 300),
        (13, "mixed", 900, 900),
    ] {
        let svg = scene_svg(seed, 150, w, h, kind);
        let cpu = cpu_frame(&svg, w, h);
        let mut cache = DiffCache::display();
        let got = with_gpu(Some(true), || frame(&svg, w, h, &mut cache));
        match super::gpu_route::last_decision() {
            Decision::Gpu => {}
            Decision::NoDevice => {
                assert!(!require_gpu(), "VEXEL_REQUIRE_GPU=1 but no Metal device");
                return;
            }
            other => panic!("{kind} {w}x{h}: {other:?}"),
        }
        assert_eq!((got.width, got.height), (cpu.width, cpu.height));
        let (o2, o16, mae) = metrics(&got.rgba, &cpu.rgba);
        println!("parity {kind} {w}x{h}: >2={o2:.6} >16={o16:.6} mae={mae:.7}");
        assert!(o2 <= PARITY_OVER_2_MAX_FRACTION, "{kind}: >2/255 {o2}");
        assert!(o16 <= PARITY_OVER_16_MAX_FRACTION, "{kind}: >16/255 {o16}");
        assert!(mae <= PARITY_MEAN_ABS_ERROR_MAX, "{kind}: mae {mae}");
        checked += 1;
    }
    assert_eq!(checked, 4);
}

/// `scene_svg` with its paths wrapped in nested `<g opacity>` groups up to
/// `depth` levels (deterministic nesting and opacities), plus opacity-1 groups.
fn group_svg(seed: u64, count: usize, w: u32, h: u32, kinds: &str, depth: usize) -> Vec<u8> {
    let svg = String::from_utf8(scene_svg(seed, count, w, h, kinds)).unwrap();
    let (head, body) = svg.split_once("</defs>").unwrap();
    let body = body.strip_suffix("</svg>").unwrap();
    let mut out = format!("{head}</defs>");
    let mut open = 0usize;
    for (i, path) in body.split_inclusive("/>").enumerate() {
        match i % 6 {
            0 | 3 if open < depth => {
                let opacity = [0.3, 0.55, 0.8, 1.0, 0.15][(i / 3 + open) % 5];
                out += &format!("<g opacity=\"{opacity}\">");
                open += 1;
            }
            5 if open > 0 => {
                out += "</g>";
                open -= 1;
            }
            _ => {}
        }
        out += path;
    }
    out += &"</g>".repeat(open);
    out += "</svg>";
    out.into_bytes()
}

#[test]
fn group_frames_match_cpu_within_adr_bounds() {
    let _g = serial();
    gpu_reset_latch();
    let mut checked = 0;
    for (seed, kind, w, h, depth) in [
        (40, "solid", 400, 300, 1),
        (41, "mixed", 400, 300, 2),
        (42, "linear", 640, 300, 3),
        (43, "mixed", 900, 900, 4),
    ] {
        let svg = group_svg(seed, 240, w, h, kind, depth);
        let cpu = cpu_frame(&svg, w, h);
        let mut cache = DiffCache::display();
        let got = with_gpu(Some(true), || frame(&svg, w, h, &mut cache));
        match super::gpu_route::last_decision() {
            Decision::Gpu => {}
            Decision::NoDevice => {
                assert!(!require_gpu(), "VEXEL_REQUIRE_GPU=1 but no Metal device");
                return;
            }
            other => panic!("{kind} {w}x{h} depth {depth}: {other:?}"),
        }
        let (o2, o16, mae) = metrics(&got.rgba, &cpu.rgba);
        println!(
            "parity groups {kind} {w}x{h} depth {depth}: >2={o2:.6} >16={o16:.6} mae={mae:.7}"
        );
        assert!(o2 <= PARITY_OVER_2_MAX_FRACTION, "{kind}: >2/255 {o2}");
        assert!(o16 <= PARITY_OVER_16_MAX_FRACTION, "{kind}: >16/255 {o16}");
        assert!(mae <= PARITY_MEAN_ABS_ERROR_MAX, "{kind}: mae {mae}");
        checked += 1;
    }
    assert_eq!(checked, 4);
}

#[test]
fn filtered_masked_and_too_deep_groups_stay_on_the_cpu() {
    let _g = serial();
    gpu_reset_latch();
    let head = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"64\" viewBox=\"0 0 64 64\"><defs><filter id=\"f\"><feGaussianBlur stdDeviation=\"2\"/></filter><mask id=\"m\"><rect width=\"32\" height=\"64\" fill=\"#fff\"/></mask></defs>";
    let shape = "<rect x=\"8\" y=\"8\" width=\"40\" height=\"40\" fill=\"#336699\"/>";
    let mut cases: Vec<(String, Decision)> = vec![
        (
            format!("{head}<g opacity=\"0.5\" filter=\"url(#f)\">{shape}</g></svg>"),
            Decision::Cpu("group"),
        ),
        (
            format!("{head}<g opacity=\"0.5\" mask=\"url(#m)\">{shape}</g></svg>"),
            Decision::Cpu("group"),
        ),
    ];
    let nest = |n: usize| {
        format!(
            "{head}{}{shape}{}</svg>",
            "<g opacity=\"0.9\">".repeat(n),
            "</g>".repeat(n)
        )
    };
    cases.push((
        nest(5),
        Decision::Unsupported(vexel_gpu_scene::UnsupportedReason::GroupTooDeep),
    ));
    for (svg, want) in cases {
        let mut cache = DiffCache::display();
        let got = with_gpu(Some(true), || frame(svg.as_bytes(), 64, 64, &mut cache));
        let decision = super::gpu_route::last_decision();
        if decision == Decision::NoDevice {
            continue;
        }
        assert_eq!(decision, want, "{svg}");
        assert_eq!(got, cpu_frame(svg.as_bytes(), 64, 64));
    }
}

#[test]
fn policy_sends_light_scenes_and_warm_caches_to_the_cpu() {
    let _g = serial();
    gpu_reset_latch();
    let light = scene_svg(30, 40, 200, 150, "solid");
    let heavy = scene_svg_scaled(31, 300, 900, 900, "linear", 2.0);
    set_gpu_display_policy();
    let mut cache = DiffCache::display();
    let a = frame(&light, 200, 150, &mut cache);
    assert_eq!(super::gpu_route::last_decision(), Decision::Cpu("light"));
    assert_eq!(a, cpu_frame(&light, 200, 150));
    // Heavy gradient scene: GPU on a cold cache, then a CPU full render that
    // seeds the diff state, then the differential path.
    let mut cache = DiffCache::display();
    let first = frame(&heavy, 900, 900, &mut cache);
    let d1 = super::gpu_route::last_decision();
    assert!(matches!(d1, Decision::Gpu | Decision::NoDevice), "{d1:?}");
    if d1 == Decision::Gpu {
        let second = frame(&heavy, 900, 900, &mut cache);
        assert_eq!(super::gpu_route::last_decision(), Decision::Cpu("diff"));
        assert_eq!(second, cpu_frame(&heavy, 900, 900));
        let (o2, _, mae) = metrics(&first.rgba, &second.rgba);
        assert!(o2 <= PARITY_OVER_2_MAX_FRACTION && mae <= PARITY_MEAN_ABS_ERROR_MAX);
        let third = frame(&heavy, 900, 900, &mut cache);
        assert_eq!(super::gpu_route::last_decision(), Decision::Cpu("diff"));
        assert_eq!(third, second);
    } else {
        assert!(!require_gpu(), "VEXEL_REQUIRE_GPU=1 but no Metal device");
    }
    set_gpu_display(None);
}

#[test]
fn switching_gpu_and_cpu_never_mixes_cache_state() {
    let _g = serial();
    gpu_reset_latch();
    let (w, h) = (320, 240);
    let a = scene_svg(20, 120, w, h, "mixed");
    let mut b = scene_svg(20, 120, w, h, "mixed");
    // Edit: move one shape so a differential render would patch a region.
    let text = String::from_utf8(b.clone()).unwrap();
    b = text
        .replacen("<path d=\"M", "<path transform=\"translate(9 4)\" d=\"M", 1)
        .into_bytes();
    let mut cache = DiffCache::display();
    // GPU (or CPU without device) -> CPU -> GPU -> CPU, each against a fresh CPU render.
    let steps: [(Option<bool>, &[u8]); 5] = [
        (Some(true), &a),
        (Some(false), &b),
        (Some(false), &a),
        (Some(true), &b),
        (Some(false), &a),
    ];
    for (n, (force, svg)) in steps.iter().enumerate() {
        let got = with_gpu(*force, || frame(svg, w, h, &mut cache));
        let used_gpu = super::gpu_route::last_decision() == Decision::Gpu;
        if force == &Some(false) {
            assert_eq!(
                got,
                cpu_frame(svg, w, h),
                "step {n} must equal a fresh CPU render"
            );
        } else if used_gpu {
            let (o2, _, mae) = metrics(&got.rgba, &cpu_frame(svg, w, h).rgba);
            assert!(
                o2 <= PARITY_OVER_2_MAX_FRACTION && mae <= PARITY_MEAN_ABS_ERROR_MAX,
                "step {n}"
            );
        }
    }
}

/// Release sweep: `cargo test -p vexel-app --release --lib gpu_vs_cpu_crossover -- --ignored --nocapture`
#[test]
#[ignore]
fn gpu_vs_cpu_crossover() {
    let _g = serial();
    gpu_reset_latch();
    for kind in ["solid", "linear", "radial"] {
        for (count, scale) in [
            (300usize, 0.5),
            (300, 1.0),
            (300, 2.0),
            (300, 4.0),
            (1000, 0.5),
            (1000, 1.0),
            (1000, 2.0),
            (5000, 0.5),
            (5000, 1.0),
        ] {
            let svg = scene_svg_scaled(77, count, 900, 900, kind, scale);
            let mut row = Vec::new();
            for force in [false, true] {
                let mut times = Vec::new();
                for _ in 0..7 {
                    let mut cache = DiffCache::display();
                    let t = std::time::Instant::now();
                    let _ = with_gpu(Some(force), || frame(&svg, 900, 900, &mut cache));
                    times.push(t.elapsed().as_secs_f64() * 1000.0);
                }
                times.sort_by(f64::total_cmp);
                row.push(times[times.len() / 2]);
            }
            println!(
                "static 900px {kind} n={count} scale={scale}: work {:.2} Mpx | cpu {:.2} ms gpu {:.2} ms ({:?})",
                super::gpu_route::last_work() / 1e6,
                row[0],
                row[1],
                super::gpu_route::last_decision()
            );
        }
    }
}
