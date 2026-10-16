//! Argument parsing and subcommand dispatch.

use crate::run::{self, Config};
use crate::verdict::Verdict;
use crate::{compare, report, runner};
use std::path::{Path, PathBuf};

const USAGE: &str = "usage:\n  vexel-corpus run --manifest <path> --pins <path> --out <dir> [--size 128x128] \
[--max-bytes 33554432] [--timeout-secs 60] [--rss-limit-mib 2048] [--only <id>...]\n  \
vexel-corpus compare --baseline [label=]<json>... --current <json> [--append <report.md>]";

fn num<T: std::str::FromStr>(flag: &str, v: Option<&String>) -> Result<T, String> {
    v.ok_or(format!("{flag} needs a value"))?
        .parse()
        .map_err(|_| format!("{flag}: invalid number"))
}

fn parse_size(text: &str) -> Result<(u32, u32), String> {
    let (w, h) = text.split_once('x').ok_or("--size must be WxH")?;
    match (w.parse::<u32>(), h.parse::<u32>()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => Ok((w, h)),
        _ => Err("--size must be WxH with positive integers".to_string()),
    }
}

fn parse_run(args: &[String]) -> Result<Config, String> {
    let (mut manifest, mut pins, mut out) = (None, None, None);
    let mut cfg = Config {
        manifest: PathBuf::new(),
        pins: PathBuf::new(),
        out: PathBuf::new(),
        width: 128,
        height: 128,
        max_bytes: 33_554_432,
        timeout_secs: 60,
        rss_limit_mib: 2048,
        only: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--manifest" => manifest = it.next().map(PathBuf::from),
            "--pins" => pins = it.next().map(PathBuf::from),
            "--out" => out = it.next().map(PathBuf::from),
            "--size" => {
                (cfg.width, cfg.height) = parse_size(it.next().ok_or("--size needs a value")?)?
            }
            "--max-bytes" => cfg.max_bytes = num(flag, it.next())?,
            "--timeout-secs" => cfg.timeout_secs = num(flag, it.next())?,
            "--rss-limit-mib" => cfg.rss_limit_mib = num(flag, it.next())?,
            "--only" => {
                while let Some(id) = it.clone().next().filter(|a| !a.starts_with("--")) {
                    cfg.only.push(id.clone());
                    it.next();
                }
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    cfg.manifest = manifest.ok_or("--manifest is required")?;
    cfg.pins = pins.ok_or("--pins is required")?;
    cfg.out = out.ok_or("--out is required")?;
    Ok(cfg)
}

fn cmd_run(args: &[String]) -> Result<i32, String> {
    let cfg = parse_run(args)?;
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let (manifest, results) = run::run_all(&exe, &cfg)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let meta = report::Meta {
        git_sha: report::tool_git_sha(&root),
        manifest_sha: manifest.sha256.clone(),
        manifest_label: report::display_path(&cfg.manifest),
        host: report::host_string(),
    };
    report::write_reports(&cfg, &meta, &results)?;
    let bad = results.iter().any(|r| r.verdict == Verdict::GuardFailed);
    for (v, n) in report::histogram(&results) {
        eprintln!("{:<13} {n}", v.name());
    }
    Ok(if bad { 3 } else { 0 })
}

fn cmd_compare(args: &[String]) -> Result<i32, String> {
    let (mut baselines, mut current, mut append) = (Vec::new(), None, None);
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let value = it.next().ok_or(format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--baseline" => baselines.push(value.clone()),
            "--current" => current = Some(value.clone()),
            "--append" => append = Some(PathBuf::from(value)),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let current = current.ok_or("--current is required")?;
    if baselines.is_empty() {
        return Err("at least one --baseline is required".to_string());
    }
    let load = |spec: &str| {
        let (label, path) = spec.split_once('=').unwrap_or((spec, spec));
        compare::load(label, Path::new(path))
    };
    let loaded = baselines
        .iter()
        .map(|b| load(b))
        .collect::<Result<Vec<_>, _>>()?;
    let md = compare::markdown(&loaded, &load(&format!("current Rust run={current}"))?);
    match append {
        Some(path) => {
            let mut text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            if let Some(i) = text.find("## Comparison with the legacy C baselines") {
                text.truncate(i);
            }
            text.push_str(&md);
            std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        None => print!("{md}"),
    }
    Ok(0)
}

/// Entry point shared by the binary; returns the process exit code.
pub fn main_with(args: &[String]) -> i32 {
    let result = match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("compare") => cmd_compare(&args[1..]),
        Some("--one") if args.len() == 4 && args[2] == "--size" => {
            match (Path::new(&args[1]), parse_size(&args[3])) {
                (p, Ok((w, h))) => Ok(runner::run_one(p, w, h)),
                (_, Err(e)) => Err(e),
            }
        }
        Some("--sleep") if args.len() == 2 => Ok(runner::run_sleep(args[1].parse().unwrap_or(1))),
        _ => Err(USAGE.to_string()),
    };
    result.unwrap_or_else(|e| {
        eprintln!("vexel-corpus: {e}");
        2
    })
}
