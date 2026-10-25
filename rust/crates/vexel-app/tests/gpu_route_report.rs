//! GPU display route report: which demo scenes are eligible and per-tick
//! GPU vs CPU times for synthetic eligible scenes (not a pass/fail gate).
//! `cargo test -p vexel-app --release --test gpu_route_report -- --ignored --nocapture --test-threads=1`

use std::path::Path;
use std::time::Instant;

use vexel_app::animation::playback::ManualClock;
use vexel_app::compose::{gpu_frames, gpu_last_decision, set_gpu_display, set_gpu_display_policy};
use vexel_app::native_animation::NativeAnimationHost;
use vexel_app::preview::Control;
use vexel_app::session::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn p50(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

/// `count` triangles in a 512 box; `kind` is solid, linear or radial.
fn synthetic_doc(kind: &str, count: u32) -> AppDocument {
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\"><defs>",
    );
    let stops =
        "<stop offset=\"0\" stop-color=\"#e63946\"/><stop offset=\"1\" stop-color=\"#1d3557\"/>";
    for g in 0..8 {
        svg.push_str(&format!(
            "<linearGradient id=\"l{g}\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">{stops}</linearGradient><radialGradient id=\"r{g}\" cx=\"0.5\" cy=\"0.5\" r=\"0.6\" fx=\"0.4\" fy=\"0.45\">{stops}</radialGradient>"
        ));
    }
    svg.push_str("</defs>");
    let side = (f64::from(count)).sqrt().ceil() as u32;
    let cell = 512.0 / f64::from(side);
    for i in 0..count {
        let (x, y) = (f64::from(i % side) * cell, f64::from(i / side) * cell);
        let fill = match kind {
            "solid" => format!("#{:02x}66{:02x}", (i * 7) % 256, (i * 13) % 256),
            "linear" => format!("url(#l{})", i % 8),
            _ => format!("url(#r{})", i % 8),
        };
        svg.push_str(&format!(
            "<path id=\"e{i}\" d=\"M{x:.1} {y:.1} L{:.1} {:.1} L{:.1} {:.1} Z\" fill=\"{fill}\" fill-opacity=\"0.9\"/>",
            x + cell * 1.2,
            y + cell * 0.3,
            x + cell * 0.4,
            y + cell * 1.1
        ));
    }
    svg.push_str("</svg>");
    let path = std::env::temp_dir().join(format!(
        "vexel-gpu-{kind}-{count}-{}.svg",
        std::process::id()
    ));
    std::fs::write(&path, svg).unwrap();
    let mut doc = AppDocument::new(512, 512).unwrap();
    doc.open_path(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let settings = EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 96,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Repeat,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    };
    for n in 0..8u32 {
        let (track, target) = (format!("t{n}"), format!("e{}", n * (count / 9).max(1) + 3));
        doc.edit_timeline(settings, EditLimits::default(), |c| {
            let mut c = c.add_property_track(
                track.as_bytes(),
                target.as_bytes(),
                b"fill-opacity",
                b"scalar",
                b"linear",
                b"1",
                track.as_bytes(),
            )?;
            for k in 1..=8u32 {
                let v = if k % 2 == 0 { "0.2" } else { "1" };
                c = c.set_property_key(track.as_bytes(), k * 10, v.as_bytes())?;
            }
            Ok::<_, vexel_app::project_session::ProjectTimelineError>(c)
        })
        .unwrap();
    }
    doc
}

/// p50 tick ms over `ticks` seeks of the document, plus the last decision.
fn run(doc: &AppDocument, size: u32, ticks: u32) -> (f64, String, u64) {
    let mut host = NativeAnimationHost::new(ManualClock::default());
    let _ = host.snapshot(doc, None);
    let before = gpu_frames();
    let (mut times, mut last) = (Vec::new(), String::new());
    for i in 0..ticks {
        let _ = host.mcp_control(doc, Control::Seek(f64::from(i) / 24.0), true);
        let _ = host.snapshot(doc, None);
        let t = Instant::now();
        let frame = host.tick(doc, (size, size));
        times.push(ms(t));
        if frame.is_some() && last.is_empty() {
            last = format!("{:?}", gpu_last_decision());
        }
    }
    (p50(&mut times), last, gpu_frames() - before)
}

#[test]
#[ignore]
fn eligibility_of_demo_scenes() {
    set_gpu_display_policy();
    for path in [
        "/private/tmp/vexel-demo/walk_home.vexel",
        "/private/tmp/vexel-demo/logo.vexel",
        "/private/tmp/vexel-demo/loader.vexel",
        "/private/tmp/vexel-accept/walk_home.vexel",
    ] {
        let path = Path::new(path);
        if !path.exists() {
            println!("eligibility {}: absent", path.display());
            continue;
        }
        let mut doc = AppDocument::new(512, 512).unwrap();
        doc.open_path(path).unwrap();
        let mut cpu = [0.0f64; 3];
        let mut pol = [0.0f64; 3];
        let mut last = String::new();
        for round in 0..3 {
            set_gpu_display(Some(false));
            cpu[round] = run(&doc, 900, 24).0;
            set_gpu_display_policy();
            let (t, l, _) = run(&doc, 900, 24);
            pol[round] = t;
            last = l;
        }
        println!(
            "eligibility {}: first decision {last} | p50 tick cpu {:?} policy {:?} ms",
            path.file_name().unwrap().to_string_lossy(),
            cpu.map(|v| (v * 100.0).round() / 100.0),
            pol.map(|v| (v * 100.0).round() / 100.0)
        );
    }
    for kind in ["solid", "linear", "radial"] {
        let doc = synthetic_doc(kind, 1000);
        let (_, last, _) = run(&doc, 900, 4);
        println!("eligibility synthetic {kind} 1000: first decision {last}");
    }
    set_gpu_display(None);
}

#[test]
#[ignore]
fn gpu_vs_cpu_playback_ticks() {
    for kind in ["solid", "linear", "radial"] {
        for count in [1000u32, 5000] {
            let doc = synthetic_doc(kind, count);
            set_gpu_display(Some(false));
            let (cpu, _, _) = run(&doc, 900, 24);
            set_gpu_display(Some(true));
            let (gpu, last, frames) = run(&doc, 900, 24);
            set_gpu_display_policy();
            let (pol, plast, pframes) = run(&doc, 900, 24);
            println!(
                "tick 900px {kind} {count}: cpu {cpu:.2} ms | gpu forced {gpu:.2} ms ({last}, {frames} gpu frames) | policy {pol:.2} ms ({plast}, {pframes} gpu frames)"
            );
        }
    }
    set_gpu_display(None);
}
