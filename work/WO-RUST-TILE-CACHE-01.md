---
id: WO-RUST-TILE-CACHE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 13e4a1365c8477b027992641fe0a428f617b34d0ecda77bb33fda884d93fc429
parallel_safe: true
network: false
---
# Goal and frozen scope
Port full tile_cache.c/h mutable owner cache, creation/get/put/epoch/stats/drop and clock rollover test seam. Own accepted Tile values; borrowed get reference cannot survive mutable call in safe Rust. Put transfers ownership only on success; return unchanged owned tile on failure.256MiB/65536 entry caps; account actual reserved entry metadata plus tiles with checked arithmetic and fallible bounded allocation. Preserve C byte-cap boundary and LRU replacement/eviction/tie/clock-epoch rollover ordering; justify exact layout/charge adaptation if needed, never ignore metadata or report only live payload. Epoch change clears all entries synchronously; same epoch retains. Test budget boundaries, rejected tile ownership/caller data, replacement without spurious eviction, touched LRU, count/stats, generation/scale invalidation and clock wrap matching original cases. No huge throughput/performance campaign or actual render integration claim.

# Reference paths
src/raster/tile_cache.c/h, tile.h; focused tile-cache assertions in tests/raster; accepted vexel-raster Tile API
Legacy reference provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json; no external algorithms/source fetch.

# Files allowed to touch
- `work/WO-RUST-TILE-CACHE-01.md`
- `rust/crates/vexel-tile-cache/src/**`, `rust/crates/vexel-tile-cache/tests/**`
- `artifacts/WO-RUST-TILE-CACHE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy source/fixtures/oracles, accepted other-crate code, all Cargo/config/lock/toolchain metadata, other worker trees, coordinator records/docs. Existing same-crate accepted behavior/assertions preserved. Worker only own src/tests/artifacts; coordinator alone shared integration. No manifest edit, code outside scope, dependency/download/install, unsupported OS code, Cshim, staging/commit/delete/reset, network/recursive agents, broad UI/E2E/C/performance campaign. Safe owned/lifetime-bound data, bounded inputs/checked arithmetic/fallible significant allocations, no unsafe in new modules; preserve accepted platform sys boundary. Source<800lines. No TODO/unimplemented/production stub for named scope. Map every named C API/helper to Rust symbols, focused tests, source/diagnostic spans, ownership/limits/cleanup and exact safe adaptations in parity-map.json. If dependency or contract cannot meet full scope stop with concrete issue; do not narrow deliverable to convenient subset.

# Execution and verification
Only .worktrees/rust3-cache; Standard subscription LunaMedium, thin scope/context, one writer and focused independent review/repair. Read OWNER-DIRECTIVES/AGENTS and this order; consolidated goal governs full finish line. Preserve accepted underlying interfaces. Root serializes shared metadata/integration.
cargo +stable check -p vexel-tile-cache --locked --offline
cargo +stable test -p vexel-tile-cache --locked --offline
python3 scripts/wo_validate.py WO-RUST-TILE-CACHE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing1.99/macOS13, repo-local Cargo/cache/tmp/targets. No rustfmt/clippy installation or unsupported claims. Original fixtures/assertions unchanged; add focused Rust regressions with exact outputs, not only is_err. Run no extra broad suite.

# Output
Leave uncommitted; result<=400words, parity-map.json/verification.txt with actual checks, failures/repairs/adaptations and remaining app scope. Candidate only until independent review, exact scope/main audit and named integrated checks. Existing allowance only; no API billing/credits; stop/checkpoint on rate/allowance errors rather than retryloop.
