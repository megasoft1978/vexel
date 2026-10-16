---
id: WO-RUST-SVG-ANIMATION-CONTROL-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 88a26c879d8b9faee5313b4fe3b8812370d699cd3c9fa572cda4adac52e89675
parallel_safe: true
network: false
---
# Goal and full additive scope
Implement complete controlled SVG animation PARSE and EVALUATE using accepted
base WorkControl/WorkKind/Stop/ControlledError, completing the animation control
APIs required by the accepted renderer plan. Preserve every public borrowed/
owned/view API, same SMIL/timing/spline/motion grammar/limits/errors/diagnostics/
f64operationorder and12accepted original tests/assertions VERBATIM, same parser
and evaluator, no parallel shadow engines. This order implements real controlled
one-shot operations; it does not claim resumable animation sessions/UIlatency.

Add parse_controlled(source,tokens,control)->Result<Document<'src>,
ControlledError<ParseError>>, parse_owned_controlled(&Arc<Vec<u8>>,tokens,
control)->Result<OwnedDocument,ControlledError<ParseError>>; owned source Arc
cloned only after complete success, move same tables, no copied source/tokens/
tracks/selfreference. Existing parse/parse_owned drive SAME internal engines
with no-stop policy preserving original errors/limit offsets/diagnostics.

Add EvaluationError {InvalidTime, InvalidTrack, OutOfMemory, Canceled, Limit,
Overflow} and evaluate_view_controlled(DocumentView,time,control)->
Result<Overlay,EvaluationError>; optional evaluate_controlled(&Document,...)
wrapper. Legacy evaluate/evaluate_view drive SAME evaluator and map failures
back to None preserving old behavior. Explicit owned OutOfMemory from fallible
reserve, invalidtime and malformed/cyclic/publicinvalidtrack failures separated,
all Stop variants retained. Overlay partialitems never published on failure.
Same resolve/dependency/start/fill/end/repeat/additive/accumulate/spline/motion/
interpolation operationorder; no silent static frame on unsupported/error.

Instrument EVERY potentially long parser/token/attribute/ID/string/number/
key/spline/motion scan and evaluator dependency/track/key/motion/spline loop with
real source/token/AnimationTransform/Edge work charges and cancellation checks
at<=256operation units. No full parse/evaluate followed by bookkeeping/no-op
check; canceled/budget errors happen during actualwork with exact priority when
stopped; unlimited mode retains every original outcome. Use acceptedWorkKind
only. No callbacks holding model/native resources, no foreign pointers/unsafe/
newdeps/global/reparse/sourcecopies. Document workunit categories and limits of
one-shot API; real parser resumption/composed responsiveness is a later gate.
Actual retained capacity storage getter stays exact incl sharedsource once;
all significant allocations fallible, no infallible vectorclone publication.

# Focused correctness
All12existing tests and meaningful assertions unchanged. Controlled/no-stop
parse/evaluation exact arrays/diagnostics/overlay equality at original
end/repeat/spline/motion/mpath/additive/sync cases and exact parseinvalid/tracklimit
errors. Meaningful cancellation/limit at source/token/key/motion/dependency/
interpolation phases with custom WorkControl that stops on actualwork kind;
source-owner strongcount unchanged on stopped/errorownedparse. Invalidtime/
publicmalformedtrack/cycle typed failures vs oldNone, no partialoverlay, overflow
Stop typed, successfulworkcounts nonzero/category evidence. Deterministic
checkpoints/charges and same worker-safe ownedspan views, no UI/fuzz/perf campaign.
Source/tests<800, split focused modules; no skipped acceptance assertions.

# Files allowed to touch
- `work/WO-RUST-SVG-ANIMATION-CONTROL-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-animation/src/**`, `rust/crates/vexel-svg-animation/tests/**`
- `artifacts/WO-RUST-SVG-ANIMATION-CONTROL-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected and execution
All peers/legacy603/oracles/manifests/lock/config/docs/orders/otherartifacts/index.
Launch after accepted OWNED-ANIMATION and WORK-CONTROL; one isolated Standard
LunaHigh writer, ownsrc/tests/ownartifacts only. No network/deps/install/unsafe/
agents/Cshim/unsupportedOS/stage/commit/reset/delete/broadUI/E2E. ExactRust1.99
+stable/mac13/localcache/tmp/approvedregistry. Existing subscription/no purchased
credits/APIbilling; stop quota/rateerrors without retryloop. Missing realcontract
reported rather than fake API/stubs/unaccepted peer imports or smaller grammar.

# Verification
cargo +stable check -p vexel-svg-animation --locked --offline
cargo +stable test -p vexel-svg-animation --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-ANIMATION-CONTROL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Complete candidate result<=350words, parity-map/verification exact workunits/
loops/errors/sourceownership/assertions. IndependentHigh review and exactaudit/
freshmain namedchecks beforeacceptance. No resumableanimation/wholeSVG/renderer/
100MBworkflow/UIlatency/productcompletion claim.
