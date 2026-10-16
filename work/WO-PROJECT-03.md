---
id: WO-PROJECT-03
revision: 1
status: READY
milestone: PROJECT
owner: project-owner
executor: gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 081061195aa76e23badc8c0b8be171c10a787db48600d3e4fcab159e0942d4e9
parallel_safe: false
network: false
---
# Goal
Expose a document edit API that marks the current edit state as saved without changing source bytes, selection, or undo history. This is a prerequisite for atomic non-SVG document saves.

# Context
- `document_edit_save_as` currently marks a state saved only after its SVG write succeeds.
- Native project files are written through a separate container writer; that path must mark the exact current editor state saved only after publication succeeds.
- Keep the operation small and independent of file I/O.

# Files allowed to touch
- `work/WO-PROJECT-03.md`
- `src/document/document_edit.h`, `src/document/document_edit_history.c`, `src/document/document_edit_piece_table.c`
- `tests/edit/test_document_edit.c`
- `CMakeLists.txt`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-PROJECT-03/**`

# Protected paths
- All paths outside the allowlist, including app integration and all frozen orders.
- Preserve pre-existing working tree changes.

# Required implementation
1. Add `document_edit_mark_saved(document_edit *)` that updates only the saved checkpoint to the current state.
2. Preserve an undo boundary when a coalescible edit is followed by a new edit after marking saved. Test clean initial state, dirty after edit, clean after mark, dirty after subsequent edit, undo back to the marked state becomes clean, redo becomes dirty, and re-marking updates the checkpoint.
3. Document that file publication must have succeeded before calling the API.

# Acceptance criteria
- Focused Debug and ASan/UBSan document-edit tests pass.
- Work order validator, ledger validator, and `git diff --check` pass.
- Independent read-only Codex review passes.
- No UI or end-to-end test suites run.

# Verification commands
```text
cmake --preset mac-debug
cmake --build build/mac-debug --target document_edit_test
ctest --test-dir build/mac-debug -R '^document-edit$' --output-on-failure
cmake --preset mac-asan
cmake --build build/mac-asan --target document_edit_test
ctest --test-dir build/mac-asan -R '^document-edit$' --output-on-failure
python3 scripts/wo_validate.py WO-PROJECT-03 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```

# Output
Saved-checkpoint API and focused evidence; resume WO-PROJECT-02.
