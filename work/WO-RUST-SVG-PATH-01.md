---
id: WO-RUST-SVG-PATH-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 30fca048df6a30760553fc3f87647506b016483d44cc8b605aa04c842e3982b8
parallel_safe: true
network: false
---
# Goal and frozen scope
Port every function in shape_path.c (absolute/relative M/L/H/V/C/S/Q/T/A/Z, implicit/repeated commands, reflected controls, subpaths/closure, elliptical arcs, command/number provenance) plus required add_edge/add_curve/flattening helpers from shape_geometry.c and transformed flatness/number reader helpers in shape.c. Safe owned PathDocument edges/subpaths/segments/numbers with checked source Span offsets and accepted Point/Affine; expose parse_path(source, span, transform, options) returning complete owned path data or typed diagnostic with exact original offsets. Preserve4M-command/edge and28M-number caps, coordinate/affine limits, depth16/transformed tolerance and exact grammar. Preserve path source number and segment spans for future node edits. Test all commands, relative/implicit/smooth/reset rules, compact arc flags, degenerate/corrected arcs, transform/tolerance, coordinate/limits/failure diagnostics and original path-number provenance cases. Local input views may adapt accepted SVG values helpers; do not impose a16MiB cap on the entire containing SVG source or reduce C hard limits. Larger integration/shape styles/XML scene traversal are later; no full SVG/render claim.

# Reference paths
src/svg/shape/shape_path.c, shape_geometry.c (required edge/curve helper functions only), shape.c (number/tolerance helpers), shape.h, shape_internal.h; tests/svg/shape/path_number_test.c and scoped path assertions in existing shape tests/fixtures
Legacy reference provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json; no external algorithms/source fetch.

# Files allowed to touch
- `work/WO-RUST-SVG-PATH-01.md`
- `rust/crates/vexel-svg-path/src/**`, `rust/crates/vexel-svg-path/tests/**`
- `artifacts/WO-RUST-SVG-PATH-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy source/fixtures/oracles, accepted other-crate code, all Cargo/config/lock/toolchain metadata, other worker trees, coordinator records/docs. Existing same-crate accepted behavior/assertions preserved. Worker only own src/tests/artifacts; coordinator alone shared integration. No manifest edit, code outside scope, dependency/download/install, unsupported OS code, Cshim, staging/commit/delete/reset, network/recursive agents, broad UI/E2E/C/performance campaign. Safe owned/lifetime-bound data, bounded inputs/checked arithmetic/fallible significant allocations, no unsafe in new modules; preserve accepted platform sys boundary. Source<800lines. No TODO/unimplemented/production stub for named scope. Map every named C API/helper to Rust symbols, focused tests, source/diagnostic spans, ownership/limits/cleanup and exact safe adaptations in parity-map.json. If dependency or contract cannot meet full scope stop with concrete issue; do not narrow deliverable to convenient subset.

# Execution and verification
Only .worktrees/rust3-paths; Standard subscription LunaHigh, thin scope/context, one writer and focused independent review/repair. Read OWNER-DIRECTIVES/AGENTS and this order; consolidated goal governs full finish line. Preserve accepted underlying interfaces. Root serializes shared metadata/integration.
cargo +stable check -p vexel-svg-path --locked --offline
cargo +stable test -p vexel-svg-path --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-PATH-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing1.99/macOS13, repo-local Cargo/cache/tmp/targets. No rustfmt/clippy installation or unsupported claims. Original fixtures/assertions unchanged; add focused Rust regressions with exact outputs, not only is_err. Run no extra broad suite.

# Output
Leave uncommitted; result<=400words, parity-map.json/verification.txt with actual checks, failures/repairs/adaptations and remaining app scope. Candidate only until independent review, exact scope/main audit and named integrated checks. Existing allowance only; no API billing/credits; stop/checkpoint on rate/allowance errors rather than retryloop.
