---
id: WO-RUST-WAVE8-SETUP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: c323de1ad71fe8bda0a6d695b54efd0ef8546638bd4d4e0cffc695273b99ac2f
parallel_safe: false
network: false
---
# Goal
Register one std-only SVG animation crate and freeze the full SMIL parse/eval
scope plus native-binding audit. Accepted CSS is available; active source-editor
candidate remains isolated and is not consumed. Preserve the full product goal.

# Files allowed to touch
- `work/WO-RUST-WAVE8-SETUP-01.md`, `work/WO-RUST-SVG-ANIMATION-01.md`, `work/WO-RUST-NATIVE-BINDINGS-AUDIT-01.md`
- `Cargo.toml`, `Cargo.lock`
- `rust/crates/vexel-svg-animation/Cargo.toml`, `rust/crates/vexel-svg-animation/src/lib.rs`
- `artifacts/WO-RUST-WAVE8-SETUP-01/**`
- `artifacts/WO-RUST-WAVE7-SETUP-01/integrate_candidate.py` (exact accepted registration audit only)
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and contracts
All accepted Rust/source/assertions, original603legacy files, other manifests/
orders/evidence, active source-editor candidate and index/HEAD. New crate depends
only on accepted base/XML; no dependency on unfinished shape/editor/native
ports. One isolated LunaHigh animation writer; native binding audit is an
independent SolMedium documentation/evidence writer, no dependency adoption.
No system install, unsupportedOS, Cshim, stage/commit/reset/delete or broadUI/E2E.
Older editor integration accepts only exact accepted wave8 registration hashes.
Independent setup review required before implementation launch.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check --workspace --locked --offline
python3 scripts/wo_validate.py WO-RUST-WAVE8-SETUP-01 --ready
python3 scripts/wo_validate.py WO-RUST-SVG-ANIMATION-01 --ready
python3 scripts/wo_validate.py WO-RUST-NATIVE-BINDINGS-AUDIT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact Rust1.99/mac13/localcache; original603/index/acceptedRust preserved.

# Output
Accepted registrations/scopes only, fresh overlays and process registry; no
module/app/performance or final acceptance claim.
