---
id: WO-RUST-SVG-PATH-GEOMETRY-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: e1336267dc1de1f0fdefc8be766ae29f1616253f5ebb4bc99d10ade8f9de504a
parallel_safe: true
network: false
---
# Goal and full additive scope
Expose the accepted transformed path geometry algorithms through complete safe public wrappers for shape/text decode. Preserve every parse_path API/grammar/limits/error/source offset/f64operation order and all9accepted tests/assertions verbatim. Add transformed_tolerance(transform:Affine)->f64 exactly using accepted flatten::tolerance policy. Add append_edge(from:Point,to:Point,transform:Affine,max_edges:usize,source_offset:usize,out:&mut Vec<Edge>)->Result<(),Diagnostic>; add flatten_cubic(points:[Point;4],transform:Affine,options:ParseOptions,source_offset:usize,out:&mut Vec<Edge>)->Result<(),Diagnostic>. Reuse SAME internal edge/curve engines, no new curve/path parser. Validate transform via accepted affine_valid and inputpoints finite/bounded MAX_COORDINATE before recursion; invalidtransform ->InvalidTransform, invalidpoint ->Number at supplied absolute offset. Clamp effective edge cap at MAX_PATH_EDGES, count preexisting output edges, preserve16depth/0.25transformed tolerance/try_reserve/erroroffset semantics. Preserve internal parse_path behavior and error order; wrapper validation should not alter accepted parser. Document partial append on failure exactly as existing curve engine; no invented transactional promise.
Focused tests compare cubic wrapper exact Edge vectors with accepted parse_path equivalent commands for identity/scaled/sheared transforms and degenerate cases; append_edge exact transformed result and existing output; hard/smaller cap with prefilledbuffer, nonfinite/out-of-bound points/transforms, valid and failure offsets, recursion/limit and no output mutation on upfrontvalidation failure. Original9assertions unchanged, no UI/fuzz/perf campaign, no deps/unsafe/sourcecopies. Source/tests<800.

# Files allowed to touch
- `work/WO-RUST-SVG-PATH-GEOMETRY-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-path/src/**`, `rust/crates/vexel-svg-path/tests/**`
- `artifacts/WO-RUST-SVG-PATH-GEOMETRY-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected and execution
All peer/legacy603/oracle/manifests/lock/config/docs/orders/otherartifacts/index.
One isolated Standard LunaMedium writer in .worktrees/rust-path-geometry; owncrate src/tests
and ownartifacts only. No network/fetch/install/dependencies/unsafe/agents/Cshim/
unsupportedOS/stage/commit/reset/delete or broadUI/E2E. Accepted exactnative
registry seeded for offlineworkspace resolution; no nativeapp startup. Rust1.99
+stable/mac13/repolocalcache/tmp. Existing subscription only; no purchasedcredits
or APIbilling; stop quota/rateerrors without retryloop.

# Verification
cargo +stable check -p vexel-svg-path --locked --offline
cargo +stable test -p vexel-svg-path --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-PATH-GEOMETRY-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Complete uncommitted additive candidate, result<=350words, parity-map.json and
verification.txt with exact outcomes. Independent review, originalassertion/
protectedsource audit and freshmain namedchecks before acceptance. No renderer/
largeworkflow/performance/nativeapp/productcompletion claim. Report concrete
missing contract rather than shadowparser/subset/mocks/skipped tests.
