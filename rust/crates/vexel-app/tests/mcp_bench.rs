//! Scripted-scene benchmark: 120 elements (scene cap 128) and 50 tracks x 8 keys through the
//! headless MCP rig. Prints p50/p95 milliseconds per tool.
//! Run: `cargo test -p vexel-app --release --test mcp_bench -- --ignored --nocapture`

mod common;

use common::{Dir, Rig, element_id, policy_for};
use std::collections::BTreeMap;
use std::time::Instant;

/// The live preview scene cap is 128 elements, so the benchmark stays below it.
const ELEMENTS: u32 = 120;

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

#[test]
#[ignore]
fn build_scripted_scene() {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let mut samples = Samples::new();
    let total = Instant::now();
    let mut ids = Vec::new();
    for i in 0..ELEMENTS {
        let attrs = format!(
            r##""parentId":"root","template":"rect","attributes":{{"x":"{}","y":"{}","width":"20","height":"20","fill":"#3366cc"}}"##,
            (i % 12) * 40,
            (i / 12) * 40
        );
        let reply = timed(&mut rig, &mut samples, "element_add", &attrs);
        ids.push(element_id(reply.ok()));
    }
    for (n, id) in ids.iter().take(50).enumerate() {
        let track = format!("bench-{n}");
        timed(
            &mut rig,
            &mut samples,
            "native_timeline_add_property_track",
            &format!(
                r#""trackId":"{track}","targetId":"{id}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#
            ),
        );
        for k in 0..8u32 {
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
    let wall = total.elapsed().as_secs_f64();
    println!("tool                                   n    p50 ms    p95 ms");
    for (name, values) in &mut samples {
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "{name:<36} {:>4} {:>9.2} {:>9.2}",
            values.len(),
            percentile(values, 0.5),
            percentile(values, 0.95)
        );
    }
    println!("total wall {wall:.2} s");
}
