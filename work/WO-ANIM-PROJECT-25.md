---
id: WO-ANIM-PROJECT-25
revision: 2
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 730844098792e74bc7bc664afffe8ef29d2609830ce81f7f45f9296e2be6d463
parallel_safe: false
network: false
---
# Goal
Add user-facing native transform key authoring so a user can animate a selected SVG object by repositioning it on the canvas and keying the resulting transform at the playhead.

# Context
- Read `OWNER-DIRECTIVES.md`, `AGENTS.md`, ADR-0018 through ADR-0020, `docs/product/vexel-goal-and-delivery-plan.md`, and the focused native timeline tests.
- Native project schema v1, composition, deterministic playback/export, MCP keys, and timeline UI opacity keying are implemented.
- Transform tracks are already a supported `transform2d` property with six affine coefficients. The timeline UI currently exposes only `Key Opacity`, leaving users without direct transform key authoring.
- Preserve current SVG bytes and unknown metadata through the existing document edit/history model. Keep the final comprehensive UI/E2E pass deferred.

# Contracts
- A transform key captures the selected identified SVG element's current supported affine transform at the exact native integer tick under the playhead.
- Support identity/no-transform and the SVG `matrix(a b c d e f)` form. If the selected element uses another transform syntax or malformed matrix, refuse atomically with a clear status; do not approximate or rewrite it.
- The first transform key must be at tick zero so the project has an explicit baseline. If no matching transform track exists and the playhead is later than zero, refuse with a clear instruction to create the baseline key at tick zero first. At later ticks, create/reuse the stable-target transform track and upsert the current key in one undoable transaction.
- Existing transform track keys are upserted at the playhead. Repeating the action with unchanged data is a no-op.
- Refresh the timeline and preview after success; selection, current scene, and project dirty state remain coherent. Undo/redo must restore scene and timeline bytes together.
- Expose the action with a discoverable timeline control and an equivalent semantic accessibility node. Hide or disable it where the current layout cannot fit the control.

# Files allowed to touch
- `src/app/viewer_editor.h`
- `src/app/viewer_editor_document.c`
- `src/app/viewer_timeline.h`
- `src/app/viewer_timeline_layout.c`
- `src/app/viewer_timeline_ui.c`
- `src/app/viewer_timeline_accessibility.c`
- `src/app/viewer_timeline_transform.c` (new focused module)
- `CMakeLists.txt`
- `tests/app/viewer_timeline_native_test.c`
- `work/WO-ANIM-PROJECT-25.md`
- `artifacts/WO-ANIM-PROJECT-25/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, ADRs, prior frozen work orders/artifacts, `src/project/**`, MCP, SVG parser/rasterizer, broad UI/E2E suites, website, release assets, and every path outside the allowlist.
- Preserve all existing unrelated worktree changes. Do not run the comprehensive UI/E2E acceptance suite or commit.

# Required implementation
1. Add a bounded selected-object API that returns its stable SVG ID and affine transform, recognizing only absent transform or a valid six-number `matrix(...)`; reject duplicate IDs, unsupported transform functions, malformed values, non-finite numbers, and out-of-range input.
2. Add an isolated timeline action that creates/reuses a stable-ID `transform` / `transform2d` property track and keys the captured value at the current native tick. A later first key is rejected atomically; do not invent a baseline from the already-moved scene. Track creation plus its tick-zero key and later key upserts each use one session transaction.
3. Add a timeline control with layout-aware visibility, a click handler, clear success/error status, preview invalidation, selected track update, and the existing mutation notification path.
4. Add an accessible semantic button matching the visible control and keyboard activation behavior supported by the existing UI framework.
5. Add focused tests for matrix parsing (including whitespace/sign/exponent), identity, unsupported transforms, invalid/non-finite/out-of-range input, tick-zero baseline requirement, later key upserts, rejection of a later first key, track reuse, atomic failure, scene/timeline dirty state, undo/redo, accessibility semantics, and layout visibility at compact and regular widths.
6. Keep all changed C and `.inc` sources below 800 lines. Update focused evidence and `STATE.md`/ledgers; do not modify the product goal to mark broader animation complete.

# Verification
- Freeze this work order hash before implementation.
- Run strict Debug, ASan/UBSan, and configured UBSan builds for the focused native timeline test and `svgworkbench`; run only the focused test executable.
- Validate work-order hash, ledger, changed-path scope, source line caps, and `git diff --check`.
- Do not run comprehensive UI/E2E, corpus, or performance suites.

# Output
Leave changes uncommitted. Record the user-facing transform key workflow, focused verification evidence, and remaining property authoring, recovery, fidelity/scale, MCP progress/cancellation, final acceptance, and showcase work.
