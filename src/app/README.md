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
its output raster is limited to 4096 pixels per axis. `--time` selects a bounded
SMIL animation sample in seconds and defaults to the static document render.
Malformed, unsupported,
missing, unreadable, oversized, and out-of-range inputs produce a clear error
and a distinct error image when a frame can be written.

`--render-animation <file.svg> --fps N --duration D --out <existing-dir>` writes
`frame-0000.ppm` and subsequent frames using one decoded document and explicit
sample times. FPS is 1..60, duration is at most one hour, and the sequence is
capped at 300 frames. PPM frames flatten transparency over white. Both CLI
paths use the same deterministic animation renderer; this lane does not add
timeline controls or frame caching to the interactive viewer. A 32-million
total-frame-pixel budget is checked before rendering begins.

The retained editor session in `viewer_editor.*` borrows the loaded source for
`document_edit`, builds a bounded structure/inspector chrome, and routes every
edit through the source-preserving edit API. The canvas uses a separate
navigation viewport that fits on open and continues to refit on resize until
the user changes zoom. After a user zoom, resizing preserves the document point
at the canvas center. The HUD reports zoom, pointer coordinates in rendered
document pixels, decoded dimensions, and visible sharp-tile progress.

`viewer_editor_canvas.c` maps hit-tested rendered shapes to source offsets and
keeps selection in sync with the structure and inspector. Move, resize, and
rotate previews do not modify source bytes; release commits one bounded
`transform` attribute edit, and undo restores exact prior bytes. The overlay
draws a clipped selection box, eight resize handles, a rotate handle, and a
shape-drawing preview. Rect, ellipse, and line tools insert bounded templates
into the selected `<g>` or the SVG root. The select, pan, zoom, rect, ellipse,
and line shortcuts are V, H, Z, R, O, and L; Space temporarily pans. Arrow keys
nudge a canvas selection by one document unit, and Shift+Arrow uses ten units.
Escape cancels an active gesture.

Edited source snapshots are decoded and rasterized on the existing worker queue
with generation cancellation. A failed edited preview leaves the last good
canvas visible and reports the decoder diagnostic in the editor status bar.

Save, undo/redo, element delete/duplicate/group/ungroup/reorder, editable
attribute rows, and text content edits remain wired into the viewer. Multi-
selection and group transforms, polygon/polyline and path-pen creation, path
node editing, snapping, SVG clipboard exchange, and PNG export remain future
work. Transform handles currently use rendered axis-aligned bounds; transforms
are serialized by adding or prepending an SVG `transform` attribute so unknown
geometry and authored coordinates remain intact.

Save As, dirty-file confirmation dialogs, clipboard access, and IME marked text
still need platform services. Dirty open/close/reload requests are blocked and
external file changes are checked before overwrite. System appearance changes
invalidate the editor and trigger the next viewer-loop presentation. SVGZ
remains unsupported; rendering is limited to the subset accepted by the current
decoder and rasterizer.
