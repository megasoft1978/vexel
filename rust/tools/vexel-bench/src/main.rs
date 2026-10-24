#![deny(unsafe_code)]
//! Benchmark harness: seeded scenes, headless app operations, per-case child
//! processes, a committed baseline and a hash/timing gate. See `docs/PERF.md`.
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
compile_error!("Vexel initially supports Apple Silicon macOS 13+ only");

mod alloc;
mod anim;
mod child;
mod gate;
mod host;
mod json;
mod matrix;
mod ops;
mod orch;
mod report;
mod scene;
mod sha256;
mod stats;

use json::Json;
use matrix::Level;
use orch::Opts;
use report::Report;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[global_allocator]
static COUNTING: alloc::Counting = alloc::Counting;

const USAGE: &str = "Usage: vexel-bench run [--full|--hash-only] [--out DIR] [--only TEXT] \
[--trials N] [--warmups N] [--budget SECONDS]\n       \
vexel-bench check [--full|--hash-only] [--update-baseline] [--only TEXT] [--trials N] [--warmups N]\n       \
vexel-bench case OP PROFILE N --input FILE --tmp DIR [--trials N] [--warmups N] [--budget S] [--no-diag]";

fn root() -> PathBuf {
    let raw = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    std::fs::canonicalize(&raw).unwrap_or(raw)
}

struct Flags {
    rest: Vec<String>,
}

impl Flags {
    fn take(&mut self, name: &str) -> bool {
        let found = self.rest.iter().position(|a| a == name);
        found.map(|i| self.rest.remove(i)).is_some()
    }
    fn value(&mut self, name: &str) -> Result<Option<String>, String> {
        let Some(i) = self.rest.iter().position(|a| a == name) else {
            return Ok(None);
        };
        self.rest.remove(i);
        if i < self.rest.len() {
            Ok(Some(self.rest.remove(i)))
        } else {
            Err(format!("{name} needs a value"))
        }
    }
    fn parsed<T: std::str::FromStr>(&mut self, name: &str) -> Result<Option<T>, String> {
        self.value(name)?
            .map(|v| v.parse().map_err(|_| format!("{name}: bad value `{v}`")))
            .transpose()
    }
}

fn opts(flags: &mut Flags) -> Result<Opts, String> {
    let level = match (flags.take("--full"), flags.take("--hash-only")) {
        (true, true) => return Err("--full and --hash-only are exclusive".into()),
        (true, false) => Level::Full,
        (false, true) => Level::HashOnly,
        (false, false) => Level::Reduced,
    };
    let mut o = Opts::new(level);
    o.trials = flags.parsed("--trials")?.unwrap_or(o.trials);
    o.warmups = flags.parsed("--warmups")?.unwrap_or(o.warmups);
    o.budget_s = flags.parsed("--budget")?.unwrap_or(o.budget_s);
    o.only = flags.value("--only")?;
    Ok(o)
}

fn measure(root: &Path, o: Opts) -> Result<Report, String> {
    let cases = matrix::cases(o.level, o.only.as_deref());
    if cases.is_empty() {
        return Err("no cases selected".into());
    }
    orch::prepare_scenes(root, &cases)?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let results = orch::run_cases(root, &exe, &cases, &o);
    Ok(Report {
        date: host::today(),
        git_sha: host::git_sha(root),
        host: host::identity(),
        opts: o,
        cases: results,
    })
}

fn write_report(report: &Report, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let write = |name: &str, text: String| {
        std::fs::write(dir.join(name), text).map_err(|e| format!("{name}: {e}"))
    };
    write("bench.json", report.to_json().to_pretty())?;
    write("summary.md", report::markdown(report))?;
    println!("wrote {}/bench.json and summary.md", dir.display());
    Ok(())
}

fn print_summary(report: &Report) {
    println!("\nSlowest cases by p50:\n{}", report::top_table(report, 12));
    for line in report.skipped() {
        println!("skipped {line}");
    }
    for line in report.errors() {
        println!("ERROR {line}");
    }
}

