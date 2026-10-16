//! `compare`: histogram and per-file deltas between legacy/Rust reports.
//! Both schemas carry `files[].{id,verdict}`; legacy files also have
//! categories (`path`, `viewport`) with no Rust verdict of the same name.

use crate::jsonx::Json;
use crate::manifest::sha256_file;
use std::collections::BTreeMap;
use std::path::Path;
use vexel_json::JsonDocument;

pub struct Report {
    pub label: String,
    pub sha256: String,
    /// (id, verdict) in file order.
    pub files: Vec<(String, String)>,
}

/// Rust category a legacy-only category corresponds to (Malformed path data
/// and viewport are `ComposeError::Malformed`, which this tool calls `xml`).
pub fn legacy_mapping(category: &str) -> Option<&'static str> {
    match category {
        "path" | "viewport" => Some("xml"),
        _ => None,
    }
}

fn mapped(category: &str) -> &str {
    legacy_mapping(category).unwrap_or(category)
}

/// Legacy `limit` (one verdict before the decode/render split) matches either
/// new limit verdict, in both directions.
fn limit_pair(a: &str, b: &str) -> bool {
    let is_limit = |v: &str| matches!(v, "limit" | "limit_decode" | "limit_render");
    is_limit(a) && is_limit(b) && (a == "limit" || b == "limit")
}

pub fn load(label: &str, path: &Path) -> Result<Report, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc = JsonDocument::parse(&bytes).map_err(|e| format!("{}: {e:?}", path.display()))?;
    let j = Json::new(&doc);
    let files = j.get(j.root(), "files").ok_or("report has no \"files\"")?;
    let mut out = Vec::new();
    for f in j.elements(files) {
        let id = j.str_at(f, "id").ok_or("file without id")?;
        let verdict = j.str_at(f, "verdict").ok_or("file without verdict")?;
        out.push((id, verdict));
    }
    Ok(Report {
        label: label.to_string(),
        sha256: sha256_file(path)?,
        files: out,
    })
}

fn counts(r: &Report) -> BTreeMap<&str, usize> {
    let mut m = BTreeMap::new();
    for (_, v) in &r.files {
        *m.entry(v.as_str()).or_insert(0) += 1;
    }
    m
}

