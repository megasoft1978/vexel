---
id: WO-RUST-DOCUMENT-EDIT-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: d388ed30b6b76e35a1b4a6e78ea7617ff6bf78e58ce7b517e7d002e2e559fea8
parallel_safe: true
network: false
---
# Goal and frozen scope
Port ENTIRE document_edit.c/h/internal.h, ALL document_edit_piece_table/history/ops/group/insert/snapshot/region/text_insert.c, plus document_view.h inline APIs. Safe DocumentEdit borrows callerArc<Vec<u8>> onlyduringcreation, clonesowner tokeeporiginalalive (explicit stronger snapshotownership than borrowedC), preservesdefault16MiBsource/default1000commands64MiBhistory/max300000commands1GiBhistory/1Mpieces/64KiBfragment/128groupnodes/64label/128templateattrs/4096autoIDattempts/256pathdepth/64MiBindex token cap. Source piece table/Arc-owned insert blobs plus owned cachedmodel andindex spans; no unsafe/selfreferences/sourcewholecopy peroperation/query or expensive query-time reparsing. Cache materializes/rebuilds once after successfulsource-state invalidation asC; useaccepted ownedDocument forsafeownership, preserve alllegacyquery behaviors. Complete root/find_id/offset/tag/span/attrs/decode/text/path/model access, byte_length/materialize/outputcapacities, frozenworker-readableSnapshots via copiedpiece descriptors/shared immutablebuffers (snapshot outliveseditor safely; no fulldocumentcopy atcapture), history counts/cursor/entries/jump/savedcursor/labels anddirty/mark_saved/mark_unsaved. Full set/add/removeattrs/text/delete/duplicate/reorder/wrap/unwrap/group/ungroup/templates/exactsingleXMLfragment/textinsert/XMLregion/rawsourceSpan/undo/redo/save_as APIs/helpers. Match escaping/UTF8/name/duplicateattribute/decodedID/rootdelete/namespacecontext/siblingorder/baseline/error priority/diagnosticoffsets/coalescing/history trimming/redo discard/stateIDs andstalehandle invalidation. Rejectedoperations preserveexactsource/history/handlevalidity/saveflags. Meaningfulbyte preservation includes unknownnamespaces/editormetadata/DOCTYPE/comments/whitespace/quotes andsourceoutsidepatch, not canonicalfullrewrite. Significantallocationsfallible andactualblob/piece/historycapacitycharges whereCcontract defines; Arccontrolblock stdallocation exceptiondocumented no fakeOOM. Ownershipcleanup/mutationtransaction atomic onanyreject/allocationfail; deterministic significantallocation testgate allowed butnoglobalallocator/unsafe. Save usesaccepted source_save_noop; compact ownedsummary maydropborrowedtokenresult aftercopyingdiagnostic/nativeflags. PreserveCpostcommit mark-saved-onrenamed behavior explicitly withdurabilitywarning result (no new claim durable); full100MBediting/save API remainsseparatelarge-integration feature, notclaimedbythislegacyparity port. All namedCscope implemented, no convenient smallereditor/subset. Port meaningful original test_document_edit/history/snapshot/model-viewpagination cases with exactbytes/errors/cursors/labels/source lifetime/output-no-partial-write; fixtures only.cache/edit-* forsave, no GUI/E2E/perfcampaign. Split cohesive source<800.

# Reference paths
src/document/document_edit.c/h/internal.h andALL document_edit_*.c; document_view.h; tests/edit/test_document_edit.c, document_edit_history_test.c, document_edit_snapshot_test.c; viewrange assertion in tests/document/test_document_model.c; accepted document/owned APIs andsource-save/atomicservices
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-DOCUMENT-EDIT-01.md` (coordinator freeze only)
- `rust/crates/vexel-document-edit/src/**`, `rust/crates/vexel-document-edit/tests/**`
- `artifacts/WO-RUST-DOCUMENT-EDIT-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust7-document-edit; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-document-edit --locked --offline
cargo +stable test -p vexel-document-edit --locked --offline
python3 scripts/wo_validate.py WO-RUST-DOCUMENT-EDIT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
