---
id: WO-RUST-SCENE-MODEL-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: b4b48fcfaa8409e2fc05bde34802b6921bd70c7532d7f471a3e3135fd66ba840
parallel_safe: false
network: false
---
# Full named scope
Implement the complete owned SCENE DATA MODEL gate in vexel-scene, consuming
accepted geometry/path/values/animation/raster/base. All SourceOwner, IDs/ranges/
errors/ignored/source/transform/path/stroke/paint/shape/viewport/image/clip/
pattern/mask/filter/group/text/media and graph/builder/accounting model groups
from accepted render-contract plan. This model is prerequisite for full decoder
and renderer; later shared kernel/decoder continuation interfaces and full
sampling/raster/selection/animation composition are still required, not stubs
or reduced renderer here. Coordinator owns shared contracts and registration.

Graph immutable/source-coherent Send+Sync Rust data; same Arc<Vec<u8>> owner as
accepted OwnedDocument; no copiedsource/selfreferences/nativeobjects. Child
scenes seal bottom-up, masks link sealedchild only, immutableviews of all tables.
Private handles bind graph/scene/slot and monotonically assigned insertion
serial: reject crossgraph/crossscene/stale-after-rollback handles. Per-edge shape
indices stay compact u32 in validated scoped tables; materialized ShapeId checks
scope. Record this safety adaptation from proposed unscoped u32 wrappers.
All complete scene tables retained, sparse paints/stops/dashes, exact rawsource
ranges/colors/pathmetadata, parent ancestry/clips/groups and typed mask/filter/
pattern/media records. No successful fake scene decoding/native provider.

Fallible checked builder mutators for every table, append shape consumes real
PathDocument plus per-edge inputcolors (proposal omitted required color input;
explicit ShapeInput adaptation), rebases edges/subpaths/segmentnumbers without
changing coordinates/commands/implicit markers/source spans. Atomic logical
publication: all reserves before related table writes; errors no partialshape.
Checkpoint/rollback truncates all related tables/resources and serials, rejects
stale marks; counters/work/allocation history not fabricated or rolled back.
Before growth create checked replacement Vec if needed, inspect actualcapacity
before moving olditems, bound retained and transient overlap by caller memory
policy. Actual Vec capacities incl nested media/clip/tile/animation arrays and
source capacity once; no logical-byte accounting presented as retained memory.
Source/shape/edge/path/dash/filter/pattern/mask/text/image caps preserve original
macros; caller smaller policy explicit, no smaller default parity envelope.
Seal/freeze validate every table range/reference/finitegeometry/viewport/stride/
source span/parent ordering before immutablegraph publication, cancellation and
work charged in loops through accepted WorkControl; legacy no-stop adapter.

# Required scoped proof
Named model tests: sourceowner pointer/capacitycoherence and caller-drop/worker
move; alltable immutableviews; exact color/path range rebases; wrong/stale handles
and stale marks reject; sealing/DAG/parent/range/image invariants; real memory
capacity incl nested/padded/pattern buffers and overlap; small cap failure and
cancel no partialshape; rollback drops resources and preserves prior handles;
no partially published graph, original parser tests/fixtures unaffected.
This newmodel gate has no existing sceneRust tests to weaken; original full
renderer/golden assertions remain their separate full port/composition gates.
Source/tests<800, cohesive modules, safeRust/minimaldeps/no externalfetch/GPL.

# Files allowed to touch
- work/WO-RUST-SCENE-MODEL-01.md (coordinator freeze only)
- Cargo.toml, Cargo.lock (coordinator local pathregistration only)
- rust/crates/vexel-scene/**
- artifacts/WO-RUST-SCENE-MODEL-01/**
- STATE.md, LEDGER.md, ledger.jsonl, docs/product/rust-port-goal-and-plan.md

# Protected and execution
All other product/Rust code/config/legacy603/oracles/previousorders/evidence/index.
No network/newthirdpartydeps/install/unsupportedOS/Cshim/newunsafe/stage/commit/
reset/delete/broadUI/E2E. Rootdirect sharedmodel writer; independent Luna review
before acceptance; no peers consume unaccepted model. ExactRust1.99+stable/mac13
localcache/tmp, existing subscription/no extra billing. Keep full product scope.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py WO-RUST-SCENE-MODEL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact protected/root audit and independent review; no renderer/nativeapp/large
workflow/performance/fullQA claim. Record all APIs/types/adaptations/real checks.
