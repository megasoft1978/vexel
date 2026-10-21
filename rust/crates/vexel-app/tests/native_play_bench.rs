//! Release bench: switch to Animate, then play 96 native ticks.
//! `cargo test -p vexel-app --release --test native_play_bench -- --ignored --nocapture`

use std::path::Path;
use std::time::Instant;

use vexel_app::animation::playback::ManualClock;
use vexel_app::native_animation::NativeAnimationHost;
use vexel_app::preview::Control;
use vexel_app::session::AppDocument;

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
    let (mut snap, mut tick, mut got) = (vec![], vec![], 0);
    for i in 0..ticks {
        let seconds = f64::from(i) / 24.0;
        let _ = host.mcp_control(doc, Control::Seek(seconds), true);
        let t = Instant::now();
        let _ = host.snapshot(doc, None);
        snap.push(ms(t));
        let t = Instant::now();
        if host.tick(doc, (size, size)).is_some() {
            got += 1;
        }
        tick.push(ms(t));
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
    vexel_app::profile::enable();
    run("walk_home", &doc, 900);
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
