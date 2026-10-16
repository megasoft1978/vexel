---
id: WO-RUST-WORK-CONTROL-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: f0017049d34add85bb6a11a7b2e2ad47f710834c36a6d3dc76fdb3322386e0c9
parallel_safe: true
network: false
---
# Goal and complete additive control contract
Implement the shared safe work-control API in accepted vexel-base, prerequisite
for full incremental parser/renderer tasks. Preserve all existing base API,
arena behavior and7original tests/assertions verbatim. No shadow types in peers.
Add exported WorkKind variants exactly: SourceRead, TokenVisit, ReferenceProbe,
Transform, Expansion, Edge, PaintVisit, ClipChain, ClipChild, ClipEdge, ShapePixel,
ShapeSample, FillEdgeTest, FillIntervalTest, StrokeTest, ScanlineEdgeEntry,
ShapeRowVisit, BoundaryMark, FilterPixel, AnimationTransform, Media.
Add Stop {Canceled, Limit, Overflow} and object-safe WorkControl trait:
charge(&mut self,kind:WorkKind,units:u64)->Result<(),Stop>,
checkpoint(&mut self)->Result<(),Stop>. Export ControlledError<E> with
Original(E), Canceled, Limit, Overflow, OutOfMemory and explicit From<Stop>
preserving each stop. Export ControlledStep<T,E> with Pending{charged_units:u64},
Complete(T), Failed(ControlledError<E>); these are real shared result types,
not fabricated parser progress. No parser/session/renderer implementation in
this order. Document workers must check/charge bounded batches<=256workunits;
this API cannot itself prove UIlatency/resumption or existing parser cancellation.

Provide a concrete bounded BudgetControl<'a> borrowing &'a AtomicBool cancel
flag, total u64 worklimit plus optional per-kind u64 limits (or fixed array with
u64::MAX default). No allocation/Arc constructor/global/reentrant closure.
charge checks cancellation, all checked totals/per-kind additions and configured
caps before publishing accepted counters; error never partially updates stats.
Successful0charge still checks cancellation. First stop latches for the job;
later charge/checkpoint return same stop and do not mutate counters. Immutable
stats access total/per-kind accepted units, private storage. checkpoint reads
real cancel flag and honors sticky stop, charges no invented work. Explicit
caller policy constructors, no lowered legacy limits or elapsedtime claims.
Safe Send by Rust types, cross-thread cancellation via real AtomicBool; no UI or
native resources. Document atomic load ordering appropriate to cancellation
only, not synchronization of arbitrary data. Count does not roll back completed
work when a later higher-level source transaction rolls back.

# Required focused proof
All7original tests/assertions verbatim. Add trait-object invocation and exact
all-kind accounting, zero and boundary charges, smaller per-kind/total caps,
checkedoverflow without allocating huge input, failure stats unchanged, sticky
first-stop precedence, real scopedworker cancellation/checkpoint (deterministic
thread synchronization), noreset/global/native behavior, and From<Stop> exact
controlled error variants. Source/tests<800; focused newmodule andtests.

# Files allowed to touch
- `work/WO-RUST-WORK-CONTROL-01.md` (coordinator freeze only)
- `rust/crates/vexel-base/src/**`, `rust/crates/vexel-base/tests/**`
- `artifacts/WO-RUST-WORK-CONTROL-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected and execution
All peers/source603/oracles/manifests/lock/config/docs/orders/otherartifacts/index.
One isolated Standard LunaMedium writer, owncrate src/tests/artifacts only.
No newdeps/unsafe/network/install/agents/Cshim/unsupportedOS/stage/commit/reset/
delete or broadUI/E2E. ExactinstalledRust1.99+stable/mac13/localcache/tmp seeded
from approvedregistry. Existing subscription only, no purchasedcredits/APIbilling;
stop quota/rateerrors without retryloop.

# Verification
cargo +stable check -p vexel-base --locked --offline
cargo +stable test -p vexel-base --locked --offline
python3 scripts/wo_validate.py WO-RUST-WORK-CONTROL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Complete additive candidate result<=300words, parity-map/verification exact
API/overflow/counters/cancel/lifetime outcomes. Independent review/protected
source/originalassertion audit and freshnamedmain checks before acceptance.
No fake progress, parser/renderer/application/latency acceptance.
