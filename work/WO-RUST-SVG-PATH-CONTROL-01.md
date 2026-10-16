---
id: WO-RUST-SVG-PATH-CONTROL-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 5f54042de9de6be6359a1e4a101cc7609d7f859275a923b71449bb8c202acc02
parallel_safe: true
network: false
---
# Goal and full additive scope
Implement complete controlled/resumable SVG path parsing using accepted base WorkControl/WorkKind/Stop/ControlledError/ControlledStep and accepted path/value/geometry engines. Add PathParseSession<'src>::new(source,span,transform,options)->Result<Self,ControlledError<Diagnostic>>, step(quantum,control)->ControlledStep<(),Diagnostic>, finish(self)->Result<PathDocument,ControlledError<Diagnostic>>, parse_path_controlled(source,span,transform,options,control). Legacy parse_path drives SAME state engine with no-stop policy; no second parser. Add flatten_cubic_controlled(points,transform,options,source_offset,out,control) through same internal curve traversal, old flatten_cubic delegates no-stop with old validation/error/partialappend behavior. Existing append_edge/tolerance unchanged; share curve walker with parsed cubic/quadratic commands.
Retain REAL source/commavalidation/command/operand number/exponent/separator/flag cursors, repeated/relative M/L/H/V/C/S/Q/T/A/Z and current/subpath/control-point reflect state, exact metadata spans/offsets, iterative pending cubic recursion stack or equivalent owned state, arc sample cursor and partial geometry. new O(1) span/transform validation only; full comma-validation pass and grammar inside bounded step, preserving original comma-before-parse errorprecedence. No digit/whitespace/comma-neighbor loop or curve recursion/arc sample generation hidden as unbounded one unit; byte numeric scanner/flattening must genuinely pause/resume preserving EXACT f64 operation order/math/ties and16depth/0.25transformed tolerance. No synthetic steps or restarting entire path on everycall. All significant allocations fallible, fixedcaps commands/edges/numbers unchanged, original optionclamps and invalidpoint/transform handling retained. PathDocument owned vectors/source spans unchanged, no selfreference/unsafe/global/sourcecopy/shadow number/parser.
Add focused tinyquantum/zeroquantum/finish-before-complete/terminalstep/drop tests comparing exact PathDocument (edges/subpaths/segments/numbers) and diagnostics for ALL original12tests/assertions unchanged; repeated/relative/allcommands/flags/comma/adjoining signs/errors/limitcases. Long numeric/whitespace/comma streams and highlysubdividedcurve/arcs must produce meaningful multiple Pending and bounded actualcharges, without whole-input rescans. Cancel/limit at validation/operand/curve/arc/metadata phases, checkedoverflowtyped, no partial PathDocument success. Controlled cubic appends same exact geometry, prefilledcap/partialappend failure docs and validation no-mutation preserved. No newlimits/unsupported-path subset or performance/UI claims.

# Common resumable contract
Consume accepted base types only, no competing enums/traits. Every longloop
checkpoint/charge in bounded batches<=256workunits; positivequantum bounds real
units THISstep, Pending charged_units is actualacceptedwork, step0 onlychecks
control/no progress, firstfailure terminal/sticky, repeatedterminal no duplicate
work. finish fails originalerror unless complete or returns terminalfailure;
never manufacture success. Real loop cursors and intermediate owned state,
not one-shot parsing followed by simulated yields. Completed work/counters not
rolled back on higher-level transaction failure. Preserve 12 original tests
and meaningful assertions VERBATIM; record exact baseline copies from current
uncommitted accepted Rust source, gitHEAD predates migration. Checked actual
capacity/math/bounded allocations, source/tests<800, split cohesive modules.
No abstract interface-only/stub/mocked backend or reduced feature/grammar scope.

# Files allowed to touch
- `work/WO-RUST-SVG-PATH-CONTROL-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-path/src/**`, `rust/crates/vexel-svg-path/tests/**`
- `artifacts/WO-RUST-SVG-PATH-CONTROL-01/**`
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
cargo +stable check -p vexel-svg-path --locked --offline
cargo +stable test -p vexel-svg-path --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-PATH-CONTROL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Complete uncommitted candidate result<=350words, parity-map/verification real
statephases/charges/errors/capacity/originalassertion evidence. IndependentHigh
review/protected audits and fresh main namedchecks before acceptance. No full
renderer/100MBworkflow/UIlatency/application/finaltest claim.
