//! Per-file isolation: the parent re-invokes this binary under
//! `/usr/bin/time -l` in its own process group, polls for exit and kills the
//! whole group on timeout; the child (`--one`) runs the pipeline.

use crate::verdict::{self, Verdict};
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use vexel_app::compose::compose_document;

const RESULT_TAG: &str = "VEXEL-CORPUS-RESULT";
const CAP: u64 = 64 * 1024;

#[derive(Debug, Clone)]
pub struct Outcome {
    pub verdict: Verdict,
    pub detail: String,
    pub wall_ms: u64,
    pub peak_rss_bytes: Option<u64>,
}

/// Child side of `--one`: compose one file and print one result line.
pub fn run_one(path: &Path, width: u32, height: u32) -> i32 {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            println!("{RESULT_TAG}\tcrash\tread failed: {e}");
            return 0;
        }
    };
    match compose_document(&bytes, width, height, &|| false) {
        Ok(c) => println!(
            "{RESULT_TAG}\tok\t{}x{} frame, {} disclosure(s)",
            c.frame.width,
            c.frame.height,
            c.frame.disclosures.len()
        ),
        Err(e) => {
            let (v, d) = verdict::from_compose_error(&e);
            println!(
                "{RESULT_TAG}\t{}\t{}",
                v.name(),
                d.replace(['\t', '\n'], " ")
            );
        }
    }
    0
}

/// Test-only child mode: sleeps so the parent's timeout path can be tested.
pub fn run_sleep(secs: u64) -> i32 {
    std::thread::sleep(Duration::from_secs(secs));
    0
}

fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = (&mut pipe).take(CAP).read_to_end(&mut buf);
        let _ = std::io::copy(&mut pipe, &mut std::io::sink());
        String::from_utf8_lossy(&buf).into_owned()
    })
}

fn parse_rss(stderr: &str) -> Option<u64> {
    stderr
        .lines()
        .find(|l| l.contains("maximum resident set size"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn parse_result(stdout: &str) -> Option<(Verdict, String)> {
    let line = stdout.lines().find(|l| l.starts_with(RESULT_TAG))?;
    let mut parts = line.splitn(3, '\t');
    parts.next()?;
    let v = Verdict::parse(parts.next()?)?;
    Some((v, parts.next().unwrap_or("").to_string()))
}

/// Runs `exe args...` under `/usr/bin/time -l` with a wall-clock timeout and a
/// post-hoc RSS limit.
pub fn run_isolated(exe: &Path, args: &[String], timeout: Duration, rss_limit_mib: u64) -> Outcome {
    let start = Instant::now();
    let fail = |detail: String, start: Instant| Outcome {
        verdict: Verdict::Crash,
        detail,
        wall_ms: start.elapsed().as_millis() as u64,
        peak_rss_bytes: None,
    };
    let spawned = Command::new("/usr/bin/time")
        .arg("-l")
        .arg(exe)
        .args(args)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => return fail(format!("spawn failed: {e}"), start),
    };
    let out = child.stdout.take().map(drain);
    let err = child.stderr.take().map(drain);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() >= timeout => {
                timed_out = true;
                let pgid = child.id().to_string();
                let _ = Command::new("/bin/kill")
                    .args(["-s", "KILL", "--", &format!("-{pgid}")])
                    .status();
                let _ = child.kill();
                break child.wait().ok();
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break None,
        }
    };
    let wall_ms = start.elapsed().as_millis() as u64;
    let stdout = out.and_then(|h| h.join().ok()).unwrap_or_default();
    let stderr = err.and_then(|h| h.join().ok()).unwrap_or_default();
    let peak = parse_rss(&stderr);
    if timed_out {
        return Outcome {
            verdict: Verdict::Timeout,
            detail: format!("killed after {} s wall-clock timeout", timeout.as_secs()),
            wall_ms,
            peak_rss_bytes: None,
        };
    }
    let abnormal = stderr.contains("Command terminated abnormally");
    let code = status.and_then(|s| s.code());
    if let Some((v, mut d)) = verdict::from_exit(code, abnormal) {
        if let Some(line) = stderr.lines().rev().find(|l| l.contains("panicked")) {
            d = format!("{d}; {}", line.trim());
        }
        return Outcome {
            verdict: v,
            detail: d,
            wall_ms,
            peak_rss_bytes: peak,
        };
    }
    let Some((v, d)) = parse_result(&stdout) else {
        return Outcome {
            verdict: Verdict::Crash,
            detail: "child exited 0 without a result line".to_string(),
            wall_ms,
            peak_rss_bytes: peak,
        };
    };
    if let Some((rv, rd)) = verdict::from_rss(peak, rss_limit_mib) {
        return Outcome {
            verdict: rv,
            detail: format!("{rd} (pipeline verdict was {}: {d})", v.name()),
            wall_ms,
            peak_rss_bytes: peak,
        };
    }
    Outcome {
        verdict: v,
        detail: d,
        wall_ms,
        peak_rss_bytes: peak,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_time_output() {
        let s = "        0.01 real  0.00 user  0.00 sys\n   1572864  maximum resident set size\n";
        assert_eq!(parse_rss(s), Some(1_572_864));
        assert_eq!(parse_rss("nothing"), None);
    }

    #[test]
    fn parses_result_line() {
        let (v, d) = parse_result("noise\nVEXEL-CORPUS-RESULT\tlimit\tx y\n").unwrap();
        assert_eq!((v, d.as_str()), (Verdict::Limit, "x y"));
        assert!(parse_result("VEXEL-CORPUS-RESULT\tbogus\tx").is_none());
    }
}
