---
id: WO-RUST-SCENE-CONSUMER-CONTRACTS-01
revision: 2
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 5a7715ca1f24df4402f16b291a8e1ace25667ca6102621160915deb6e7443edd
parallel_safe: false
network: false
---
# Concrete consumer gaps
Close real PAINT and ANIMATION worker blockers without shrinking their full scope.
Shared PaintCache needs source-coherent complete resolution context keys including
viewport, inverse and geometry bounds, plus actual owned stops; storage measures
all capacities with source counted once. ResolveContext validates source-bound
cache identity through actual retained per-entry SourceOwner witnesses and
controlled validation. Preserve existing three-field PaintCache literal by storing
owned stops/source/full context in GradientCacheEntry; measure nested stop capacities
without recounting retained source allocation. No gradient
resolver substitute: PAINT full actual algorithm stays required.

SceneView supplies checked serial/owner/scene-bound handles for every existing
shape/transform/clip/pattern/mask/filter/group slot, including zero-edge shapes.
Out-of-range/foreign/stale/reused handles fail; no unchecked public constructor
or mutable escape. SceneGraph can enumerate checked scene identity. Effective
frame views preserve exactly the same handles and source ownership.

EvaluatedFrame permits transactional full clip-table replacement after finish.
Validate all records/unchanged parent structure/IDs/geometry, work/cancel and
64MiB clip cap before publication; preserve original configured owned byte budget
and account actual old/new capacity overlap/history. Failure keeps effective
frame, base scene and stats unchanged. No partial edit or finished frame mutation
without validation. Clip-owner transforms must use current effective transforms.
Every original57scene test unchanged; add meaningful enumeration/stale/frame/
foreignsource/cachecapacity/atomic-cancel/limit/parent-regression proof.

# Files allowed to touch
- work/WO-RUST-SCENE-CONSUMER-CONTRACTS-01.md (freeze only)
- rust/crates/vexel-scene/src/**
- rust/crates/vexel-scene/tests/consumer_contracts.rs (new only)
- rust/crates/vexel-scene/README.md
- artifacts/WO-RUST-SCENE-CONSUMER-CONTRACTS-01/**
- STATE.md, LEDGER.md, ledger.jsonl, docs/product/rust-port-goal-and-plan.md
Coordinator may seed exact independently accepted source delta to PAINT/ANIMATION
isolated shared source inputs only after their processes are confirmed terminal,
recording exact before/after hashes and full original scopes retained. Do not
mutate live peer inputs or immutable catalogs; new contract supplement is recorded
in own artifacts/consumer-seed.json and copied as read-only coordinator evidence.

# Protected paths
All manifests/config/other Rust/tests/peer code/immutable catalogs/previous
orders/evidence/source603/157oracles/HEAD/index. No newdeps/network/GPL/download/
install/newunsafe/unsupportedOS/stage/commit/reset/delete/GUI/broadUI/E2E. All
live workers remain isolated. Source/tests<800. Root sharedonly; independent
LunaHigh review, fresh scoped check/test/WO/ledger/diff and exact protected audit
before consumers resume. Standard existing allowance only, no API billing.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py WO-RUST-SCENE-CONSUMER-CONTRACTS-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact original57test files and603/157frozenoracles/acceptedpeerRust/HEAD/index
preserved. Named interface fixes only, no worker/app/fullQA completion claim.

# Output
Accepted complete shared consumer interfaces, actual tests/review/audit and
terminal-worker seed/resume; full engines/native/UI/application/release/demo goal
remains active and unfinished.

# R2 compatibility clarification
R1 initial cache layout is preserved, not accepted. R2 retains original PaintCache
struct fields so original57tests stay byte-identical, moves owned stops and actual
SourceOwner witness into each entry, and provides bounded controlled source/context/
stop-range validation and capacity measurement. Full resolver still PAINT-owned.
