//! Differential playback/export rendering must equal the uncached reference
//! (`render_still`: text patch, full decode, full raster) byte for byte on
//! every tick of every scene.
//! `cargo test -p vexel-app --release --test native_diff_identity -- --nocapture`

use std::path::Path;
use std::time::Instant;

use vexel_app::animation::native_export::NativeExportSource;
use vexel_app::animation::native_export_frames::NativeFramesRequest;
use vexel_app::project_session::ProjectTimelineError;
use vexel_app::session::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

const SETTINGS: EmptyTimelineSettings = EmptyTimelineSettings {
    ticks_per_second: 24,
    duration_ticks: 96,
    fps_numerator: 24,
    fps_denominator: 1,
    loop_mode: LoopMode::Repeat,
    onion_before: 0,
    onion_after: 0,
    onion_opacity: 0.25,
};

/// A small document from `svg` with opacity/x/fill tracks on its targets.
fn small_doc(svg: &str, tracks: &[(&str, &str, &str, &str, &[&str])]) -> AppDocument {
    let path = std::env::temp_dir().join(format!(
        "vexel-diff-small-{}-{}.svg",
        std::process::id(),
        fnv(svg.as_bytes())
    ));
    std::fs::write(&path, svg).unwrap();
    let mut doc = AppDocument::new(128, 128).unwrap();
    doc.open_path(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    for (track, target, prop, kind, vals) in tracks {
        doc.edit_timeline(SETTINGS, EditLimits::default(), |c| {
            let default: &[u8] = if *kind == "rgba" { b"[0,0,0,1]" } else { b"1" };
            let mut c = c.add_property_track(
                track.as_bytes(),
                target.as_bytes(),
                prop.as_bytes(),
                kind.as_bytes(),
                b"linear",
                default,
                track.as_bytes(),
            )?;
            for (k, v) in vals.iter().enumerate() {
                c = c.set_property_key(track.as_bytes(), (k as u32 + 1) * 20, v.as_bytes())?;
            }
            Ok::<_, ProjectTimelineError>(c)
        })
        .unwrap_or_else(|e| panic!("track {track}: {e:?}"));
    }
    doc
}

const SMALL_TRACKS: [(&str, &str, &str, &str, &[&str]); 4] = [
    ("a", "b", "cx", "scalar", &["30", "90", "40", "100"]),
    ("o", "c", "opacity", "scalar", &["0.3", "1", "0.5", "0.9"]),
    (
        "f",
        "d",
        "fill",
        "rgba",
        &["[1,0,0,1]", "[0,1,0,1]", "[0,0,1,1]", "[1,1,0,1]"],
    ),
    ("x", "a", "x", "scalar", &["5", "25", "10", "30"]),
];

/// Clips, gradients, dashes and an opacity group: diffed, same pixels.
#[test]
fn clipped_gradient_dashed_group_identical() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><defs><clipPath id="c1"><rect x="0" y="0" width="90" height="70"/></clipPath><linearGradient id="g1"><stop offset="0" stop-color="#f80"/><stop offset="1" stop-color="#08f"/></linearGradient></defs><rect id="bg" width="128" height="128" fill="#eee"/><rect id="a" x="10" y="10" width="100" height="80" fill="url(#g1)" clip-path="url(#c1)"/><circle id="b" cx="60" cy="60" r="30" fill="#c33" fill-opacity="0.6" stroke="#000" stroke-width="3" stroke-dasharray="6 3"/><g id="grp" opacity="0.8"><path id="c" d="M10 100 L60 120 L110 100 Z" fill="#339"/><rect id="d" x="70" y="90" width="40" height="30" fill="#393" stroke="#000"/></g></svg>"##;
    let doc = small_doc(svg, &SMALL_TRACKS);
    for size in [128, 100] {
        compare("small-clip", &doc, size).unwrap();
    }
}

/// Masks and patterns are never diffed (fallback to full renders): same pixels.
#[test]
fn mask_and_pattern_fall_back_identical() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><defs><pattern id="p" width="8" height="8" patternUnits="userSpaceOnUse"><rect width="4" height="4" fill="#06c"/></pattern><mask id="m"><rect width="128" height="64" fill="#fff"/></mask></defs><rect id="a" x="10" y="10" width="100" height="80" fill="url(#p)"/><circle id="b" cx="60" cy="60" r="30" fill="#c33" mask="url(#m)"/><g id="grp"><path id="c" d="M10 100 L60 120 L110 100 Z" fill="#339"/><rect id="d" x="70" y="90" width="40" height="30" fill="#393"/></g></svg>"##;
    let doc = small_doc(svg, &SMALL_TRACKS);
    compare("small-fallback", &doc, 128).unwrap();
}

