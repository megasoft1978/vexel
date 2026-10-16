---
id: WO-RUST-RENDER-FIXTURE-SETUP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 12b66e441abbd10af67d6bb0b5f0a788c90292d1815e51dd49594114d09c7413
parallel_safe: true
network: false
---
# Goal
Close proven fixture-construction dependency blockers without fake SceneView
providers: add exact accepted firstparty dev dependencies only to seven new
render-owner crates. Existing LexicalDocument::parse supplies real XML tokens;
accepted svg-animation::parse_owned constructs actual OwnedAnimation and
svg-path::parse_path constructs owned shape input. RootSceneBuilder fixtures
exercise actual immutable sealed scenes and source ownership. No production
source/dependency/package change or smaller scope/default synthetic scene.

# Files allowed to touch
- work/WO-RUST-RENDER-FIXTURE-SETUP-01.md (freeze only)
- rust/crates/vexel-svg-paint/Cargo.toml (exact dev-dependencies only)
- rust/crates/vexel-svg-definitions/Cargo.toml (exact dev-dependencies only)
- rust/crates/vexel-svg-media/Cargo.toml (exact dev-dependencies only)
- rust/crates/vexel-scene-query/Cargo.toml (exact dev-dependencies only)
- rust/crates/vexel-svg-scalar-index/Cargo.toml (exact dev-dependencies only)
- rust/crates/vexel-svg-scalar-sample/Cargo.toml (exact dev-dependencies only)
- rust/crates/vexel-svg-animation-frame/Cargo.toml (exact dev-dependencies only)
- Cargo.lock (local dependency edges only)
- docs/adr/0028-first-party-render-fixtures.md
- docs/dependencies/render-fixtures.lock.json
- artifacts/WO-RUST-RENDER-FIXTURE-SETUP-01/**
- STATE.md, LEDGER.md, ledger.jsonl, docs/product/rust-port-goal-and-plan.md

# Protected paths
All source/tests/othermanifests/config/original603/157oracles/previousorders/
evidence/immutable catalogs/HEAD/index. PAINT live source inputs stay unchanged;
coordinator seeds accepted metadata only after that worker terminal. No new
external package/version/network/install/unsafe/GPL/unsupportedOS/GUI/broadUI/
stage/commit/reset/delete/paidcredits/billing switch. Standard existing allowance.

# Requirements
Exact fixture-dependencies.json additions as dev-dependencies only, all paths to
already accepted crates/workspace version0.1.0/MPL2. ADR/pinnedlocal provenance
records exact manifest/source baselines and prerequisite acceptance, no external
pins/checksums change. No unused new runtime dependency, no actual fixture or
renderer acceptance from metadata; meaningful original pure proofs follow full
source scopes. Independent LunaLow metadata review +freshworkspace build/order/
ledger/diff/protected audits before terminal peer seed/resume through dedicated
coordination order. Root alone manifests/locks/shared integration.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check --workspace --locked --offline
python3 scripts/wo_validate.py WO-RUST-RENDER-FIXTURE-SETUP-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact oldsource/tests/allothermanifests/original603/157/HEAD/index preserved;
external pins/version/checksums unchanged, no application or UI tests for metadata.

# Output
Accepted fixture graph registration, enabling complete original source/algorithm
assertions on real owned scenes. Full engines/nativeUI/liveMCP/performance/tooling/
finalQA/release/recorded-demo goal remains active and incomplete.
