# WO-RUST-TOOLING-CI-POLICY-01 result

Status: implementation complete; independent read-only review CLEAN. No commit or staging was performed because the frozen work order protects `git HEAD`/index and prohibits staging. The worktree remains a candidate.

Frozen snapshot: revision 1, `spec_sha256` `21f7dc4a0fc453ad243f1db93124fdb171cf09aace347e6ca7b042721e96b169`, supplied `base_sha` `d3daba12acc6e1307267218217d09e9307889710`.

The actual initial and final worktree `HEAD` is `c61b192e4c87d15264aae3efe809612d1441da54`, which differs from the supplied `base_sha`. `HEAD` and the index are unchanged.

## Implementation

- Added a dependency-free Rust text checker matching `scripts/check_ci.py` policy ordering, rejection messages, whitespace behavior, action pinning, and the case-sensitive `brew install librsvg` exception. It preserves the Python 3.12.8 / Unicode 15.0.0 word-boundary behavior used by that exception.
- Added `vexel-dev check-ci WORKFLOW.yml`, with exactly one path argument, exit 2 for invalid invocation, exit 1 with `CI policy error:` for read/policy errors, and success stdout `macOS-only CI policy: pass`.
- The CLI normalizes CRLF and CR like Python `Path.read_text()`. It does not invoke Python.
- No dependencies, workflow edits, Python reference edits, or protected-path changes.

## Verification

Commands below were run in the task worktree. The recorded argv list is shell-tokenized.

| Command | Exit | Outcome |
|---|---:|---|
| `cargo +stable test -p vexel-dev --locked --offline` | 0 | 37 passed, 0 failed. Covers current workflow, each policy rejection class, exception behavior, parser argument errors, regex parity edge cases, and LF/CRLF/CR normalization. |
| `cargo +stable run -p vexel-dev --locked --offline -- check-ci .github/workflows/ci.yml` | 0 | App stdout exactly `macOS-only CI policy: pass`. |
| `cargo +stable fmt --manifest-path Cargo.toml --package vexel-dev -- --check` | 0 | Passed. |
| `git diff --check` | 0 | Passed. |
| `python3 scripts/check_ci.py .github/workflows/ci.yml` | 0 | Reference output exactly `macOS-only CI policy: pass`. |
| `sh -c 'set +e; vexel-dev check-ci; status_code=$?; test "$status_code" -eq 2'` | 0 | Full wrapper below; executable returned 2 and printed usage to stderr. |
| `sh -c 'set +e; vexel-dev check-ci .github/workflows/ci.yml extra; status_code=$?; test "$status_code" -eq 2'` | 0 | Full wrapper below; executable returned 2 and printed the extra-argument diagnostic. |
| `sh -c 'set +e; vexel-dev check-ci build/check-ci-invalid.yml; status_code=$?; test "$status_code" -eq 1'` | 0 | Full wrapper below; executable returned 1 and printed `CI policy error: the workflow must have exactly one macos-15 runner`. Fixture was generated under ignored `build/`. |
| `sh -c 'set +e; vexel-dev check-ci build/no-such-workflow.yml; status_code=$?; test "$status_code" -eq 1'` | 0 | Full wrapper below; executable returned 1 with the `CI policy error:` prefix. |
| Direct CLI checks against LF, CRLF, and CR copies of the current workflow | 0 | All returned 0 and printed the exact success line. Independent review confirmed matching Python/Rust stdout, stderr, and status across those newline forms. |

The four shell wrappers above ran the following exact scripts (using the binary path shown):

```sh
set +e
build/rust/aarch64-apple-darwin/debug/vexel-dev check-ci >build/audit-missing.out 2>build/audit-missing.err
status_code=$?
printf 'missing-argument exit=%s\n' "$status_code"
cat build/audit-missing.err
test "$status_code" -eq 2
```

```sh
set +e
build/rust/aarch64-apple-darwin/debug/vexel-dev check-ci .github/workflows/ci.yml extra >build/audit-extra.out 2>build/audit-extra.err
status_code=$?
printf 'extra-argument exit=%s\n' "$status_code"
cat build/audit-extra.err
test "$status_code" -eq 2
```

```sh
set +e
build/rust/aarch64-apple-darwin/debug/vexel-dev check-ci build/check-ci-invalid.yml >build/audit-policy.out 2>build/audit-policy.err
status_code=$?
printf 'policy-violation exit=%s\n' "$status_code"
cat build/audit-policy.err
test "$status_code" -eq 1
```

```sh
set +e
build/rust/aarch64-apple-darwin/debug/vexel-dev check-ci build/no-such-workflow.yml >build/audit-read.out 2>build/audit-read.err
status_code=$?
printf 'missing-file exit=%s\n' "$status_code"
cat build/audit-read.err
test "$status_code" -eq 1
```

```sh
set +e
for path in .github/workflows/ci.yml build/check-ci-crlf.yml build/check-ci-cr.yml; do
  build/rust/aarch64-apple-darwin/debug/vexel-dev check-ci "$path" >"build/$(basename "$path").out" 2>"build/$(basename "$path").err"
  status_code=$?
  printf '%s exit=%s output=' "$path" "$status_code"
  cat "build/$(basename "$path").out"
  if test "$status_code" -ne 0; then exit 1; fi
done
```

The policy implementation also passed a Python/Rust differential audit of 2,395 cases and a full 1,114,112-code-point check of its frozen Unicode 15.0.0 word table; both reported zero mismatches.

Changed paths are limited to:

- `rust/tools/vexel-dev/src/args.rs`
- `rust/tools/vexel-dev/src/main.rs`
- `rust/tools/vexel-dev/src/ci_policy.rs`
- `artifacts/WO-RUST-TOOLING-CI-POLICY-01/result.md`
- `artifacts/WO-RUST-TOOLING-CI-POLICY-01/report.json` (local copy; see note below)

No manifest or lockfile diff exists. The tracked workflow and Python checker are unchanged. No command in the new Rust `check-ci` path invokes Python.

Audit commands: `git diff --exit-code -- Cargo.toml Cargo.lock rust/tools/vexel-dev/Cargo.toml .github/workflows/ci.yml scripts/check_ci.py` (exit 0); `git diff --check` (exit 0); `! rg -n 'std::process::Command|Command::new|python3' rust/tools/vexel-dev/src/ci_policy.rs` (exit 0); `! git status --short --untracked-files=all | awk '{print $2}' | grep -Ev '^(rust/tools/vexel-dev/src/args\\.rs|rust/tools/vexel-dev/src/main\\.rs|rust/tools/vexel-dev/src/ci_policy\\.rs|artifacts/WO-RUST-TOOLING-CI-POLICY-01/result\\.md|artifacts/WO-RUST-TOOLING-CI-POLICY-01/report\\.json)$'` (exit 0).

The requested report destination `/Users/megasoft78/Desktop/Freelance/vexel/artifacts/runs/WO-RUST-TOOLING-CI-POLICY-01/028e25d1-7269-45b8-8fd4-2e8815f0abcb/report.json` is outside the writable workspace. The attempted write was rejected with `operation not permitted`; a schema-shaped copy is saved at `artifacts/WO-RUST-TOOLING-CI-POLICY-01/report.json` in this worktree.
