---
id: WO-PROJECT-08
revision: 1
status: READY
milestone: PROJECT
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: eb581cc286eae92ca0e2831b864483587af2505c534f6d0114efe827adb85ee8
parallel_safe: false
network: false
---
# Goal
Discover a recovery capsule when opening its existing native source, validate its identity and payload, offer an explicit restore/discard decision, report changed-source conflicts, and remove capsules after normal save or explicit discard.

# Context
- Follow OWNER-DIRECTIVES.md, ADR-0021, and WO-PROJECT-04 through WO-PROJECT-07.
- WO-PROJECT-07 writes capsules only for dirty native projects. No startup discovery or user choice exists.
- The current recovery store key resolver canonicalizes only existing source files. Missing-source discovery is excluded and requires a separate identity/API design.
- A recovered document must remain unsaved and must never replace the source file without an explicit later Save action.

# Files allowed to touch
- `work/WO-PROJECT-08.md`
- `src/app/viewer_internal.h`, `src/app/viewer_load.h`, `src/app/viewer_load.c`, `src/app/viewer.c`, `src/app/viewer_recovery.c`, `src/app/viewer_save.c`, `src/app/viewer_runtime.c`, `src/app/mcp_broker_hook.c`
- `src/document/document_edit.h`, `src/document/document_edit_history.c`
- `src/platform/macos/platform_dialog.h`, `src/platform/macos/platform_dialog.m`
- `tests/app/viewer_recovery_test.c`, `tests/platform/platform_dialog_test.m`, `tests/test_map.json`, `CMakeLists.txt`
- `src/app/README.md`, `src/project/README.md`, `artifacts/WO-PROJECT-08/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Recovery capsule codec/store implementation and other platform adapters; project format/schema and timeline semantics; website/deployment; frozen prior work orders/artifacts; all paths outside the allowlist. Preserve all working-tree changes.

# Required implementation
1. Inspect the expected capsule for an opened existing `.vexel` source on a worker. Bound file reads by `PROJECT_RECOVERY_MAX_BYTES`; validate capsule CRC, embedded bundle, and exact source path before offering recovery.
2. Compare the capsule's saved source size/mtime with the current source stamp. Matching snapshots offer explicit Restore / Discard / Cancel choices. Changed-source snapshots show a conflict and offer explicit recovery as unsaved work or discard; never write the source during restore.
3. Restoring loads the complete recovered project, including scene, animation, metadata, and assets, as dirty/unsaved state. Preserve source path and baseline stamp. Normal open remains current if user discards or cancels.
4. Remove the keyed capsule after explicit discard and after a successful normal save to the same source. Do not remove it on failed/canceled save or failed recovery load. Make cleanup failures visible without claiming the capsule was removed.
5. Add focused tests for valid/matching recovery choice, changed-source conflict, wrong-path/corrupt capsule rejection, restored dirty state/assets/timeline, discard, save cleanup, and failure preservation. No broad UI/E2E.
6. Document missing-source discovery, retention, migration, large-file recovery-memory behavior, and final UI/E2E as remaining work.

# Verification
- Strictly build `svgworkbench`, `viewer_recovery_test`, and `platform_dialog_test` in Debug, ASan/UBSan, and UBSan presets.
- Run only `viewer-recovery`, `platform-dialog`, `project-recovery`, and `project-file` focused CTests in each preset.
- Validate this frozen order and ledger; run `git diff --check`, verify `tests/test_map.json`, and report source sizes.
- Do not run broad UI/E2E, corpus, or performance suites.

# Output
Leave changes uncommitted. Record exact results and remaining recovery limitations in `artifacts/WO-PROJECT-08/verification.txt`, `STATE.md`, and the ledgers.
