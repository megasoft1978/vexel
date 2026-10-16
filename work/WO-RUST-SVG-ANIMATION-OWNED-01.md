---
id: WO-RUST-SVG-ANIMATION-OWNED-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: a4eaf78af4f2ed1a733d35d2f0e480885f5d22d1cde14f9b8212882b9c7dee3f
parallel_safe: true
network: false
---
# Goal and complete additive scope
Add owned source-coherent SVG animation data and allocation-free borrowed views
to accepted vexel-svg-animation, so the full owned scene/native worker pipeline
can retain animation without self-reference, source/track copies or reparsing.
Preserve every existing public Document/parse/evaluate API, grammar/defaults,
limits/diagnostic/error/operationorder and all9accepted tests/assertions.
New `OwnedDocument` borrows caller `&Arc<Vec<u8>>` only at creation, calls the SAME
accepted parser, moves its owned tracks/keys/splines/motion/diagnostics into the
owned model and clones source owner only on success. No Clone implementation
that copies tables. `parse_owned(source:&Arc<Vec<u8>>, tokens:&[XmlToken])`
returns the existing parse error semantics; no caller token borrow survives.
Provide immutable source_owner/source/tracks/diagnostics accessors and lightweight
`DocumentView` borrowing slices/source. Both borrowed and owned documents expose
views. `evaluate_view(view,time)` uses the SAME evaluator engine, with accepted
`evaluate(&Document,time)` delegating without table/source clone or reparse.
Scene consumers can inspect actual retained source/vector capacities through a
checked storage-byte getter (source charged separately once; nested vectors and
track/diagnostic tables actual capacity included), not fabricated tracking.
All significant new allocations fallible where required; source Arc clone has
no source allocation. No globals/unsafe/Cshim. Source/lifetime/span coherence
and Send/Sync from owned Rust data, no selfreferences. Bounds valid returned
views; existing parsed data and diagnostics unchanged on errors. Source-owner
strong counts/failure publication follow success-only clone contract.

# Required focused correctness
Retain original9tests and meaningful assertions verbatim. Add owner lifetime after
caller drop, pointer/source sharing and unchanged spare-capacity source bytes;
view table pointers/lengths prove no copies; owned/borrrowed evaluation equality
at end/repeat/spline/motion/path cases; invalid time/out-of-range preserves exact
existing behavior. Bad parse/tracklimit leaves caller owner count unchanged;
view borrows cannot outlive owner. Move owned model into scoped worker and
verify raw spans/evaluation without native resources. Capacity accounting checks
all existing nested storage types and overflow-safe sums. No UI/E2E/fuzz/perf
campaign or invented constructor mocks. Source/tests<800.

# Files allowed to touch
- `work/WO-RUST-SVG-ANIMATION-OWNED-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-animation/src/**`, `rust/crates/vexel-svg-animation/tests/**`
- `artifacts/WO-RUST-SVG-ANIMATION-OWNED-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected and execution
All peers/manifests/locks/config/legacy603/oracles/orders/docs/otherartifacts/index.
One isolated Standard LunaMedium writer, owncrate src/tests/artifacts only.
Accepted native packages/registry seeded for Cargo offlineworkspace resolution,
no network/install/newdependencies/unsafe/recursiveagents/commit/stage/reset/
delete/unsupportedOS. ExactRust1.99+stable/mac13/localcache/tmp, existing
subscription/no extra credits/APIbilling; stop allowance/rateerrors no retryloop.

# Verification
cargo +stable check -p vexel-svg-animation --locked --offline
cargo +stable test -p vexel-svg-animation --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-ANIMATION-OWNED-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Complete uncommitted additive source-ownership/view candidate; result<=350words,
parity-map/verification exact APIs/bytes/errors/capacity/source/clone evidence.
Root independent review and exact prior9assertion/preservedsource audit and
freshmain namedchecks beforeacceptance. No full renderer/nativeapp/perf claim.
