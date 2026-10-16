---
id: WO-RUST-SVG-VALUES-POLICY-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 289c38eb5952522a8034892e5ef1742819712a6dc2e90a027785ae9ff04abd1b
parallel_safe: true
network: false
---
# Goal and full additive scope
Add complete safe policy-aware value APIs and an allocation-free number/length cursor to the accepted values crate. Preserve every existing public API, grammar, default16MiB/256 limits, errors/absolute offsets/f64 operation order and all12accepted tests/assertions verbatim. Same algorithms only, no duplicate parser/rebase loophole. Add ValueLimits {source_bytes:usize, attributes:usize}, Default preserves existing constants, effective caps clamp at1GiB/4096attributes (smaller caller limits enforced). AttributeList::new_with_limits validates whole original source/all spans and stores effective policy with immutable limits accessor. Existing new delegates Default. Add parse_number_with_limits, parse_number_list_with_limits, parse_length_with_limits and parse_transform_with_limits; existing APIs delegate Default through SAME engine. Every viewport/root/nested/symbol/viewbox/preserve-aspect and transform-node operation receiving a validated AttributeList must carry its policy throughout internal value calls. Add parse_preserve_aspect_with_limits as needed; existing preserves defaults. No hidden default16MiB rejection of large policy-approved original source. Fixed-count number-list cap64 stays unchanged.
Expose NumberCursor<'src> with private accepted Reader state. new(source,span,limits)->Result<Self,ValueError> validates whole source cap and absolute span; number()->Result<f64,ValueError>, length(reference)->Result<f64,ValueError>, separator()->bool, finished()->bool (consume accepted whitespace only), absolute()->usize. Refactor unit parsing into SAME engine used by cursor and parse_length, preserve unit grammar/reference/error behavior and cursor delimiters. No allocations in cursor. Caller handles lists; existing strict fixed-count parse semantics unchanged. Bounds/overflow checked, no new dependencies/unsafe/global sourcecopies.
Add focused cases >16MiB original source with nonzero spans and viewport/transform/preserve-aspect through policy, >256attrs allowed up to4096 with source-backed spans, smaller bounds/default rejection/error precedence/invalidspans; policies above hard caps cannot bypass enforcement. Cursor mixednumber/unit/percentage/exponent/separators and exact absoluteerrors vs existing parsers, invalidreference/units and finish behavior. Retain original12test functions/assertions verbatim. No benchmark/UI campaign. Source/tests<800; split focused helpers before cap. Document clamping/default and partialcursor-on-error semantics.

# Files allowed to touch
- `work/WO-RUST-SVG-VALUES-POLICY-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-values/src/**`, `rust/crates/vexel-svg-values/tests/**`
- `artifacts/WO-RUST-SVG-VALUES-POLICY-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected and execution
All peer/legacy603/oracle/manifests/lock/config/docs/orders/otherartifacts/index.
One isolated Standard LunaMedium writer in .worktrees/rust-values-policy; owncrate src/tests
and ownartifacts only. No network/fetch/install/dependencies/unsafe/agents/Cshim/
unsupportedOS/stage/commit/reset/delete or broadUI/E2E. Accepted exactnative
registry seeded for offlineworkspace resolution; no nativeapp startup. Rust1.99
+stable/mac13/repolocalcache/tmp. Existing subscription only; no purchasedcredits
or APIbilling; stop quota/rateerrors without retryloop.

# Verification
cargo +stable check -p vexel-svg-values --locked --offline
cargo +stable test -p vexel-svg-values --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-VALUES-POLICY-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Complete uncommitted additive candidate, result<=350words, parity-map.json and
verification.txt with exact outcomes. Independent review, originalassertion/
protectedsource audit and freshmain namedchecks before acceptance. No renderer/
largeworkflow/performance/nativeapp/productcompletion claim. Report concrete
missing contract rather than shadowparser/subset/mocks/skipped tests.
