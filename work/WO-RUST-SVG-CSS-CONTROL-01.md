---
id: WO-RUST-SVG-CSS-CONTROL-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: b56e9049d11bbe01a06d74ad0a23747b025a499e586fe38f1acbb99445c8043f
parallel_safe: true
network: false
---
# Goal and full additive scope
Implement complete controlled/resumable stylesheet BUILD and COMPUTE using accepted base WorkControl/WorkKind/Stop/ControlledError/ControlledStep. Add CssBuildSession<'src>::new(source,tokens), step(quantum,control)->ControlledStep<(),CssError>, finish(self)->Result<CssSheet<'src>,ControlledError<CssError>>. Add CssSheet::create_controlled(source,tokens,control), compute_controlled(element,parent,control), allocated_bytes()->usize; legacy create/compute drive SAME new internal engines with no-stop policy. Add genuine CssComputeSession<'ctx,'src> created by sheet.begin_compute(element,parent); mutable sheet borrow is 'ctx, source/paint 'src, copy small parent CssPaint value instead of retaining short parent reference. step returns ControlledStep<(),CssError>, finish returns complete CssPaint<'src> only. Do not tie temporary ctx lifetime to source lifetime or publish partial paint.
All39properties/rawspan/same-source parent checks/inheritance resets/importance/specificity/order/unsupported-stop behavior, CDATA handling, selector/descendant/class grammar, stylesheet/presentation/inline parsing and cumulative counters/hit_limit behavior unchanged under no-stop. Same MAX_BYTES1MiB(CSSbytes, not whole SVGsource), rules4096/declarations16384/classes16/selector4/match4194304 and overflow/OOM behavior; no reduced caps or new whole-source16MiB restriction. Session build walks real token/style text/Cdata/parser comment/identifier/selector/declaration cursors. Session compute retains rule/selector ancestor/token/attribute/class/byte comparison/offer/inline/winner/inheritance state. No full unbounded byte/attribute/ancestor/rule loop hidden in one step or new/finish, no restarting source or fake progress. All raw-source comparisons including long names/classes/comments/inline values must resume in bounded units. Allocations fallible, exact retained capacities incl ruleVec + every declarationVec getter, source/tokens borrowed not chargedowned, no copies/reparse/global/unsafe.
Add focused tinyquantum/zeroquantum/finish-before-complete/step terminal behavior vs legacy and controlled one-shot for every original cascade/unsupported/error family and exact13assertions; long identifiers/classes/comments and many rules/ancestors/CDATA must produce real Pending progress, meaningful operation counts and complete equal spans/propertybytes. Cancellation/limit during parsing/matching/inline/winner phases return typed terminalfailure, partial paint never published, counters actualwork not rolled back; cross-source parent rejection unchanged. Test capacity getter real nested capacities and stopped/drop cleanup. No fake tick queues or reduced grammar. Unsupported original inputs stay unsupported.

# Common resumable contract
Consume accepted base types only, no competing enums/traits. Every longloop
checkpoint/charge in bounded batches<=256workunits; positivequantum bounds real
units THISstep, Pending charged_units is actualacceptedwork, step0 onlychecks
control/no progress, firstfailure terminal/sticky, repeatedterminal no duplicate
work. finish fails originalerror unless complete or returns terminalfailure;
never manufacture success. Real loop cursors and intermediate owned state,
not one-shot parsing followed by simulated yields. Completed work/counters not
rolled back on higher-level transaction failure. Preserve 13 original tests
and meaningful assertions VERBATIM; record exact baseline copies from current
uncommitted accepted Rust source, gitHEAD predates migration. Checked actual
capacity/math/bounded allocations, source/tests<800, split cohesive modules.
No abstract interface-only/stub/mocked backend or reduced feature/grammar scope.

# Files allowed to touch
- `work/WO-RUST-SVG-CSS-CONTROL-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-css/src/**`, `rust/crates/vexel-svg-css/tests/**`
- `artifacts/WO-RUST-SVG-CSS-CONTROL-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected and execution
All peers/legacy603/oracles/manifests/lock/config/docs/orders/otherartifacts/index.
Launch after accepted WORK-CONTROL-01; path also accepted PATH-GEOMETRY and
VALUES-POLICY gates. One isolated Standard LunaHigh writer, own src/tests/own
artifacts only. No deps/unsafe/network/install/agents/Cshim/unsupportedOS/stage/
commit/reset/delete/broadUI/E2E. ExactRust1.99+stable/mac13/localcache/tmp seeded
approvedregistry. Existing subscription/no purchasedcredits/APIbilling; stop
quota/rateerrors no retryloop. Missing realcontract ->report, not fake replacement.

# Verification
cargo +stable check -p vexel-svg-css --locked --offline
cargo +stable test -p vexel-svg-css --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-CSS-CONTROL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Complete uncommitted candidate result<=350words, parity-map/verification real
statephases/charges/errors/capacity/originalassertion evidence. IndependentHigh
review/protected audits and fresh main namedchecks before acceptance. No full
renderer/100MBworkflow/UIlatency/application/finaltest claim.
