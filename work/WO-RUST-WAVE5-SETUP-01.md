---
id: WO-RUST-WAVE5-SETUP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 35116d81eeedaa1f1379252cb6c47486a7a09ba8471f4ce4a243298d51cfb9d7
parallel_safe: false
network: false
---
# Goal
Register/freeze two newly ready XML-index and document-model ports consuming accepted XML2 and taskqueue contracts. Two std-only shells; no implementation acceptance.

# Files allowed to touch
- `work/WO-RUST-WAVE5-SETUP-01.md`
- `work/WO-RUST-SVG-INDEX-01.md`
- `work/WO-RUST-DOCUMENT-MODEL-01.md`
- `Cargo.toml`
- `Cargo.lock`
- `rust/crates/vexel-svg-index/Cargo.toml`
- `rust/crates/vexel-svg-index/src/lib.rs`
- `rust/crates/vexel-document/Cargo.toml`
- `rust/crates/vexel-document/src/lib.rs`
- `artifacts/WO-RUST-WAVE5-SETUP-01/**`
- `artifacts/WO-RUST-WAVE4-SETUP-01/integrate_candidate.py`
- `docs/product/rust-port-goal-and-plan.md`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`

# Protected paths and contracts
All accepted Rust source/existing manifests, original603legacy files/oracles/assertions, prior frozen orders and evidence, active wave4 worker trees. No deps/download/install/network/unsafe/C shim/unsupported OS/stage/commit/delete/reset. One writer per distinct newcrate; mac13+ only. Index safe caller-borrowed Arc<Vec> plus cloned worker owner avoids self-reference; actual background bounded validation required. Model borrows source and owns bounded tables/handles. Root alone shared interfaces/integration; update older wave4 helper only to accept exact new registration hashes after this setup accepted. Standard subscription LunaHigh, no APIbilling/extra credits/recursive agents. Independent setup review before launch.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check --workspace --locked --offline
python3 scripts/wo_validate.py WO-RUST-WAVE5-SETUP-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Validate both new frozen orders and original603/index/acceptedRust source hashes. Existing1.99/localcaches/mac13. No application/UI tests for setup.

# Output
Accepted registrations only, ready scopes, current-source isolated overlays/process registry and complete product goal retained.
