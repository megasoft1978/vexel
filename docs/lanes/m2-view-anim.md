# m2-view-anim lane

This lane adds MCP controls for view/tool state, app commands, editor state,
window capture, animation timing, animation authoring, preview, and export.
Every broker tool response carries `mode: "live"` or `mode: "headless"`.

## Live host contract

`src/mcp/mcp_ui_host.h` defines the host vtable for view, tool, UI state,
command, screenshot, and timeline operations. The default headless host stores
local view/tool/timeline state; it has no app commands or window capture.
Live hosts marshal callbacks to the editor owner thread and serialize UI
changes with GUI edits. They can queue while a gesture is active within their
bounded capacity, or return a busy result for MCP to report as `ui_busy`.
Window capture returns malloc-owned PNG bytes, which the broker bounds and
validates.

All document edit paths call `mcp_broker_commit_mutation` after applying an
edit. Its headless default is a no-op; the live app host supplies the refresh
hook so the app can update canvas, selection, layers, inspector, dirty/title,
and timeline state through the shared document and undo stack. The m0-live
lane supplies that app-side host; this lane tests the contract with an
in-process fake host and does not add a second live bridge.

`screenshot_window` uses the live window callback when the host is live. The
headless broker instead renders the current document into a PNG and labels the
result `captureMode: "headless_composed_frame"`. A live host missing capture
support returns `screenshot_unavailable`; it does not silently change modes.

## Tools and bounds

- `view_get` / `view_set`: zoom, pan, fit, and actual-size state.
- `tool_get` / `tool_set`: the editor tool ids exposed by the schema.
- `ui_state`: selection, tool, view, panel visibility, dirty flag, history
  depth/cursor, and focused document.
- `command_list` / `command_run`: available app command ids and execution.
- `screenshot_window`: 1..512 pixels per axis, at most 2 MiB of PNG data.
- `render_png`: source region, center-cropping zoom/legacy raster scale,
  background, animation time, and optional output dimensions; output is at most
  512 per axis and 262,144 pixels total.
- `animation_keyframe_add`, `animation_keyframe_move`,
  `animation_keyframe_delete`, `animation_keyframe_set_value`, and
  `animation_easing_set`: validated, source-preserving edits through
  `document_edit_replace_source_span`; no document-edit engine is duplicated.
- `timeline_get`, `timeline_play`, `timeline_pause`, `timeline_seek`, and
  `timeline_loop`: timeline state and playback controls.
- `animation_preview`: 1..16 sample times as PNG image items or a contact
  sheet, at most 16 million sampled pixels, with sheet axes capped at 512.
- `export_animation`: GIF, PNG sequence, or byte-preserved SVG. Exports remain
  within granted directories and create new paths without replacing files.

Keyframe tools require an explicit SMIL `values` track. Times are unique and
strictly increasing between normalized endpoints 0 and 1. Values are limited
to 2,048 bytes. Paced interpolation computes times from values, so add/move/
delete returns `paced_keyframe_times`; value edits remain available. Each
successful document mutation participates in `document_edit` undo/redo. An
identical key value reports `changed: false` and does not add an undo entry.

## Verification

- `./dev build`: passed.
- `ctest --test-dir build/mac-debug -R '^mcp-view-anim$' --output-on-failure`:
  passed (1/1). The in-process fake host checks live/headless mode, UI state,
  command/timeline operations, screenshot capture, shared selection/history,
  mutation notifications, render changes, animation keyframe/easing edits,
  preview, and undo.
- `python3 tests/mcp/test_e2e.py`: skipped (1 test) because this sandbox denies
  Unix-domain socket binding with `EPERM`.
- `./dev t --all`: CTest 60/62 and Python 69/69. The pre-existing
  `index-replay` test cannot obtain an offscreen Metal queue, and
  `native-viewer-smoke` fails in this environment's LaunchServices setup.
- `./dev mcp-fuzz`: passed 200,000 cases, 50,013 parsed requests and argument
  validations, seed 12648430.
- `git diff --check`: passed.

The actual app-side `mcp_ui_host` and mutation-refresh callback are provided by
the parallel m0-live lane; this verification uses the broker's fake live host.

## Codex continuation verification (2026-10-11)

A resumed `./dev t -k mcp` run exposed that `animation_easing_set` changed the
source but `animation_list` did not return each track's `calcMode`. The serializer
now includes `linear`, `spline`, `discrete`, or `paced` from the parsed track.
After the fix, `./dev t -k mcp` passes all five selected CTests and the Python
stdio e2e (no socket skip). `git diff --check` passes.
