//! Track flush benchmark: 200 tracks / 3200 keys (16 trs keys each, as the
//! 7-number array form) written through `batch` (chunks of 40 ops) and through
//! one `native_timeline_tracks_set` call. Prints wall seconds and per-phase
//! totals. `VEXEL_BENCH_ONLY=batch|bulk` runs one of them.
//! Run: `cargo test -p vexel-app --release --test mcp_flush_bench -- --ignored --nocapture`

mod common;

use common::{Rig, no_grants};
use std::time::Instant;
use vexel_app::profile;

const DOC: &str = "preview-document";
const TRACKS: usize = 200;
const KEYS: usize = 15;

fn trs(i: usize, k: usize) -> String {
    format!("[{}.5,{}.25,{}.5,1.0,1.0,10,10]", i + k, k * 3, k)
}

fn scene() -> (Rig, Vec<String>) {
    let mut rig = Rig::new(no_grants());
    let ids = (0..TRACKS)
        .map(|i| rig.add_rect((i as u32 % 32) * 15, (i as u32 / 32) * 15, "#336699"))
        .collect();
    (rig, ids)
}

fn track_json(i: usize, id: &str) -> String {
    let keys: Vec<String> = (1..=KEYS)
        .map(|k| format!(r#"{{"tick":{},"value":{}}}"#, k * 3, trs(i, k)))
        .collect();
    format!(
        r#""trackId":"t{i}","targetId":"{id}","property":"transform","valueType":"trs","interpolation":"cubic","value":{},"keys":[{}]"#,
        trs(i, 0),
        keys.join(",")
    )
}

fn report(label: &str, secs: f64) {
    println!(
        "{label}: {secs:.3} s ({TRACKS} tracks, {} keys)",
        TRACKS * (KEYS + 1)
    );
    let phases = profile::take();
    let mut rows: Vec<_> = phases
        .iter()
        .map(|(n, v)| (*n, v.len(), v.iter().sum::<f64>()))
        .collect();
    rows.sort_by(|a, b| b.2.total_cmp(&a.2));
    for (name, n, total) in rows {
        println!("  {name:<18} n={n:<5} total {total:>9.1} ms");
    }
}

fn selected(name: &str) -> bool {
    std::env::var("VEXEL_BENCH_ONLY").map_or(true, |only| only == name)
}

#[test]
#[ignore]
fn flush_200_tracks_bulk() {
    if !selected("bulk") {
        return;
    }
    profile::enable();
    let (mut rig, ids) = scene();
    profile::take();
    let start = Instant::now();
    let tracks: Vec<String> = (0..TRACKS)
        .map(|i| format!("{{{}}}", track_json(i, &ids[i])))
        .collect();
    rig.doc_tool(
        "native_timeline_tracks_set",
        &format!(r#""tracks":[{}]"#, tracks.join(",")),
    )
    .ok();
    report(
        "bulk flush (native_timeline_tracks_set)",
        start.elapsed().as_secs_f64(),
    );
}

#[test]
#[ignore]
fn flush_200_tracks_batch() {
    if !selected("batch") {
        return;
    }
    profile::enable();
    let (mut rig, ids) = scene();
    profile::take();
    let start = Instant::now();
    for chunk in (0..TRACKS).collect::<Vec<_>>().chunks(40) {
        let ops: Vec<String> = chunk
            .iter()
            .map(|&i| {
                format!(
                    r#"{{"tool":"native_timeline_add_property_track","arguments":{{{}}}}}"#,
                    track_json(i, &ids[i])
                )
            })
            .collect();
        rig.tool(
            "batch",
            &format!(r#"{{"documentId":"{DOC}","ops":[{}]}}"#, ops.join(",")),
        )
        .ok();
    }
    report("batch flush", start.elapsed().as_secs_f64());
}
