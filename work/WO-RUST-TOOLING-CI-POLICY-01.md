---
id: WO-RUST-TOOLING-CI-POLICY-01
revision: 3
status: READY
milestone: FULL-RUST-TOOLING
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 6aab4a76be6ca1ce03690e53840c12f72541ca8f
spec_sha256: deae5ce9de126b4dc39edeefda7be733f1f77dc4cbb9adffaf18f96dc67917cb
parallel_safe: true
network: false
---
# Goal
Port the macOS CI policy checker into `vexel-dev` while preserving the existing policy and command-line result.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `docs/product/rust-port-goal-and-plan.md`
- Python behavior reference: `scripts/check_ci.py`
- Current workflow: `.github/workflows/ci.yml`
- Rust CLI: `rust/tools/vexel-dev/src/{args,main}.rs`
- No GPL/editor sources or network access.

# Existing contracts and invariants
- macOS-only CI: exactly one `macos-15` runner; triggers are exactly push, pull_request, workflow_dispatch; explicit `contents: read`; no secret references; no package installs or cache actions except the existing `brew install librsvg` exception; all external `uses:` actions pinned to lowercase 40-character commit SHAs.
- Preserve the existing Python checker output on success (`macOS-only CI policy: pass`) and its policy-error behavior (exit 1); invalid invocation exits 2.
- Keep `actionlint` responsible for YAML syntax. The policy checker validates text policy as the existing Python tool does. Preserve the exact `re.MULTILINE` behavior of `scripts/check_ci.py` at its permissions regex, including `$` matching at the end of the `contents: read` line even when later permission lines follow; do not make the Rust policy stricter.
- No dependencies, no CI matrix or non-macOS support.

# Files allowed to touch
- `rust/tools/vexel-dev/src/args.rs`
- `rust/tools/vexel-dev/src/main.rs`
- `rust/tools/vexel-dev/src/ci_policy.rs`
- `artifacts/WO-RUST-TOOLING-CI-POLICY-01/**`

# Protected paths
- All other source, tests, fixtures, workflows, docs, Python reference tools, manifests/lockfiles, work orders, shared ledgers/status, git HEAD/index, and all C/Objective-C migration references.
- No dependencies, network, installs, GUI actions, commits to `main`, staging, resets or deletions.

# Required implementation
1. Add a pure Rust text checker matching `scripts/check_ci.py`'s exact policy checks and diagnostic messages.
2. Expose it through `vexel-dev check-ci WORKFLOW.yml`; keep the existing invocation style of one required path argument.
3. Preserve success stdout and policy-error prefix/exit semantics. Do not modify the workflow or Python reference.
4. Add tests for the current workflow and each policy rejection: runner count/value, missing triggers, permissions, secrets, install/cache exception, and unpinned/missing actions. Add CLI argument tests.
5. Record exact verification commands and outcomes in `artifacts/WO-RUST-TOOLING-CI-POLICY-01/result.md`.

# Acceptance criteria (machine-checkable)
- AC-1: `cargo run -p vexel-dev --locked --offline -- check-ci .github/workflows/ci.yml` exits 0 and prints exactly `macOS-only CI policy: pass`.
- AC-2: Rust unit tests pass the current workflow and reject each policy violation without changing or weakening `.github/workflows/ci.yml`; tests preserve exact Python regex behavior, including acceptance of a later permission line after `contents: read` under the existing multiline anchor semantics.
- AC-3: The parser reports an invalid or missing workflow argument with exit code 2; policy violations exit 1 and retain the Python reference diagnostics.
- AC-4: `cargo +stable test -p vexel-dev --locked --offline` exits 0.
- AC-5: `cargo +stable fmt --manifest-path Cargo.toml --package vexel-dev -- --check` and `git diff --check` exit 0; changed paths satisfy the allowlist.
- AC-6: No third-party dependency is added and the new Rust command does not invoke Python.

# Verification commands
```text
cargo +stable test -p vexel-dev --locked --offline
cargo +stable run -p vexel-dev --locked --offline -- check-ci .github/workflows/ci.yml
cargo +stable fmt --manifest-path Cargo.toml --package vexel-dev -- --check
git diff --check
```

Audit rejection cases, command exit codes, exact success output, and changed-path allowlist.

# Non-goals
Porting the Rust-format checker, CI workflow itself, YAML parser, other CI providers, Python reference removal, or general work-order/test tooling.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 60 min; output-token soft cap: 10000; dependency/network: none.

# Failure and escalation
Stop if faithfully preserving the text policy requires adding a YAML dependency or changing workflow behavior. Report the exact case; do not loosen policy to pass the current workflow.

# Output
Candidate only until independent read-only review. Do not update shared status/ledgers or commit to `main` from the task worktree. Executor returns the required JSON report and `result.md` evidence.