fn run_command(mut flags: Flags) -> Result<ExitCode, String> {
    let out = flags.value("--out")?;
    let o = opts(&mut flags)?;
    let root = root();
    let report = measure(&root, o)?;
    let dir = out
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("bench/results").join(&report.date));
    write_report(&report, &dir)?;
    print_summary(&report);
    Ok(if report.errors().is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn read_json(path: &Path) -> Result<Option<Json>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => json::parse(&text)
            .map(Some)
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn check_command(mut flags: Flags) -> Result<ExitCode, String> {
    let update = flags.take("--update-baseline");
    let o = opts(&mut flags)?;
    let timing = o.level != Level::HashOnly;
    let root = root();
    let baseline_path = root.join("bench/baseline.json");
    let old = read_json(&baseline_path)?;
    if old.is_none() && !update {
        return Err(format!(
            "{} is missing; create it with --update-baseline",
            baseline_path.display()
        ));
    }
    let report = measure(&root, o)?;
    write_report(&report, &root.join(".cache/bench/check"))?;
    print_summary(&report);
    if update {
        let new = gate::update(&report, old.as_ref())?;
        std::fs::write(&baseline_path, new.to_pretty()).map_err(|e| e.to_string())?;
        println!("baseline updated: {}", baseline_path.display());
        return Ok(ExitCode::SUCCESS);
    }
    let streak_path = root.join(".cache/bench/streaks.json");
    let mut streaks = read_json(&streak_path)?.unwrap_or(Json::Obj(Vec::new()));
    let outcome = gate::check(
        &report,
        old.as_ref().ok_or("no baseline")?,
        &streaks,
        timing,
    );
    if timing && !outcome.streaks.is_empty() {
        if let Json::Obj(pairs) = &mut streaks {
            for (name, count) in &outcome.streaks {
                pairs.retain(|(k, _)| k != name);
                pairs.push((name.clone(), count.clone()));
            }
        }
        std::fs::write(&streak_path, streaks.to_pretty()).map_err(|e| e.to_string())?;
    }
    for line in &outcome.notes {
        println!("note: {line}");
    }
    for line in &outcome.warnings {
        println!("warning: {line}");
    }
    for line in &outcome.failures {
        println!("FAIL: {line}");
    }
    let ok = outcome.failures.is_empty();
    println!("bench check: {}", if ok { "pass" } else { "FAIL" });
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn case_command(mut flags: Flags) -> Result<ExitCode, String> {
    let diag = !flags.take("--no-diag");
    let input = flags.value("--input")?.ok_or("--input is required")?;
    let tmp = flags.value("--tmp")?.ok_or("--tmp is required")?;
    let trials = flags.parsed("--trials")?.unwrap_or(15);
    let warmups = flags.parsed("--warmups")?.unwrap_or(2);
    let budget_s = flags.parsed("--budget")?.unwrap_or(30.0);
    let [op, profile, n] =
        <[String; 3]>::try_from(flags.rest).map_err(|_| "case needs OP PROFILE N")?;
    let args = child::CaseArgs {
        op: ops::Op::parse(&op).ok_or_else(|| format!("unknown op `{op}`"))?,
        profile,
        n: n.parse().map_err(|_| format!("bad N `{n}`"))?,
        input: input.into(),
        tmp: tmp.into(),
        trials,
        warmups,
        budget_s,
        diag,
    };
    let result = child::run(&args)?;
    println!("{}{}", child::RESULT_PREFIX, result.to_compact());
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let flags = Flags {
        rest: args.collect(),
    };
    let result = match command.as_str() {
        "run" => run_command(flags),
        "check" => check_command(flags),
        "case" => case_command(flags),
        _ => Err("missing or unknown command".into()),
    };
    match result {
        Ok(code) => code,
        Err(message) => {
            eprintln!("vexel-bench: {message}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}
