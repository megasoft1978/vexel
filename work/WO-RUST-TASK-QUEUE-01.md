---
id: WO-RUST-TASK-QUEUE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 3f2f48a95e25bf16842778c8419423f98560ad3acafed163f8558c7d284d44da
parallel_safe: true
network: false
---
# Goal and frozen scope
Port full platform_task.c/h bounded native queue contract to Rust std threads/synchronization: capacity1..64/workers1..4, submit/shared or owned contexts, IDs/generations, duplicate/stale/full/stopped errors, queued/running cooperative cancel, generation advancement/discard, polling/retirement, shutdown joins and cleanup exactly once outside mutex. Safe Send+'static closures/Arc-owned shared context replace unsafe borrowed void*; failure must return ownership and never run cleanup for rejected submission. Public CancellationToken, TaskResult{id,generation,succeeded,value}, TaskQueue and typed statuses; no stub C callback invocation. Freeze exact semantics/status priority to C; owned completion/cleanup and cross-thread handoff, no data races/deadlocks/leaks, catch user callback panic so queue lifecycle remains bounded (document Rust-only adaptation). Fallible significant bounded allocations/thread-start error rollback, explicit safe test allocation/failure seam for preserved failure cases; do not falsely claim deterministic system OOM injection. Test pending vs running cancel, generation races, duplicates/full, ownership/failure cleanup/reentrancy, no cancel-vs-success publication race, thread-start/shutdown/quiescence. Pure native worker checks only, no app/UI tests. Platform macOSarm guard via base; no unsupported OS shims, no unsafe/deps. MCP/export job wiring later.

# Reference paths
src/platform/macos/platform_task.c, src/platform/platform_task.h; focused native task driver/queue cases in tests/platform; accepted base and native task-related behavioral records
Legacy reference provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json; no external algorithms/source fetch.

# Files allowed to touch
- `work/WO-RUST-TASK-QUEUE-01.md`
- `rust/crates/vexel-platform-tasks/src/**`, `rust/crates/vexel-platform-tasks/tests/**`
- `artifacts/WO-RUST-TASK-QUEUE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy source/fixtures/oracles, accepted other-crate code, all Cargo/config/lock/toolchain metadata, other worker trees, coordinator records/docs. Existing same-crate accepted behavior/assertions preserved. Worker only own src/tests/artifacts; coordinator alone shared integration. No manifest edit, code outside scope, dependency/download/install, unsupported OS code, Cshim, staging/commit/delete/reset, network/recursive agents, broad UI/E2E/C/performance campaign. Safe owned/lifetime-bound data, bounded inputs/checked arithmetic/fallible significant allocations, no unsafe in new modules; preserve accepted platform sys boundary. Source<800lines. No TODO/unimplemented/production stub for named scope. Map every named C API/helper to Rust symbols, focused tests, source/diagnostic spans, ownership/limits/cleanup and exact safe adaptations in parity-map.json. If dependency or contract cannot meet full scope stop with concrete issue; do not narrow deliverable to convenient subset.

# Execution and verification
Only .worktrees/rust3-queue; Standard subscription LunaHigh, thin scope/context, one writer and focused independent review/repair. Read OWNER-DIRECTIVES/AGENTS and this order; consolidated goal governs full finish line. Preserve accepted underlying interfaces. Root serializes shared metadata/integration.
cargo +stable check -p vexel-platform-tasks --locked --offline
cargo +stable test -p vexel-platform-tasks --locked --offline
python3 scripts/wo_validate.py WO-RUST-TASK-QUEUE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing1.99/macOS13, repo-local Cargo/cache/tmp/targets. No rustfmt/clippy installation or unsupported claims. Original fixtures/assertions unchanged; add focused Rust regressions with exact outputs, not only is_err. Run no extra broad suite.

# Output
Leave uncommitted; result<=400words, parity-map.json/verification.txt with actual checks, failures/repairs/adaptations and remaining app scope. Candidate only until independent review, exact scope/main audit and named integrated checks. Existing allowance only; no API billing/credits; stop/checkpoint on rate/allowance errors rather than retryloop.
