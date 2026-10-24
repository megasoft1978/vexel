//! Parent process: writes scene files, runs each case in a child under
//! `/usr/bin/time -l` (peak RSS), and collects the child results.

use crate::child::{RESULT_PREFIX, name};
use crate::json::{self, Json};
use crate::matrix::{CaseId, Level, svg_path, vexel_path};
use crate::{anim, scene};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Opts {
    pub level: Level,
    pub trials: usize,
    pub warmups: usize,
    pub budget_s: f64,
    pub only: Option<String>,
}

impl Opts {
    pub fn new(level: Level) -> Self {
        let hash_only = level == Level::HashOnly;
        Self {
            level,
            trials: if hash_only { 1 } else { 15 },
            warmups: if hash_only { 0 } else { 2 },
            budget_s: 12.0,
            only: None,
        }
    }
}

/// Writes every scene file the selected cases read.
pub fn prepare_scenes(root: &Path, cases: &[CaseId]) -> Result<(), String> {
    let keys: BTreeSet<(usize, &str)> = cases.iter().map(|c| (c.n, c.profile.name())).collect();
    for (n, profile_name) in keys {
        let profile = scene::Profile::parse(profile_name).ok_or("unknown profile")?;
        let svg = svg_path(root, profile, n);
        std::fs::create_dir_all(svg.parent().ok_or("no scene dir")?).map_err(|e| e.to_string())?;
        std::fs::write(&svg, scene::generate(profile, n)).map_err(|e| e.to_string())?;
        let needs_anim = cases
            .iter()
            .any(|c| c.n == n && c.profile == profile && c.op.animated());
        let vexel = vexel_path(root, profile, n);
        if needs_anim && !vexel.exists() {
            eprintln!("building animated {profile_name}-{n}");
            anim::build_animated(&svg, &vexel, n)?;
        }
    }
    Ok(())
}

fn parse_rss(stderr: &str) -> Option<f64> {
    stderr
        .lines()
        .find(|l| l.contains("maximum resident set size"))
        .and_then(|l| l.split_whitespace().next())
        .and_then(|v| v.parse().ok())
}

fn run_child(root: &Path, exe: &Path, id: &CaseId, opts: &Opts) -> Result<Json, String> {
    let label = name(id.op, id.profile.name(), id.n);
    let hash_only = opts.level == Level::HashOnly;
    let tmp: PathBuf = root.join(".cache/bench/tmp").join(label.replace('/', "_"));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let input = if id.op.animated() {
        vexel_path(root, id.profile, id.n)
    } else {
        svg_path(root, id.profile, id.n)
    };
    let mut cmd = if hash_only {
        Command::new(exe)
    } else {
        let mut c = Command::new("/usr/bin/time");
        c.arg("-l").arg(exe);
        c
    };
    cmd.args(["case", &id.op.name(), id.profile.name(), &id.n.to_string()])
        .arg("--input")
        .arg(&input)
        .arg("--tmp")
        .arg(&tmp)
        .args(["--trials", &opts.trials.to_string()])
        .args(["--warmups", &opts.warmups.to_string()])
        .args(["--budget", &opts.budget_s.to_string()]);
    if hash_only {
        cmd.arg("--no-diag");
    }
    let out = cmd.output().map_err(|e| format!("spawn: {e}"));
    let _ = std::fs::remove_dir_all(&tmp);
    let out = out?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let line = stdout
        .lines()
        .find_map(|l| l.strip_prefix(RESULT_PREFIX))
        .ok_or_else(|| {
            let tail: Vec<&str> = stderr.lines().rev().take(4).collect();
            format!("no result ({}): {}", out.status, tail.join(" | "))
        })?;
    let mut result = json::parse(line)?;
    if let (Json::Obj(pairs), Some(rss)) = (&mut result, parse_rss(&stderr)) {
        pairs.push(("rss_bytes".into(), Json::Num(rss)));
    }
    Ok(result)
}

/// Runs all cases; a failing case is recorded with an `error`, not dropped.
pub fn run_cases(root: &Path, exe: &Path, cases: &[CaseId], opts: &Opts) -> Vec<Json> {
    let mut results = Vec::new();
    for (i, id) in cases.iter().enumerate() {
        let label = name(id.op, id.profile.name(), id.n);
        let result = run_child(root, exe, id, opts).unwrap_or_else(|error| {
            Json::obj(vec![
                ("name", Json::str(&label)),
                ("error", Json::str(error)),
            ])
        });
        let status = match (
            result.get("error"),
            result.get("skipped"),
            result.get("p50_ms"),
        ) {
            (Some(e), _, _) => format!("ERROR {}", e.text().unwrap_or("")),
            (_, Some(_), _) => "skipped".to_owned(),
            (_, _, Some(p)) => format!("p50 {:.2} ms", p.num().unwrap_or(0.0)),
            _ => String::new(),
        };
        eprintln!("[{}/{}] {label}: {status}", i + 1, cases.len());
        results.push(result);
    }
    results
}
