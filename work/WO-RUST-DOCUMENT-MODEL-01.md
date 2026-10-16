---
id: WO-RUST-DOCUMENT-MODEL-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 97ff544663b6ead932020383bff7fadba6938bedaffbfcdbbf5a9fc006e0ea00
parallel_safe: true
network: false
---
# Goal and frozen scope
Port EVERY function/helper in src/document/document_model.c/h. Safe lifetime-bound DocumentModel borrows immutable original source bytes, retains no edits/normalization and uses accepted real bounded XmlTokenStream, with64MiB actual allocated tokenizer storage cap, 1Mnodes/256depth, exact parse diagnostic and resource exhaustion/error mapping. Safe owned bounded node/attribute tables preserve root/parent/child/sibling/depth/tag/attribute/source-spans/node-at/count queries and subtree removal. DocumentHandle{document_id:u64,slot:u32,generation:u32} rejects wrong/stale/deleted/cross-model handles. Native atomic document IDs monotonic, fail before wrapping/reusing0/MAX; safe isolated deterministic exhaustion seam without weakening real production IDs. Source spans encompass complete closing tag/empty element and raw names/values preserve unknown prefixes/editor metadata. Removed subtree generations/livecounts/child-sibling traversal/root removal match reference; stale handles cannot resolve. Port all meaningful original tests/source exact bytes/spans/UTF8/DOCTYPE/comments/unknown metadata, parse failure categories/offsets, limits/resource behavior, wrong handles/delete parent/live counts/lifetimes and IDs. Fallible significant allocations and checked sizes, no public fabricated parse-success hooks. No source-edit/history/piece table implementation in this order; those depend on accepted model contract next, not product omission.

# Reference paths
src/document/document_model.c/h, document_view.h; tests/document/test_document_model.c; accepted vexel-xml tokenizer/stream
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-DOCUMENT-MODEL-01.md` (coordinator freeze only)
- `rust/crates/vexel-document/src/**`, `rust/crates/vexel-document/tests/**`
- `artifacts/WO-RUST-DOCUMENT-MODEL-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust5-document-model; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-document --locked --offline
cargo +stable test -p vexel-document --locked --offline
python3 scripts/wo_validate.py WO-RUST-DOCUMENT-MODEL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
