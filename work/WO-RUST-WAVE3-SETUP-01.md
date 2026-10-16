---
id: WO-RUST-WAVE3-SETUP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: b5d9dd1905f7b257205d3701c968c420cf0c14e0133ecc0aeee95a97f406af7c
parallel_safe: false
network: false
---
# Goal
Register and freeze six dependency-ready third-batch Rust ports using accepted foundation/value/UI/system modules. No implementation or parity claim for new shells.

# Files allowed to touch
- `work/WO-RUST-WAVE3-SETUP-01.md`
- `work/WO-RUST-SVG-PATH-01.md`, `work/WO-RUST-TIMELINE-01.md`, `work/WO-RUST-UI-DRAW-01.md`, `work/WO-RUST-ATOMIC-FILES-01.md`, `work/WO-RUST-TILE-CACHE-01.md`, `work/WO-RUST-TASK-QUEUE-01.md`
- `Cargo.toml`, `Cargo.lock`
- `rust/crates/vexel-svg-path/Cargo.toml`, `rust/crates/vexel-svg-path/src/lib.rs`
- `rust/crates/vexel-timeline/Cargo.toml`, `rust/crates/vexel-timeline/src/lib.rs`
- `rust/crates/vexel-tile-cache/Cargo.toml`, `rust/crates/vexel-tile-cache/src/lib.rs`
- `rust/crates/vexel-platform-tasks/Cargo.toml`, `rust/crates/vexel-platform-tasks/src/lib.rs`
- `docs/product/rust-port-goal-and-plan.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-RUST-WAVE3-SETUP-01/**`

# Protected paths and contracts
All accepted Rust implementations, existing crate manifests, legacy C/ObjC/Python/shell, fixtures/oracles, active XML worker, prior frozen work orders and evidence. Std/internal dependencies only; no downloads, external dependency decisions, new unsafe bindings, global installation, unsupported OS code, staging/commit/delete/reset. Register svg-path(base,geometry,svg-values), timeline(base,project), tile-cache(base,raster), platform-tasks(base). UI draw extends accepted ui crate and atomic files extend accepted platform-macos without manifest changes. All scopes disjoint from XML and each other, one writer per crate. Domain code forbids unsafe; atomic std filesystem code does not broaden accepted sys.rs FFI boundary. Each full named reference scope retained; adapters must preserve limits/errors/ownership and existing assertions, not replace actual functions with stubs. Coordinator shared changes only. Subscription-only Standard Luna High except routine tile-cache Medium.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check --workspace --locked --offline
python3 scripts/wo_validate.py WO-RUST-WAVE3-SETUP-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Validate all six new frozen orders and exact legacy603/index/accepted Rust implementation hashes. Exact existing1.99/macOS13; repo-local Cargo/cache/tmp/targets. No application/UI tests or broad suite for setup.

# Output
Buildable four new crate registrations, six ready scoped orders, current-source worktree baseline/process registry and truthful roadmap checkpoint. No module acceptance until independent review/main checks.
