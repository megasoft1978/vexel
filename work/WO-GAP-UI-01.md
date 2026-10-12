---
id: WO-GAP-UI-01
revision: 1
status: READY
milestone: M6
owner: project-owner
executor: deepseek-flash
base_sha: 27e7781207c746a9012dd4b15d98df0478cd82ef
spec_sha256: 969d99f3cca0267864077383631d08fc0e15db0ac4db656420c9a8cc5a5cc440
parallel_safe: false
network: false
---
# Goal
Close the failed and unverifiable acceptance criteria of `work/WO-UI-01.md` revision 2 by fixing the code to that spec (owner decision: fix code to spec; the parent spec is neither weakened nor superseded).

# Context pointers
- Parent spec `work/WO-UI-01.md` (r2) and verdict `artifacts/retro-verification-2026-10-12/WO-UI-01.verdict.txt`.
- `src/app/viewer_workspace.{c,h}` (table names `Geometry`/`Appearance`/`Attributes`), `src/app/viewer.c` (`viewer_persist_workspace`, `VEXEL_TEST_WORKSPACE`), `src/platform/platform_settings.h`, `src/app/viewer_chrome_editor.c` (status readout), `viewer_editor_set_operation_progress` in `src/app/viewer_editor.c` (no production callers), `src/app/viewer_export.c`, `src/app/viewer_chrome_render.c`.
- Inspector tabs from WO-UI-02: `EDITOR_BUTTON_INSPECTOR_TAB_{OBJECT,FILL,STROKE,DOCUMENT}` in `src/app/viewer_editor_internal.h`.
- `tests/editor/viewer_workspace_test.c`, `tests/editor/native_smoke_actions.sh`, `artifacts/WO-UI-01/golden-changes.md`, `scripts/pixdiff.py`.

# Existing contracts and invariants
- All parent invariants hold: workspace switching never touches the document; widget IDs stable; macOS-only, C17, no new dependency, files under 800 lines.
- No golden fixture, oracle or frozen test is weakened; a changed fixture needs a baseline-change entry with a side-by-side comparison.

# Files allowed to touch
- `src/app/**`, `src/ui/**`, `src/platform/**` (settings persistence only), `tests/editor/**`, `tests/ui/**` (not `tests/oracle/**`, `tests/golden/**`), `CMakeLists.txt`, `docs/ui/**`, `artifacts/WO-UI-01/golden-changes.md`, `artifacts/WO-UI-01/side-by-side-*.png`, `artifacts/WO-GAP-UI-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`.

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `tests/oracle/**`, `tests/golden/**`, `src/svg/**`, `src/raster/**`, `src/document/**`, `bench/**`, `reports/**`, every other prior artifact, every path outside the allowlist.

# Required implementation
1. Workspace table per parent item 2: `Draw` activates inspector tab `Object`; `Nodes` activates tab `Object` with its geometry/path section expanded; `Animate` keeps its current section and shows the timeline; `Inspect` activates tab `Document`. The table stays the single source for layout and semantic tree.
2. Settings persistence test: a focused CTest test stores a workspace name through the same store/load functions the app uses, reloads it, and gets the same workspace; a stored unknown name loads as `Draw` and performs zero settings writes. The test must not touch the user's real preferences (use a test-only domain or path; add that seam in `src/platform/**` if absent). Record the settings key in `docs/ui/README.md`.
3. Status readout: the right region shows selection count, zoom and document size, where document size is the source byte length formatted with a unit (e.g. `1.2 MB`). Route open/index, export and render progress messages through `viewer_editor_set_operation_progress` into the centre region; list every call site changed in `artifacts/WO-GAP-UI-01/progress-routing.md`.
4. Native smoke (`tests/editor/native_smoke_actions.sh`), starting from the pinned `VEXEL_TEST_WORKSPACE=draw`: save with Cmd-S and copy the saved file; then from `Draw` press `Control-PageUp` and assert `Inspect` is checked (backward wrap), press `Control-PageDown` and assert `Draw` is checked (forward wrap), and visit `Nodes`, `Animate` and `Inspect` with assertions; then save again and `cmp` the file with the copy taken before the sequence.
5. Side-by-side comparisons: for every PNG row in `artifacts/WO-UI-01/golden-changes.md`, write `artifacts/WO-UI-01/side-by-side-<fixture-stem>.png` (before left, after right, same scale) and add it to that row. Before = blob at `e31084f`; after = the blob whose SHA-256 prefix equals the row's after hash. Any fixture this order changes gets the same treatment in `artifacts/WO-GAP-UI-01/golden-changes.md`. A stdlib-only helper may live in `artifacts/WO-GAP-UI-01/`.
6. Record each verification command, its exit code and the last 40 output lines in `artifacts/WO-GAP-UI-01/check-NN.txt`.

# Acceptance criteria
- AC-1 `cmake --preset mac-debug && cmake --build --preset mac-debug && ctest --test-dir build/mac-debug --output-on-failure` exits 0.
- AC-2 `./dev verify` and `./dev t --all` exit 0, with output recorded.
- AC-3 `viewer_workspace_test` asserts the four table records of item 1, the settings round-trip and the unknown-name fallback with zero writes.
- AC-4 The status bar test asserts the document-size readout; `progress-routing.md` names each production call site of `viewer_editor_set_operation_progress`, and at least one exists for each of open/index, export and render.
- AC-5 `./dev editor-smoke` exits 0 and performs item 4, including both wraps, all four workspaces and the `cmp` of the two saves.
- AC-6 Every PNG row of `artifacts/WO-UI-01/golden-changes.md` names an existing side-by-side image; every fixture changed by this order has an entry in `artifacts/WO-GAP-UI-01/golden-changes.md`.
- AC-7 `python3 -m unittest tests.test_ui_chrome_golden tests.test_w15_accessibility_localization` exits 0.
- AC-8 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-UI-01 --ready`, `python3 scripts/wo_validate.py WO-GAP-UI-01 --ready` and `git diff --check` exit 0.
- AC-9 A fresh read-only verifier re-judges AC-1..AC-10 of `work/WO-UI-01.md` and AC-1..AC-8 here against the candidate commit and reports pass in `artifacts/WO-GAP-UI-01/verdict.txt`.

# Verification commands
```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
./dev verify
./dev t --all
python3 -m unittest tests.test_ui_chrome_golden tests.test_w15_accessibility_localization
./dev editor-smoke
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-UI-01 --ready
python3 scripts/wo_validate.py WO-GAP-UI-01 --ready
git diff --check
```

# Non-goals
No new workspace, no pane splitting, no change to editing semantics, renderer, parser or document model; no regeneration of goldens other than those the required changes alter.

# Budget
Executor max runs: 3; verifier max rounds: 2; elapsed hard limit: 240 min; dependency/network: none.

# Failure and escalation
If a parent after-hash matches no blob, record the row as `after blob not found` and stop for planner review. If progress for a path cannot be routed without touching a protected path, record the exact call site and stop rather than faking a progress message.

# Output
One commit `fix(ui): close WO-UI-01 gaps`, evidence under `artifacts/WO-GAP-UI-01/`, updated `STATE.md` and both ledgers, and the schema-valid executor report.
