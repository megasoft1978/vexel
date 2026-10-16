---
id: WO-RUST-MCP-TASK-STORE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 4633ebbe2c523001ab1ac4d734c0d75b9cb7984a22abf5ee198e5bef1daeb488
parallel_safe: true
network: false
---
# Goal and frozen scope
Port full mcp_task_store.c/h to safe concurrent owned TaskStore using Mutex and accepted native fill_random for real128bit IDs. Preserve active2/retained8/total10, result2MiB, TTL900000ms, message127bytes/32hexID, owner isolation, start/update/request_cancel/cancel_requested/finish/get/status-name and exact lifecycle/expiry/reuse/eviction/progress/time overflow behavior. Result JSON remains opaque bounded bytes, owned and copied only as needed; get must preserve capacity rejection/no partial outputs via explicit snapshot/result API. Successful entropy failure must return error, no weak fallback or ID reused; test injection may be safe deterministic generator, production always native entropy. Limit-sensitive allocation fallible; lock poisoning policy documented, no callback under lock. Port original meaningful task tests (owner mismatch, active cap, retained eviction, expiration active cancellation, terminal retention, result error flags, zero/oversized result, concurrent access, times/overflow). Keep task store separate from accepted protocol crate; caller protocol/queue/export/app wiring later, no claim liveMCP.

# Reference paths
src/mcp/mcp_task_store.c/h; tests/mcp/mcp_async_task_test.c; accepted vexel-platform-macos entropy
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-MCP-TASK-STORE-01.md` (coordinator freeze only)
- `rust/crates/vexel-mcp-tasks/src/**`, `rust/crates/vexel-mcp-tasks/tests/**`
- `artifacts/WO-RUST-MCP-TASK-STORE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust4-task-store; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-mcp-tasks --locked --offline
cargo +stable test -p vexel-mcp-tasks --locked --offline
python3 scripts/wo_validate.py WO-RUST-MCP-TASK-STORE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
