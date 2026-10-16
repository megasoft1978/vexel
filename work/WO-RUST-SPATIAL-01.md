---
id: WO-RUST-SPATIAL-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 07df3d1d290917b798aea10edc7b859b8e924562b4269bfc54a0199c5d286e86
parallel_safe: true
network: false
---
# Goal and frozen scope
Port immutable owned spatial index create/count/query/destroy behavior, finite inclusive bounds, stable original record indices/query ordering,1million record/192MiB limits and all sizing/overflow/fallible allocation. Query returns required count and insufficient output capacity writes nothing. Input copies survive original mutation/drop; concurrent queries through immutable Rust ownership. Preserve C bounds rules without imposing unrelated geometry caps. Tests edges/degenerate bounds/duplicates/invalidfinite/limits/buffer-nonmutation/order/owned data and parallelqueries; no full millionrecord perf campaign.

# Reference paths
src/spatial/index.c, index.h, spatial-focused tests/cases

# Files allowed to touch
- `work/WO-RUST-SPATIAL-01.md`
- `rust/crates/vexel-spatial/src/**`, `rust/crates/vexel-spatial/tests/**`
- `artifacts/WO-RUST-SPATIAL-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All old C/ObjC/include/Python/shell, original tests/fixtures/oracles, other Rust crates, all Cargo manifests/lock/config/toolchain, coordinator docs/records. Preserve accepted same-crate behavior/assertions while extending it. No main edits, deps/downloads/system tools/formatters, unsupported OS code, C runtime shim, staging/commits/reset/clean, recursive agents/network, broad UI/E2E/C tests/corpus/perf campaign. Domain unsafe forbidden; only native-system order permits documented sys.rs boundary under ADR0024.

# Evidence and quality
Read OWNER-DIRECTIVES/AGENTS/ADR0022-0024/Rust goal then this order. Safe owned or lifetime-bound data, checked math and fallible bounded allocation. Map named Csymbols to Rustsymbols/focused tests/API adaptations in parity-map.json. No TODO/unimplemented/stub for namedscope; no whole-subsystem completion claim. Source<800lines, cohesive modules. If API changes outside scope needed stop with concrete dependency conflict, no manifest edit. Preserve original assertions and port meaningful cases, not merely is_err for diagnostic positions. Root reviews source and integrated checks before acceptance.

# Execution and verification
Only .worktrees/rust2-spatial; Luna high, one implementation plus one narrowed review repair, thin context. Root serializes integration/shared metadata.
cargo +stable check -p vexel-spatial --locked --offline
cargo +stable test -p vexel-spatial --locked --offline
python3 scripts/wo_validate.py WO-RUST-SPATIAL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact compiler1.99.0 via installed stable, repo-local CARGO_HOME/TMPDIR/target, MACOSX_DEPLOYMENT_TARGET13. No rustfmt/clippy installed; do not claim/install them. Workers only own src/tests/artifact; never root records.

# Output
Leave uncommitted. verification.txt/parity-map.json with exact behavior/checks/errors/adaptations/remaining scopes. Return<=450words. No additional billing/credit purchase; stop/checkpoint on rate/allowance errors instead of retryloop.
