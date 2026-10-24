//! Host identity, git revision and date, for result provenance.

use crate::json::Json;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default()
}

/// Model, chip, memory, cores, architecture, OS and compiler identity. Wall
/// clock results are comparable only between equal blocks.
pub fn identity() -> Json {
    let sysctl = |key: &str| run("/usr/sbin/sysctl", &["-n", key]);
    Json::obj(vec![
        ("model", Json::str(sysctl("hw.model"))),
        ("chip", Json::str(sysctl("machdep.cpu.brand_string"))),
        ("memory_bytes", Json::str(sysctl("hw.memsize"))),
        ("logical_cpus", Json::str(sysctl("hw.logicalcpu"))),
        ("arch", Json::str(run("/usr/bin/uname", &["-m"]))),
        (
            "os",
            Json::str(run("/usr/bin/sw_vers", &["-productVersion"])),
        ),
        ("compiler", Json::str(run("rustc", &["--version"]))),
        ("build", Json::str("release")),
    ])
}

/// HEAD commit read from the `.git` files (works in linked worktrees).
pub fn git_sha(root: &Path) -> String {
    let read = |p: &Path| std::fs::read_to_string(p).ok().map(|s| s.trim().to_owned());
    let dot = root.join(".git");
    let gitdir =
        match read(&dot).and_then(|s| s.strip_prefix("gitdir:").map(|g| g.trim().to_owned())) {
            Some(g) => PathBuf::from(g),
            None => dot,
        };
    let common = read(&gitdir.join("commondir"))
        .map(|c| gitdir.join(c))
        .unwrap_or_else(|| gitdir.clone());
    let Some(head) = read(&gitdir.join("HEAD")) else {
        return "unknown".into();
    };
    let Some(reference) = head.strip_prefix("ref:").map(str::trim) else {
        return head;
    };
    [&gitdir, &common]
        .into_iter()
        .find_map(|dir| read(&dir.join(reference)))
        .or_else(|| {
            read(&common.join("packed-refs")).and_then(|p| {
                p.lines()
                    .find(|l| l.ends_with(reference))
                    .and_then(|l| l.split_whitespace().next().map(str::to_owned))
            })
        })
        .unwrap_or_else(|| "unknown".into())
}

/// UTC calendar date `YYYY-MM-DD`.
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil(i64::try_from(secs / 86_400).unwrap_or(0));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 to (year, month, day) (Hinnant's algorithm).
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::civil;

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_750), (2026, 10, 24));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }
}
