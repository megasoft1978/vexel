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
pan, checkerboard/background choices, a pixel grid above 8x zoom, and a
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
rotate previews leave source bytes unchanged; release commits one bounded
`transform` edit, and undo restores exact prior bytes. The selection overlay
draws a clipped bounding box, eight resize handles, a rotate handle, path
anchors and handles, and snapping guides. Rect, ellipse, line, polygon,
polyline, pen, and text tools commit one undoable edit when finished. V, H, Z,
R, O, L, Y, U, P, and T select those tools; Space temporarily pans. Arrow keys
nudge by one document unit and Shift+Arrow by ten units. Escape cancels an
active gesture or draft.

Edited source snapshots are decoded and rasterized on the existing worker queue
with generation cancellation. A failed edited preview leaves the last good
canvas visible and reports the decoder diagnostic in the editor status bar.

Dirty native projects with a saved on-disk baseline receive a best-effort
background recovery capsule every 30 seconds. The worker materializes an
immutable scene snapshot, reads a pinned timeline source, and serializes copied
asset descriptors with retained payload bytes. Captures preserve dirty and
undo state; source-file changes during capture prevent publication. Opening an
existing native project checks its validated capsule and asks whether to
restore, discard, or keep it. A changed source is reported as a conflict;
restore loads the complete recovered project as dirty work without writing the
source. Explicit discard and successful project save remove the keyed capsule;
cleanup failures are reported. Still outstanding: missing-source discovery
(the store key requires an existing canonical source), capsule retention and
expiry, project-version migration policy, measured peak memory for very large
recovery snapshots, and final comprehensive UI/E2E acceptance.

The editor supports Shift-click, marquee, and Cmd+A selection, up to 128
rendered elements. Group move, resize, rotate, nudge, delete, duplicate, and
z-order edits work for selected siblings under one parent and one bounded
64 KiB source region. Cmd+[ / Cmd+] reorder one element; Cmd+Shift+[ / ] send it
to the back or front. Move and resize transforms are composed into a matrix
while retaining any authored transform suffix.

Polygon and polyline tools collect clicked points. The pen tool creates straight
and cubic segments and closes on the first anchor. Return or double-click
finishes; Escape cancels. The path tool exposes anchors and cubic controls for
supported `M`, `L`, `H`, `V`, `C`, `S`, `Q`, `T`, and `A` commands. Node drags
replace only the affected number tokens. Double-click inserts a node on a line
segment; Delete removes a noninitial node. Snapping to a one-unit grid and
rendered element edges/centers is enabled by default; Cmd+' toggles it.

The text tool places an inline draft and commits one escaped `<text>` element
when Return is pressed. New text starts at 16 units in `#172033`; the ordinary
attribute inspector exposes `font-size` and `fill` for editing. Cmd+C/X/V copy,
cut, and paste SVG fragments; Cmd+Shift+V pastes in place. Multi-selection is
copied as one `<g>` fragment. Pasted input must be a well-formed, single-root
fragment no larger than 64 KiB. Cmd+Shift+E exports PNG with a scale and
document/selection chooser; Cmd+Option+C copies PNG. Export is bounded to 4096
pixels per axis and 16 million pixels.

Save As and dirty open/close/quit choices use native dialogs. Save failures and
external-file conflicts show native error alerts. SVGZ remains unsupported;
rendering is limited to the subset accepted by the current decoder and
rasterizer. The editor native smoke covers pointer selection, move, resize,
rectangle and polygon creation/undo, and exact save when run in an interactive
macOS session.
