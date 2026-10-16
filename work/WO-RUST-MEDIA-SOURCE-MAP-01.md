---
id: WO-RUST-MEDIA-SOURCE-MAP-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: badb3cedc21a94e1e8f83632a3254808def8194383d9b6d7aadc95128f694932
parallel_safe: true
network: false
---
# Goal
Close confirmed MEDIA text/image use-host mapping gap. Add explicit SourceMap
(checked host span, instance transform and optional transform-node identity) to
TextDecodeRequest and ImageDecodeRequest. ElementView continues identifying
original referenced media tokens; never replace/reparse them with use host.
Validate canonical element source/token ownership against SceneBuilder source,
scene identity, source-map span bounds/finite transform/live same-scene transform
handle and cancellation before provider/append work. Geometry transform remains
independent explicit request input; do not assume every source annotation CTM
must equal it. No source rewrite, fake media provider or mutable scene escape.
Original82scene tests unchanged; new meaningful distinctusehost/reference span,
foreign source/transform/scene/stale/cancel/malformed map/standalone proof.

# Files allowed to touch
- work/WO-RUST-MEDIA-SOURCE-MAP-01.md (freeze only)
- rust/crates/vexel-scene/src/decode_contract.rs
- rust/crates/vexel-scene/src/media_source_map.rs (new validator only)
- rust/crates/vexel-scene/src/lib.rs (module declaration only)
- rust/crates/vexel-scene/tests/media_source_map.rs (new only)
- rust/crates/vexel-scene/README.md (API note)
- artifacts/WO-RUST-MEDIA-SOURCE-MAP-01/**
- STATE.md, LEDGER.md, ledger.jsonl, docs/product/rust-port-goal-and-plan.md

# Protected paths
All manifests/config/other source/tests/82oldassertions/source603/157oracles/
previousorders/evidence/catalogs/HEAD/index. Fixture metadata separatelyunderreview;
PAINT live inputs unchanged; root onlysharedcode, no parallelsharedsourcewriter.
No newdep/network/GPL/install/unsafe/unsupportedOS/stage/commit/reset/delete/GUI/
broadUI. Source/tests<800. Independent LunaMedium review +freshmainscopedcheck/
test/WO/ledger/diff/protected audit before MEDIA/DECODE consumed. Existing allowance.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py WO-RUST-MEDIA-SOURCE-MAP-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Original82test files/main HEAD/index and legacy603/157 unchanged. Inputinterface
only, full media/parser/nativeprovider/core/app/runtime QA stillmandatory.

# Output
Accepted source-map input contract with exact meaningful proof, ready for real
MEDIA/DECODE adapters. Full product/release/liveMCPrecording goal remainsactive.
