---
id: WO-RUST-NATIVE-BINDINGS-SETUP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: aff857afaac562867aff9be853812a634cd85217748e1a729886da798626f5c4
parallel_safe: false
network: true
---
# Goal
Review/freeze the native Apple ABI/dependency decision, then register the minimal
pinned published native binding features in one platform-only Rust crate and
freeze real main-thread/panic/foreign-message-boundary implementation. Full Rust
product goal remains; no native GUI or macOS13 runtime acceptance by setup.

# Files allowed to touch
- `work/WO-RUST-NATIVE-BINDINGS-SETUP-01.md`, `work/WO-RUST-NATIVE-BOUNDARY-01.md` (coordinator freeze only)
- `docs/adr/0026-rust-apple-bindings-and-abi.md`
- `docs/dependencies/objc2-macos.lock.json`, `docs/provenance.md`
- `Cargo.toml`, `Cargo.lock`
- `rust/crates/vexel-platform-apple/Cargo.toml`, `rust/crates/vexel-platform-apple/src/lib.rs` (registration shell only)
- `artifacts/WO-RUST-NATIVE-BINDINGS-SETUP-01/**`
- `artifacts/WO-RUST-WAVE7-SETUP-01/integrate_candidate.py` (exact acceptedregistration hashes only)
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Sequencing and protected contracts
Read OWNER-DIRECTIVES/AGENTS. Accepted native audit pins/feature/16package graph
are source. ADR and exact provenance reviewed BEFORE Cargo adoption/fetch.
The only justified feature extension is Foundation NSException for real guarded
exception probes/diagnostics, no new package. Direct dependencies pinned exactly,
defaults disabled, host-build pins separated; no dev/example/nonApple features.
Cargo may download ONLY the approved16archives to repo-local.cache/cargo with
checksum verification; no system install or unreviewed dependency. Confirm lock
resolved graph/features/buildtarget against reviewed plan, mac13deployment and
actualcompiler+linking. Resolve any mismatch explicitly; do not weaken graph or
constraints. Third-party published ObjC exceptionhelper is a reviewed FFI runtime
dependency, not firstparty C shim. No foreign exception escapes public platform
entry/ABIcallback. Scoped audited C-unwind inside dependency fence is explicit;
no blanket zero foreignunwind everywhere or cleanup recovery claim. First-party
code/tooling stays Rust, customUI/ownedSVG/mac13AppleSilicon. No GPLsource,
externalSVG fetch, unsupportedOS, stage/commit/delete/reset/recursiveagents or
broadUI/E2E. All acceptedRust/legacy603/fixtures/index/earlier orders protected.
Older editor integration checks EXACTaccepted setup hashes, not unaccepted ones.
Independent ADR review before registration and setup review before writer launch.

# Verification after reviewed ADR
cargo +stable generate-lockfile
cargo +stable fetch --locked --target aarch64-apple-darwin
cargo +stable check --workspace --locked --offline
python3 scripts/wo_validate.py WO-RUST-NATIVE-BINDINGS-SETUP-01 --ready
python3 scripts/wo_validate.py WO-RUST-NATIVE-BOUNDARY-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Compiler exactRust1.99, MACOSX_DEPLOYMENT_TARGET13/localcache/tmp/targets;
record registry URLs/checksums, actual lock/features/transitive16graph and source
hash preservation. No dependency/CLI retries on quota or mismatched input.

# Output
Reviewed ADR/pinned provenance/compiled registrations only; frozen complete
boundary APIs and current-source isolated writer after setup accepted. Remaining
native shell/input/IME/text/image/Metal/menu/AX and all product gates explicit.
