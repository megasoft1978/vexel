---
id: WO-RUST-TOOLING-SBOM-01
revision: 2
status: READY
milestone: FULL-RUST-TOOLING
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 73195e6742f5911997d647594c6bceb413c2bd40
spec_sha256: 7fccbeff99055a929951cd05350aa70f30983ea6b170152952dee554b6921b55
parallel_safe: true
network: false
---
# Goal
Generate the application's CycloneDX SBOM entirely in the Rust release tool, removing its runtime dependency on `scripts/make_sbom.py`.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `docs/product/rust-port-goal-and-plan.md`
- Existing Rust entry point: `rust/tools/vexel-dev/src/release.rs::make_sbom`
- Python reference to preserve: `scripts/make_sbom.py`
- Existing runner/test contracts: `rust/tools/vexel-dev/src/runner.rs`, `release.rs` tests
- No GPL/editor sources or network access.

# Existing contracts and invariants
- Keep the existing `vexel-dev sbom [--out DIR]` CLI and output name `sbom.cyclonedx.json`.
- Output must be deterministic CycloneDX 1.5 JSON with version 1, Vexel application version, MPL-2.0, executable SHA-256, unique linked libraries sorted by full path, `pkg:generic/<basename>?os=macos` purls, and the existing system-library descriptions.
- Preserve the current behavior that a failing `otool -L` contributes no linked-library entries. A missing executable or SHA command failure must return a clear error.
- Do not add dependencies. macOS 13+ system commands `otool` and `shasum` may be invoked via the existing `Runner` interface.

# Files allowed to touch
- `rust/tools/vexel-dev/src/release.rs`
- `artifacts/WO-RUST-TOOLING-SBOM-01/**`

# Protected paths
- All other source, tests, fixtures, docs, manifests/lockfiles, Python reference tools, work orders, shared ledgers/status, git HEAD/index, and all C/Objective-C migration references.
- No dependencies, network, installs, GUI actions, commits to `main`, staging, resets or deletions.

# Required implementation
1. Replace the `python3 scripts/make_sbom.py` call with native Rust generation in `vexel-dev`.
2. Obtain the executable digest with `shasum -a 256` through `Runner`; parse and validate exactly 64 hexadecimal digest characters.
3. Obtain linked libraries with `otool -L` through `Runner`; retain the Python reference's nonzero-exit behavior (empty library list).
4. Emit valid UTF-8 JSON with correct string escaping, stable component ordering and no timestamp. Write the same filename under the selected output directory.
5. Add focused unit tests for valid output, deterministic ordering, JSON string escaping, missing executable, command errors and nonzero `otool` behavior using a fake runner.
6. Record exact commands and outcomes in `artifacts/WO-RUST-TOOLING-SBOM-01/result.md`.

# Acceptance criteria (machine-checkable)
- AC-1: `make_sbom` performs no Python invocation and writes `sbom.cyclonedx.json` using Rust code.
- AC-2: Unit tests parse/check the emitted JSON structure and pin CycloneDX version, app version, MPL-2.0, executable SHA-256, sorted unique libraries, purls, and descriptions.
- AC-3: Tests cover quotes, backslashes and control characters in emitted strings; malformed SHA output and missing executable fail without claiming an SBOM was written.
- AC-4: `cargo +stable test -p vexel-dev --locked --offline` exits 0.
- AC-5: `cargo +stable fmt --manifest-path Cargo.toml --package vexel-dev -- --check` and `git diff --check` exit 0; changed paths satisfy the allowlist.
- AC-6: No third-party dependency is added, and `make_sbom` no longer invokes Python.

# Verification commands
```text
cargo +stable test -p vexel-dev --locked --offline
cargo +stable fmt --manifest-path Cargo.toml --package vexel-dev -- --check
git diff --check
```

After those commands, inspect generated fixture JSON and verify fake-runner command invocations, scope, and Python non-invocation.

# Non-goals
Porting the general Python test runner, ledger/WO tooling, CI policy checks, changing SBOM schema/version, changing distribution signing, or deleting the Python migration reference.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 60 min; output-token soft cap: 10000; dependency/network: none.

# Failure and escalation
Stop if byte-compatible deterministic JSON requires a dependency or if the system SHA command is unavailable on the pinned target. Report exact evidence; do not silently omit the digest or invent linked-library metadata.

# Output
Candidate only until independent read-only review. Do not update shared status/ledgers or commit to `main` from the task worktree. Executor returns the required JSON report and `result.md` evidence.
