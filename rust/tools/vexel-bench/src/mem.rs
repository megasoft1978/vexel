//! Memory mode: per-scenario child processes report peak live heap (counting
//! allocator), live 1 MiB+ buffers and peak RSS (`/usr/bin/time -l`), plus a
//! content hash. `mem --check` gates hashes (exact) and peak heap (baselines
//! only ratchet down). See `docs/PERF.md`.

use crate::json::{self, Json};
use crate::ops::{Failure, compose_bytes, frame_hash, open};
use crate::{alloc, anim, scene, sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use vexel_app::animation::playback::ManualClock;
use vexel_app::canvas::{artboard_pixels, target_compose_size};
use vexel_app::native_animation::NativeAnimationHost;
use vexel_app::preview::Control;
use vexel_app::session::AppDocument;
use vexel_v1_document::TransformSpec;

pub const RESULT_PREFIX: &str = "MEM_RESULT ";
const MIB: f64 = 1024.0 * 1024.0;
/// Peak-heap growth tolerated by the gate before it fails.
const HEAP_SLACK: f64 = 1.03;

/// Every scenario name; `run_scenario` implements each.
pub fn scenarios() -> Vec<String> {
    let mut out = Vec::new();
    for n in [1000, 5000, 20000] {
        for size in [512, 900, 2048] {
            out.push(format!("open_render/{size}/{n}"));
        }
    }
    for zoom in [100, 400, 1600] {
        out.push(format!("zoom/{zoom}"));
    }
    out.extend(
        [
            "layers/nested50",
            "layers/opac1000",
            "journal/200",
            "leak/edit_undo500",
            "play/ticks2000",
            "export/apng",
        ]
        .map(String::from),
    );
    out
}

struct Phases(Vec<(String, alloc::Heap)>);

impl Phases {
    fn run<T>(&mut self, name: &str, f: impl FnOnce() -> T) -> T {
        alloc::reset_peak();
        let out = f();
        self.0.push((name.to_owned(), alloc::heap()));
        out
    }
    fn mark(&mut self, name: &str) {
        self.0.push((name.to_owned(), alloc::heap()));
    }
}

fn tmp_dir() -> Result<PathBuf, Failure> {
    let dir = std::env::temp_dir().join(format!("vexel-mem-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn write_scene(dir: &Path, svg: &str) -> Result<PathBuf, Failure> {
    let path = dir.join("scene.svg");
    std::fs::write(&path, svg)?;
    Ok(path)
}

fn profile(name: &str) -> Result<scene::Profile, Failure> {
    scene::Profile::parse(name).ok_or_else(|| Failure::Fail(format!("profile {name}")))
}

fn nested(depth: usize) -> String {
    let mut s = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\">",
    );
    for _ in 0..depth {
        s.push_str("<g opacity=\"0.9\">");
    }
    s.push_str("<rect x=\"40\" y=\"40\" width=\"300\" height=\"300\" fill=\"#3366cc\"/>");
    for _ in 0..depth {
        s.push_str("</g>");
    }
    s.push_str("</svg>");
    s
}

fn edit_step(doc: &mut AppDocument, i: usize, ids: usize) -> Result<(), Failure> {
    let id = format!("s{}", (i * 7) % ids.max(1));
    let spec = TransformSpec {
        translate: ((i % 13) as f64 + 1.0, -((i % 7) as f64) - 1.0),
        ..TransformSpec::default()
    };
    doc.edit(|d| d.set_transform(&id, &spec))?;
    Ok(())
}

/// Runs one scenario in this process; returns phases and a content hash.
fn run_scenario(name: &str, ph: &mut Phases) -> Result<String, Failure> {
    let dir = tmp_dir()?;
    let parts: Vec<&str> = name.split('/').collect();
    let result = match parts.as_slice() {
        ["open_render", size, n] => {
            let (size, n): (u32, usize) = (size.parse()?, n.parse()?);
            let path = write_scene(&dir, &scene::generate(profile("mix")?, n))?;
            alloc::reset_peak();
            let doc = ph.run("open", || open(&path))?;
            let bytes = doc.source_bytes()?;
            let frame = ph.run("render", || compose_bytes(&bytes, size, size))?;
            Ok(frame_hash(&frame))
        }
        ["zoom", pct] => {
            let zoom = pct.parse::<f64>()? / 100.0;
            let path = write_scene(&dir, &scene::generate(profile("fill")?, 1000))?;
            let doc = open(&path)?;
            let bytes = doc.source_bytes()?;
            let (w, h) = doc.size();
            let natural = (f64::from(w), f64::from(h));
            let first = artboard_pixels(natural, 0.5, 2);
            alloc::reset_peak();
            let cached = ph.run("cached", || compose_bytes(&bytes, first.0, first.1))?;
            let target = target_compose_size(natural, zoom, 2, Some(first));
            // The app keeps the cached frame while the next one composes.
            let next = ph.run("recompose", || compose_bytes(&bytes, target.0, target.1))?;
            drop(cached);
            ph.mark("swap");
            Ok(frame_hash(&next))
        }
        ["layers", kind] => {
            let svg = if *kind == "nested50" {
                nested(50)
            } else {
                scene::generate(profile("opac")?, 1000)
            };
            let size = if *kind == "nested50" { 512 } else { 2048 };
            let bytes = svg.into_bytes();
            alloc::reset_peak();
            let frame = ph.run("render", || compose_bytes(&bytes, size, size))?;
            Ok(frame_hash(&frame))
        }
        ["journal", edits] => {
            let edits: usize = edits.parse()?;
            let mut n = 1000;
            let svg = loop {
                let svg = scene::generate(profile("mix")?, n);
                if svg.len() >= 512 * 1024 {
                    break svg;
                }
                n *= 2;
            };
            let path = write_scene(&dir, &svg)?;
            let mut doc = open(&path)?;
            ph.mark("opened");
            for i in 0..edits {
                edit_step(&mut doc, i, n)?;
                if (i + 1) % 50 == 0 {
                    ph.mark(&format!("edit{}", i + 1));
                }
            }
            let edited = sha256::hex(&doc.source_bytes()?);
            while doc.undo()? {}
            ph.mark("undone");
            Ok(format!("{edited}:{}", sha256::hex(&doc.source_bytes()?)))
        }
        ["leak", _] => {
            let path = write_scene(&dir, &scene::generate(profile("mix")?, 500))?;
            let mut doc = open(&path)?;
            let mut log = sha256::Sha256::default();
            for i in 0..500 {
                edit_step(&mut doc, i, 500)?;
                log.update(&doc.source_bytes()?);
                doc.undo()?;
                if i % 100 == 99 {
                    ph.mark(&format!("cycle{}", i + 1));
                }
            }
            log.update(&doc.source_bytes()?);
            Ok(log.finish_hex())
        }
        ["play", _] => {
            let svg = write_scene(&dir, &scene::generate(profile("mix")?, 500))?;
            let vexel = dir.join("scene.vexel");
            anim::build_animated(&svg, &vexel, 500)?;
            let doc = open(&vexel)?;
            let mut host = NativeAnimationHost::new(ManualClock::default());
            host.snapshot(&doc, None)
                .map_err(|e| Failure::Fail(format!("snapshot: {e:?}")))?;
            ph.mark("snapshot");
            let mut log = sha256::Sha256::default();
            for k in 0..2000u32 {
                let tick = k % anim::TICKS;
                host.mcp_control(
                    &doc,
                    Control::Seek(f64::from(tick) / f64::from(anim::TPS)),
                    true,
                )
                .map_err(|e| Failure::Fail(format!("seek: {e}")))?;
                let frame = host
                    .tick(&doc, (512, 512))
                    .ok_or_else(|| Failure::Fail("no frame".into()))?;
                if k % 97 == 0 {
                    log.update(frame_hash(&frame).as_bytes());
                }
                if k == 99 || k == 499 || k == 999 || k == 1999 {
                    ph.mark(&format!("tick{}", k + 1));
                }
            }
            Ok(log.finish_hex())
        }
        ["export", "apng"] => {
            let svg = write_scene(&dir, &scene::generate(profile("mix")?, 100))?;
            let vexel = dir.join("scene.vexel");
            anim::build_animated(&svg, &vexel, 100)?;
            let mut doc = open(&vexel)?;
            let out = dir.join("out.apng");
            alloc::reset_peak();
            ph.run("export", || {
                vexel_app::frames_export::write_apng(&mut doc, out.clone())
            })
            .map_err(|e| Failure::Fail(format!("export: {e}")))?;
            Ok(sha256::hex(&std::fs::read(&out)?))
        }
        _ => Err(Failure::Fail(format!("unknown scenario {name}"))),
    };
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn mib(bytes: i64) -> f64 {
    bytes as f64 / MIB
}

/// Child entry: runs the scenario and prints one result line.
pub fn case_command(name: &str) -> Result<(), String> {
    alloc::track_start();
    let mut ph = Phases(Vec::new());
    let base = alloc::heap().live;
    let outcome = run_scenario(name, &mut ph);
    let result = match outcome {
        Ok(hash) => {
            let peak = ph.0.iter().map(|(_, h)| h.peak).max().unwrap_or(0);
            let big = ph.0.iter().map(|(_, h)| h.big_peak).max().unwrap_or(0);
            let phases =
                ph.0.iter()
                    .map(|(n, h)| {
                        Json::obj(vec![
                            ("phase", Json::str(n)),
                            ("peak_mb", Json::Num(mib(h.peak - base))),
                            ("live_mb", Json::Num(mib(h.live - base))),
                            ("big_live", Json::Num(h.big_live as f64)),
                        ])
                    })
                    .collect();
            Json::obj(vec![
                ("name", Json::str(name)),
                ("hash", Json::str(hash)),
                ("peak_heap_bytes", Json::Num((peak - base) as f64)),
                ("big_peak", Json::Num(big as f64)),
                ("phases", Json::Arr(phases)),
            ])
        }
        Err(Failure::Skip(why)) => {
            Json::obj(vec![("name", Json::str(name)), ("skipped", Json::str(why))])
        }
        Err(Failure::Fail(why)) => {
            Json::obj(vec![("name", Json::str(name)), ("error", Json::str(why))])
        }
    };
    println!("{RESULT_PREFIX}{}", result.to_compact());
    Ok(())
}

fn parse_rss(stderr: &str) -> Option<f64> {
    stderr
        .lines()
        .find(|l| l.contains("maximum resident set size"))
        .and_then(|l| l.split_whitespace().next())
        .and_then(|v| v.parse().ok())
}

fn run_child(exe: &Path, name: &str) -> Json {
    let out = Command::new("/usr/bin/time")
        .arg("-l")
        .arg(exe)
        .args(["mem-case", name])
        .output();
    let fail = |why: String| Json::obj(vec![("name", Json::str(name)), ("error", Json::str(why))]);
    let Ok(out) = out else {
        return fail("spawn failed".into());
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let Some(line) = stdout.lines().find_map(|l| l.strip_prefix(RESULT_PREFIX)) else {
        let tail: Vec<&str> = stderr.lines().rev().take(3).collect();
        return fail(format!("no result ({}): {}", out.status, tail.join(" | ")));
    };
    match json::parse(line) {
        Ok(Json::Obj(mut pairs)) => {
            if let Some(rss) = parse_rss(&stderr) {
                pairs.push(("rss_bytes".into(), Json::Num(rss)));
            }
            Json::Obj(pairs)
        }
        Ok(other) => other,
        Err(e) => fail(e),
    }
}

fn num(r: &Json, key: &str) -> f64 {
    r.get(key).and_then(Json::num).unwrap_or(0.0)
}

/// Markdown table: scenario, peak RSS MB, peak heap MB, live big buffers.
pub fn table(results: &[Json]) -> String {
    let mut s = String::from(
        "| scenario | peak RSS MB | peak heap MB | peak 1MiB+ buffers | note |\n|---|---|---|---|---|\n",
    );
    for r in results {
        let name = r.get("name").and_then(Json::text).unwrap_or("?");
        let note = r
            .get("error")
            .or_else(|| r.get("skipped"))
            .and_then(Json::text)
            .unwrap_or("");
        s.push_str(&format!(
            "| {name} | {:.1} | {:.1} | {} | {note} |\n",
            num(r, "rss_bytes") / MIB,
            num(r, "peak_heap_bytes") / MIB,
            num(r, "big_peak") as u64
        ));
    }
    s
}

fn phase_lines(results: &[Json]) -> String {
    let mut s = String::new();
    for r in results {
        let name = r.get("name").and_then(Json::text).unwrap_or("?");
        let Some(Json::Arr(items)) = r.get("phases") else {
            continue;
        };
        let parts: Vec<String> = items
            .iter()
            .map(|p| {
                format!(
                    "{} peak {:.1}/live {:.1}/big {}",
                    p.get("phase").and_then(Json::text).unwrap_or("?"),
                    num(p, "peak_mb"),
                    num(p, "live_mb"),
                    num(p, "big_live") as u64
                )
            })
            .collect();
        s.push_str(&format!("- {name}: {}\n", parts.join("; ")));
    }
    s
}

/// `mem [--check] [--update-baseline] [--only TEXT] [--out FILE]`.
pub fn command(
    root: &Path,
    check: bool,
    update: bool,
    only: Option<&str>,
    out: Option<PathBuf>,
) -> Result<bool, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let names: Vec<String> = scenarios()
        .into_iter()
        .filter(|n| only.is_none_or(|f| n.contains(f)))
        .collect();
    if names.is_empty() {
        return Err("no scenarios selected".into());
    }
    let mut results = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let r = run_child(&exe, name);
        eprintln!(
            "[{}/{}] {name}: heap {:.1} MB rss {:.1} MB",
            i + 1,
            names.len(),
            num(&r, "peak_heap_bytes") / MIB,
            num(&r, "rss_bytes") / MIB
        );
        results.push(r);
    }
    let report = format!("{}\n{}", table(&results), phase_lines(&results));
    println!("{report}");
    if let Some(path) = out {
        std::fs::write(&path, &report).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let baseline_path = root.join("bench/mem_baseline.json");
    let old = std::fs::read_to_string(&baseline_path)
        .ok()
        .and_then(|t| json::parse(&t).ok());
    let mut ok = results.iter().all(|r| r.get("error").is_none());
    if update {
        let mut pairs: Vec<(String, Json)> =
            old.as_ref().map(|o| o.pairs().to_vec()).unwrap_or_default();
        for r in results.iter().filter(|r| r.get("hash").is_some()) {
            let name = r.get("name").and_then(Json::text).unwrap_or("?").to_owned();
            let new_heap = num(r, "peak_heap_bytes");
            let prior = old.as_ref().and_then(|o| o.get(&name));
            let keep = prior
                .map(|p| num(p, "peak_heap_bytes"))
                .filter(|h| *h < new_heap);
            let entry = Json::obj(vec![
                ("hash", r.get("hash").cloned().unwrap_or(Json::Null)),
                ("peak_heap_bytes", Json::Num(keep.unwrap_or(new_heap))),
            ]);
            pairs.retain(|(k, _)| *k != name);
            pairs.push((name, entry));
        }
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        std::fs::write(&baseline_path, Json::Obj(pairs).to_pretty()).map_err(|e| e.to_string())?;
        println!(
            "mem baseline updated (only ratchets down): {}",
            baseline_path.display()
        );
        return Ok(ok);
    }
    if check {
        let old = old
            .ok_or_else(|| format!("{} missing; use --update-baseline", baseline_path.display()))?;
        for r in &results {
            let name = r.get("name").and_then(Json::text).unwrap_or("?");
            let Some(base) = old.get(name) else {
                println!("note: {name} has no memory baseline");
                continue;
            };
            if r.get("hash").and_then(Json::text) != base.get("hash").and_then(Json::text) {
                println!("FAIL: {name}: hash differs from baseline");
                ok = false;
            }
            let (heap, limit) = (
                num(r, "peak_heap_bytes"),
                num(base, "peak_heap_bytes") * HEAP_SLACK,
            );
            if heap > limit {
                println!(
                    "FAIL: {name}: peak heap {:.1} MB over baseline limit {:.1} MB",
                    heap / MIB,
                    limit / MIB
                );
                ok = false;
            }
        }
        println!("mem check: {}", if ok { "pass" } else { "FAIL" });
    }
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_names_are_unique() {
        let names = scenarios();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len());
    }

    #[test]
    fn nested_scene_has_matching_tags() {
        let s = nested(3);
        assert_eq!(s.matches("<g ").count(), s.matches("</g>").count());
    }
}
