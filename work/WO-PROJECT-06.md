---
id: WO-PROJECT-06
revision: 1
status: READY
milestone: PROJECT
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 3722bb5ef7e2d53a5bfc10bd55b4101512f0c28e76f4968ec6fe402016e8ff0c
parallel_safe: false
network: false
---
# Goal
Add an immutable, worker-readable document-edit snapshot so large edited SVG scenes can be materialized off the UI thread for native project recovery.

# Context
- Follow OWNER-DIRECTIVES.md, docs/CODING_STANDARD.md, WO-PROJECT-04/05, and current document-edit ownership rules.
- `document_edit` uses a piece table: each successful edit publishes a new piece descriptor array and inserted/replaced data lives in immutable reference-counted blobs.
- The original source is borrowed. Vexel keeps its loaded document alive while its single worker job runs; callers must keep the edit model and original source alive until the snapshot is destroyed.
- Snapshot creation may allocate/copy piece descriptors, but must not materialize or copy document bytes. Worker-side materialization is bounded by the current document byte length and caller capacity.

# Files allowed to touch
- `work/WO-PROJECT-06.md`
- `src/document/document_edit.h`, `src/document/document_edit_internal.h`, `src/document/document_edit_piece_table.c`, `src/document/document_edit_snapshot.c`
- `tests/edit/document_edit_snapshot_test.c`, `tests/test_map.json`, `CMakeLists.txt`
- `src/document/README.md`, `artifacts/WO-PROJECT-06/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- App/runtime scheduling, platform code, recovery capsule/store implementation, website/deployment, frozen prior work orders/artifacts, and every path outside the allowlist. Preserve all current working-tree changes.

# Required implementation
1. Add an opaque `document_edit_snapshot` API to create, query byte length, materialize, and destroy immutable snapshots.
2. Snapshot creation copies only piece descriptors and retains each immutable blob; it must not copy/materialize the full SVG source. Keep the existing borrowed original-source lifetime explicit.
3. Existing edits, undo/redo, and model index invalidation must not change snapshot bytes. Worker materialization must use only immutable snapshot state and must not read mutable `document_edit` fields.
4. Require snapshot destruction before its owning `document_edit` is destroyed; document the lifecycle. Ensure partial allocation failures release all retained blobs and allocations.
5. Add focused tests for unedited and edited round trip, snapshot stability across subsequent edits/undo/redo, independent concurrent worker materialization while the owner mutates, undersized output, and destruction/refcount cleanup.
6. Document that recovery scheduling, app worker integration, baseline/assets/timeline snapshot composition, restart discovery, restore/conflict UI, and broad UI/E2E remain outstanding.
7. Keep changed C files below 800 lines; add no dependencies or cross-platform shims.

# Verification
- Strictly build `svgworkbench`, `document_edit_snapshot_test`, `document_edit_test`, and `document_edit_history_test` in Debug, ASan/UBSan, and UBSan presets.
- Run only `document-edit-snapshot`, `document-edit`, and `document-edit-history` focused CTests in each preset.
- Validate this frozen order and ledger; run `git diff --check`, verify `tests/test_map.json`, and check changed C source sizes.
- Do not run broad UI/E2E, corpus, or performance suites.

# Output
Leave changes uncommitted. Record exact commands/results and app-integration constraints in `artifacts/WO-PROJECT-06/verification.txt`, `STATE.md`, and `LEDGER.md`.
