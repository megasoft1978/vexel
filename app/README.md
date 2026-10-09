# Application entry points

`main.c` dispatches viewer, render-frame, native test, and smoke commands.
`viewer.c` owns the interactive AppKit viewer; `render_frame_cli.c` is its
headless PPM path. `index_replay.c` is a scriptable headless path for a borrowed
SVG session: it advances the owner index
in byte-budgeted steps, updates the UI loading model, schedules shape decoding
and raster work, then verifies the tile through offscreen Metal readback. The
offscreen replay keeps progress and determinism checks independent of AppKit
window activation; visible window presentation remains covered by the native
Metal smoke command.
