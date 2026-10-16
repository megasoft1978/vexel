---
id: WO-RUST-PROTOCOL-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 1b83f57278d6aad84ba343bace11b3806472b6ff6988d8887de7ca6bf3f9e71d
parallel_safe: true
network: false
---
# Goal and frozen scope
Port full owned sync+opt-in async protocol, version/rate/init/discovery/tools/resources/prompts and schema registry, Tasks validation/negotiation/get/cancel and legacy deferred/cancel/progress helper. Safe Rust backend traits replace void pointers; no app/OS/broker implementation stubs. Preserve2026-07-28 per-request support/readiness, metadata merging/envelopes/original IDs, flatTask creation metadata vs DetailedTask terminalpayload and emptycancelACK; completed isError vs failedRPC distinction. Legacy2025 deferred+progress/cancel, no new experimentalTasks API. Current Tasks never progressnotifications. Bound payload/contentblocks/ISOtimestamps/task IDs NUL/progress tokens/priorrollback. Tests parse EVERY emitted JSON and assert envelope+payload+IDs/capabilityreset, syncfallback, malformed backend/metadata, legacycancelwithoutresponse. Production app integration remains outside scope; fake test backends are okay.

# Reference paths
src/mcp/mcp_protocol.c/h, mcp_protocol_async.c/h, mcp_tools.c/h and owned tool schema source; tests/mcp protocol+async suites and docs/mcp; accepted Rust JSON core

# Files allowed to touch
- `work/WO-RUST-PROTOCOL-01.md`
- `rust/crates/vexel-protocol/src/**`, `rust/crates/vexel-protocol/tests/**`
- `artifacts/WO-RUST-PROTOCOL-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All old C/ObjC/include/Python/shell, original tests/fixtures/oracles, other Rust crates, all Cargo manifests/lock/config/toolchain, coordinator docs/records. Preserve accepted same-crate behavior/assertions while extending it. No main edits, deps/downloads/system tools/formatters, unsupported OS code, C runtime shim, staging/commits/reset/clean, recursive agents/network, broad UI/E2E/C tests/corpus/perf campaign. Domain unsafe forbidden; only native-system order permits documented sys.rs boundary under ADR0024.

# Evidence and quality
Read OWNER-DIRECTIVES/AGENTS/ADR0022-0024/Rust goal then this order. Safe owned or lifetime-bound data, checked math and fallible bounded allocation. Map named Csymbols to Rustsymbols/focused tests/API adaptations in parity-map.json. No TODO/unimplemented/stub for namedscope; no whole-subsystem completion claim. Source<800lines, cohesive modules. If API changes outside scope needed stop with concrete dependency conflict, no manifest edit. Preserve original assertions and port meaningful cases, not merely is_err for diagnostic positions. Root reviews source and integrated checks before acceptance.

# Execution and verification
Only .worktrees/rust2-protocol; Luna high, one implementation plus one narrowed review repair, thin context. Root serializes integration/shared metadata.
cargo +stable check -p vexel-protocol --locked --offline
cargo +stable test -p vexel-protocol --locked --offline
python3 scripts/wo_validate.py WO-RUST-PROTOCOL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact compiler1.99.0 via installed stable, repo-local CARGO_HOME/TMPDIR/target, MACOSX_DEPLOYMENT_TARGET13. No rustfmt/clippy installed; do not claim/install them. Workers only own src/tests/artifact; never root records.

# Output
Leave uncommitted. verification.txt/parity-map.json with exact behavior/checks/errors/adaptations/remaining scopes. Return<=450words. No additional billing/credit purchase; stop/checkpoint on rate/allowance errors instead of retryloop.
