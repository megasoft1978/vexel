---
id: WO-RUST-WAVE7-SETUP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: ccc21710dd5803b343cf03aa385f7e2cb539ffd3fac996296e2b2e1e9537bd54
parallel_safe: false
network: false
---
# Goal
Register/freeze full source-editor port after acceptedownedDocument13/sourceSave5 interfaces. One newstd-only crate, no application/editor implementation acceptance.

# Files allowed to touch
- `work/WO-RUST-WAVE7-SETUP-01.md`, `work/WO-RUST-DOCUMENT-EDIT-01.md`
- `Cargo.toml`, `Cargo.lock`
- `rust/crates/vexel-document-edit/Cargo.toml`, `rust/crates/vexel-document-edit/src/lib.rs`
- `artifacts/WO-RUST-WAVE7-SETUP-01/**`
- `artifacts/WO-RUST-WAVE6-SETUP-01/integrate_candidate.py` (root approvedregistration audit only)
- `docs/product/rust-port-goal-and-plan.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl` (coordinatoronly)

# Protected / contracts
All acceptedRust implementations/existingmanifests/fixtures603legacy andearlierorders/evidence, activeCSS candidate. Source editor consumesonlyaccepted base/xml/document/document-io, notunfinishedCSS/native bindings. One isolatedLunaHigh writer. Owned cachedmodel+Arc original/insertblob/pieces/snapshots, no selfreference; fullall10Cfiles+viewinline scope;16Mlegacy/default/max limits/state/error semantics andallmeaningfuloracles retained. Full100Meditable/saveworkflow remainslater explicitgoal, notnarrowed. Root only sharedmetadata/integration. No externaldeps/install/network/newunsafe/Cshim/unsupportedOS/stage/commit/delete/reset;Standard existing subscription. AuditolderCSS sharedmetadata againstEXACTaccepted setup7 manifesthashes only. Independentsetupreview beforelaunch.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check --workspace --locked --offline
python3 scripts/wo_validate.py WO-RUST-WAVE7-SETUP-01 --ready
python3 scripts/wo_validate.py WO-RUST-DOCUMENT-EDIT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact1.99/mac13/localcache; original603/index/acceptedRust hashes retained. No app/UItests forsetup.

# Output
Acceptedregistration only, fullreadyscope/current-source overlay/processregistry, unmodifiedwholeproduct goal andfinal gates.
