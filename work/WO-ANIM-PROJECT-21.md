---
id: WO-ANIM-PROJECT-21
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: d8fd1603d52ed1b219904594321be0c8be0d0d33c8f03fa6be9285b7d7ecf96f
parallel_safe: false
network: false
---
# Goal
Allow artists to create a native drawing track from the timeline UI for the
currently selected identified SVG object, then use the existing Add Cel action
to place drawings on that track.

# Context
- Follow ADR-0018, ADR-0019, ADR-0020 and owner directives. New native drawing
  tracks require stable SVG target IDs and a valid tick-zero drawing exposure.
- WO-20 added live asset selection and cel placement on an existing selected
  drawing track. The timeline currently has no direct track-creation action.
- Use `viewer_editor_selected_opacity_target` as the existing bounded stable-ID
  selection accessor; require a selected object with a supported effective
  opacity representation. Do not insert IDs or modify SVG source in this order.
- Keep all C and `.inc` files under 800 lines. Preserve unrelated worktree data.

# Contracts
- The existing drawing action presents “Add Track” when the selected timeline
  row is not a drawing track and “Add Cel” when it is. The accessible name
  describes the current operation.
- Add Track requires the selected asset to match the scene canvas and an
  identified selected SVG object. It creates a unique stable drawing-track ID,
  targets that object, and creates the model-required initial tick-zero cel.
- A successful action refreshes rows, selects the new track, invalidates the
  preview, signals the existing native mutation path, and reports the result.
  One session undo removes the created track; redo restores it.
- Add Cel behavior for a selected drawing track remains unchanged.
- Focused timeline model/accessibility checks only. Do not run broad UI/E2E or
  release acceptance; no commit.

# Files allowed to touch
- `src/app/viewer_timeline.h`, `src/app/viewer_timeline_ui.c`,
  `src/app/viewer_timeline_accessibility.c`, `src/app/viewer_timeline_cel.c`
- `tests/app/viewer_timeline_test.c`
- `work/WO-ANIM-PROJECT-21.md`, `artifacts/WO-ANIM-PROJECT-21/**`,
  `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Required behavior
1. Add a query for the timeline action label based on selected track kind.
2. When no drawing track is selected, derive the selected SVG target, choose
   the current valid asset, generate a unique bounded track ID, and add the
   track through the existing session API.
3. Refresh native rows and select the new track. Ensure its tick-zero exposure
   renders; verify undo and redo.
4. Update painted and accessible control labels. Test missing selection,
   property-track selection, duplicate-ID avoidance, action state, initial cel,
   preview, dirty/mutation, and undo/redo in the focused timeline executable.

# Verification
- Freeze before implementation and validate the recorded SHA-256.
- Strict Debug, ASan/UBSan, and configured UBSan builds of `viewer_timeline_test`
  and `svgworkbench`; directly run the focused timeline test binary in each.
- Validate ready order, ledger, file allowlist, changed C/`.inc` line caps, and
  `git diff --check`.
- Do not run comprehensive UI/E2E or release suites.

# Output
Leave changes uncommitted. Report Add Track/Add Cel interaction, verification,
and remaining stable-ID creation for untagged nodes, deterministic animation
export, final acceptance, and the recorded Codex/Vexel website/README showcase.
