---
id: WO-RUST-LINT-CLEANUP-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 9be6d0bf5f7458ee81d78311afacb2b0ce8e518f68b56c777cabe12b7632a7a3
parallel_safe: true
network: false
---
# Goal
Remove two compiler-confirmed, behavior-neutral cleanup items that are already recorded in accepted project evidence: an unnecessary `mut` in `session/tests_playback.rs` and an unused import in `tests/ui_canvas_pointer.rs`. Do not alter test behavior, workload, assertions, or performance thresholds.

# Evidence and scope
- Existing accepted result artifacts report the `unused_mut` warning at `rust/crates/vexel-app/src/session/tests_playback.rs:140`.
- Existing accepted result artifacts report an unused import at `rust/crates/vexel-app/tests/ui_canvas_pointer.rs:11`.
- Allowed source files: `rust/crates/vexel-app/src/session/tests_playback.rs`, `rust/crates/vexel-app/tests/ui_canvas_pointer.rs`.
- Allowed evidence: `artifacts/WO-RUST-LINT-CLEANUP-01/**`.
- Coordinator alone owns this work order and `ledger.jsonl`.

# Acceptance
- AC-1: only the two cited unused items are removed; no surrounding behavior or oracle changes.
- AC-2: final diff shows the two exact cleanups and no unrelated edits.
- AC-3: required Rust and repository checks pass in the consolidated final verification phase, after all planned implementation slices are complete.

# Protected paths
All other source, tests, manifests/locks, dependencies, workflows, thresholds/oracles, ADRs, owner directives, work orders/evidence outside this order, external apps, and non-macOS platform code. Do not run tests, builds, benchmarks, or GUI checks during implementation; coordinator runs the consolidated checks only after implementation is complete.

# Final verification
```text
cargo check --workspace --locked
cargo test --workspace --locked -- --test-threads=1
python3 -m unittest tests.test_ledger tests.test_wo_runner
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py --ready WO-RUST-LINT-CLEANUP-01
git diff --check
```