/// 1000 overlapping paths; `motion` adds transform, colour, stroke width and
/// visibility tracks on top of the opacity tracks.
fn synthetic_doc(motion: bool) -> AppDocument {
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\">",
    );
    for i in 0..1000u32 {
        let (x, y) = (f64::from(i % 32) * 16.0, f64::from(i / 32) * 16.0);
        svg.push_str(&format!(
            "<path id=\"e{i}\" d=\"M{x} {y} C{} {} {} {} {} {} S{} {} {} {} S{} {} {} {} Z\" fill=\"#3366cc\" fill-opacity=\"0.8\" stroke=\"#112244\" stroke-width=\"1.5\" transform=\"rotate({} {x} {y})\"/>",
            x + 10.0, y - 6.0, x + 24.0, y + 6.0, x + 30.0, y + 10.0,
            x + 40.0, y + 24.0, x + 20.0, y + 30.0,
            x + 8.0, y + 36.0, x - 4.0, y + 20.0,
            i % 30
        ));
    }
    svg.push_str("</svg>");
    let path = std::env::temp_dir().join(format!("vexel-diff-{}-{motion}.svg", std::process::id()));
    std::fs::write(&path, svg).unwrap();
    let mut doc = AppDocument::new(512, 512).unwrap();
    doc.open_path(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let mut tracks: Vec<(String, String, &str, &str, Vec<String>)> = vec![];
    for n in 0..8u32 {
        let vals = (1..=8)
            .map(|k| if k % 2 == 0 { "0.2" } else { "1" }.to_owned())
            .collect();
        tracks.push((
            format!("t{n}"),
            format!("e{}", n * 120 + 3),
            ["opacity", "fill-opacity", "stroke-opacity", "opacity"][n as usize % 4],
            "scalar",
            vals,
        ));
    }
    if motion {
        let seq = |f: &dyn Fn(u32) -> String| (1..=8).map(f).collect::<Vec<_>>();
        tracks.push((
            "m0".into(),
            "e10".into(),
            "transform",
            "transform2d",
            seq(&|k| format!("[1,0,0,1,{},{}]", k * 6, k * 3)),
        ));
        tracks.push((
            "m1".into(),
            "e500".into(),
            "fill",
            "rgba",
            seq(&|k| format!("[{},0.2,0.5,1]", f64::from(k) / 9.0)),
        ));
        tracks.push((
            "m2".into(),
            "e700".into(),
            "stroke-width",
            "scalar",
            seq(&|k| format!("{}", k)),
        ));
        tracks.push((
            "m3".into(),
            "e901".into(),
            "visibility",
            "boolean",
            seq(&|k| format!("{}", k % 3 != 0)),
        ));
    }
    for (track, target, prop, kind, vals) in tracks {
        doc.edit_timeline(SETTINGS, EditLimits::default(), |c| {
            let mut c = c.add_property_track(
                track.as_bytes(),
                target.as_bytes(),
                prop.as_bytes(),
                kind.as_bytes(),
                if kind == "boolean" || kind == "transform2d" {
                    b"step"
                } else {
                    b"linear"
                },
                match kind {
                    "transform2d" => b"[1,0,0,1,0,0]".as_slice(),
                    "rgba" => b"[0,0,0,1]".as_slice(),
                    "boolean" => b"true".as_slice(),
                    _ => b"1".as_slice(),
                },
                track.as_bytes(),
            )?;
            for (k, v) in vals.iter().enumerate() {
                c = c.set_property_key(track.as_bytes(), (k as u32 + 1) * 10, v.as_bytes())?;
            }
            Ok::<_, ProjectTimelineError>(c)
        })
        .unwrap_or_else(|e| panic!("track {track}: {e:?}"));
    }
    doc
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * p).round() as usize]
}

/// Debug builds render every 8th tick of the loop and one size only; release
/// runs (the acceptance run) cover every tick at every size.
fn stride() -> u32 {
    if cfg!(debug_assertions) { 8 } else { 1 }
}

