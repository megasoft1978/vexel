---
id: WO-RUST-WAVE6-SETUP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 56e200f568615302cf1f2f66980372f26ad5ffc0d8c8eb2eb0f6a949c807ab15
parallel_safe: false
network: false
---
# Goal
Register/freeze4dependency-ready ports: full CSS/reference/source-save and additiveowned/largedocumentAPI. Three std-only crate shells; existing document manifest unchanged. Separate read-only native integration plan scope, no externaldependency adoption or framework code fromsetup.

# Files allowed to touch
- `work/WO-RUST-WAVE6-SETUP-01.md`
- `work/WO-RUST-SVG-CSS-01.md`
- `work/WO-RUST-SVG-REFERENCE-01.md`
- `work/WO-RUST-SOURCE-SAVE-01.md`
- `work/WO-RUST-DOCUMENT-OWNED-01.md`
- `work/WO-RUST-NATIVE-PLAN-01.md`
- `Cargo.toml`
- `Cargo.lock`
- `rust/crates/vexel-svg-css/Cargo.toml`
- `rust/crates/vexel-svg-css/src/lib.rs`
- `rust/crates/vexel-svg-reference/Cargo.toml`
- `rust/crates/vexel-svg-reference/src/lib.rs`
- `rust/crates/vexel-document-io/Cargo.toml`
- `rust/crates/vexel-document-io/src/lib.rs`
- `artifacts/WO-RUST-WAVE6-SETUP-01/**`
- `docs/product/rust-port-goal-and-plan.md`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`

# Protected and contracts
All acceptedRustsource/existingmanifests/9documenttests,603legacyfiles/oracles/earlierorders/evidence. Rootonlyshared metadata/interfacefreeze. css(base,xml), reference(base,xml), document-io(base,xml,platform-macos); ownedDocument extendsdocument(existingbase/xml). Reference typedlocalfailure avoids unfinishedshape dependency; later conversion explicit. OwnedDocument safeArc/topologysharing, no selfreference, preserveborrowedAPI/default16M/oldtests; large1GiB/64MiBtoken cap/cancellation andrawbytes. No workersconsumeunfinishedownedAPI; documenteditor followsacceptance. One writer/crate. Std-only/no downloads/install/newunsafe/deps/OS/staging/commit/delete/reset. Standardexisting subscriptionLunaHigh; nativeplan read-only SolMedium scopedarchitecture. Independent setup review beforelaunch; setupnotmoduleacceptance.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check --workspace --locked --offline
python3 scripts/wo_validate.py WO-RUST-WAVE6-SETUP-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Validate4portorders/nativereadonlyorder, original603/index/allacceptedRusthashes. Existing1.99/mac13/localcaches, no application/UItests.

# Output
Acceptedregistrations only; readyfullscopes/current-source overlays/processregistry; entirefullproductgoal/gates unchanged.
