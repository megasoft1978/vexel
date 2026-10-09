# Application entry points

`main.c` dispatches native test and smoke commands. `index_replay.c` is a
scriptable headless path for a borrowed SVG session: it advances the owner index
in byte-budgeted steps, updates the UI loading model, schedules shape decoding
and raster work, then verifies the tile through offscreen Metal readback. The
offscreen replay keeps progress and determinism checks independent of AppKit
window activation; visible window presentation remains covered by the native
Metal smoke command.