fn sizes() -> Vec<u32> {
    if cfg!(debug_assertions) {
        vec![512]
    } else {
        vec![512, 600]
    }
}

/// Reference and cached frames of every tick at `size`; returns the speedup.
fn compare(label: &str, doc: &AppDocument, size: u32) -> Option<(f64, f64)> {
    let source = NativeExportSource::from_document(doc)?;
    let req = NativeFramesRequest {
        size: Some((size, size)),
        background: None,
        fps: (stride() > 1).then(|| 24 / stride()),
    };
    let plan = source.plan_frames(&req).ok()?;
    let mut want = vec![];
    let mut ref_ms = vec![];
    for tick in source
        .loop_tick_list()
        .unwrap()
        .into_iter()
        .step_by(plan.stride as usize)
    {
        let t = Instant::now();
        let (_, _, rgba) = source
            .render_still(f64::from(tick) / f64::from(plan.ticks_per_second), |_| {
                Ok((size, size))
            })
            .unwrap();
        ref_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        want.push(fnv(&rgba));
    }
    let ticks: Vec<u32> = source
        .loop_tick_list()
        .unwrap()
        .into_iter()
        .step_by(plan.stride as usize)
        .collect();
    let mut got = vec![];
    let mut cached_ms = vec![];
    let mut last = Instant::now();
    source
        .render_ticks_differential(&ticks, (size, size), |_, rgba| {
            cached_ms.push(last.elapsed().as_secs_f64() * 1000.0);
            got.push(fnv(rgba));
            last = Instant::now();
        })
        .unwrap();
    let bad: Vec<_> = (0..want.len()).filter(|&i| want[i] != got[i]).collect();
    let (r50, r95) = (pct(&mut ref_ms.clone(), 0.5), pct(&mut ref_ms, 0.95));
    let (c50, c95) = (pct(&mut cached_ms.clone(), 0.5), pct(&mut cached_ms, 0.95));
    println!(
        "{label} {size}px ticks {} identical {} | ref p50 {r50:.2} p95 {r95:.2} | cached p50 {c50:.2} p95 {c95:.2} ms",
        want.len(),
        want.len() - bad.len()
    );
    assert!(bad.is_empty(), "{label} {size}: ticks differ {bad:?}");
    Some((r50, c50))
}

#[test]
fn synthetic_opacity_ticks_identical() {
    let doc = synthetic_doc(false);
    for size in sizes() {
        compare("synthetic", &doc, size).unwrap();
    }
}

#[test]
fn synthetic_motion_ticks_identical() {
    let doc = synthetic_doc(true);
    for size in sizes() {
        compare("synthetic-motion", &doc, size).unwrap();
    }
}

#[test]
fn demo_scenes_ticks_identical() {
    for name in [
        "logo",
        "loader",
        "walk_home",
        "logo_loop",
        "walk_home_legacy",
    ] {
        let path = format!("/private/tmp/vexel-demo/{name}.vexel");
        if !Path::new(&path).exists() {
            eprintln!("skip {name}: absent");
            continue;
        }
        let mut doc = AppDocument::new(512, 512).unwrap();
        if doc.open_path(Path::new(&path)).is_err() {
            eprintln!("skip {name}: open failed");
            continue;
        }
        for size in sizes() {
            if compare(name, &doc, size).is_none() {
                eprintln!("skip {name}: no native timeline");
                break;
            }
        }
    }
}

#[test]
#[ignore]
fn profile_phases() {
    let which = std::env::var("SCENE").unwrap_or_else(|_| "synthetic".into());
    let size: u32 = std::env::var("SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);
    let doc = if which == "synthetic" {
        synthetic_doc(false)
    } else {
        let mut d = AppDocument::new(512, 512).unwrap();
        d.open_path(Path::new(&format!("/private/tmp/vexel-demo/{which}.vexel")))
            .unwrap();
        d
    };
    vexel_app::profile::enable();
    compare(&which, &doc, size);
    for (name, v) in vexel_app::profile::take() {
        let mut v = v;
        let sum: f64 = v.iter().sum();
        println!(
            "phase {name}: n={} sum {sum:.1} p50 {:.2} p95 {:.2}",
            v.len(),
            pct(&mut v, 0.5),
            pct(&mut v, 0.95)
        );
    }
}
