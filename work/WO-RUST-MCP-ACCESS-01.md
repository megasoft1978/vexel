---
id: WO-RUST-MCP-ACCESS-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: ef11d3120093d47f208eed2e98a6dff37d434600a0529730f3c0e7060196c11e
parallel_safe: true
network: false
---
# Goal and frozen scope
Port full mcp_access.c grants/resolution behavior to safe std filesystem paths using accepted native current_uid. Public existing regular-file resolution, path_allowed, save-path resolution (existing or new target canonical parent), granted directory resolution. Preserve 1023byte/NUL/input/output bounds, canonical symlink behavior and directory component boundary (including root), ENV colon allowed dirs bounded8191bytes, owner-only non-symlink config regular file/permissions, max128 lines and bounded per-line parsing/trimming/comments, exact deny/allow outcomes. Explicit owned AccessPolicy environment/home inputs plus from_environment adapter to avoid tests modifying global env; default production semantics match C. Config reader memory bounded; do not follow untrusted config symlink or relax owner/mode checks. Native TOCTOU behavior from reference must be documented, no claim security guarantee beyond source contract. All fixtures repo-local .cache/access-* only, no read/change real user config or unrelated files. Test canonical paths/symlinks/collision names, missing new parent, traversal escapes, root grants, deny malformed/oversized/nonregular, insecure config permissions/symlink, line cap and env precedence. No unsafe/deps/unsupported OS or caller atomic publication wiring; app permission UI later.

# Reference paths
src/mcp/mcp_access.c, mcp_broker_internal.h (access declarations/path cap only); focused access/save grant fixtures in tests/mcp
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-MCP-ACCESS-01.md` (coordinator freeze only)
- `rust/crates/vexel-mcp-access/src/**`, `rust/crates/vexel-mcp-access/tests/**`
- `artifacts/WO-RUST-MCP-ACCESS-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust4-access; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-mcp-access --locked --offline
cargo +stable test -p vexel-mcp-access --locked --offline
python3 scripts/wo_validate.py WO-RUST-MCP-ACCESS-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
