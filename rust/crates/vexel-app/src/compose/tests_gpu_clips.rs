//! GPU display route clips: frame parity against the CPU and the clip
//! profile crossover sweep.

use super::*;

/// `scene_svg` with clip paths (rect, even-odd star, union of children,
/// bounding-box units, transformed, nested) applied to runs of paths through
/// `<g clip-path>` and to single paths.
fn clip_svg(seed: u64, count: usize, w: u32, h: u32, kinds: &str) -> Vec<u8> {
    let svg = String::from_utf8(scene_svg(seed, count, w, h, kinds)).unwrap();
    let (head, body) = svg.split_once("</defs>").unwrap();
    let body = body.strip_suffix("</svg>").unwrap();
    let (fw, fh) = (f64::from(w), f64::from(h));
    let mut out = format!("{head}");
    for k in 0..4 {
        let (x, y) = (fw * (0.05 + 0.2 * k as f64), fh * (0.05 + 0.15 * k as f64));
        let (cw, ch) = (fw * 0.55, fh * 0.5);
        out += &format!(
            "<clipPath id=\"c{k}a\"><rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{cw:.1}\" height=\"{ch:.1}\"/></clipPath>\
<clipPath id=\"c{k}b\"><path clip-rule=\"evenodd\" d=\"M{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} Z\"/></clipPath>\
<clipPath id=\"c{k}c\"><circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\"/><rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\"/></clipPath>\
<clipPath id=\"c{k}d\" clipPathUnits=\"objectBoundingBox\"><circle cx=\".5\" cy=\".5\" r=\".47\"/></clipPath>\
<clipPath id=\"c{k}e\"><rect transform=\"rotate({} {:.1} {:.1}) scale(0.9 1.1)\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{cw:.1}\" height=\"{ch:.1}\"/></clipPath>\
<clipPath id=\"c{k}f\" ><circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\"/></clipPath>",
            x + cw * 0.5,
            y,
            x + cw,
            y + ch,
            x,
            y + ch * 0.4,
            x + cw,
            y + ch * 0.4,
            x,
            y + ch,
            x + cw * 0.5,
            y + ch * 0.5,
            ch * 0.45,
            x + cw * 0.5,
            y + ch * 0.2,
            cw * 0.45,
            ch * 0.3,
            15 + 20 * k,
            fw / 2.0,
            fh / 2.0,
            x + cw * 0.5,
            y + ch * 0.5,
            ch * 0.6,
        );
    }
    out += "</defs>";
    let tags = ["a", "b", "c", "d", "e", "f"];
    let mut open = false;
    for (i, path) in body.split_inclusive("/>").enumerate() {
        let id = format!("c{}{}", (i / 6) % 4, tags[(i / 4 + i) % 6]);
        if i % 5 == 0 && !open {
            out += &format!("<g clip-path=\"url(#{id})\">");
            open = true;
        } else if i % 5 == 4 && open {
            out += "</g>";
            open = false;
        }
        if i % 7 == 3 {
            out += &path.replacen("<path ", &format!("<path clip-path=\"url(#{id})\" "), 1);
        } else {
            out += path;
        }
    }
    if open {
        out += "</g>";
    }
    out += "</svg>";
    out.into_bytes()
}

#[test]
fn clip_frames_match_cpu_within_adr_bounds() {
    let _g = serial();
    gpu_reset_latch();
    let mut checked = 0;
    for (seed, kind, w, h) in [
        (50, "solid", 400, 300),
        (51, "mixed", 400, 300),
        (52, "linear", 640, 300),
        (53, "mixed", 900, 900),
    ] {
        let svg = clip_svg(seed, 160, w, h, kind);
        let cpu = cpu_frame(&svg, w, h);
        let mut cache = DiffCache::display();
        let got = with_gpu(Some(true), || frame(&svg, w, h, &mut cache));
        match super::gpu_route::last_decision() {
            Decision::Gpu => {}
            Decision::NoDevice => {
                assert!(!require_gpu(), "VEXEL_REQUIRE_GPU=1 but no Metal device");
                return;
            }
            other => panic!("{kind} {w}x{h} clips: {other:?}"),
        }
        let (o2, o16, mae) = metrics(&got.rgba, &cpu.rgba);
        println!("parity clips {kind} {w}x{h}: >2={o2:.6} >16={o16:.6} mae={mae:.7}");
        assert!(o2 <= PARITY_OVER_2_MAX_FRACTION, "{kind}: >2/255 {o2}");
        assert!(o16 <= PARITY_OVER_16_MAX_FRACTION, "{kind}: >16/255 {o16}");
        assert!(mae <= PARITY_MEAN_ABS_ERROR_MAX, "{kind}: mae {mae}");
        checked += 1;
    }
    assert_eq!(checked, 4);
}

/// The `vexel-bench` clip profile shape: `n` filled shapes, each clipped by one
/// of `min(n, 48)` shared circle / triangle clip paths.
fn bench_clip_svg(n: usize, size: u32, scale: f64) -> Vec<u8> {
    let mut rng = Lcg(91);
    let c = f64::from(size);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" viewBox=\"0 0 {size} {size}\"><defs>"
    );
    let defs = n.clamp(1, 48);
    for k in 0..defs {
        let (x, y, r) = (
            rng.next() * c,
            rng.next() * c,
            (30.0 + rng.next() * 90.0) * scale,
        );
        if k % 2 == 0 {
            svg += &format!(
                "<clipPath id=\"c{k}\"><circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{r:.1}\"/></clipPath>"
            );
        } else {
            svg += &format!(
                "<clipPath id=\"c{k}\"><path d=\"M{x:.1} {y:.1} l{r:.1} {:.1} l{:.1} {r:.1} z\"/></clipPath>",
                r / 3.0,
                -r / 2.0
            );
        }
    }
    svg += "</defs>";
    for _ in 0..n {
        let (x, y) = (rng.next() * c, rng.next() * c);
        let (w, h) = (
            (10.0 + rng.next() * 80.0) * scale,
            (10.0 + rng.next() * 80.0) * scale,
        );
        let k = (rng.next() * defs as f64) as usize % defs;
        svg += &format!(
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"#{:02x}{:02x}{:02x}\" clip-path=\"url(#c{k})\"/>",
            (rng.next() * 255.0) as u8,
            (rng.next() * 255.0) as u8,
            (rng.next() * 255.0) as u8
        );
    }
    svg += "</svg>";
    svg.into_bytes()
}

/// Release sweep: `cargo test -p vexel-app --release --lib gpu_vs_cpu_clip -- --ignored --nocapture`
#[test]
#[ignore]
fn gpu_vs_cpu_clip_crossover() {
    let _g = serial();
    gpu_reset_latch();
    for (n, scale) in [
        (500usize, 1.0),
        (2000, 1.0),
        (500, 0.3),
        (2000, 0.3),
        (500, 3.0),
        (2000, 3.0),
    ] {
        let svg = bench_clip_svg(n, 900, scale);
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
            "clip profile 900px n={n} scale={scale}: work {:.2} Mpx | cpu {:.2} ms gpu {:.2} ms ({:?})",
            super::gpu_route::last_work() / 1e6,
            row[0],
            row[1],
            super::gpu_route::last_decision()
        );
    }
}
