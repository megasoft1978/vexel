---
id: WO-RUST-WAVE2-SETUP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: ccd731b2e35b72d852af152b3db770b909fe7231f52fa50d77c0c988e0ac51bf
parallel_safe: false
network: false
---
# Goal
Serially register six second-batch Rust scopes after accepted foundations; no implementation/parity claim for new crate shells.

# Files allowed to touch
- `work/WO-RUST-WAVE2-SETUP-01.md`, `Cargo.toml`, `Cargo.lock`
- `rust/crates/vexel-spatial/Cargo.toml`, `rust/crates/vexel-spatial/src/lib.rs`
- `rust/crates/vexel-svg-values/Cargo.toml`, `rust/crates/vexel-svg-values/src/lib.rs`
- `rust/crates/vexel-protocol/Cargo.toml`, `rust/crates/vexel-protocol/src/lib.rs`
- `rust/crates/vexel-ui/Cargo.toml`, `rust/crates/vexel-ui/src/lib.rs`
- `rust/crates/vexel-platform-macos/Cargo.toml`, `rust/crates/vexel-platform-macos/src/lib.rs`
- `docs/adr/0024-rust-native-system-ffi.md`, `docs/product/rust-port-goal-and-plan.md`
- `artifacts/WO-RUST-WAVE2-SETUP-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths and contract
All legacy C/fixtures and accepted Rust implementations unchanged. Std/internal deps only; no third-party crates/downloads/global installs. New members spatial(base), SVGvalues(base,geometry), protocol(base,json), UI(base,raster), platformmacos(base). Existing XMLcrate gets later tokenizer order without shared manifest edits. Domain crates forbid unsafe; platform crate denies unsafe except explicitly allowed tiny sys module in its native-system workorder, documented FFI ABI/lifetimes and macOS-only compile guards, no first-party C shim. Planned safe APIs fill_random<=256bytes with zero-on-failure, current_uid, peer_credentials(BorrowedFd) using Apple Security/System OS functions. References fetched2026-10-14: Rustnomicon FFI, Apple SecRandomCopyBytes/kSecRandomDefault Markdown and getpeereid manual; no code copied. These services are not AppKit/UI completion. SVGvalues may adapt C token-array attr lookup to explicit validated name/value span attributes; full XML/source/scene coupling not fabricated. UI order only all functions defined ui_core.c incl basic rectangles; ui_draw/widgets/icons remain separate. Full tokenizer order preserves current reader assertions. Coordinator alone owns manifests/interfaces/ledgers; six disjoint writers after registration. Source files<800.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check --workspace --locked --offline
python3 scripts/wo_validate.py WO-RUST-WAVE2-SETUP-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Repo-local Cargo/cache/tmp, exactexisting1.99/macOS13, no GUI/runtime/performance claim.

# Output
Validated workspace registrations and native FFI ADR; no code ports accepted by scaffolding.
