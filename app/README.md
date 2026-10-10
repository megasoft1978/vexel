# Application entry points

`main.c` dispatches viewer, render-frame, native test, and smoke commands. With
no file argument it opens the interactive viewer's empty state, which prompts
for Open or drag and drop. `viewer.c` owns the interactive AppKit viewer;
`render_frame_cli.c` is its headless PPM path with bounded zoom and source-region
rerendering. `index_replay.c` is a scriptable headless path for a borrowed
SVG session: it advances the owner index
in byte-budgeted steps, updates the UI loading model, schedules shape decoding
and raster work, then verifies the tile through offscreen Metal readback. The
offscreen replay keeps progress and determinism checks independent of AppKit
window activation; visible window presentation remains covered by the native
Metal smoke command.

`editor_chrome_render.c` is the headless dark/light editor chrome golden
renderer. The native editor itself remains part of `svgworkbench`; its scripted
interactive save smoke is `tests/editor/native_smoke.sh`, exposed through
`./dev editor-smoke` for a maintainer-run desktop session.
