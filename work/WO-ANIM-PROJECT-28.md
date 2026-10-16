---
id: WO-ANIM-PROJECT-28
revision: 1
status: READY
milestone: M5
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: 3295000e3f27ebac918d520a8c507d8e9b7c434846ee12d2ddcb3ea3a81230aa
---
# Goal
Route standard Undo/Redo to native animation history in the Animate workspace,
with SVG edit history as fallback and coherent app state after timeline changes.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/CODING_STANDARD.md`,
  ADR-0018 through ADR-0021, and `docs/product/vexel-goal-and-delivery-plan.md`.
- WO-ANIM-PROJECT-27 established native session timeline edits and deferred
  integration with standard Undo/Redo.
- Preserve pre-existing working-tree changes; this repository is intentionally
  dirty and uncommitted.

# Existing contracts and invariants
- Native timeline and SVG edit histories remain separate stacks.
- Routing policy: only in Animate workspace, prefer a native timeline stack
  when the requested direction has available history; otherwise use SVG history.
  Other workspaces always use SVG history.
- Undo History popover remains SVG-only and must be labeled/documented accurately.
- Refresh timeline borrowed views after native history mutations. Invalidate
  preview, update dirty title and render state. Do not reload the document.

# Files allowed to touch
- `work/WO-ANIM-PROJECT-28.md`
- `src/project/project_timeline_session.c`, `src/project/project_timeline_session.h`
- `src/app/viewer_editor.h`, `src/app/viewer_editor_internal.h`
- `src/app/viewer_editor_input.c`, `src/app/viewer_editor_document.c`
- `src/app/viewer_editor.c`, `src/app/viewer_editor_actions.c` (new)
- `src/app/viewer_chrome_editor.c`, `src/app/viewer_editor_palette.c`
- `src/app/viewer_history.c` (new), `src/app/viewer_internal.h`, `src/app/viewer.c`
- `CMakeLists.txt`, `tests/test_map.json`
- `tests/app/viewer_history_test.c` (new)
- `src/project/README.md`, `docs/product/vexel-goal-and-delivery-plan.md`
- `artifacts/WO-ANIM-PROJECT-28/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all pre-existing working-tree changes outside this order's exact
  edits. Do not edit SVG parsing/rasterization, MCP semantics, animation schema,
  project container, corpus/goldens/oracles, platform support, or prior frozen
  work orders/evidence.
- No dependencies, platforms, broad UI/E2E tests, corpus, fuzz, performance, or
  release suites.

# Required implementation
1. Expose safe native timeline session undo and redo availability.
2. Provide an editor history override callback used by can-undo/can-redo and
   standard keyboard and toolbar actions. Querying availability must not mutate.
3. In Animate workspace, execute native undo/redo when that direction is
   available; fall back to SVG history otherwise. Other workspaces use SVG.
   Ensure menus and command dispatch continue to reflect shared enablement.
4. After native mutation refresh timeline state, invalidate preview, mark frame
   dirty, update title and user status. Do not trigger SVG edit rendering.
5. Keep Undo History popover clearly limited to SVG edit history.
6. Add focused regressions for availability, Animate routing/fallback, other
   workspace behavior, undo/redo state refresh, and unchanged SVG history.
   Keep modified source/test files below 800 lines.

# Verification
- Confirm registered focused CTest selector before execution.
- Build `svgworkbench` and focused `viewer-history` test in Debug, ASan/UBSan,
  and configured UBSan build directories.
- Run only `viewer-history` CTest in those configurations.
- Validate frozen order, test map, ledger, changed-path scope, line limits, and
  `git diff --check`.
- Do not run comprehensive UI/E2E, corpus, fuzz, performance, or release suites.

# Output
Leave changes uncommitted. Record exact evidence in
`artifacts/WO-ANIM-PROJECT-28/verification.txt`, `STATE.md`, `LEDGER.md`, and
`ledger.jsonl`. Keep the full product goal active.
