---
id: WO-ANIM-PROJECT-11
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 5399b5e79feb529a2a62a067f0137b6a75f60a9aeef8185ef51364c7d79ba716
parallel_safe: false
network: false
---
# Goal
Make native-v1 drawing animation visible and playable in the application. The existing timeline dock must show native track names and cel exposure markers, while its existing play, pause, speed, mode, and scrubber controls drive the integer-tick native renderer.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0018/0019/0020, WO-ANIM-PROJECT-08/09/10, viewer load/render/editor ownership, and timeline UI/layout/accessibility code and tests.
- Existing app timeline is SMIL-specific in its track rows, key hit testing, easing controls, and source selection. Native project sessions are separate and may coexist with SVG SMIL. Do not reinterpret native tracks as SMIL or send native clicks through SMIL editing callbacks.
- Native `project_timeline_session_render_frame` validates against current scene/bundle on every call. The viewer must materialize and decode the current editor source when it is dirty so preview follows edited artwork. For native bundle validation, construct a temporary current bundle view that substitutes the materialized scene and current session timeline; assets/manifest remain borrowed from the loaded bundle.
- The renderer currently places drawings in root SVG coordinates. Keep this limitation visible in docs/status and do not invent target transforms.
- No broad UI/E2E test suite, no network, no commit.

# Existing contracts
- macOS Apple Silicon only, C17, no dependencies, custom UI and accessibility semantics.
- Native-v1 session owns edited timeline bytes; bundle assets borrow loaded project storage; scene/render tiles own their respective allocations.
- Integer ticks, `ticks_per_second`, `duration_ticks`; playback seconds must map deterministically to a bounded tick in `[0, duration_ticks)`.
- Rendering and timeline row enumeration are bounded. Timeline source can contain up to 256 tracks and 1,000,000 items; UI must virtualize rows and bound visible markers/work.
- SMIL timeline edit, easing, and selection behavior must remain unchanged.

# Required implementation
1. Add a native mode to `viewer_timeline` that borrows the active session and project bundle only for the loaded document lifetime, stores a decoded timeline view refreshed after native MCP mutation, derives track count/labels/duration from native timeline metadata, and resets safely on document replacement/close.
2. Add native frame preparation using the current editor source snapshot and project assets. Render integer ticks from the playback clock, cache only the current preview, and display deterministic status on decode/render failure without falling back to a misleading frame silently.
3. Reuse dock playback, scrub, speed, play mode, zoom/pan/fit controls for native timelines. Convert seconds to ticks with explicit finite/bounds handling and ensure the last time sample never produces `tick == duration_ticks`.
4. Draw native track labels and bounded cel start markers in the existing timeline panel. Native rows are read-only in this slice; row selection and scrubbing must never invoke SMIL target-selection, key dragging, or easing edits. Hide/disable easing controls for native timelines.
5. Add native row and cel marker accessibility semantics with stable bounded labels/values; keep play/scrub controls discoverable. Expose timing/frame rate metadata in the time readout or equivalent accessible value.
6. Refresh the native decoded view and invalidate the preview after successful MCP native timeline mutations; session edits must be visible on the next presented frame.
7. Add focused timeline model/paint/accessibility and project render bridge tests for native duration, integer tick mapping, row labels/markers, read-only pointer behavior, mutation refresh, and no regression to existing SMIL timeline tests. Do not run full UI/E2E automation.
8. Document visible native playback, tick conversion, root-coordinate placement limitation, and that native row editing, target-relative placement, onion skin, and export remain future work.

# Files allowed to touch
- `src/app/viewer_timeline.h/.c`, `src/app/viewer_timeline_ui.c`, `src/app/viewer_timeline_layout.c`, `src/app/viewer_timeline_accessibility.c`, `src/app/viewer_timeline_easing.c`
- `src/app/viewer.c`, `src/app/viewer_internal.h`, `src/app/viewer_render.c`, `src/app/viewer_editor_result.c`, `src/app/mcp_broker_hook.c`
- focused `tests/app/viewer_timeline_test.c` and any new focused native timeline view/model test under `tests/app/`
- `CMakeLists.txt` app/timeline lane only, `tests/test_map.json`
- `src/project/README.md`, `src/mcp/README.md` only if required
- this work order, `artifacts/WO-ANIM-PROJECT-11/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
Native schema/container/codec, session/render semantics, document edit internals, SVG/XML/raster implementation, unrelated editor panels, release files, scripts, presets, and paths outside allowlist. No broad UI/E2E suite.

# Acceptance criteria
- AC-1: strict Debug `svgworkbench` and focused timeline test targets build.
- AC-2: focused native app timeline tests and existing SMIL `viewer-timeline` tests pass.
- AC-3: focused changed timeline/project tests pass under ASan/UBSan where supported by the configured targets.
- AC-4: native project opens with a visible timeline; play/scrub select the expected integer cel at timing boundaries; MCP cel edits refresh row markers and rendered preview; errors remain visible and do not present stale cached frames.
- AC-5: native row input is read-only and does not mutate/select SMIL data; existing SMIL edit/easing behavior remains intact.
- AC-6: accessibility exposes native tracks, cel starts, play, and timeline position.
- AC-7: independent read-only review has no blocking/high findings.
- AC-8: frozen order, ledger, changed-path allowlist, source line limits, and `git diff --check` validate.

# Verification commands
- Build `svgworkbench`, `viewer_timeline_test`, and focused native project/MCP targets in Debug.
- Run focused `viewer-timeline`, project timeline session/render, and native MCP tests.
- Run those focused tests under ASan/UBSan.
- Run independent read-only review limited to allowed implementation/test paths.
- Record exact commands and outcomes under `artifacts/WO-ANIM-PROJECT-11/`.
- Do not run broad UI/E2E tests.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Do not cache views across edits or bundle replacement. Do not map an out-of-range playback sample to the next project duration boundary. Preserve project source and existing SMIL behavior. If stale/edited scene synchronization cannot be made bounded and correct, show preview unavailable and report the precise seam rather than rendering stale art.

# Output
Leave changes uncommitted. Report visible native timeline playback and focused verification; state the remaining native row editing, target-relative placement, onion skin, export, and final broad UI/E2E acceptance work.
