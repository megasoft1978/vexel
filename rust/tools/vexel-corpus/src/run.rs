//! The `run` driver: guard each file, skip oversize ones, isolate the rest.

use crate::manifest::{self, Entry, Manifest};
use crate::runner::{self, Outcome};
use crate::verdict::{self, Verdict};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub manifest: PathBuf,
    pub pins: PathBuf,
    pub out: PathBuf,
    pub width: u32,
    pub height: u32,
    pub max_bytes: u64,
    pub timeout_secs: u64,
    pub rss_limit_mib: u64,
    pub only: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileResult {
    pub id: String,
    pub tier: String,
    pub kind: String,
    pub bytes: u64,
    pub verdict: Verdict,
    pub detail: String,
    pub wall_ms: u64,
    pub peak_rss_bytes: Option<u64>,
}

fn guarded(entry: &Entry, max_bytes: u64) -> Result<u64, (Verdict, String, u64)> {
    let len = match std::fs::metadata(&entry.path) {
        Ok(m) => m.len(),
        Err(e) => return Err((Verdict::GuardFailed, format!("cannot stat file: {e}"), 0)),
    };
    if let Some(want) = entry.bytes
        && want != len
    {
        let d = format!("size {len} differs from manifest {want}");
        return Err((Verdict::GuardFailed, d, len));
    }
    if let Some((v, d)) = verdict::skip_for_size(len, max_bytes) {
        return Err((v, d, len));
    }
    match manifest::sha256_file(&entry.path) {
        Ok(h) if h == entry.sha256 => Ok(len),
        Ok(h) => Err((
            Verdict::GuardFailed,
            format!("sha256 {h} differs from manifest {}", entry.sha256),
            len,
        )),
        Err(e) => Err((Verdict::GuardFailed, e, len)),
    }
}

pub fn run_entry(exe: &Path, cfg: &Config, entry: &Entry) -> FileResult {
    let mk = |verdict, detail, bytes, wall_ms, peak| FileResult {
        id: entry.id.clone(),
        tier: entry.tier.clone(),
        kind: entry.kind.clone(),
        bytes,
        verdict,
        detail,
        wall_ms,
        peak_rss_bytes: peak,
    };
    match guarded(entry, cfg.max_bytes) {
        Err((v, d, bytes)) => mk(v, d, bytes, 0, None),
        Ok(bytes) => {
            let args = vec![
                "--one".to_string(),
                entry.path.display().to_string(),
                "--size".to_string(),
                format!("{}x{}", cfg.width, cfg.height),
            ];
            let Outcome {
                verdict,
                detail,
                wall_ms,
                peak_rss_bytes,
            } = runner::run_isolated(
                exe,
                &args,
                Duration::from_secs(cfg.timeout_secs),
                cfg.rss_limit_mib,
            );
            mk(verdict, detail, bytes, wall_ms, peak_rss_bytes)
        }
    }
}

/// Loads the pinned manifest and runs every selected entry in manifest order.
pub fn run_all(exe: &Path, cfg: &Config) -> Result<(Manifest, Vec<FileResult>), String> {
    let pins = manifest::load_pins(&cfg.pins)?;
    let manifest = manifest::load_manifest(&cfg.manifest, &pins)?;
    for id in &cfg.only {
        if !manifest.entries.iter().any(|e| &e.id == id) {
            return Err(format!("--only id {id:?} is not in the manifest"));
        }
    }
    let mut results = Vec::new();
    for entry in &manifest.entries {
        if !cfg.only.is_empty() && !cfg.only.contains(&entry.id) {
            continue;
        }
        let r = run_entry(exe, cfg, entry);
        eprintln!("{:<40} {:<12} {} ms", r.id, r.verdict.name(), r.wall_ms);
        results.push(r);
    }
    Ok((manifest, results))
}
