//! `vexel-dev bench`: builds the Release `vexel-bench` harness and runs it.
use crate::args::UsageError;
use crate::bundle::{CargoEnv, TARGET_TRIPLE, cargo_invocation};
use crate::runner::Invocation;
use std::path::{Path, PathBuf};

const SWITCHES: [&str; 4] = ["--check", "--update-baseline", "--hash-only", "--full"];
const VALUED: [&str; 5] = ["--only", "--out", "--trials", "--warmups", "--budget"];

/// Turns `bench` flags into the harness arguments: `run ...` or `check ...`
/// (`--check` and `--update-baseline` both select the gate).
pub fn parse(rest: &[String]) -> Result<Vec<String>, UsageError> {
    let mut gate = false;
    let mut forwarded = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        let name = flag.as_str();
        if !SWITCHES.contains(&name) && !VALUED.contains(&name) {
            return Err(if name.starts_with("--") {
                UsageError::UnknownFlag {
                    command: "bench".into(),
                    flag: flag.clone(),
                }
            } else {
                UsageError::UnexpectedArgument {
                    command: "bench".into(),
                    argument: flag.clone(),
                }
            });
        }
        if seen.contains(&name) {
            return Err(UsageError::DuplicateFlag { flag: flag.clone() });
        }
        seen.push(name);
        if name == "--check" {
            gate = true;
            continue;
        }
        forwarded.push(flag.clone());
        if VALUED.contains(&name) {
            let value = iter
                .next()
                .filter(|v| !v.starts_with("--"))
                .ok_or_else(|| UsageError::MissingValue { flag: flag.clone() })?;
            forwarded.push(value.clone());
        }
    }
    gate |= seen.contains(&"--update-baseline");
    let mut args = vec![if gate { "check" } else { "run" }.to_owned()];
    args.extend(forwarded);
    Ok(args)
}

pub fn release_build(root: &Path, env: &CargoEnv) -> Invocation {
    cargo_invocation(
        &[
            "build",
            "-p",
            "vexel-bench",
            "--release",
            "--locked",
            "--offline",
            "--target",
            TARGET_TRIPLE,
        ],
        root,
        env,
    )
}

pub fn binary(env: &CargoEnv) -> PathBuf {
    env.target
        .join(TARGET_TRIPLE)
        .join("release")
        .join("vexel-bench")
}

/// Runs the harness with inherited stdio so progress and tables stream.
pub fn run_binary(binary: &Path, args: &[String], root: &Path) -> Result<(), String> {
    let status = std::process::Command::new(binary)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|e| format!("could not start `{}`: {e}", binary.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`vexel-bench {}` failed: {status}", args.join(" ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Vec<String>, UsageError> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn plain_bench_runs_and_check_gates() {
        assert_eq!(p(&[]).unwrap(), ["run"]);
        assert_eq!(p(&["--check"]).unwrap(), ["check"]);
        assert_eq!(
            p(&["--check", "--hash-only", "--only", "fill"]).unwrap(),
            ["check", "--hash-only", "--only", "fill"]
        );
        assert_eq!(
            p(&["--update-baseline"]).unwrap(),
            ["check", "--update-baseline"]
        );
    }

    #[test]
    fn rejects_bad_flags() {
        assert!(matches!(
            p(&["--nope"]),
            Err(UsageError::UnknownFlag { .. })
        ));
        assert!(matches!(
            p(&["x"]),
            Err(UsageError::UnexpectedArgument { .. })
        ));
        assert!(matches!(
            p(&["--only"]),
            Err(UsageError::MissingValue { .. })
        ));
        assert!(matches!(
            p(&["--full", "--full"]),
            Err(UsageError::DuplicateFlag { .. })
        ));
    }

    #[test]
    fn build_is_a_locked_offline_release_of_the_harness() {
        let env = crate::bundle::cargo_env(Path::new("/r"), None, None);
        let line = release_build(Path::new("/r"), &env).line();
        assert!(line.contains("build -p vexel-bench --release --locked --offline"));
        assert!(binary(&env).ends_with("aarch64-apple-darwin/release/vexel-bench"));
    }
}
