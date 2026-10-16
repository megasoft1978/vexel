---
id: WO-RUST-BOOTSTRAP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: d2f2d320fd28beecea3b8898c81c9682d9c774b8be2146ebcf8f8b116f541d35
parallel_safe: false
network: false
---
# Goal
Create buildable std-only macOS Rust workspace and frozen foundation contracts for six parallel ports. Bootstrap is scaffolding, not application/parity acceptance.

# Files allowed to touch
- `work/WO-RUST-BOOTSTRAP-01.md`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/config.toml`
- `rust/README.md`, `rust/crates/*/Cargo.toml`, `rust/crates/*/src/lib.rs`, `rust/tools/vexel-dev/**`
- `docs/adr/0023-rust-foundation-workspace.md`, `docs/product/rust-port-goal-and-plan.md`
- `artifacts/WO-RUST-BOOTSTRAP-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
All legacy source/tests/fixtures/oracles/CMake and worker candidates. No external deps/downloads, global toolchain/config/account edits, system packages, unsupported OS stubs, app/UI test campaigns, staging/commit.

# Contract
Root-only serialized Cargo: Rust1.99.0/2024/resolver3, MPL2, internal path deps only, aarch64-apple-darwin/macOS13. Exact existing compiler verified via stable alias; cargo +stable avoids automatic version-named toolchain installation. Pin1.99.0 metadata; no global download. Generated build/cache/tmp repo-local. Domain crates forbid unsafe. Members base/geometry/xml/json/project/raster and Rust developer CLI. Shared Span(start,end:usize), checked from_offset_length and bounds-checked slice; BaseStatus Invalid/Overflow/Limit/OutOfMemory. Geometry Point(x,y:f64), Affine(a,b,c,d,e,f:f64), right-first compose. Other functions await port orders; no fake app or full-module completion claim. Developer CLI info/build/check only calls Rust Cargo, verifies exact installed compiler before build/check; no C/Python backend.

# Verification
rustc +stable --version (exact1.99.0)
cargo +stable generate-lockfile --offline
cargo +stable check --workspace --locked --offline
cargo +stable test -p vexel-base --locked --offline
cargo +stable run -p vexel-dev --locked --offline -- info
python3 scripts/wo_validate.py WO-RUST-BOOTSTRAP-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Repo-local CARGO_HOME/TMPDIR/target and macOS13 env. Preserve old source/index baseline; review workspace/contract/metadata JSON. No runtime/UI/performance claim.

# Output
Buildable foundation and next isolated module scopes, uncommitted, exact evidence in own artifact. No dependencies installed.
