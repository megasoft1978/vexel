---
id: WO-RUST-RASTER-JOB-CONTRACTS-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: bc6b175bdc86a4e8045624a71c76ca5fb87aa51866826509be1a43a0370b4eaa
parallel_safe: true
network: false
---
# Entire confirmed shared raster job gap
Close actual RASTER_INDEX/RASTER_SAMPLE acceptance blockers through safe owning
job resource APIs. Scalar scratch/dash/grid/bounds/fill/rows/output must reserve
against the SAME ChargeLedger before allocation/publication and retain checked
source/job-bound leases for real owned buffer lifetimes. Actual Vec capacities,
nested payloads, registry metadata, old/new overlap, reuse/growth/replacement/
release and failure cleanup/history must be accounted, source once. No post-hoc
only counter disguised as admission, no saturation/unsafe global allocator/RSS
claim. Metadata growth itself admitted; allocation failure leaves logical buffers
and owner/lease identity intact, cancellation before publication. Preserve strict
original local scratch/dash/grid/source/output caps, no smaller default renderer.

Provide publicly usable complete allocation/reservation/ownership interfaces for
index and sample writers, compatible all existing public struct literals and
original86tests byte-identical. SceneBuilder memory reserve implementation may
inform safe pattern; no unchecked caller mutation/lease escape/disarmed double
release or source mismatch. Efficient per-buffer operations, no cloned whole
source or hidden copy allocations. Owning wrappers/explicit job lifetime/resource
release contract acceptable if fully implemented/documented/proved; no infallible
Drop allocation or leaking live data accounting. Do not claim RAII release when
explicit release is required; job destruction must clean up owned data and all
ledger metadata honestly. Prove wrong/stale/double/mismatched lease rejection and
logical/physical failure distinctions. Avoid selfreferences/borrowed ledger that
prevents concurrent same-job scratch/child/output resource use.

Controlled paint sampling must receive actual WorkControl through real variable
loops, with Result error/cancel propagation. Preserve original KernelResources
requiredmethod/literal implementations for oldtests via a separate complete
controlled interface or equivalent safe additive design. No default fake paint,
precharging/aftercharge instead of in-loop polling, unconditionalUnsupported
production placeholder or smaller stoplimit. Actual production coordinator
adapter implements new controlled interface; old unmetered fixture capability
cannot silently become the production controlled sampler. Provide clear typed
new lower entrypoint seams so sample/index can consume concrete controlled
resources while original same sample data semantics/cost/4x4/oracle behavior stay.

# Files allowed to touch
- rust/crates/vexel-scene/src/** (shared interfaces/owning operations only)
- rust/crates/vexel-scene/tests/raster_job_contracts.rs (new only)
- rust/crates/vexel-scene/README.md
- artifacts/WO-RUST-RASTER-JOB-CONTRACTS-01/** (except immutable baseline/inputseed)
Coordinator owns orderfreeze/manifests/shared integration/STATE/LEDGER/fullplan.
Writer isolatedcurrent-source tree, no other shared writer duringthis task.

# Protected paths
All manifests/config/otherRust/original86tests/peerproduction/source603/157oracles/
previousorders/evidence/catalogs/rootHEAD/index/state/docs/ledgers. Source/tests
<800, split cohesive helpers. No network/newdep/install/GPL/unsafe/unsupportedOS/
Cshim/stage/commit/reset/delete/agents/GUI/broadUIE2E/APIbilling/credits.
Actual full index/sample engines/algorithms/adapters remain separate owner scopes;
this is complete actual shared ownership/control gate, not an empty renderer.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py WO-RUST-RASTER-JOB-CONTRACTS-01 --ready
Original86assertions/literals preserved. Meaningful reserve/admit/grow/shrink/
reuse/release/wrongowner/source/stale/double/cancel/overflow/metadata pressure/
oldnewoverlap/nestedactualcapacity/history/real reserve failure proofs; controlled
paint integration can use explicit testresource with in-loop cancellation, not
fake production default. Reports distinguish globalOOMfaultinjection/unrunwhole
raster/UI/runtime. Root independent LunaHigh review/freshmain/protected audit
before peer seed. Standard existing allowance, stopquota/rateerrors.

# Output
Complete tested publicly consumable job buffer/controlled resource API, clear
index/sample integration examples and exact ownership/parity/error/limit report;
result.md<=450words, verification.json, APIcoverage/preservation proofs. Full
renderer/nativeUI/app/liveMCP/performance/tooling/finalQA/release/demo incomplete.
