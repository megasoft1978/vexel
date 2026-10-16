---
id: WO-RUST-SCENE-DRAFT-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 80aba9ca54ae778f579146351b15f3a0a158159056bbbd64dba8b8896d265033
parallel_safe: false
network: false
---
# Full named scope
Close the exact deferred decoder-postoperation dependency with safe scene draft
transactions. Opaque nonborrowing tokens permit a Decoder to own its builder
without selfreferences. Bounded64-level LIFO graph draft stack; children only
created within parent scope, parent mutations paused until child closes. Public
append marks/rollback prohibited while draft active; existing behavior unchanged
outside drafts. Internal append checkpoints/rollback retain real atomicity and
cannot strand active tokens after an error. Pending draft forbids seal/freeze;
commit validates actual scene, rollback drops all scope-created tables/media/
children and maintains actual capacities/history/monotonic handle identities.
Only records created after the authorizing scope checkpoint may be finalized:
shape attributes, exact edgecolors, transform/clip/mask/filter/group and sparse
fill/stroke paint records. Preserve structural/range/source/parent constraints,
checked nested-payload capacity/transient overlap, same admission caps, active
source/handle identity and cancellation before publication. Controlled validation
and copy loops; no partial mutation on invalid/limit/cancel, preserve base records.
No fake parser/renderer/native provider, no loose mutable table escape hatch.
Keep actual43existing assertions/tests byteidentical. Named proof: pre-scope,
stale/wrong/wrong-top tokens/handles reject; nested ordinary mark can't bypass
scope; commit pending children fails, canceledcommit remains rollbackable; after
failed append token stillusable; child rollback invalidates handles and releases
nested capacities; exact deferred shape/paint/clip/group/filter/mask/transform/
color finalization and freeze; invalidmutation or cancellation keepsprior records;
root abort removes descendants, prior tables survive, allocation/work history
not rewound; internal/ordinary checkpoints behavior unchanged. Source/tests<800.

# Files allowed to touch
- work/WO-RUST-SCENE-DRAFT-01.md (coordinator freeze only)
- rust/crates/vexel-scene/**
- artifacts/WO-RUST-SCENE-DRAFT-01/**
- STATE.md, LEDGER.md, ledger.jsonl, docs/product/rust-port-goal-and-plan.md

# Protected paths
All manifests/config/other Rust/platform/product, source603/157oracles, previous
frozen orders/evidence/index. Nativewindow isolated writer disjoint; no newdeps/
network/download/install/GPL/editorSource/unsupportedOS/Cshim/newunsafe/agents/
stage/commit/reset/delete/GUI/broadUI/E2E. Root owns sharedcontracts; independent
LunaHigh review +freshmaincheck/protected audit before peers consume APIs.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py WO-RUST-SCENE-DRAFT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact original43tests/protected603/157oracle/HEAD/index audit. Local existing
Rust1.99/cache/tmp, AppleSilicon mac13+, Standard subscription only.

# Output
Complete buildable tested scoped draft-finalization gate, API/parity/adaptation
report; full actual engines/native/application/performance/finalQA/release/demo
work remains, goal active.
