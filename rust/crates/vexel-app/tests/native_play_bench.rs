//! Release bench: switch to Animate, then play 96 native ticks.
//! `cargo test -p vexel-app --release --test native_play_bench -- --ignored --nocapture`

use std::path::Path;
use std::time::Instant;

use vexel_app::animation::playback::{ManualClock, MonotonicClock};
use vexel_app::native_animation::NativeAnimationHost;
use vexel_app::preview::Control;
use vexel_app::session::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

/// 1000 rects on a grid with 8 opacity/x tracks of 8 keys: the walk_home-sized
/// fallback so the bench runs without /private/tmp files.
fn synthetic_doc() -> AppDocument {
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
    let path = std::env::temp_dir().join(format!("vexel-synth-{}.svg", std::process::id()));
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
        let (track, target) = (format!("t{n}"), format!("e{}", n * 120 + 3));
        doc.edit_timeline(settings, EditLimits::default(), |c| {
            let mut c = c.add_property_track(
                track.as_bytes(),
                target.as_bytes(),
                PROPS[n as usize % 4],
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

const PROPS: [&[u8]; 4] = [b"opacity", b"fill-opacity", b"stroke-opacity", b"opacity"];

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn run(label: &str, doc: &AppDocument, size: u32) {
    let mut host = NativeAnimationHost::new(ManualClock::default());
    let t = Instant::now();
    let first = host.snapshot(doc, None);
    println!(
        "{label}: first snapshot {:.1} ms ok={}",
        ms(t),
        first.is_ok()
    );
    let ticks = first.as_ref().map(|s| s.duration_ticks).unwrap_or(96).min(
        std::env::var("TICKS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(96),
    );
    let (mut snap, mut tick, mut got, mut hashes) = (vec![], vec![], 0, vec![]);
    for i in 0..ticks {
        let seconds = f64::from(i) / 24.0;
        let _ = host.mcp_control(doc, Control::Seek(seconds), true);
        let t = Instant::now();
        let _ = host.snapshot(doc, None);
        snap.push(ms(t));
        let t = Instant::now();
        let frame = host.tick(doc, (size, size));
        tick.push(ms(t));
        if let Some(frame) = frame {
            got += 1;
            hashes.push(format!("{i}:{:016x}", fnv(&frame.rgba)));
        }
    }
    if let Some(path) = std::env::var_os("HASH_OUT") {
        let _ = std::fs::write(path, hashes.join("\n"));
    }
    println!(
        "{label}: size {size} frames {got}/{ticks} snapshot p50 {:.2} p95 {:.2} | tick p50 {:.2} p95 {:.2} ms",
        pct(&mut snap, 0.5),
        pct(&mut snap, 0.95),
        pct(&mut tick, 0.5),
        pct(&mut tick, 0.95)
    );
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn profile_run(label: &str, doc: &AppDocument) {
    vexel_app::profile::enable();
    let size = std::env::var("SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(900);
    run(label, doc, size);
    for (name, v) in vexel_app::profile::take() {
        let mut v = v;
        let sum: f64 = v.iter().sum();
        println!(
            "phase {name}: n={} sum {sum:.1} p50 {:.2} p95 {:.2} max {:.2}",
            v.len(),
            pct(&mut v, 0.5),
            pct(&mut v, 0.95),
            v[v.len() - 1]
        );
    }
}

#[test]
#[ignore]
fn walk_home_animate_playback() {
    let path = Path::new("/private/tmp/vexel-accept/walk_home.vexel");
    if !path.exists() {
        eprintln!("skip: {} absent", path.display());
        return;
    }
    let mut doc = AppDocument::new(512, 512).unwrap();
    doc.open_path(path).unwrap();
    profile_run("walk_home", &doc);
}

#[test]
#[ignore]
fn synthetic_animate_playback() {
    let doc = synthetic_doc();
    profile_run("synthetic full 900", &doc);
    // On-screen size (S2): the same ticks rendered at the stage's pixel size.
    run("synthetic play 600", &doc, 600);
}

/// Real-time playback as the UI loop drives it for 4 s at ~8 ms per loop.
/// `sync` renders on the loop (the old behaviour); `async` posts to the
/// worker. Prints UI-thread blocked time per loop and the presented fps.
#[test]
#[ignore]
fn synthetic_realtime_playback() {
    let doc = synthetic_doc();
    for (mode, play) in [
        ("sync full 900", (900, 900)),
        ("async full 900", (900, 900)),
        ("async play 600", (600, 600)),
    ] {
        let mut host = NativeAnimationHost::new(MonotonicClock::new());
        let first = Instant::now();
        let _ = host.snapshot(&doc, None);
        host.mcp_control(&doc, Control::Play, true).unwrap();
        let mut blocked = vec![];
        let (mut frames, mut first_frame) = (0u32, None);
        let start = Instant::now();
        while start.elapsed().as_secs_f64() < 4.0 {
            let t = Instant::now();
            let got = if mode.starts_with("sync") {
                host.tick(&doc, play).is_some()
            } else {
                host.tick_async(&doc, (900, 900), play, true).is_some()
            };
            blocked.push(ms(t));
            if got {
                frames += 1;
                first_frame.get_or_insert(ms(first));
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
        let secs = start.elapsed().as_secs_f64();
        println!(
            "{mode}: loops {} ui-blocked p50 {:.2} p95 {:.2} max {:.2} ms | frames {frames} = {:.1} fps | first frame at {:.0} ms",
            blocked.len(),
            pct(&mut blocked.clone(), 0.5),
            pct(&mut blocked.clone(), 0.95),
            pct(&mut blocked, 1.0),
            f64::from(frames) / secs,
            first_frame.unwrap_or(f64::NAN)
        );
    }
}

/// Export output must stay byte-identical across playback optimisations.
#[test]
fn synthetic_export_hashes_are_stable() {
    let doc = synthetic_doc();
    let source =
        vexel_app::animation::native_export::NativeExportSource::from_document(&doc).unwrap();
    let mut hashes = vec![];
    for tick in [5u32, 12, 22, 33, 57] {
        let (w, h, rgba) = source
            .render_still(f64::from(tick) / 24.0, |s| Ok(s))
            .unwrap();
        hashes.push(format!("{w}x{h}:{:016x}", fnv(&rgba)));
    }
    println!("export hashes {hashes:?}");
    assert_eq!(hashes, EXPECTED_EXPORT_HASHES);
}

const EXPECTED_EXPORT_HASHES: [&str; 5] = [
    "512x512:aa217a0c36d12e18",
    "512x512:5b0c34d8d34344b3",
    "512x512:b521e2abe6e89815",
    "512x512:5c61b7ffbd5740d6",
    "512x512:781fde250b857ddc",
];

#[test]
#[ignore]
fn walk_home_dump_tracks() {
    let path = Path::new("/private/tmp/vexel-accept/walk_home.vexel");
    if !path.exists() {
        return;
    }
    let mut doc = AppDocument::new(512, 512).unwrap();
    doc.open_path(path).unwrap();
    let mut host = NativeAnimationHost::new(ManualClock::default());
    let snap = host.snapshot(&doc, None).unwrap();
    let mut by_target = std::collections::BTreeMap::<String, Vec<String>>::new();
    for t in &snap.tracks {
        by_target.entry(t.target.clone()).or_default().push(format!(
            "{}:{}",
            t.name,
            t.items.len()
        ));
    }
    println!("tracks {} targets {}", snap.tracks.len(), by_target.len());
    for (k, v) in by_target.iter().take(60) {
        println!("{k}: {}", v.join(" "));
    }
}
