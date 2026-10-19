---
id: WO-RUST-APP-TEST-FORMAT-01
revision: 1
status: READY
milestone: FULL-RUST-TOOLING
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 2b275861c818b815ea3ff4ff2c0a1adfa7fcc4d3
spec_sha256: f088d4a07bf65f9be4a8dc717fc1ff3ce8157439cae9cc241802cd650059a290
parallel_safe: true
network: false
---
# Goal
Format two existing `vexel-app` test sources so the package's pinned `cargo fmt --check` verification succeeds without changing test logic or goldens.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`
- Files currently reported by `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check` on the frozen base: `rust/crates/vexel-app/tests/gradient_golden.rs` and `rust/crates/vexel-app/tests/mcp_draw_character.rs`.
- No image/golden regeneration; preserve all test assertions and fixture bytes.

# Existing contracts and invariants
- This is formatting only. Do not change test statements, values, control flow, fixtures, or oracle strength.
- macOS 13+ Apple Silicon only. No dependencies or platform expansion.
- All source changes must be limited to the two listed test files.

# Files allowed to touch
- `rust/crates/vexel-app/tests/gradient_golden.rs`
- `rust/crates/vexel-app/tests/mcp_draw_character.rs`
- `artifacts/WO-RUST-APP-TEST-FORMAT-01/**`

# Protected paths
- All other source, tests, fixtures, goldens, docs, manifests/lockfiles, work orders, shared ledgers/status, git HEAD/index and all C/Objective-C migration references.
- No dependency changes, network, installs, GUI actions, test weakening, golden regeneration, commits to `main`, staging, resets or deletions.

# Required implementation
1. Apply formatter-only edits to the two named Rust test files; preserve every token other than whitespace/layout where possible.
2. Record a before/after source diff audit showing no assertion, literal, identifier, or test behavior changed.
3. Record exact verification commands and outcomes in `artifacts/WO-RUST-APP-TEST-FORMAT-01/result.md`.

# Acceptance criteria (machine-checkable)
- AC-1: `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check` exits 0.
- AC-2: Changed paths are exactly the two named test files plus the required evidence artifact; no fixture/golden changed.
- AC-3: `cargo +stable test -p vexel-app --locked --offline` exits 0 with existing frozen tests unchanged.
- AC-4: `git diff --check` exits 0 and no dependency is added.

# Verification commands
```text
cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check
cargo +stable test -p vexel-app --locked --offline
git diff --check
```

# Non-goals
Implementation changes, test refactoring, changing assertions or fixtures, broad UI/E2E, and formatting the rest of the workspace.

# Budget
Executor max runs: 1; verifier: 1; elapsed hard limit: 30 min; output-token soft cap: 6000; dependency/network: none.

# Failure and escalation
Stop if rustfmt would require a semantic source change or would alter a frozen test oracle. Report exact evidence; do not bypass the formatter check.

# Output
Candidate only until independent read-only review. Do not update shared status/ledgers or commit to `main` from the task worktree. Executor returns the required JSON report and `result.md`.