pub fn markdown(baselines: &[Report], current: &Report) -> String {
    let mut s = String::from("## Comparison with the legacy C baselines\n\n");
    s.push_str("Sources (sha256 of the compared files):\n\n");
    for r in baselines.iter().chain(std::iter::once(current)) {
        s.push_str(&format!("- {}: sha256 `{}`\n", r.label, r.sha256));
    }
    let all: Vec<&Report> = baselines.iter().chain(std::iter::once(current)).collect();
    let tallies: Vec<_> = all.iter().map(|r| counts(r)).collect();
    let mut cats: Vec<&str> = vec![
        "ok",
        "unsupported",
        "limit",
        "limit_decode",
        "limit_render",
        "xml",
        "timeout",
        "rss_exceeded",
        "skipped",
        "crash",
        "guard_failed",
    ];
    for t in &tallies {
        for k in t.keys() {
            if !cats.contains(k) {
                cats.push(k);
            }
        }
    }
    s.push_str("\n| category |");
    for r in &all {
        s.push_str(&format!(" {} |", r.label));
    }
    s.push_str("\n|---|");
    for _ in &all {
        s.push_str("---|");
    }
    s.push('\n');
    for c in &cats {
        let row: Vec<usize> = tallies
            .iter()
            .map(|t| t.get(c).copied().unwrap_or(0))
            .collect();
        if row.iter().all(|n| *n == 0) {
            continue;
        }
        let note =
            legacy_mapping(c).map_or(String::new(), |m| format!(" (legacy only; maps to `{m}`)"));
        s.push_str(&format!("| {c}{note} |"));
        for n in row {
            s.push_str(&format!(" {n} |"));
        }
        s.push('\n');
    }
    s.push_str("| **ok / attempted** |");
    for t in &tallies {
        let total: usize = t.values().sum();
        let skipped = t.get("skipped").copied().unwrap_or(0);
        s.push_str(&format!(
            " {} / {} |",
            t.get("ok").copied().unwrap_or(0),
            total - skipped
        ));
    }
    s.push_str(
        "\n\nLegacy `path` and `viewport` have no Rust verdict of that name. In the tables \
        above they stay separate rows (never merged). For the per-file deltas below they are \
        compared as `xml` (Rust reports malformed path data and viewport as `Malformed`, verdict \
        `xml`); every such pair is printed with its raw legacy category.\n\n",
    );
    let cur: BTreeMap<&str, &str> = current
        .files
        .iter()
        .map(|(i, v)| (i.as_str(), v.as_str()))
        .collect();
    for b in baselines {
        s.push_str(&format!("### Per-file deltas vs {}\n\n", b.label));
        let (mut new_ok, mut regress, mut changed, mut missing) = (vec![], vec![], vec![], vec![]);
        let mut same = 0;
        for (id, old) in &b.files {
            let Some(new) = cur.get(id.as_str()) else {
                missing.push(id.clone());
                continue;
            };
            if old == new {
                same += 1;
            } else if *new == "ok" {
                new_ok.push(format!("`{id}`: {old} -> ok"));
            } else if old == "ok" {
                regress.push(format!("`{id}`: ok -> {new}"));
            } else if mapped(old) == *new || limit_pair(old, new) {
                same += 1;
                changed.push(format!(
                    "`{id}`: {old} -> {new} (same category after mapping)"
                ));
            } else {
                changed.push(format!("`{id}`: {old} -> {new}"));
            }
        }
        for (name, list) in [
            ("New ok", &new_ok),
            ("Regressions", &regress),
            ("Changed category", &changed),
        ] {
            s.push_str(&format!(
                "{name}: {}\n\n",
                if list.is_empty() {
                    "none".to_string()
                } else {
                    list.len().to_string()
                }
            ));
            for line in list {
                s.push_str(&format!("- {line}\n"));
            }
            if !list.is_empty() {
                s.push('\n');
            }
        }
        s.push_str(&format!(
            "Unchanged (including same-after-mapping): {same}. Ids not in current report: {}.\n\n",
            missing.len()
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(label: &str, files: &[(&str, &str)]) -> Report {
        Report {
            label: label.to_string(),
            sha256: "x".to_string(),
            files: files
                .iter()
                .map(|(i, v)| (i.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn legacy_limit_matches_either_new_verdict() {
        assert!(limit_pair("limit", "limit_decode"));
        assert!(limit_pair("limit", "limit_render"));
        assert!(limit_pair("limit_decode", "limit"));
        assert!(!limit_pair("limit_decode", "limit_render"));
        assert!(!limit_pair("limit", "xml"));
        let base = report("old", &[("a", "limit"), ("b", "limit"), ("c", "limit")]);
        let cur = report(
            "new",
            &[("a", "limit_decode"), ("b", "limit_render"), ("c", "ok")],
        );
        let md = markdown(&[base], &cur);
        assert!(md.contains("`a`: limit -> limit_decode (same category after mapping)"));
        assert!(md.contains("`b`: limit -> limit_render (same category after mapping)"));
        assert!(md.contains("`c`: limit -> ok"));
        assert!(md.contains("| limit_decode |") && md.contains("| limit_render |"));
    }

    #[test]
    fn split_limit_verdicts_are_distinct_from_each_other() {
        let base = report("old", &[("a", "limit_decode")]);
        let cur = report("new", &[("a", "limit_render")]);
        let md = markdown(&[base], &cur);
        assert!(md.contains("`a`: limit_decode -> limit_render\n"), "{md}");
    }

    #[test]
    fn old_reports_still_parse() {
        let dir = std::env::temp_dir().join(format!("vexel-corpus-cmp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("old.json");
        std::fs::write(
            &path,
            br#"{"files":[{"id":"a","verdict":"limit"},{"id":"b","verdict":"ok"}]}"#,
        )
        .unwrap();
        let r = load("old", &path).unwrap();
        assert_eq!(r.files[0], ("a".to_string(), "limit".to_string()));
        std::fs::remove_dir_all(&dir).ok();
    }
}
