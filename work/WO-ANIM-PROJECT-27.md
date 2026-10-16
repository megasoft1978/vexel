---
id: WO-ANIM-PROJECT-27
revision: 1
status: READY
milestone: M5
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: fe521d142ea5ced29593c88fda1bbe7fc29a411c1744b992fc03297966bd8414
parallel_safe: false
network: false
---
# Goal
Let a user convert a normal edited SVG into an immediately editable native
`.vexel` animation project, including a usable zero-track timeline from which a
first property key can be authored.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/CODING_STANDARD.md`, ADR-0018
  through ADR-0021, and the current product plan.
- Current Save As writes `{"version":1,"tracks":[]}` as the default timeline.
  This is opaque rather than the ADR-0020 native schema, so no native session is
  installed for the current editor after saving.
- A zero-track native timeline is hidden and event/layout/accessibility paths
  reject it. This prevents `Key Opacity` and `Key Transform` from creating the
  first user-facing track.
- Preserve the current SVG editor and its history during conversion. Do not
  reload the document as a shortcut; conversion should publish the project
  atomically and then install the already validated in-memory project/session.
- New shapes may not have stable SVG IDs. This order enables keying identified
  objects and reports the existing requirement clearly; automatic ID assignment
  and native timeline integration with standard Undo/Redo are separate follow-up
  work.
- Keep comprehensive UI/E2E acceptance, corpus runs, performance benchmarks,
  and fuzz suites deferred.

# Files allowed to touch
- `work/WO-ANIM-PROJECT-27.md`
- `src/project/project_file.c`, `src/project/project_file.h`
- `src/app/viewer_load.c`, `src/app/viewer_load.h`, `src/app/viewer_save.c`
- `src/app/viewer.c`, `src/app/viewer_display.c`, `src/app/viewer_render.c`
- `src/app/viewer_timeline.c`, `src/app/viewer_timeline.h`
- `src/app/viewer_timeline_layout.c`, `src/app/viewer_timeline_ui.c`
- `src/app/viewer_timeline_accessibility.c`
- `src/project/README.md`, `docs/product/vexel-goal-and-delivery-plan.md`
- `tests/app/project_save_test.c`, `tests/app/viewer_timeline_bootstrap_test.c`
- `CMakeLists.txt`, `tests/test_map.json`
- `artifacts/WO-ANIM-PROJECT-27/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all pre-existing working-tree changes. Do not edit the SVG parser or
  rasterizer, MCP transport/tool semantics, animation schema, project container
  layout, external corpus/goldens/oracles, platform support, frozen prior work
  orders/evidence, or paths outside the allowlist.
- Do not add a platform shim or dependency. Do not broaden recovery or file
  migration behavior.
- Do not run comprehensive UI/E2E, corpus, fuzz, performance, or release suites.

# Required implementation
1. Replace the invalid default timeline payload with a complete valid ADR-0020
   v1 empty timeline using explicit bounded timing, loop, and onion defaults.
2. Add a validated encoded-project publication API so SVG-to-`.vexel` conversion
   can encode once, validate and create the editable session against those exact
   bytes, atomically save those bytes, then transfer their ownership into the
   active document only after publication succeeds. Preserve the existing editor,
   decoded scene, selection, and SVG undo history. On any pre-publication error,
   leave active document state unchanged; on post-publication durability warning,
   install the published project and report the warning accurately.
3. Initialize the asset overlay and native timeline session for the converted
   project. Keep every borrowed scene, timeline, manifest, and asset slice alive
   for the full session lifetime; destroy the old owner only after all dependent
   views are replaced.
4. Make the zero-track native timeline visible, paintable, accessible, and able
   to receive pointer/keyboard events. Avoid `track_count - 1` underflow and
   other zero-count indexing. The native empty project must expose the existing
   Key Opacity and Key Transform actions for an identified selection; after the
   first successful action, refresh and display its track and native preview.
   Keep empty SVG/SMIL timeline behavior unchanged.
5. Add focused regression coverage for default payload validation, successful
   SVG-to-project conversion, exact scene preservation, atomic failure keeping
   the active editor usable, zero-track layout and semantics, first-key creation,
   and post-key rendering. Document remaining stable-ID and standard Undo/Redo
   limitations accurately.
6. Keep all modified C/Objective-C source and test files below 800 lines.

# Verification
- Confirm exact registered CTest selectors before execution.
- Strictly build `svgworkbench`, `project_save_test`, and the focused native
  timeline bootstrap test in Debug, ASan/UBSan, and configured UBSan builds.
- Run only the project-save and native-timeline-bootstrap focused CTests in all
  three configurations.
- Validate this frozen order and ledger; validate `tests/test_map.json`; verify
  focused selectors and changed-path scope; run `git diff --check`; report all
  changed C/Objective-C/test source line counts.
- No comprehensive UI/E2E suite, corpus, fuzz, performance, or release run.

# Output
Leave changes uncommitted. Record exact verification and remaining limitations
in `artifacts/WO-ANIM-PROJECT-27/verification.txt`, `STATE.md`, and both ledgers.
Keep the full product goal active.
