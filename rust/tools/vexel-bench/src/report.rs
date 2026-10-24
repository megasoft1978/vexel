//! bench.json, the markdown summary and the terminal table.

use crate::json::Json;
use crate::orch::Opts;

pub struct Report {
    pub date: String,
    pub git_sha: String,
    pub host: Json,
    pub opts: Opts,
    pub cases: Vec<Json>,
}

impl Report {
    pub fn to_json(&self) -> Json {
        Json::obj(vec![
            ("schema", Json::str("vexel-bench/1")),
            ("date", Json::str(&self.date)),
            ("git_sha", Json::str(&self.git_sha)),
            ("level", Json::str(self.opts.level.name())),
            (
                "protocol",
                Json::obj(vec![
                    ("warmups", Json::Num(self.opts.warmups as f64)),
                    ("trials", Json::Num(self.opts.trials as f64)),
                    ("case_budget_s", Json::Num(self.opts.budget_s)),
                ]),
            ),
            ("host", self.host.clone()),
            ("cases", Json::Arr(self.cases.clone())),
        ])
    }

    pub fn errors(&self) -> Vec<String> {
        self.cases
            .iter()
            .filter_map(|c| {
                let e = c.get("error")?.text()?;
                Some(format!("{}: {e}", c.get("name")?.text()?))
            })
            .collect()
    }

    pub fn skipped(&self) -> Vec<String> {
        self.cases
            .iter()
            .filter_map(|c| {
                let why = c.get("skipped")?.text()?;
                Some(format!("{}: {why}", c.get("name")?.text()?))
            })
            .collect()
    }
}

fn f(case: &Json, key: &str) -> f64 {
    case.get(key).and_then(Json::num).unwrap_or(f64::NAN)
}

fn cell(v: f64, digits: usize) -> String {
    if v.is_nan() {
        "-".into()
    } else {
        format!("{v:.digits$}")
    }
}

fn row(case: &Json) -> Option<String> {
    let name = case.get("name")?.text()?;
    let hash = case
        .get("hash")
        .and_then(Json::text)
        .map_or("-", |h| &h[..12.min(h.len())]);
    Some(format!(
        "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | `{hash}` |",
        cell(f(case, "p50_ms"), 2),
        cell(f(case, "p95_ms"), 2),
        cell(f(case, "p99_ms"), 2),
        cell(f(case, "max_ms"), 2),
        cell(f(case, "mad_ms"), 2),
        cell(f(case, "fps"), 1),
        cell(f(case, "rss_bytes") / 1_048_576.0, 0),
        cell(f(case, "alloc_count"), 0),
    ))
}

const HEADER: &str = "| case | p50 ms | p95 ms | p99 ms | max ms | MAD ms | fps | RSS MiB | allocs | pixel/output hash |\n\
                      |---|---:|---:|---:|---:|---:|---:|---:|---:|---|";

pub fn markdown(report: &Report) -> String {
    let host = |k: &str| {
        report
            .host
            .get(k)
            .and_then(Json::text)
            .unwrap_or("?")
            .to_owned()
    };
    let mut out = format!(
        "# Vexel bench {}\n\nLevel `{}`, commit `{}`, {} warmups, up to {} trials (case budget {} s).\n\
         Host: {}, {}, {} CPUs, macOS {}, {}.\n\n{HEADER}\n",
        report.date,
        report.opts.level.name(),
        report.git_sha,
        report.opts.warmups,
        report.opts.trials,
        report.opts.budget_s,
        host("model"),
        host("chip"),
        host("logical_cpus"),
        host("os"),
        host("compiler"),
    );
    for case in &report.cases {
        if let Some(r) = row(case) {
            out.push_str(&r);
            out.push('\n');
        }
    }
    for line in report.skipped() {
        out.push_str(&format!("\nSkipped: {line}"));
    }
    for line in report.errors() {
        out.push_str(&format!("\nERROR: {line}"));
    }
    out.push('\n');
    out
}

/// The `limit` slowest cases by p50, as a markdown table.
pub fn top_table(report: &Report, limit: usize) -> String {
    let mut timed: Vec<&Json> = report
        .cases
        .iter()
        .filter(|c| c.get("p50_ms").is_some())
        .collect();
    timed.sort_by(|a, b| f(b, "p50_ms").total_cmp(&f(a, "p50_ms")));
    let mut out = format!("{HEADER}\n");
    for case in timed.into_iter().take(limit) {
        if let Some(r) = row(case) {
            out.push_str(&r);
            out.push('\n');
        }
    }
    out
}
