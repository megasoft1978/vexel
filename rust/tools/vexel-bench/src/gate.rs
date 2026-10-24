//! `--check` against the committed baseline, and `--update-baseline`.
//!
//! Output hashes are an exact, host-independent gate. Wall clock is compared
//! only when the host identity block equals the baseline's, and a breach must
//! repeat on two consecutive runs to fail.

use crate::json::Json;
use crate::report::Report;

/// A case regresses when its median exceeds the baseline by this factor...
pub const SLOWDOWN: f64 = 1.10;
/// ...and by more than this many MADs.
pub const MAD_FACTOR: f64 = 2.0;
/// Consecutive breaching runs that turn a warning into a failure.
pub const STREAK_FAIL: u64 = 2;

const IDENTITY_KEYS: [&str; 6] = ["model", "chip", "memory_bytes", "arch", "os", "compiler"];

#[derive(Debug, Default)]
pub struct Outcome {
    pub failures: Vec<String>,
    pub warnings: Vec<String>,
    pub notes: Vec<String>,
    /// Updated consecutive-breach counts (case name to count).
    pub streaks: Vec<(String, Json)>,
}

pub fn same_host(a: &Json, b: &Json) -> bool {
    IDENTITY_KEYS
        .iter()
        .all(|k| a.get(k).and_then(Json::text) == b.get(k).and_then(Json::text))
}

fn num(case: &Json, key: &str) -> f64 {
    case.get(key).and_then(Json::num).unwrap_or(0.0)
}

pub fn check(report: &Report, baseline: &Json, streaks: &Json, timing: bool) -> Outcome {
    let mut out = Outcome::default();
    let base_cases = baseline.get("cases");
    let comparable = timing
        && baseline
            .get("host")
            .is_some_and(|h| same_host(h, &report.host));
    if !timing {
        out.notes
            .push("hash-only mode: wall clock not compared".into());
    } else if !comparable {
        out.notes
            .push("host identity differs from the baseline: wall clock not compared".into());
    }
    for case in &report.cases {
        let Some(name) = case.get("name").and_then(Json::text) else {
            continue;
        };
        if let Some(error) = case.get("error").and_then(Json::text) {
            out.failures.push(format!("{name}: failed to run: {error}"));
            continue;
        }
        if case.get("skipped").is_some() {
            continue;
        }
        let Some(base) = base_cases.and_then(|c| c.get(name)) else {
            out.warnings.push(format!("{name}: not in baseline"));
            continue;
        };
        if let Some(want) = base.get("hash").and_then(Json::text) {
            let got = case.get("hash").and_then(Json::text);
            if got != Some(want) {
                out.failures.push(format!(
                    "{name}: output hash {} != baseline {}",
                    got.map_or("none", |g| &g[..12.min(g.len())]),
                    &want[..12.min(want.len())]
                ));
            } else if case.get("hash_stable") == Some(&Json::Bool(false)) {
                out.failures
                    .push(format!("{name}: output differs between trials"));
            }
        }
        if !comparable {
            continue;
        }
        let (now, then) = (num(case, "p50_ms"), num(base, "p50_ms"));
        let gap = now - then;
        let mad = num(case, "mad_ms").max(num(base, "mad_ms"));
        let breach = now > then * SLOWDOWN && gap > MAD_FACTOR * mad;
        let previous = streaks.get(name).and_then(Json::num).unwrap_or(0.0) as u64;
        let streak = if breach { previous + 1 } else { 0 };
        out.streaks
            .push((name.to_owned(), Json::Num(streak as f64)));
        if breach {
            let line = format!(
                "{name}: p50 {now:.2} ms vs baseline {then:.2} ms (+{gap:.2} ms, MAD {mad:.2}), \
                 breach {streak} of {STREAK_FAIL}"
            );
            if streak >= STREAK_FAIL {
                out.failures.push(line);
            } else {
                out.warnings.push(line);
            }
        }
    }
    out
}

fn entry(case: &Json) -> Json {
    let keep = [
        "hash",
        "hash_stable",
        "p50_ms",
        "mad_ms",
        "p95_ms",
        "p99_ms",
        "max_ms",
        "fps",
        "rss_bytes",
        "alloc_count",
        "trials",
    ];
    Json::Obj(
        keep.iter()
            .filter_map(|k| Some(((*k).to_owned(), case.get(k)?.clone())))
            .collect(),
    )
}

