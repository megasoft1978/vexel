---
id: WO-RUST-KERNEL-CONTRACTS-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 9d19b58e7d211f575a3938e4aea9e9f4a8882eddcf6eef7f429bd0a504b3b7b0
parallel_safe: false
network: false
---
# Full named scope
Implement remaining complete shared render/decode contract DATA and INTERFACE
groups from accepted69-file plan in vexel-scene: Work/Accounting, RasterIndex/
Events/Fill/Stroke/Render/AnimationFrame, Traversal/Selection/Plans/Definitions/
Caches/DecodeOperations/MediaDecodeRequests/FlatKernel/KernelResources, pattern
request/ticket lifecycle types, validated source/context access, limits and
fallible job accounting. No Decoder engine or kernel/provider implementations
are claimed here. Fully typed safe APIs consumed by full parallel lower owners;
no synthetic renderer/backend, fake progress or returning partial success.
Reuse base WorkControl/geometry/raster, owned scene and accepted CSS/reference/
XML/animation APIs. Root writes sharedtypes/contracts/localdependencies. Correct
source-vs-context lifetimes for DefinitionPlan and Text/Image decode requests;
no selfreferences, clonedwhole source/tokens/track reparse. EvaluatedFrame uses
safe owned changed tables plus borrowed unchanged tables and checked identity;
no alternate incompatible scene ID or bypassing frame mutations for render.
Job-local fallible retained/transient allocation accounting includes nested
capacities and checked counters, old/new overlap; no allocator/RSS claims.
Preserve legacy numeric/limit/reference envelopes and same painter order;
public malformed table/range/source/scale inputs error, not panic. Every new
collection operation bounded/polled through WorkControl. Data layout/proposal
adaptations recorded with reasons, no deprecated smaller scope as substitute.
Named tests: actual adapters object/lifetime-check against full interface,
source/context independence compile success, pattern-ticket stale/wrong/single
use behavior, accounting admission/transient/overflow/free/tag preservation,
checked view mapping/boundary/ranges/sample inputs, actual nested capacity and
frame unchanged/changed table access/source and stale handle protection, final
contract coverage against all named groups. These are focused model/contracts
checks, not renderer fidelity, performance or wholeapp acceptance.

# Files allowed to touch
- work/WO-RUST-KERNEL-CONTRACTS-01.md (coordinator freeze only)
- Cargo.toml, Cargo.lock (existing local path dependencies only)
- rust/crates/vexel-scene/**
- artifacts/WO-RUST-KERNEL-CONTRACTS-01/**
- STATE.md, LEDGER.md, ledger.jsonl, docs/product/rust-port-goal-and-plan.md

# Protected paths
All other Rust/product/legacy603/oracles/previous frozen orders/evidence/index.
No thirdparty dependencies/network/download/install/unsupportedOS/newunsafe/
Cshim/broadUI/E2E/stage/commit/reset/delete. Preserve all accepted27 scene tests
and actual source/private-handle constraints; independent Luna review, main
named checks before acceptance. No peer consumes unaccepted APIs.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py WO-RUST-KERNEL-CONTRACTS-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact protected/root603/157frozenoracle/HEAD/index and API coverage audit.
Use local cache/tmp/Rust1.99/AppleSilicon mac13+, subscription only Standard.

# Output
Complete buildable shared contract gate +independentreview/checks/adaptations;
all true decoder/kernel/native/application/fullQA/release/demo work retained.
