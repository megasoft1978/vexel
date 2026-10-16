---
id: WO-ANIM-PROJECT-19
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 1245a6e254a6630f0e0188d6a77d9729ede2a9b33ca140550c9b9e03016ce3fc
parallel_safe: false
network: false
---
# Goal
Let an artist choose an existing native SVG drawing asset and place it as a cel
on the selected drawing track at the timeline playhead, with visible and
accessible controls and undoable session behavior.

# Context
- Follow ADR-0018, ADR-0019, ADR-0020, and owner directives. Native cels refer
  to immutable named drawing assets and use integer project ticks.
- WO-18 added live bounded asset lookup and MCP asset insertion. The timeline UI
  can key selected object opacity but cannot select a drawing asset or author a
  cel exposure.
- This order adds cel exposure authoring to an existing selected drawing track.
  Track creation and drawing import remain separate workflows.
- Keep all C and `.inc` files under 800 lines. Preserve unrelated worktree data.

# Contracts
- Native timeline keeps a selected asset index bounded by the current live
  bundle. Cycling the selector traverses only renderable `image/svg+xml` assets.
- Show the selected asset name in a compact toolbar control. The adjacent Add
  Cel action upserts a cel on the selected drawing track at the integer tick
  derived from the playhead; it refuses property tracks, invalid tick positions,
  missing assets, and non-native sessions with a visible status message.
- A successful add refreshes native timeline rows, invalidates cached frame and
  onion previews, marks a native mutation for the existing undo/history path,
  and reports the asset and tick. Existing session undo/redo applies.
- Expose both controls in timeline accessibility with the current asset name and
  clear action name. Fit them into the existing responsive toolbar.
- Focused timeline model/accessibility checks only. Do not run broad UI/E2E or
  release acceptance; no commit.

# Files allowed to touch
- `src/app/viewer_timeline.h`, `src/app/viewer_timeline_layout.c`,
  `src/app/viewer_timeline_ui.c`, `src/app/viewer_timeline_accessibility.c`
- `tests/app/viewer_timeline_test.c`
- `work/WO-ANIM-PROJECT-19.md`, `artifacts/WO-ANIM-PROJECT-19/**`,
  `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Required behavior
1. Add bounded native asset selection and cycling over SVG assets in the live
   bundle; the label identifies the selected asset.
2. Add a cel to the selected drawing track at the current playhead tick using
   the selected asset. Replacing a same-tick cel follows existing session
   upsert semantics and remains undoable.
3. Refresh row data and invalidate timeline preview after mutation. Ensure
   playback at the changed tick renders the selected drawing.
4. Add responsive painting, pointer handling, and accessibility for selector
   and Add Cel. Test asset cycling, invalid/no selection, tick boundary,
   mutation notification, cel data, preview refresh, undo/redo, and accessible
   labels in the focused timeline executable.

# Verification
- Freeze before implementation and validate the recorded SHA-256.
- Strict Debug, ASan/UBSan, and configured UBSan builds of `viewer_timeline_test`
  and `svgworkbench`; directly run the focused timeline test binary in each.
- Validate ready order, ledger, file allowlist, changed C/`.inc` line caps, and
  `git diff --check`.
- Do not run comprehensive UI/E2E or release suites.

# Output
Leave changes uncommitted. Report asset selection and cel placement behavior,
verification evidence, and the remaining drawing-track creation, animation
export, final acceptance, and recorded Codex/Vexel website/README showcase.
