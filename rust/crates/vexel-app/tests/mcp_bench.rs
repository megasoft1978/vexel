//! Scripted-scene benchmark through the headless MCP rig, at 120 and 1024
//! elements (the scene cap). Each size adds N rects, then 50 opacity tracks x 8
//! keys, then a 50-track bulk pass on a fresh scene. Elements sit on a 32-column
//! grid of 15 px cells so all 1024 stay inside the 512 px canvas. Prints p50/p95 ms per tool
//! and per phase (`vexel_app::profile`; phases are inclusive of nested phases).
//! Phases `scene_scan` and `recompose` in the build table come from a read-only
//! probe after each add: `native_timeline_candidate` and `compose_document`.
//! Run: `cargo test -p vexel-app --release --test mcp_bench -- --ignored --nocapture`

mod common;

use common::{Dir, Rig, element_id, policy_for};
use std::collections::BTreeMap;
use std::time::Instant;
use vexel_app::compose::compose_document;
use vexel_app::profile;
use vexel_app::session::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

/// Requests per session. The session budget is 600 requests per minute.
const SESSION_REQUESTS: u32 = 500;
const TRACKS: usize = 50;
const KEYS: u32 = 8;
/// Phases print in pipeline order; any other phase prints last.
const PHASE_ORDER: &[&str] = &[
    "decode",
    "edit_apply",
    "element_id_scan",
    "journal_scene",
    "candidate_open",
    "scene_scan",
    "scene_copy",
    "copy_assets",
    "edit_context",
    "edit_validate",
    "edit_ops",
    "edit_copy",
    "candidate_reopen",
    "payload_encode",
    "journal_admit",
    "recompose",
];

type Samples = BTreeMap<String, Vec<f64>>;

fn timed(rig: &mut Rig, samples: &mut Samples, name: &str, extra: &str) -> common::Reply {
    let start = Instant::now();
    let reply = rig.doc_tool(name, extra);
    samples
        .entry(name.to_owned())
        .or_default()
        .push(start.elapsed().as_secs_f64() * 1000.0);
    reply.ok();
    reply
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let at = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[at]
}

fn row(name: &str, values: &mut [f64]) {
    values.sort_by(|a, b| a.total_cmp(b));
    println!(
        "{name:<36} {:>5} {:>9.3} {:>9.3}",
        values.len(),
        percentile(values, 0.5),
        percentile(values, 0.95)
    );
}

fn report(section: &str, samples: &mut Samples, mut phases: BTreeMap<&'static str, Vec<f64>>) {
    println!("\n[{section}]");
    println!("tool                                   n     p50 ms    p95 ms");
    for (name, values) in samples.iter_mut() {
        row(name, values);
    }
    println!("phase                                  n     p50 ms    p95 ms");
    let mut names: Vec<&'static str> = phases.keys().copied().collect();
    names.sort_by_key(|name| {
        PHASE_ORDER
            .iter()
            .position(|p| p == name)
            .unwrap_or(usize::MAX)
    });
    for name in names {
        if let Some(values) = phases.get_mut(name) {
            row(&format!("  {name}"), values);
        }
    }
    samples.clear();
}

fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 48,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Repeat,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    }
}

/// Read-only work the UI does after a source change: the scene-id scan (the
/// cache misses on the new source) and the recompose on the compose worker.
fn probe_after_add(app: &AppDocument) {
    let _ = app.native_timeline_candidate(settings(), EditLimits::default());
    let source = app.source_bytes().expect("source");
    let (width, height) = app.size();
    let _phase = profile::phase("recompose");
    compose_document(&source, width, height, &|| false).expect("compose");
}

/// The same document in a fresh session, so the request budget restarts.
fn renew(mut rig: Rig, dir: &Dir) -> Rig {
    let app = std::mem::replace(&mut rig.app, AppDocument::new(512, 512).unwrap());
    Rig::with_document(app, policy_for(&[&dir.0]))
}

fn run(elements: u32) {
    println!("\n== {elements} elements, {TRACKS} tracks x {KEYS} keys ==");
    profile::enable();
    let dir = Dir::new();
    let mut samples = Samples::new();
    let total = Instant::now();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    profile::take();
    let mut ids = Vec::new();
    for i in 0..elements {
        if i > 0 && i % SESSION_REQUESTS == 0 {
            rig = renew(rig, &dir);
        }
        // 32 columns of 15 px cells up to 1024 elements, 64 columns of 8 px
        // cells above, so every element stays inside the 512 px canvas.
        let (cols, cell, size) = if elements > 1024 {
            (64, 8, 6)
        } else {
            (32, 15, 20)
        };
        let attrs = format!(
            r##""parentId":"root","template":"rect","attributes":{{"x":"{}","y":"{}","width":"{size}","height":"{size}","fill":"#3366cc"}}"##,
            (i % cols) * cell,
            (i / cols) * cell
        );
        let reply = timed(&mut rig, &mut samples, "element_add", &attrs);
        ids.push(element_id(reply.ok()));
        probe_after_add(&rig.app);
    }
    report("build", &mut samples, profile::take());

    rig = renew(rig, &dir);
    for (n, id) in ids.iter().take(TRACKS).enumerate() {
        let track = format!("bench-{n}");
        timed(
            &mut rig,
            &mut samples,
            "native_timeline_add_property_track",
            &format!(
                r#""trackId":"{track}","targetId":"{id}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#
            ),
        );
        for k in 0..KEYS {
            timed(
                &mut rig,
                &mut samples,
                "native_timeline_set_property_key",
                &format!(
                    r#""trackId":"{track}","tick":{},"value":{}"#,
                    (k + 1) * 5,
                    if k % 2 == 0 { "0.25" } else { "1" }
                ),
            );
        }
    }
    report("native tracks", &mut samples, profile::take());

    // Bulk tracks on a fresh 50-rect scene, so the scene cap still holds.
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let ids: Vec<String> = (0..TRACKS as u32)
        .map(|n| rig.add_rect(n * 8, 0, "#336699"))
        .collect();
    profile::take();
    let keys: Vec<String> = (0..KEYS)
        .map(|k| {
            format!(
                r#"{{"tick":{},"value":{}}}"#,
                (k + 1) * 5,
                if k % 2 == 0 { "0.25" } else { "1" }
            )
        })
        .collect();
    for (n, id) in ids.iter().enumerate() {
        let start = Instant::now();
        rig.doc_tool(
            "native_timeline_add_property_track",
            &format!(
                r#""trackId":"bulk-{n}","targetId":"{id}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1,"keys":[{}]"#,
                keys.join(",")
            ),
        )
        .ok();
        samples
            .entry("add_property_track (8 keys, bulk)".to_owned())
            .or_default()
            .push(start.elapsed().as_secs_f64() * 1000.0);
    }
    report("bulk tracks", &mut samples, profile::take());
    println!("total wall {:.2} s", total.elapsed().as_secs_f64());
}

#[test]
#[ignore]
fn build_scripted_scene() {
    for elements in [120, 1024, 2048, 4096] {
        run(elements);
    }
}
