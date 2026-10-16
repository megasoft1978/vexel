---
id: WO-ANIM-PROJECT-12
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: c27a30f0199e4d88a848c27e458b41c8563ec976157abcdc0e6628e5dcbb4f5c
parallel_safe: false
network: false
---
# Goal
Render saved native before/after onion-skin previews in the paused app timeline, with bounded work and undoable project preference changes.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, ADR-0018/0019/0020, WO-ANIM-PROJECT-11, project timeline decode/edit/session/render, viewer timeline paint/layout/accessibility, and focused tests.
- ADR-0020 already defines `onion.before`, `onion.after` (0..8 each), and `onion.opacity` (0..1). The timeline decoder validates these values but does not expose them in the decoded view. The app renderer currently renders one native tick and has no onion preview.
- The paused app preview is presentation-only. `project_animation_render_frame` and MCP export must continue to render the selected frame without onion ghosts.

# Existing contracts and invariants
- macOS Apple Silicon only, C17, no new dependency, exact user SVG/project bytes preserved outside the targeted onion setting edit, and unknown/opaque timeline payloads stay untouched.
- Session bytes and decoded view lifetimes follow `project_timeline_session.h`; edits are validated, undoable, redoable, and saved through existing `.vexel` serialization.
- Native frame ticks remain integer-addressed. Onion neighbors use the validated exact `ticks_per_frame = ticks_per_second * fps_denominator / fps_numerator`; do not wrap across the playback range.
- Onion overlays are preview-only and are suppressed during playback. They never alter normal native frame rendering, SVG SMIL rendering, MCP render/export results, or saved frame pixels.
- Extra onion rendering is bounded to 16,777,216 destination pixels per paused preview presentation. Render neighboring frames sequentially and hold at most one neighboring RGBA tile at a time in addition to the current preview. If no neighbor fits the remaining budget, keep the current preview and report the onion preview limit accessibly.
- Project source files remain under 800 lines. Keep changes uncommitted. Do not run broad UI/E2E suites.

# Files allowed to touch
- `src/project/project_timeline.h`, `src/project/project_timeline.c`
- `src/project/project_timeline_edit.h`, `src/project/project_timeline_edit.c`
- `src/project/project_timeline_session.h`, `src/project/project_timeline_session.c`
- `src/project/README.md`
- `src/app/viewer_timeline.h`, `src/app/viewer_timeline.c`, `src/app/viewer_timeline_ui.c`, `src/app/viewer_timeline_layout.c`, `src/app/viewer_timeline_accessibility.c`, `src/app/viewer_events.c`, `src/app/viewer_render.c`
- `tests/project/project_timeline_test.c`, `tests/project/project_timeline_edit_test.c`, `tests/project/project_timeline_session_test.c`, `tests/project/project_animation_render_test.c`, `tests/app/viewer_timeline_test.c`
- `CMakeLists.txt`, `work/WO-ANIM-PROJECT-12.md`, `artifacts/WO-ANIM-PROJECT-12/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Native container/schema versioning, drawing-asset model, SVG/XML/raster implementation, MCP protocol/tools, release files, scripts, presets, all UI/E2E suites, fixtures/goldens/oracles, previous artifacts and every path outside the allowlist.

# Required implementation
1. Carry validated `onion.before`, `onion.after`, and `onion.opacity` into `project_timeline_view` without changing parsing acceptance or source bytes.
2. Add `project_timeline_edit_set_onion` and session wrapper. Accept counts 0..8 and finite opacity 0..1; update only the `onion` JSON object, validate before commit, preserve other source bytes, and support atomic undo/redo.
3. Add one native timeline control that cycles symmetric onion counts `0 → 1 → 2 → 4 → 8 → 0`, preserving saved opacity. A successful change refreshes the native view, invalidates the preview, updates dirty/title/status state, and does not schedule a scene render or reset playback time.
4. While paused, render neighboring frame ticks around the current tick using the exact rational frame spacing. Composite previous samples with a red tint and following samples with a cyan tint behind the current preview, applying the saved opacity. Skip out-of-range neighbors without wrapping. Render farther neighbors only while the aggregate 16,777,216 destination-pixel budget allows; render sequentially and release each temporary tile before the next.
5. Suppress onion rendering during playback. Cache the composite only for its current tick, settings, context, and paused state. Setting changes, seeking, playback changes, scene edits, MCP timeline mutations, or context refresh must never display stale onion pixels.
6. Expose the onion control and its enabled/count state through accessibility. If a frame size leaves no budget for any onion sample, expose a concise “onion preview limited” status while still presenting the normal current frame.
7. Add focused tests for decoded preference metadata, validation/error bounds, setting edit source preservation and undo/redo, previous/next overlay pixels and tint order, range boundaries, paused-only behavior, cache invalidation, and the work budget. Verify the existing project frame renderer and SMIL timeline behavior remain unchanged.
8. Document that onion skins are paused editor previews only and are excluded from project render/export output.

# Acceptance criteria
- AC-1: focused Debug app/project targets build with strict warnings.
- AC-2: project and viewer timeline focused tests pass, including setting persistence, undo/redo, tint/order, boundary skipping, and cache invalidation.
- AC-3: the same focused tests pass under ASan; the configured UBSan timeline/project tests pass where registered.
- AC-4: only the paused native app preview displays bounded tinted before/after samples; normal project renderer, MCP export, SMIL preview, and saved project SVG/timeline source retain their existing semantics.
- AC-5: setting changes persist through `.vexel` save, undo, and redo; opacity and counts remain within ADR-0020 limits.
- AC-6: focused accessibility semantics expose the onion control, its current counts, and work-limit status.
- AC-7: `python3 scripts/wo_validate.py WO-ANIM-PROJECT-12 --ready`, `python3 scripts/ledger.py validate ledger.jsonl`, source line counts, allowlist, and `git diff --check` pass.
- AC-8: scoped independent review finds no blocking/high issue.

# Verification sequence
- Build Debug and ASan targets `svgworkbench`, `viewer_timeline_test`, `project_timeline_test`, `project_timeline_edit_test`, `project_timeline_session_test`, and `project_animation_render_test`.
- Run only focused CTests matching `^(viewer-timeline|project-timeline|project-timeline-integration|project-timeline-edit|project-timeline-session|project-animation-render)$` in Debug and ASan. Build/run the same available focused UBSan targets.
- Run the frozen order validator, ledger validator, source line checks for changed C files, changed-path allowlist check, and `git diff --check`.
- Request one independent read-only review limited to allowed implementation/test paths.
- Record exact commands and outputs under `artifacts/WO-ANIM-PROJECT-12/`. Do not run `./dev t --all`, UI golden automation, or broad UI/E2E tests.

# Non-goals
- Native drawing asset creation/editing, onion settings over MCP, property-track evaluation, target-relative placement, export, animation playback authoring, and broad UI/E2E acceptance.

# Budget
One implementation pass and one independent verifier; elapsed hard limit 120 minutes; no network; no commit.

# Failure and escalation
- If exact rational frame spacing cannot be derived from a validated view, disable onion previews and report the relevant timing values; do not round key positions.
- If the aggregate render work, tile memory, or setting edit cannot remain bounded and source-preserving, stop at the failing seam, retain focused evidence, and leave the goal active.

# Output
Leave changes uncommitted. Record visible paused onion preview, focused verification evidence, and remaining animation authoring/export/UI acceptance gaps.
