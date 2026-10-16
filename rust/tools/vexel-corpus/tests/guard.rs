use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};
use vexel_corpus::manifest::sha256_file;
use vexel_corpus::runner::run_isolated;
use vexel_corpus::verdict::Verdict;

const EXE: &str = env!("CARGO_BIN_EXE_vexel-corpus");
const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"><rect width="5" height="5" fill="red"/></svg>"#;

struct Fixture {
    root: PathBuf,
}

/// Builds `<root>/corpus/{manifest.json, a.svg, ...}` and `<root>/pins.json`.
fn fixture(name: &str, files: &[(&str, &[u8])], wrong_hash: Option<&str>) -> Fixture {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    let corpus = root.join("corpus");
    fs::create_dir_all(&corpus).unwrap();
    let mut entries = Vec::new();
    for (file, bytes) in files {
        fs::write(corpus.join(file), bytes).unwrap();
        let hash = if Some(*file) == wrong_hash {
            "0".repeat(64)
        } else {
            sha256_file(&corpus.join(file)).unwrap()
        };
        entries.push(format!(
            r#"{{"id":"{file}","tier":"T1","path":"{file}","bytes":{},"sha256":"{hash}","kind":"test"}}"#,
            bytes.len()
        ));
    }
    fs::write(
        corpus.join("manifest.json"),
        format!("[{}]", entries.join(",")),
    )
    .unwrap();
    let pin = sha256_file(&corpus.join("manifest.json")).unwrap();
    fs::write(
        root.join("pins.json"),
        format!(r#"{{"pins":[{{"sha256":"{pin}"}}]}}"#),
    )
    .unwrap();
    Fixture { root }
}

fn run(f: &Fixture, extra: &[&str]) -> Output {
    Command::new(EXE)
        .args(["run", "--manifest"])
        .arg(f.root.join("corpus/manifest.json"))
        .arg("--pins")
        .arg(f.root.join("pins.json"))
        .arg("--out")
        .arg(f.root.join("out"))
        .args(extra)
        .output()
        .unwrap()
}

fn report(f: &Fixture) -> String {
    fs::read_to_string(f.root.join("out/report.json")).unwrap()
}

fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push((p.clone(), fs::read(&p).unwrap()));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn altered_manifest_byte_is_refused() {
    let f = fixture("guard_manifest", &[("a.svg", SVG)], None);
    let path = f.root.join("corpus/manifest.json");
    let mut bytes = fs::read(&path).unwrap();
    bytes[2] = b'X';
    fs::write(&path, bytes).unwrap();
    let out = run(&f, &[]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("refusing to run"));
    assert!(
        !f.root.join("out").exists(),
        "pipeline must not run or report"
    );
}

#[test]
fn file_hash_mismatch_is_guard_failed() {
    let f = fixture(
        "guard_file",
        &[("a.svg", SVG), ("b.svg", SVG)],
        Some("a.svg"),
    );
    let out = run(&f, &[]);
    assert_eq!(out.status.code(), Some(3));
    let json = report(&f);
    let want = format!(
        r#""id": "a.svg", "tier": "T1", "kind": "test", "bytes": {}, "verdict": "guard_failed""#,
        SVG.len()
    );
    assert!(json.contains(&want), "{json}");
    assert!(json.contains(r#""id": "b.svg""#) && json.contains(r#""guard_failed": 1"#));
    assert!(json.contains(r#""ok": 1"#));
}

#[test]
fn writes_only_under_out() {
    let f = fixture("guard_writes", &[("a.svg", SVG)], None);
    let before = snapshot(&f.root);
    let out = run(&f, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let after = snapshot(&f.root);
    let outside = |s: &[(PathBuf, Vec<u8>)]| {
        s.iter()
            .filter(|(p, _)| !p.starts_with(f.root.join("out")))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(outside(&before), outside(&after));
    let new: Vec<_> = after
        .iter()
        .filter(|(p, _)| p.starts_with(f.root.join("out")))
        .collect();
    assert_eq!(new.len(), 2, "report.json and report.md only");
    assert!(report(&f).contains(r#""verdict": "ok""#));
}

#[test]
fn rss_limit_reports_rss_exceeded() {
    let f = fixture("guard_rss", &[("a.svg", SVG)], None);
    let out = run(&f, &["--rss-limit-mib", "1"]);
    assert!(out.status.success());
    let json = report(&f);
    assert!(json.contains(r#""verdict": "rss_exceeded""#), "{json}");
    assert!(json.contains(r#""rss_exceeded": 1"#));
}

#[test]
fn synthetic_verdicts_through_the_pipeline() {
    let f = fixture(
        "guard_verdicts",
        &[
            ("ok.svg", SVG),
            ("bad.svg", b"<svg"),
            ("utf8.svg", &[0xff, 0xfe, b'<']),
            ("big.svg", SVG),
        ],
        None,
    );
    // ok.svg and big.svg exceed the threshold and are skipped; the others fit.
    let out = run(&f, &["--max-bytes", &(SVG.len() - 1).to_string()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = report(&f);
    assert!(json.contains(r#""skipped": 2"#), "{json}");
    assert!(json.contains(r#""xml": 2"#), "{json}");
    assert!(json.contains(r#""crash": 0"#) && json.contains(r#""guard_failed": 0"#));
}

#[test]
fn timeout_kills_the_process_group() {
    let start = Instant::now();
    let outcome = run_isolated(
        Path::new(EXE),
        &["--sleep".to_string(), "30".to_string()],
        Duration::from_secs(2),
        2048,
    );
    assert_eq!(outcome.verdict, Verdict::Timeout, "{outcome:?}");
    assert!(
        start.elapsed() < Duration::from_secs(4),
        "took {:?}",
        start.elapsed()
    );
}

#[test]
fn non_zero_exit_is_crash_with_status() {
    let outcome = run_isolated(
        Path::new("/usr/bin/false"),
        &[],
        Duration::from_secs(5),
        2048,
    );
    assert_eq!(outcome.verdict, Verdict::Crash);
    assert!(
        outcome.detail.contains("exit status 1"),
        "{}",
        outcome.detail
    );
}
