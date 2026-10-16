//! `report.json` / `report.md` writers. No timestamps in the body.

use crate::jsonx::quote;
use crate::run::{Config, FileResult};
use crate::verdict::Verdict;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Meta {
    pub git_sha: String,
    pub manifest_sha: String,
    pub manifest_label: String,
    pub host: String,
}

/// HEAD commit read from the `.git` files (no git invocation). The working
/// tree may be dirty; this names the base commit only.
pub fn tool_git_sha(root: &Path) -> String {
    fn read(p: &Path) -> Option<String> {
        std::fs::read_to_string(p)
            .ok()
            .map(|s| s.trim().to_string())
    }
    let dot = root.join(".git");
    let gitdir =
        match read(&dot).and_then(|s| s.strip_prefix("gitdir:").map(|g| g.trim().to_string())) {
            Some(g) => PathBuf::from(g),
            None => dot,
        };
    let common = read(&gitdir.join("commondir"))
        .map(|c| gitdir.join(c))
        .unwrap_or_else(|| gitdir.clone());
    let Some(head) = read(&gitdir.join("HEAD")) else {
        return "unknown".to_string();
    };
    let Some(reference) = head.strip_prefix("ref:").map(str::trim) else {
        return head;
    };
    for dir in [&gitdir, &common] {
        if let Some(sha) = read(&dir.join(reference)) {
            return sha;
        }
    }
    read(&common.join("packed-refs"))
        .and_then(|p| {
            p.lines()
                .find(|l| l.ends_with(reference))
                .and_then(|l| l.split_whitespace().next().map(str::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn host_string() -> String {
    let run = |cmd: &str, args: &[&str]| {
        Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    };
    format!(
        "{}; {}",
        run("/usr/bin/sw_vers", &[]),
        run("/usr/bin/uname", &["-m"])
    )
}

/// Replaces the home directory prefix so reports do not embed it.
pub fn display_path(path: &Path) -> String {
    let text = path.display().to_string();
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && text.starts_with(&home) => text.replacen(&home, "~", 1),
        _ => text,
    }
}

pub fn histogram(results: &[FileResult]) -> Vec<(Verdict, usize)> {
    Verdict::ALL
        .iter()
        .map(|v| (*v, results.iter().filter(|r| r.verdict == *v).count()))
        .collect()
}

fn top<'a>(results: &'a [FileResult], key: impl Fn(&FileResult) -> u64) -> Vec<&'a FileResult> {
    let mut v: Vec<&FileResult> = results
        .iter()
        .filter(|r| r.verdict != Verdict::Skipped)
        .collect();
    v.sort_by(|a, b| key(b).cmp(&key(a)).then_with(|| a.id.cmp(&b.id)));
    v.truncate(5);
    v
}

fn rss_key(r: &FileResult) -> u64 {
    r.peak_rss_bytes.unwrap_or(0)
}

pub fn to_json(cfg: &Config, meta: &Meta, results: &[FileResult]) -> String {
    let mut s = String::new();
    s.push_str("{\n  \"schema_version\": 1,\n");
    s.push_str(&format!(
        "  \"tool\": {{\"name\": \"vexel-corpus\", \"git_sha\": {}}},\n",
        quote(&meta.git_sha)
    ));
    s.push_str(&format!(
        "  \"manifest\": {{\"path\": {}, \"sha256\": {}}},\n",
        quote(&meta.manifest_label),
        quote(&meta.manifest_sha)
    ));
    s.push_str(&format!(
        "  \"size\": \"{}x{}\",\n  \"max_bytes\": {},\n  \"timeout_secs\": {},\n  \"rss_limit_mib\": {},\n",
        cfg.width, cfg.height, cfg.max_bytes, cfg.timeout_secs, cfg.rss_limit_mib
    ));
    s.push_str(&format!(
        "  \"host\": {},\n  \"files\": [\n",
        quote(&meta.host)
    ));
    for (i, r) in results.iter().enumerate() {
        let rss = r
            .peak_rss_bytes
            .map_or("null".to_string(), |b| b.to_string());
        s.push_str(&format!(
            "    {{\"id\": {}, \"tier\": {}, \"kind\": {}, \"bytes\": {}, \"verdict\": {}, \"detail\": {}, \"wall_ms\": {}, \"peak_rss_bytes\": {}}}{}\n",
            quote(&r.id), quote(&r.tier), quote(&r.kind), r.bytes, quote(r.verdict.name()),
            quote(&r.detail), r.wall_ms, rss,
            if i + 1 < results.len() { "," } else { "" }
        ));
    }
    s.push_str("  ],\n  \"histogram\": {");
    let h: Vec<String> = histogram(results)
        .iter()
        .map(|(v, n)| format!("\"{}\": {n}", v.name()))
        .collect();
    s.push_str(&h.join(", "));
    let wall: u64 = results.iter().map(|r| r.wall_ms).sum();
    s.push_str(&format!(
        "}},\n  \"totals\": {{\"files\": {}, \"wall_ms_sum\": {wall}}},\n",
        results.len()
    ));
    let list = |rs: Vec<&FileResult>, f: &dyn Fn(&FileResult) -> u64| {
        let items: Vec<String> = rs
            .iter()
            .map(|r| format!("{{\"id\": {}, \"value\": {}}}", quote(&r.id), f(r)))
            .collect();
        format!("[{}]", items.join(", "))
    };
    s.push_str(&format!(
        "  \"slowest\": {},\n",
        list(top(results, |r| r.wall_ms), &|r| r.wall_ms)
    ));
    s.push_str(&format!(
        "  \"largest_rss\": {}\n}}\n",
        list(top(results, rss_key), &rss_key)
    ));
    s
}

pub fn to_markdown(cfg: &Config, meta: &Meta, results: &[FileResult]) -> String {
    let mut s = String::new();
    s.push_str("# Rust corpus measurement\n\n");
    s.push_str(
        "These are verdict counts and timings only, produced by one run on one host. They are \
         not fidelity, conformance or performance claims. The corpus bytes are not in this \
         repository; only manifest metadata, verdicts and measurements are recorded.\n\n",
    );
    s.push_str(&format!(
        "- tool: vexel-corpus, base HEAD `{}` (working tree may be dirty)\n- manifest: `{}` sha256 `{}`\n- render size: {}x{}, skip over {} bytes, timeout {} s, RSS limit {} MiB\n- host: {}\n\n",
        meta.git_sha, meta.manifest_label, meta.manifest_sha, cfg.width, cfg.height,
        cfg.max_bytes, cfg.timeout_secs, cfg.rss_limit_mib, meta.host
    ));
    s.push_str("## Verdict histogram\n\n| verdict | files |\n|---|---|\n");
    for (v, n) in histogram(results) {
        s.push_str(&format!("| {} | {n} |\n", v.name()));
    }
    let wall: u64 = results.iter().map(|r| r.wall_ms).sum();
    s.push_str(&format!(
        "\nTotal: {} files, summed wall time {wall} ms.\n\n",
        results.len()
    ));
    for (title, rs, f) in [
        (
            "Slowest five (wall ms)",
            top(results, |r| r.wall_ms),
            (|r: &FileResult| r.wall_ms) as fn(&FileResult) -> u64,
        ),
        (
            "Largest peak RSS five (bytes)",
            top(results, rss_key),
            rss_key as fn(&FileResult) -> u64,
        ),
    ] {
        s.push_str(&format!("## {title}\n\n"));
        for r in rs {
            s.push_str(&format!("- `{}` {}\n", r.id, f(r)));
        }
        s.push('\n');
    }
    s.push_str("## Per file\n\n| id | tier | kind | bytes | verdict | wall ms | peak RSS | detail |\n|---|---|---|---|---|---|---|---|\n");
    for r in results {
        let rss = r.peak_rss_bytes.map_or("-".to_string(), |b| b.to_string());
        let detail = r.detail.replace('|', "\\|");
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.id,
            r.tier,
            r.kind,
            r.bytes,
            r.verdict.name(),
            r.wall_ms,
            rss,
            detail
        ));
    }
    s
}

pub fn write_reports(cfg: &Config, meta: &Meta, results: &[FileResult]) -> Result<(), String> {
    std::fs::create_dir_all(&cfg.out).map_err(|e| format!("create --out: {e}"))?;
    std::fs::write(cfg.out.join("report.json"), to_json(cfg, meta, results))
        .map_err(|e| format!("write report.json: {e}"))?;
    std::fs::write(cfg.out.join("report.md"), to_markdown(cfg, meta, results))
        .map_err(|e| format!("write report.md: {e}"))
}
