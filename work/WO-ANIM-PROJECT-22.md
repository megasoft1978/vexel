---
id: WO-ANIM-PROJECT-22
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 208bd1aac4d8180ace463d2b33453d382a5f0c2351ebafe9e59a6df131a883af
parallel_safe: false
network: false
---
# Goal
Complete native drawing-track creation from the timeline UI and extract its
focused native-timeline fixture into a separate test executable so all C files
stay below 800 lines.

# Context
- Follow ADR-0018, ADR-0019, ADR-0020 and owner directives. Drawing tracks have
  stable SVG target IDs and a tick-zero exposure.
- WO-20 added asset selection and cel placement. WO-21 implemented an Add
  Track/Add Cel timeline action for identified SVG nodes, but its extra checks
  took the existing timeline test source above the repository 800-line cap.
- Preserve the WO-21 implementation; move the native project timeline fixture
  and its related checks into a dedicated focused executable.
- Existing selected-ID access requires a supported effective opacity value.
  This slice does not generate IDs for untagged nodes or unify undo stacks.
- Preserve unrelated worktree data. No broad UI/E2E or release acceptance; no
  commit.

# Contracts
- “Add Track” appears when the selected timeline row is not a drawing track;
  “Add Cel” appears for a selected drawing track. Accessibility uses a matching
  action description.
- Add Track uses a selected identified SVG node and canvas-matched SVG asset,
  creates a unique track ID and a named drawing track with its tick-zero cel,
  refreshes rows/preview, selects the new track and signals the existing native
  mutation path.
- A session undo removes the created track and redo restores it. Add Cel for a
  selected drawing track retains same-tick upsert behavior.
- Keep `viewer_timeline_test.c` and every new/changed C or `.inc` source below
  800 lines by moving its native project timeline test function to
  `viewer_timeline_native_test.c` with a separate executable.

# Files allowed to touch
- `CMakeLists.txt`
- `src/app/viewer_timeline.h`, `src/app/viewer_timeline_ui.c`,
  `src/app/viewer_timeline_accessibility.c`, `src/app/viewer_timeline_cel.c`
- `tests/app/viewer_timeline_test.c`,
  `tests/app/viewer_timeline_native_test.c`
- `work/WO-ANIM-PROJECT-21.md`, `work/WO-ANIM-PROJECT-22.md`,
  `artifacts/WO-ANIM-PROJECT-21/**`, `artifacts/WO-ANIM-PROJECT-22/**`,
  `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Required behavior
1. Keep the dynamic Add Track/Add Cel action and unique stable ID generation
   for identified selected objects; retain canvas compatibility checks.
2. Move native project timeline test fixture from the oversized general timeline
   test source into a separate focused test executable; retain its playback,
   cel, property-key, accessibility and new track-creation assertions.
3. Verify initial cel contents and render, action labels, stable-ID target,
   unique ID across repeated track creation, dirty/mutation signal, and undo/redo.
4. Verify both focused timeline executables in Debug, ASan/UBSan and configured
   UBSan, together with the strict app build.

# Verification
- Freeze before implementation and validate the recorded SHA-256.
- Strict Debug, ASan/UBSan, and configured UBSan builds of both focused timeline
  executables and `svgworkbench`; run only those focused executables.
- Validate ready order, ledger, allowed paths, all changed C/`.inc` line caps,
  and `git diff --check`.
- Do not run comprehensive UI/E2E or release suites.

# Output
Leave changes uncommitted. Report Add Track/Add Cel behavior, tests and evidence.
Remaining: unique-ID generation for untagged nodes with coherent undo, animation
export, final broad acceptance, and the recorded Codex/Vexel website/README
showcase.
