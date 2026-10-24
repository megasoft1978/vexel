//! Child process: runs one case (warmups, timed trials, one diagnostic trial)
//! and prints a single `BENCH_RESULT {json}` line for the parent.

use crate::json::Json;
use crate::ops::{Failure, Op, Runner};
use crate::{alloc, sha256, stats};
use std::path::PathBuf;
use std::time::Instant;

pub const RESULT_PREFIX: &str = "BENCH_RESULT ";
/// Fewest timed trials kept when a case is too slow for the full count.
pub const MIN_TRIALS: usize = 3;

pub struct CaseArgs {
    pub op: Op,
    pub profile: String,
    pub n: usize,
    pub input: PathBuf,
    pub tmp: PathBuf,
    pub trials: usize,
    pub warmups: usize,
    /// Seconds a case may spend on timed trials before the count is reduced.
    pub budget_s: f64,
    /// Run the untimed allocation and phase-timer trial.
    pub diag: bool,
}

pub fn name(op: Op, profile: &str, n: usize) -> String {
    format!("{}/{profile}/{n}", op.name())
}

fn nums(values: &[f64]) -> Json {
    Json::Arr(values.iter().map(|v| Json::Num(*v)).collect())
}

fn phases() -> Json {
    let taken = vexel_app::profile::take();
    Json::Obj(
        taken
            .into_iter()
            .map(|(phase, samples)| {
                let s = stats::summarize(&samples);
                let sum: f64 = samples.iter().sum();
                let body = Json::obj(vec![
                    ("n", Json::Num(s.n as f64)),
                    ("sum_ms", Json::Num(sum)),
                    ("p50_ms", Json::Num(s.p50)),
                    ("max_ms", Json::Num(s.max)),
                ]);
                (phase.to_owned(), body)
            })
            .collect(),
    )
}

/// Runs the case. A `Skip` (unsupported here) is a result; a `Fail` is an error.
pub fn run(args: &CaseArgs) -> Result<Json, String> {
    let id = name(args.op, &args.profile, args.n);
    let head = |extra: Vec<(&str, Json)>| {
        let mut pairs = vec![
            ("name", Json::str(&id)),
            ("op", Json::str(args.op.name())),
            ("profile", Json::str(&args.profile)),
            ("n", Json::Num(args.n as f64)),
        ];
        pairs.extend(extra);
        Json::obj(pairs)
    };
    match measure(args) {
        Ok(extra) => Ok(head(extra)),
        Err(Failure::Skip(why)) => Ok(head(vec![("skipped", Json::str(why))])),
        Err(Failure::Fail(why)) => Err(format!("{id}: {why}")),
    }
}

fn measure(args: &CaseArgs) -> Result<Vec<(&'static str, Json)>, Failure> {
    let mut runner = Runner::new(args.op, &args.profile, args.n, &args.input, &args.tmp)?;
    let mut warmed = 0;
    let mut wall = 0.0;
    while warmed < args.warmups {
        let start = Instant::now();
        runner.trial()?;
        wall = start.elapsed().as_secs_f64();
        warmed += 1;
        if wall > args.budget_s / 4.0 {
            break; // too slow to warm repeatedly
        }
    }
    let mut trials = args.trials;
    if wall > 0.0 {
        trials = trials.min(((args.budget_s / wall) as usize).max(MIN_TRIALS));
    }
    let (mut samples, mut hashes) = (Vec::new(), Vec::new());
    let mut slowest: f64 = wall;
    for _ in 0..trials {
        let start = Instant::now();
        let trial = runner.trial()?;
        slowest = slowest.max(start.elapsed().as_secs_f64());
        samples.extend(trial.samples);
        hashes.push(trial.hash);
    }
    let hash = hashes.first().cloned().flatten();
    let stable = hashes.windows(2).all(|w| w[0] == w[1]);
    let s = stats::summarize(&samples);
    let input_bytes = std::fs::read(&args.input)?;
    let mut extra = vec![
        ("input_sha256", Json::str(sha256::hex(&input_bytes))),
        ("trials", Json::Num(trials as f64)),
        ("warmups", Json::Num(warmed as f64)),
        ("samples", Json::Num(s.n as f64)),
        ("p50_ms", Json::Num(s.p50)),
        ("p95_ms", Json::Num(s.p95)),
        ("p99_ms", Json::Num(s.p99)),
        ("max_ms", Json::Num(s.max)),
        ("mad_ms", Json::Num(s.mad)),
        (
            "fps",
            Json::Num(if s.p50 > 0.0 { 1000.0 / s.p50 } else { 0.0 }),
        ),
        ("hash", hash.map_or(Json::Null, Json::Str)),
        ("hash_stable", Json::Bool(stable)),
        ("samples_ms", nums(&samples)),
    ];
    if args.diag && slowest <= args.budget_s * 2.0 {
        let _ = vexel_app::profile::take();
        vexel_app::profile::enable();
        alloc::start();
        let diag = runner.trial();
        let (count, bytes) = alloc::stop();
        diag?;
        extra.push(("alloc_count", Json::Num(count as f64)));
        extra.push(("alloc_bytes", Json::Num(bytes as f64)));
        extra.push(("phases", phases()));
    }
    if !runner.notes.is_empty() {
        extra.push((
            "notes",
            Json::Arr(runner.notes.iter().map(Json::str).collect()),
        ));
    }
    Ok(extra)
}
