---
id: WO-RUST-SVG-REFERENCE-CONTROL-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 440109bb481066d11a5b98d9d605c8dc9529a1d97b36924a38e88d813e0ba8b7
parallel_safe: true
network: false
---
# Goal and full named additive scope
Implement genuine controlled, resumable reference-index BUILD and ID LOOKUP in
accepted vexel-svg-reference, using accepted vexel-base WorkControl/WorkKind/
Stop/ControlledError/ControlledStep. Launch only after WORK-CONTROL-01 accepted.
Preserve all existing public reference functions, rawIDhash/equality/math,
validation/errorprecedence/offsets, fixed131072slot/65536IDs caps, emptyallocation
behavior, actualcapacity accounting and11original tests/assertions verbatim.
No second reference parser, rebuilding index per step or synthetic progress.

Add ReferenceBuildSession<'src>::new(source,tokens), step(quantum:u64,
control:&mut dyn WorkControl)->ControlledStep<(),ReferenceDiagnostic>,
finish(self)->Result<ReferenceIndex<'src>,ControlledError<ReferenceDiagnostic>>.
Session retains REAL cursors for token/spanvalidation, IDcount pass, fixedtable
zero initialization, insertionpass/attributevisit/hashbyte/probe/collision
comparison. No unbounded full token/IDbyte/attribute/table loop outside step.
new does not scan wholeinput; finish fails Original InvalidArgument offset0
unless completed (or returns terminalfailure), never fabricates partialsuccess.
Preserve original validate-all-before-IDcount-before-insertion error order.
Fallible exactslots reservation and excesscapacity rejection preserved; actual
allocation may be indivisible, no latency/performance promise. Zeroingtable and
long IDhash/equality work must resume, not all happen inside a single token.
step0 observes control and returns Pending0 without progress; positivequantum
limits actual operation units, everylongloop checkpoint/charge <=256units.
Pending charged_units describes actual accepted units THISstep; terminal stops
sticky, repeatedcomplete/failure steps no duplicate work. Dropping unfinished
session releases own allocation/sourceborrow normally.

Add ReferenceIndex::create_controlled with same session engine; legacy create
drives SAME engine with internal no-stop policy, exact old semantics/errors.
Add ReferenceIndex::find_controlled(id,control) typedresult; preserve legacy find
Option/exactrawbytes/probe counts using SAME lookup engine. Add real borrowed
ReferenceFindSession<'idx,'src> (e.g begin_find) with step/finish following same
terminal semantics and real IDhash/probe/attribute/bytecompare cursors; controlled
one-shot drives it. No full hash/equality loop outside accounted step. Legacy
badspan/empty/no-table ->None behavior preserved; controlled also Ok(None).
Index lookup_probes counts actual occupied/terminating probes performed, remains
saturating as accepted; canceled work charges/probes aren't rolled back.
Add allocated_bytes getter matching actual retained slotVeccapacity, no source
charge because index borrows source/tokens; don't fabricate logical usedcounts.
All new source/metadata allocations fallible, newstate bounded/private, safe
borrowlifetimes and no source/token copying. Other support helpers keep exact
accepted behavior; their decoder continuations remain a separate composition
gate, no blanket full-reference/UIresponsiveness claim.

# Required focused proof
All11old assertions verbatim and originals unchanged. Meaningful tinyquantum
build/find vs legacy/controlled complete equality for all originalvalid/error
cases, duplicate/empty/unboundspan/oversizeIDs offsets and validationprecedence.
Long authored rawIDhash including suffixdistinctions/sharedprefix must yield
multiple Pending steps with real monotonic byte cursors/progress, preserve full
rawhash and collisions, not reparsed everycall. Compare exact indexcapacity and
probe counts, zeroquantum/finish-before-complete/droppedunfinished sessions,
cancel/limit at validation/count/zero/hash/probe/comparison phases with no partial
success; controlledcounter overflow typed. Deterministic phase/quantum tests,
no sleeping/mockbackend/UIcampaign. Source/tests<800, split cohesive modules.

# Files allowed to touch
- `work/WO-RUST-SVG-REFERENCE-CONTROL-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-reference/src/**`, `rust/crates/vexel-svg-reference/tests/**`
- `artifacts/WO-RUST-SVG-REFERENCE-CONTROL-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected and execution
All peers/legacy603/oracles/manifests/lock/config/docs/orders/otherartifacts/index.
One isolated Standard LunaHigh writer after acceptedbasecontrol prerequisite,
owncrate src/tests/artifacts only. No network/deps/install/unsafe/agents/Cshim/
unsupportedOS/stage/commit/reset/delete/broadUI/E2E. Rust1.99+stable/mac13/local
cache/tmp/approvedregistry, existing subscription/no extra credits/APIbilling;
stop quota/rate errors without retryloop. No loweredlimits/shadowtypes/stubs or
fake progress; report realmissing dependency rather than silently reduce scope.

# Verification
cargo +stable check -p vexel-svg-reference --locked --offline
cargo +stable test -p vexel-svg-reference --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-REFERENCE-CONTROL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Complete additive candidate result<=350words, parity-map and verification exact
session phases/counters/capacity/errors/assertions. IndependentHigh review and
exact source/assertion/protected audits plus freshmain checks before acceptance.
No wholedecoder/renderer/100MBworkflow/UIperformance/application claim.