/// New baseline from `report`. On the same host a case keeps its old timing
/// unless the new median is lower (baselines only ratchet down); hashes always
/// take the new value because updating is an explicit act.
pub fn update(report: &Report, old: Option<&Json>) -> Result<Json, String> {
    if let Some(error) = report.errors().first() {
        return Err(format!("refusing to update with a failed case: {error}"));
    }
    let same = old
        .and_then(|o| o.get("host"))
        .is_some_and(|h| same_host(h, &report.host));
    let mut cases: Vec<(String, Json)> = match old.and_then(|o| o.get("cases")) {
        Some(c) => c.pairs().to_vec(),
        None => Vec::new(),
    };
    for case in &report.cases {
        let Some(name) = case.get("name").and_then(Json::text) else {
            continue;
        };
        if case.get("p50_ms").is_none() {
            continue;
        }
        let mut new = entry(case);
        let slot = cases.iter().position(|(k, _)| k == name);
        if let (true, Some(i)) = (same, slot) {
            let prior = &cases[i].1;
            if num(prior, "p50_ms") > 0.0 && num(prior, "p50_ms") <= num(&new, "p50_ms") {
                let hash = new.get("hash").cloned().unwrap_or(Json::Null);
                new = prior.clone();
                if let Json::Obj(pairs) = &mut new {
                    pairs.retain(|(k, _)| k != "hash");
                    pairs.insert(0, ("hash".into(), hash));
                }
            }
        }
        match slot {
            Some(i) if same => cases[i].1 = new,
            _ => cases.push((name.to_owned(), new)),
        }
    }
    if !same {
        // A different host starts a new baseline; entries recorded elsewhere do not carry over.
        let names: Vec<&str> = report
            .cases
            .iter()
            .filter_map(|c| c.get("name").and_then(Json::text))
            .collect();
        cases.retain(|(k, _)| names.contains(&k.as_str()));
    }
    cases.sort_by(|a, b| a.0.cmp(&b.0));
    cases.dedup_by(|a, b| a.0 == b.0);
    Ok(Json::obj(vec![
        ("schema", Json::str("vexel-bench-baseline/1")),
        ("level", Json::str(report.opts.level.name())),
        ("date", Json::str(&report.date)),
        ("git_sha", Json::str(&report.git_sha)),
        ("host", report.host.clone()),
        (
            "protocol",
            Json::obj(vec![
                ("warmups", Json::Num(report.opts.warmups as f64)),
                ("trials", Json::Num(report.opts.trials as f64)),
            ]),
        ),
        ("cases", Json::Obj(cases)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::Level;
    use crate::orch::Opts;

    fn host(os: &str) -> Json {
        Json::obj(vec![("model", Json::str("m")), ("os", Json::str(os))])
    }

    fn case(name: &str, hash: &str, p50: f64, mad: f64) -> Json {
        Json::obj(vec![
            ("name", Json::str(name)),
            ("hash", Json::str(hash)),
            ("hash_stable", Json::Bool(true)),
            ("p50_ms", Json::Num(p50)),
            ("mad_ms", Json::Num(mad)),
        ])
    }

    fn report(host: Json, cases: Vec<Json>) -> Report {
        Report {
            date: "d".into(),
            git_sha: "g".into(),
            host,
            opts: Opts::new(Level::Reduced),
            cases,
        }
    }

    fn baseline(h: Json) -> Json {
        update(&report(h, vec![case("a/x/1", "aa", 10.0, 0.1)]), None).unwrap()
    }

    #[test]
    fn hash_mismatch_fails_on_any_host() {
        let b = baseline(host("1"));
        let r = report(host("2"), vec![case("a/x/1", "bb", 10.0, 0.1)]);
        let o = check(&r, &b, &Json::Obj(vec![]), true);
        assert_eq!(o.failures.len(), 1);
        assert!(o.notes[0].contains("host identity differs"));
    }

    #[test]
    fn slowdown_needs_two_consecutive_runs_to_fail() {
        let b = baseline(host("1"));
        let slow = report(host("1"), vec![case("a/x/1", "aa", 12.0, 0.1)]);
        let first = check(&slow, &b, &Json::Obj(vec![]), true);
        assert!(first.failures.is_empty() && first.warnings.len() == 1);
        let state = Json::Obj(first.streaks);
        assert_eq!(check(&slow, &b, &state, true).failures.len(), 1);
    }

    #[test]
    fn noise_within_mad_or_ten_percent_passes() {
        let b = baseline(host("1"));
        for p50 in [10.5, 11.0, 9.0] {
            let r = report(host("1"), vec![case("a/x/1", "aa", p50, 0.1)]);
            let o = check(&r, &b, &Json::Obj(vec![]), true);
            assert!(o.failures.is_empty() && o.warnings.is_empty());
        }
        let wide = report(host("1"), vec![case("a/x/1", "aa", 12.0, 1.5)]);
        assert!(
            check(&wide, &b, &Json::Obj(vec![]), true)
                .warnings
                .is_empty()
        );
    }

    #[test]
    fn update_only_ratchets_timing_down_on_the_same_host() {
        let b = baseline(host("1"));
        let slower = report(host("1"), vec![case("a/x/1", "cc", 12.0, 0.1)]);
        let u = update(&slower, Some(&b)).unwrap();
        let e = u.get("cases").and_then(|c| c.get("a/x/1")).unwrap();
        assert_eq!(e.get("p50_ms").and_then(Json::num), Some(10.0));
        assert_eq!(e.get("hash").and_then(Json::text), Some("cc"));
        let faster = report(host("1"), vec![case("a/x/1", "aa", 8.0, 0.1)]);
        let u = update(&faster, Some(&b)).unwrap();
        let e = u.get("cases").and_then(|c| c.get("a/x/1")).unwrap();
        assert_eq!(e.get("p50_ms").and_then(Json::num), Some(8.0));
    }
}
