---
id: WO-PROJECT-07
revision: 1
status: READY
milestone: PROJECT
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 018e6841c819cd0d8666fbe35a68f00887a2caa31bcc57e6fb110678f11a1a52
parallel_safe: false
network: false
---
# Goal
Integrate periodic background recovery snapshots for dirty native `.vexel` documents using immutable scene/timeline views and retained project assets.

# Context
- Follow OWNER-DIRECTIVES.md, ADR-0021, and WO-PROJECT-04 through WO-PROJECT-06.
- Vexel uses one worker job at a time. Document replacement waits for current work to finish, so the loaded project and asset blocks remain alive while a recovery job runs.
- Scene snapshot materialization must remain on the worker. Timeline source needs an immutable lease because edit history can be trimmed during continued authoring. Project asset payloads are immutable after insertion; copy only asset descriptors on the UI thread.
- Capture only dirty native projects with a saved on-disk source baseline. Do not change source file, dirty flags, undo history, or saved-state markers.

# Files allowed to touch
- `work/WO-PROJECT-07.md`
- `src/project/project_timeline_edit.h`, `src/project/project_timeline_edit.c`, `src/project/project_timeline_session.h`, `src/project/project_timeline_session.c`
- `src/app/viewer_internal.h`, `src/app/viewer_editor.h`, `src/app/viewer_editor_document.c`, `src/app/viewer.c`, `src/app/viewer_recovery.c`
- `tests/project/project_timeline_edit_test.c`, `tests/app/viewer_recovery_test.c`, `tests/test_map.json`, `CMakeLists.txt`
- `src/project/README.md`, `src/app/README.md`, `artifacts/WO-PROJECT-07/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Recovery capsule codec and store implementation, other project formats, platform adapters, website/deployment, frozen prior work orders/artifacts, and all paths outside the allowlist. Preserve existing working-tree changes.

# Required implementation
1. Add an immutable timeline-source lease that pins the current timeline bytes across edits and history trimming. It must expose bytes without copying and be destroyed on the UI thread before the owning session is destroyed.
2. Add a recovery worker job for dirty native `.vexel` documents. Capture the scene piece-table snapshot, timeline lease, copied asset descriptors, manifest, source path, and current saved-file size/mtime before submission.
3. Materialize the scene and atomically publish the complete capsule only on the worker. Never mutate source, dirty state, timeline saved state, asset overlay state, or undo history.
4. Schedule a best-effort capture at a bounded interval when the document is dirty and no load/close transition is pending. Use the WO-PROJECT-05 private store path. Expose a concise status for success/failure without modal UI.
5. Destroy the document snapshot and timeline lease after worker completion, including cancellation and failed submission paths. Keep all borrowed project/asset bytes valid until then.
6. Add focused tests for timeline lease stability after edits/history eviction and recovery job round-trip/preservation of dirty state and project assets. No broad UI/E2E.
7. Document remaining restore discovery/conflict UI, capsule removal on save/discard, retention, migration, and final broad acceptance.

# Verification
- Strictly build `svgworkbench`, focused recovery test targets, timeline edit test, and existing project recovery test in Debug, ASan/UBSan, and UBSan presets.
- Run only `viewer-recovery`, `document-edit-snapshot`, `project-timeline-edit`, and `project-recovery` focused CTests in each preset.
- Validate this frozen order and ledger; run `git diff --check`, verify `tests/test_map.json`, and check changed C files remain below 800 lines.
- Do not run broad UI/E2E, corpus, or performance suites.

# Output
Leave changes uncommitted. Record exact results and remaining recovery limitations in `artifacts/WO-PROJECT-07/verification.txt`, `STATE.md`, and `LEDGER.md`.
