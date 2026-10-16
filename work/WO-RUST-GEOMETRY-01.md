---
id: WO-RUST-GEOMETRY-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: e17f4ad8ec927bfef5e317f560682fc7d4c9fcc1529a5a87e033cc2217bb74e8
parallel_safe: true
network: false
---
# Goal
Port finite point checks, affine identity/validation/right-first composition, point mapping and bounds mapping with existing f64 operation semantics. Preserve frozen Point/Affine fields. Reject nonfinite/intermediate overflow/invalid dimensions consistently, no FMA/SIMD change. Public free functions finite_point, affine_identity, affine_valid, affine_compose, affine_map_point, affine_map_bounds returning typed Option/Result where C returned bool/out. Tests cover composition order, all rectangle corners, zero/negative dimensions and NaN/infinity/overflow.

# Reference scope
src/geom/geometry.c, src/geom/geometry.h and geometry reference cases
Read OWNER-DIRECTIVES/AGENTS, ADR0022/0023, Rust goal/plan and this order first.

# Files allowed to touch
- `work/WO-RUST-GEOMETRY-01.md`
- `rust/crates/vexel-geometry/src/**`, `rust/crates/vexel-geometry/tests/**`
- `artifacts/WO-RUST-GEOMETRY-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All legacy C/ObjC/include/Python/shell source, original tests/fixtures/oracles, other Rust crates, Cargo manifests/lock/config/toolchain, docs/records. No main edits, dependencies, unsafe, global tool installs, network, broad app/UI/E2E/C tests, staging/commit/reset/clean, recursive agents. Preserve all dirty baseline files.

# Contracts and evidence
Safe idiomatic Rust2024, bounded input and checked arithmetic/fallible allocation, C public behavior accounted in parity-map.json (C symbol -> Rust symbol -> named focused tests or explicit documented API adaptation). Do not hide unimplemented named behavior behind TODO/unimplemented/stub. Preserve original assertions; port their cases to Rust, do not run old C suite. Source files<800lines, cohesive modules. Root Span/BaseStatus and geometry types remain stable; no shared-interface edits outside your crate. Build standalone package without modifying any workspace manifest. No full subsystem/product/performance claim beyond named scope.

# Execution and verification
Only .worktrees/rust-geometry; Luna medium, one implementation plus one narrowed review repair. Root serializes source/manifest/ledger integration.
cargo +stable check -p vexel-geometry --locked --offline
cargo +stable test -p vexel-geometry --locked --offline
python3 scripts/wo_validate.py WO-RUST-GEOMETRY-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Set CARGO_HOME/TMPDIR/CARGO_TARGET_DIR to this worktree .cache/cargo, .cache/tmp and build/rust, MACOSX_DEPLOYMENT_TARGET13. Verify rustc +stable version1.99.0; no downloads. Rustfmt/clippy are not installed; do not install or claim those run. Independent review, exact baseline/allowed path audit, frozen original tests and main integrated named checks required.

# Output
Leave uncommitted; verification.txt + parity-map.json under own artifact with exact checks/public API/adaptations/remaining scope. Final response<=450words. Do not modify coordinator state or claim acceptance yourself.
