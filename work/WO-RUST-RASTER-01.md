---
id: WO-RUST-RASTER-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: bbc1a87e2fc2116e98a6521ad37f4f3ab3c4141737b8adf603e0cda24f507510
parallel_safe: true
network: false
---
# Goal
Port RGBA tile creation/destruction/storage contracts using safe owned fallible Vec or boxed bytes. Preserve checked dimensions/byte-count/stride/pixel representation/error states. Expose Tile width/height/byte_count and immutable/mutable RGBA slices, explicit Limits only if preserving existing callers requires configurable budget. Drop replaces manual destroy, no dangling pointer/doublefree. Tests zero/overflow/limit/error cleanup, dimensions/zero initialization/row storage per current semantics. This is tile storage only, no full renderer or visual parity claim.

# Reference scope
src/raster/tile.c, src/raster/tile.h and existing tile/storage cases
Read OWNER-DIRECTIVES/AGENTS, ADR0022/0023, Rust goal/plan and this order first.

# Files allowed to touch
- `work/WO-RUST-RASTER-01.md`
- `rust/crates/vexel-raster/src/**`, `rust/crates/vexel-raster/tests/**`
- `artifacts/WO-RUST-RASTER-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All legacy C/ObjC/include/Python/shell source, original tests/fixtures/oracles, other Rust crates, Cargo manifests/lock/config/toolchain, docs/records. No main edits, dependencies, unsafe, global tool installs, network, broad app/UI/E2E/C tests, staging/commit/reset/clean, recursive agents. Preserve all dirty baseline files.

# Contracts and evidence
Safe idiomatic Rust2024, bounded input and checked arithmetic/fallible allocation, C public behavior accounted in parity-map.json (C symbol -> Rust symbol -> named focused tests or explicit documented API adaptation). Do not hide unimplemented named behavior behind TODO/unimplemented/stub. Preserve original assertions; port their cases to Rust, do not run old C suite. Source files<800lines, cohesive modules. Root Span/BaseStatus and geometry types remain stable; no shared-interface edits outside your crate. Build standalone package without modifying any workspace manifest. No full subsystem/product/performance claim beyond named scope.

# Execution and verification
Only .worktrees/rust-raster; Luna medium, one implementation plus one narrowed review repair. Root serializes source/manifest/ledger integration.
cargo +stable check -p vexel-raster --locked --offline
cargo +stable test -p vexel-raster --locked --offline
python3 scripts/wo_validate.py WO-RUST-RASTER-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Set CARGO_HOME/TMPDIR/CARGO_TARGET_DIR to this worktree .cache/cargo, .cache/tmp and build/rust, MACOSX_DEPLOYMENT_TARGET13. Verify rustc +stable version1.99.0; no downloads. Rustfmt/clippy are not installed; do not install or claim those run. Independent review, exact baseline/allowed path audit, frozen original tests and main integrated named checks required.

# Output
Leave uncommitted; verification.txt + parity-map.json under own artifact with exact checks/public API/adaptations/remaining scope. Final response<=450words. Do not modify coordinator state or claim acceptance yourself.
