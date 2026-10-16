---
id: WO-RUST-DOCUMENT-OWNED-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: ba1c5417fad26e6f4d92619bbdb6d861e5193ec866b3149f71eb5ba8d0bbc737
parallel_safe: true
network: false
---
# Goal and frozen scope
Add safe OWNED document and LARGE source APIs needed for actualRustapp/source-editor; existing borrowedDocumentModel/parse_document and all9acceptedtests/assertions remain byte-identical with16MiBlegacy cap/erroroffset0. Frozen new entrypoints: parse_document_large(&[u8]), parse_document_large_cancellable(&[u8], FnMut()->bool), parse_owned_document(&Arc<Vec<u8>>), parse_owned_document_large(&Arc<Vec<u8>>), parse_owned_document_large_cancellable(&Arc<Vec<u8>>, FnMut()->bool), plus OwnedDocumentModel exposing same fullquery/remove/root/count/handle API, source()->&[u8], source_owner()->&Arc<Vec<u8>>. Typed new LargeDocumentError/OwnedParseError may wrap existingDocumentParseError with explicitCanceled{offset}; do not add/change existingDocumentError variants or borrowedreturnlifetimes. Large source cap1GiB, retain64MiBactualtokenstorage/1Mnodes/1Mattributes/256depth andoriginalXMLgrammar/text/namebounds/unknownbytes; large APIs do not imply wholeapplication100MBperformance. Borrowedlargefunctions fail withtypedParse/Canceled; owned constructors BORROW callerArc, cloneonlyon success, preservingcallerownership onreject withoutwhole source clone or selfreference. Owned cached topology/tables are built once; query mustnotreparse/fullsourceclone/rebuildmodels/tables. Reuse shared privatequery/build helpers as needed, no unsafe/lifetime erasure/leakedstaticrefs, no duplicatepublicAPIbehavior. Preserve globalmonotonicID allocator and stale/foreign handle semantics, no Clone implementation duplicatingmodelidentity; ownedmodel Send+Sync, queriesborrowself, removedsubtree/source unchanged. Cancellation mustpoll duringXMLvalidation/parse AND longnode/attribute tablebuild, no partialmodel returns, release temporary allocations; offsetdefinition anderror precedence explicit. Fallible bounded significantallocations, exactstorage accounting; rootfreeze API names shared downstream onlyafteracceptance. Test actualsource>16MiB andrepresentative100MB correctness withinownedlimits (bounded repeated comments orlargepaths allowedgrammar), legacycapstillrejects0offset, sourcepointer/Arcsharing/noinputcopy, oldoriginalArc drop/leases/thread lifetime, complete query/delete/foreignhandle parity, malformed/source/resource caps andcancellation duringvalidation andnodebuild. Focused correctness only, no app/perfcampaign or speed claim; immutable originalfixtures and9tests preserved, source<800.

# Reference paths
accepted rust/crates/vexel-document model/parse/types and9tests; accepted vexel-xml stream/storage/cancellation; original src/document/document_model.c/h for unchangedparity. Newowned/largesource API contract here is additive product integration feature.
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-DOCUMENT-OWNED-01.md` (coordinator freeze only)
- `rust/crates/vexel-document/src/**`, `rust/crates/vexel-document/tests/**`
- `artifacts/WO-RUST-DOCUMENT-OWNED-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust6-owned-document; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-document --locked --offline
cargo +stable test -p vexel-document --locked --offline
python3 scripts/wo_validate.py WO-RUST-DOCUMENT-OWNED-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
