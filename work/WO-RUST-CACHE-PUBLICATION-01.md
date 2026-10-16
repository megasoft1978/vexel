---
id: WO-RUST-CACHE-PUBLICATION-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 5aac5d4cb2b6ae7ce01346c5b81e3267cbe09d8672a165de941dec3cd7b95e07
parallel_safe: false
network: false
---
# Logical cache generations and physical overlap admission
Start ONLY after source-owned initialization contract is independently accepted,
fresh-main checked, and previous shared writer/reviewer terminal. Coordinator
seeds exact current accepted Scene/tests and schedules one shared writer across
this and shape-publication order; never mutate while another is live. Retain
all original accepted assertions/literals/APIs and603/157source/oracles.

Confirmed scalar R2 blocker: current RasterJob.check_pool enforces combined
16MiB Dash+Grid across all physically live old+staged caches; check_grid likewise
counts both generations against65536cells/1000000refs. Original retained bound is
per cache. Reproducer builds exactly16777216-byte cache then replacement fails
before any allocation under caller256MiB physical ledger. Staging/rebuild must
retain old usable cache on failure, so release-old-before-build/Scratchbuffers/
copy-unmeteredVec is not valid. Original review missing-cap premise is clarified
in scalar coordinator-cache-cap-clarification.md; owned per-cache guard now exists.

Provide public safe opaque same-job/source logical-cache generation/replacement
transaction (or equivalent owning state). Existing public pool APIs/default caps
stay compatible. New explicit production construction/replacement keeps each
logical cache at16MiB actual retained bytes across shapes/segments/joins/grids,
65536retained cells and1000000refs; caller's ChargeLedger separately admits ALL
physically simultaneous old/new payload/actual native capacities/old-new buffer
growth/transaction metadata. No arbitrary doubled global byte limit or zero-byte
probe/posthoc accounting. Both role/source/job/serial identity and mixed/stale/
foreign/replayed/duplicate-commit checks remain strict. Buffer handles cannot
escape into wrong cache scope and bypass logicalcap; no raw Vec/lease escapes.
Transaction capacities/history allocation/fallible cleanup honest and bounded.

Lower builder must be able to construct segment/join/shape/grid buffers using
actual accepted mutable job APIs and commit them without copying. Cache read/
composition validates actual current cache and owns a borrow preventing release/
commit invalidation; no per-sample full cache revalidation or row allocation.
Keep source-only/copy metadata and original JobDashCache literals compatible;
new provenance can be private job state/opaque ticket or additive descriptor.
Success atomically publishes actual staged handles and retires old ownership;
error/cancel/failed admission/validation/abort keeps old handles/payload/reading
usable and releases new logical ownership. Drop/abandoned stage cannot yield a
successful apparently finalized cache/job with unresolved ownership; physical
history and retained registry capacity need not rewind. Never raise or remove
frozen defaultpool caps or weaken assertions to satisfy new transaction.

Prove real near-cap replacement success with old+new physical overlap under
explicitly sufficient ledger; insufficientphysicalbudget fails leavingold; logical
aggregate limit refuses independent ofphysicalbudget/stepcap; real cells/refs
old+new remain independently bounded per generation; replay/mixedscope/wrongjob/
source/stale/cancel/abort/repeated failure cleanup and actual composition reads.
Convert scalar's existing passing defect reproducer into required success proof
ONLY in downstream adoption after this gate, not by changing its originaldata.
Do not port the lower scalar engine here. Expose complete usable lower example.

# Allowed paths
- rust/crates/vexel-scene/src/** (cache allocation/publication ownership only)
- rust/crates/vexel-scene/tests/cache_publication.rs (new only)
- rust/crates/vexel-scene/README.md
- artifacts/WO-RUST-CACHE-PUBLICATION-01/** (except coordinator frozen baseline/seed)
Coordinator alone freezes metadata/manifests/locks/shared registration outside
this source scope, integrates main, seeds peers and updates docs/state/ledgers.

# Protected paths
All oldtests/assertions/literals/othercrates/manifests/config/lock/source603/157/
oldorders/evidence/HEAD/index/state/docs/ledgers. No unsafe/newdeps/network/GPL/
Cshim/install/unsupportedOS/agents/reviewCLI/GUI/broadUI/stage/commit/reset/delete/
APIbilling/credits. Source/tests<800, bounded checked fallible safe ownership/work.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py WO-RUST-CACHE-PUBLICATION-01 --ready
Named Sol6.1Medium resource lifetime/accounting design; Standard subscriptiononly,
stopquota/rateerrors no retry/billingchange. Exactverification/APIcoverage and
result<=450words. Root independent LunaHigh review/protectedaudit/freshmain before
acceptance or seeding. Full renderer/native/app/finalQA/release/demo pending.
