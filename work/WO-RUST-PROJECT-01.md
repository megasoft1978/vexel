---
id: WO-RUST-PROJECT-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: de73e92ad4bb435f942d5118d8767df508c14173a054d2ec1c9bf28493a75a1d
parallel_safe: true
network: false
---
# Goal
Port strict native bundle v1 constants/binary little-endian layout/CRC32 validation/limits/input sizing/encoding/decoding/asset lookup/magic/error names. Own or lifetime-bound source with no dangling views; explicit typed Asset/Input/View/Error. Preserve all strict checks and byte-identical valid round trips, unknown opaque metadata bytes, version/count/duplicates/truncation/flags/CRC/overflow rejection. Scalar owned IEEE CRC matching current code, no new dependency. No timeline/overlay/recovery/app integration claimed; prevalidated capture candidate not in scope.

# Reference scope
src/project/project_bundle.c, src/project/project_bundle.h, tests/project bundle codec cases
Read OWNER-DIRECTIVES/AGENTS, ADR0022/0023, Rust goal/plan and this order first.

# Files allowed to touch
- `work/WO-RUST-PROJECT-01.md`
- `rust/crates/vexel-project/src/**`, `rust/crates/vexel-project/tests/**`
- `artifacts/WO-RUST-PROJECT-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths
All legacy C/ObjC/include/Python/shell source, original tests/fixtures/oracles, other Rust crates, Cargo manifests/lock/config/toolchain, docs/records. No main edits, dependencies, unsafe, global tool installs, network, broad app/UI/E2E/C tests, staging/commit/reset/clean, recursive agents. Preserve all dirty baseline files.

# Contracts and evidence
Safe idiomatic Rust2024, bounded input and checked arithmetic/fallible allocation, C public behavior accounted in parity-map.json (C symbol -> Rust symbol -> named focused tests or explicit documented API adaptation). Do not hide unimplemented named behavior behind TODO/unimplemented/stub. Preserve original assertions; port their cases to Rust, do not run old C suite. Source files<800lines, cohesive modules. Root Span/BaseStatus and geometry types remain stable; no shared-interface edits outside your crate. Build standalone package without modifying any workspace manifest. No full subsystem/product/performance claim beyond named scope.

# Execution and verification
Only .worktrees/rust-project; Luna high, one implementation plus one narrowed review repair. Root serializes source/manifest/ledger integration.
cargo +stable check -p vexel-project --locked --offline
cargo +stable test -p vexel-project --locked --offline
python3 scripts/wo_validate.py WO-RUST-PROJECT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Set CARGO_HOME/TMPDIR/CARGO_TARGET_DIR to this worktree .cache/cargo, .cache/tmp and build/rust, MACOSX_DEPLOYMENT_TARGET13. Verify rustc +stable version1.99.0; no downloads. Rustfmt/clippy are not installed; do not install or claim those run. Independent review, exact baseline/allowed path audit, frozen original tests and main integrated named checks required.

# Output
Leave uncommitted; verification.txt + parity-map.json under own artifact with exact checks/public API/adaptations/remaining scope. Final response<=450words. Do not modify coordinator state or claim acceptance yourself.
