//! Edit-latency bench on a real scene: `VEXEL_BENCH_SCENE=/path/scene.vexel`.
//! 100 `attribute_set` edits, each followed by a 900 px `compose_document`, alternating a leaf and a group; prints p50/p95 ms
//! and RSS before/after.
//! `cargo test -p vexel-app --release --test scene_edit_bench -- --ignored --nocapture`

mod common;

use common::{Dir, Rig, policy_for};
use std::path::Path;
use std::time::Instant;

fn rss_mib() -> f64 {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .unwrap_or(0.0)
        / 1024.0
}

fn ids(source: &str, tag: &str) -> Option<String> {
    let at = source.find(&format!("<{tag} "))?;
    let rest = &source[at..source[at..].find('>')? + at];
    let id = rest.find(" id=\"")? + 5;
    Some(rest[id..id + rest[id..].find('"')?].to_owned())
}

#[test]
#[ignore]
fn env_scene_edit_latency() {
    let Some(path) = std::env::var_os("VEXEL_BENCH_SCENE") else {
        eprintln!("skip: VEXEL_BENCH_SCENE not set");
        return;
    };
    let path = Path::new(&path);
    let dir = Dir::new();
    let parent = path.parent().unwrap();
    let mut rig = Rig::with_open(policy_for(&[parent, &dir.0]), path).unwrap();
    let source = String::from_utf8_lossy(&rig.state().source).into_owned();
    let leaf = ids(&source, "path").expect("a path with an id");
    let group = ids(&source, "g").unwrap_or_else(|| leaf.clone());
    let before = rss_mib();
    let mut ms = Vec::new();
    let edits: usize = std::env::var("EDITS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    for i in 0..edits {
        let id = if i % 2 == 0 { &leaf } else { &group };
        let value = if i % 4 < 2 { "0.5" } else { "0.6" };
        let args = format!(r#""elementId":"{id}","name":"opacity","value":"{value}""#);
        let t = Instant::now();
        rig.doc_tool("attribute_set", &args).ok();
        let src = rig.state().source;
        let composed = vexel_app::compose::compose_document(&src, 900, 900, &|| false);
        assert!(composed.is_ok());
        ms.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let after = rss_mib();
    ms.sort_by(|a, b| a.total_cmp(b));
    println!(
        "edit+compose x100 (leaf {leaf}, group {group}): p50 {:.2} p95 {:.2} ms | rss {before:.0} -> {after:.0} MiB",
        ms[ms.len() / 2],
        ms[ms.len() * 95 / 100]
    );
}
