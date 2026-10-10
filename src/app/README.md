# App viewer modules

`viewer.c` owns the interactive viewer on the AppKit thread: the native window,
one-worker task queue, frame buffer, loaded immutable document, sharp-tile cache,
and generation-tagged jobs. `viewer_events.c` handles navigation and menu/file
actions; `viewer_render.c` composes and presents frames. Stale load or raster
jobs are canceled by advancing the task generation. The source remains alive
until a worker using it has completed cleanup, including when a new file opens
or the window closes mid-load.

`viewer_load.c` bounds SVG source input to 1 GiB, reads it in cancellable 1 MiB
chunks, and uses the current public shape decoder and rasterizer. The initial
resampled preview is retained while a worker renders sharp tiles for the
visible region at the current scale. Sharp tiles are at most 256 by 256 backing
pixels; the generation-scoped least-recently-used cache holds at most 128 tiles
and 32 MiB. A plan uses a conservative shape/edge work estimate and stops at
200 million work units. The rasterizer's own work and allocation limits remain
in force. A failed sharp render leaves the preview available.

The viewer window opens at 1280 by 800 points and uses backing pixels for
navigation and composition. Viewports are bounded to 8192 pixels per axis and
128 MiB per frame. The display offers fit, fit width, actual size, cursor zoom,
pan, checkerboard/background choices, a pixel grid at 8x or greater, and a
status line with zoom, pointer position in rendered document pixels, decoded
tile dimensions, and latest load/render times. Launching without a path shows
an Open SVG prompt. File changes are checked every 250 ms using file size and
modification time.

`frame_compose.c` is a pure caller-buffer RGBA composition path. It draws the
resampled preview, then overlays completed sharp tiles, and keeps the
checkerboard/background, pixel grid, status line, loading, empty, and friendly
error frames allocation-free. `render_frame_cli.c` is a synchronous headless
path that writes binary P6 PPM. Frame dimensions are bounded to 1..8192,
`--zoom` to 0.001..1000, and pan offsets to +/-10,000,000 pixels. `--region`
selects a source tile-pixel rectangle and rerenders it at the requested zoom;
its output raster is limited to 4096 pixels per axis. Malformed, unsupported,
missing, unreadable, oversized, and out-of-range inputs produce a clear error
and a distinct error image when a frame can be written.

The retained editor session in `viewer_editor.*` borrows the loaded source for
`document_edit`, builds a bounded structure/inspector chrome, and routes every
edit through the source-preserving edit API. Edited source snapshots are decoded
and rasterized on the existing worker queue with generation cancellation. A
failed edited preview leaves the last good canvas visible and reports the
decoder diagnostic in the editor status bar. The selection outline currently
has a source-offset hook; shape source offsets and bounding boxes are not exposed
by the renderer yet.

Save, undo/redo, element delete/duplicate/group/ungroup/reorder, editable
attribute rows, and text content edits are wired into the viewer. Save As,
dirty-file confirmation dialogs, system appearance changes, clipboard access,
and IME marked text require additions to the `platform/` interface. Until those
services land, dirty open/close/reload requests are blocked and an external file
change is checked again before saving to prevent overwriting the changed path.
The UI uses the dark theme by default. SVGZ remains unsupported; rendering is
limited to the subset accepted by the current decoder and rasterizer.
