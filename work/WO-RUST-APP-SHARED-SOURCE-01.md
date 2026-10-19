---
id: WO-RUST-APP-SHARED-SOURCE-01
revision: 1
status: READY
milestone: LARGE-DOCUMENT-MEMORY
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 55e85d120392fa747b1aded1016a320532c334c2
spec_sha256: 8556be931371b066c472aa86905533c832701859c8b9fd26a8bb83554ecab9b0
parallel_safe: true
network: false
---
# Goal
Remove the redundant full-document copy between the composition worker and SVG decoder while preserving the existing borrowed-byte API and all current behavior.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `docs/product/rust-port-goal-and-plan.md`, `STATE.md`
- Relevant source: `rust/crates/vexel-app/src/compose/{mod,decode,worker}.rs`
- Existing evidence: `STATE.md:1768` (measured large-document memory amplification); `rust/crates/vexel-app/src/compose/decode.rs` currently copies borrowed bytes into a new `Arc<Vec<u8>>`; the worker already owns `Arc<Vec<u8>>`.
- Frozen acceptance: current `vexel-app` compose tests, especially `compose/tests_open.rs` worker supersession/cancellation and `compose/tests_wire.rs` output checks.

# Existing contracts and invariants
- Apple Silicon macOS 13+ only; safe Rust; no dependencies or platform expansion.
- Keep the 32 MiB source cap, 8x token-storage budget, charge-ledger limits, diagnostics, cancellation checkpoints, render behavior and current public borrowed-byte compose APIs.
- The worker's `Arc<Vec<u8>>` must become the exact owner consumed by the lexical document; do not clone the complete byte buffer during the worker-owned route.
- Do not change open/save source preservation or cache publication semantics.

# Files allowed to touch
- `rust/crates/vexel-app/src/compose/mod.rs`
- `rust/crates/vexel-app/src/compose/decode.rs`
- `rust/crates/vexel-app/src/compose/worker.rs`
- `rust/crates/vexel-app/src/compose/tests_open.rs`
- `artifacts/WO-RUST-APP-SHARED-SOURCE-01/**`

# Protected paths
- All other source, tests, fixtures, goldens, work orders, artifacts, manifests, lockfiles, docs, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, git HEAD/index, and all C/Objective-C sources or migration references.
- No dependencies, network, installs, GUI actions, commits to `main`, staging, resets or deletions.

# Required implementation
1. Add an owned-source composition/decode path that accepts `Arc<Vec<u8>>` and carries it into `LexicalDocument` without making another source-sized copy.
2. Route `ComposeWorker` through the owned path. Keep the existing borrowed-byte APIs behavior-compatible by adapting them once at the boundary.
3. Preserve current cancellation, latest-generation behavior, source diagnostics and all limits. Do not broaden the file-size cap.
4. Add focused tests showing the owned and borrowed routes produce identical results and the existing supersession/cancellation behavior remains intact. Keep tests within the allowed test file.
5. Record changed paths and exact verification results in `artifacts/WO-RUST-APP-SHARED-SOURCE-01/result.md`.

# Acceptance criteria (machine-checkable)
- AC-1: The worker route passes its existing Rust `Arc<Vec<u8>>` owner into the lexical document; code inspection shows no full-buffer `to_vec()` or equivalent copy in that route.
- AC-2: A focused regression test composes the same fixture through borrowed and owned routes and compares the complete frame, disclosures, and shape results.
- AC-3: Existing worker latest-wins and cancellation tests pass unchanged; source-cap and UTF-8/error mapping tests pass unchanged.
- AC-4: `cargo +stable test -p vexel-app --locked --offline` exits 0.
- AC-5: `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check` exits 0; `git diff --check` exits 0; changed paths satisfy the allowlist.
- AC-6: The result report makes no claim that this single-copy reduction meets the 100 MiB first-visible-content or full large-document goal.

# Verification sequence
1. `cargo +stable test -p vexel-app --locked --offline`
2. `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check`
3. `git diff --check`
4. Review exact owned-source flow, cancellation/error paths, and changed-path allowlist; save evidence under the artifact directory.

# Non-goals
Streaming or lazy decode, any cap changes, benchmark target changes, edits to scene/XML interfaces, UI changes, or claims about 100 MiB files.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 60 min; output-token soft cap: 10000; dependency/network: none.

# Failure and escalation
Stop if preserving the borrowed API requires changing shared manifests/interfaces or if the scene/XML owner cannot accept the existing `Arc<Vec<u8>>` without copying. Report the exact owner/API blocker; do not expand this order to redesign storage.

# Output
Candidate only until independent read-only review. Do not update shared status/ledgers or commit to `main` from the task worktree. Executor returns the required JSON report and `result.md` evidence.
