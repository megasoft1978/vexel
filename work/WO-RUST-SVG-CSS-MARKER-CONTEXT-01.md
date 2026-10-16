---
id: WO-RUST-SVG-CSS-MARKER-CONTEXT-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 6e3cbbd3da2f98648fe92c43d741dfbad81319d76bc69b81057f9c75137348c8
parallel_safe: true
network: false
---
# Exact scope
Close concrete definitions marker inheritance API gap: CssPaint returns a copied
style with only Marker, MarkerStart, MarkerMid and MarkerEnd cleared, preserving
all other values/bytes/spans/canonical source identity. Exact owned marker.c
clearing semantics; no general mutable CSS escape or synthetic source span.
Constant four-field operation, no allocations. Keep old computed parent intact,
explicit marker properties on child still cascade normally. Preserve all original
63accepted named CSS test entries byte-identically and existing parse/cascade
behavior. Add meaningful original-style/shorthand/explicit-child/idempotence tests.

# Files allowed to touch
- work/WO-RUST-SVG-CSS-MARKER-CONTEXT-01.md (freeze only)
- rust/crates/vexel-svg-css/src/lib.rs (exact CssPaint method only)
- rust/crates/vexel-svg-css/tests/marker_context.rs (new only)
- rust/crates/vexel-svg-css/README.md (API note only)
- artifacts/WO-RUST-SVG-CSS-MARKER-CONTEXT-01/**
- STATE.md, LEDGER.md, ledger.jsonl, docs/product/rust-port-goal-and-plan.md

# Protected paths
All manifests/config/other Rust/originaltests/source603/157oracles/previousorders/
evidence/immutable catalogs/index. Accepted consumer contracts and live PAINT/
ANIMATION repairs disjoint; native read-only review runs on its fixed candidate.
No install/network/newdep/unsafe/GPL/unsupportedOS/stage/commit/reset/delete/GUI/
broadUIE2E. Source/tests<800. Independent LunaLow review and maincheck/test/
preservation audit before definitions consumes helper. Standard existing allowance.

# Verification
cargo +stable check -p vexel-svg-css --locked --offline
cargo +stable test -p vexel-svg-css --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-CSS-MARKER-CONTEXT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact original CSS files/tests except authorized method, original fixtures/source
603/157/HEAD/index unchanged. No definitions/renderer/application acceptance.

# Output
Complete accepted marker-context copy API with exact focused evidence, then
terminal definitions worker may consume through separate authorized seed. Full
definition scope/real app/liveMCP/finalQA/release/demo remain required.
