# w5-editor — UI polish and canvas interaction

## Built

- Fit document navigation to the editor canvas on open and refit on resize until
  the user changes zoom. Afterward, preserve the document point at the canvas
  center while resizing.
- Route pointer hover, press, click, drag, Space/hand pan, zoom-tool click,
  scroll, pinch, Cmd+0/Cmd+1/Cmd+plus/Cmd+minus, and canvas-coordinate HUD
  updates through the viewer bridge.
- Keep semantic group/list labels out of visible text while retaining their
  accessibility labels. Appearance and pointer-state changes request immediate
  viewer redraws.
- Select rendered shapes by source offset; draw a clipped selection box, eight
  resize handles, rotate handle, and shape-drag preview.
- Commit move, resize, and rotate as one `transform` attribute edit each.
  Rectangle, ellipse, and line tools insert bounded templates into the selected
  group or root. Arrow keys nudge by 1 unit, Shift+Arrow by 10 units.
- Add headless interaction, viewport geometry, status/redraw, clipping, and
  dark/light 1×/2× overlay coverage, plus a maintainer desktop smoke script.

## Limits and unverified work

This lane does not implement multi-selection, marquee/group transforms,
polygon/polyline or path-pen creation, path node editing, snapping, SVG
clipboard exchange, or PNG export. Transform edits preserve authored geometry
by adding/prepending `transform`; they do not update simple geometry attributes.
The native smoke requires a maintainer desktop with Accessibility permission
and was not run in this headless test environment. Native visual rendering and
Metal presentation were not exercised here.

## Verification

- `./dev build` — passed.
- Focused Debug CTest for canvas interactions, selection goldens, viewport
  geometry, editor state/chrome, style, layout fuzz, and token contrast — 9/9
  passed.
- Full Debug CTest — 37 passed, 2 failed, 1 skipped. `native-viewer-smoke` and
  `index-replay` could not obtain a Metal device/queue; `mcp-stdio-e2e` skipped
  because the sandbox denied its socket bind.
- `./dev asan` and `./dev ubsan` — all editor-specific tests passed with no
  sanitizer findings. Each full run had the same Metal-dependent failures and
  sandbox socket skip as the Debug run.
- `python3 -m unittest discover -s tests -t .` — 254 tests ran; 8 failed, 2
  errored, and 1 skipped. The failures/errors were in benchmark subprocess,
  output-capture, timeout and fixture checks, and native-driver/replay
  availability.
- `git diff --check`, `sh -n tests/editor/native_smoke.sh`, and `osacompile` on
  the smoke script's AppleScript — passed. The interactive desktop smoke itself
  was not run.
