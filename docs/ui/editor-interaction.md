# Editor canvas interaction

The editor canvas has its own viewport. A new document opens fitted to the
canvas, and window resizes keep fitting until the user changes zoom. After a
user zoom, a resize preserves the document point at the canvas center.

The HUD reports zoom percentage, pointer position in rendered document pixels,
decoded document dimensions, and visible sharp-tile progress. Moving the
pointer over a toolbar control updates hover state; pointer down and up update
pressed state and request a redraw.

## Navigation and tools

| Input | Action |
|---|---|
| `V` | Select tool |
| `H` or hold Space | Pan tool |
| `Z` | Zoom tool |
| `R` | Rectangle tool |
| `O` | Ellipse tool |
| `L` | Line tool |
| Drag with Select | Move a hit-tested rendered shape |
| Drag a selection handle | Resize or rotate |
| Arrow key | Nudge the selected shape by one document unit |
| Shift+Arrow | Nudge by ten document units |
| Cmd+0 | Fit canvas |
| Cmd+1 | Actual size |
| Cmd++ / Cmd+- | Zoom in or out |
| Scroll over canvas | Pan |
| Cmd/Ctrl+Scroll or pinch | Zoom |
| Escape | Cancel an active canvas gesture |

Rectangle, ellipse, and line creation begins on pointer down and commits on
pointer up. During a drag, the overlay previews the shape and source bytes stay
unchanged. A successful release inserts one bounded shape template as one undo
command. Created shapes go into the selected `<g>` when the structure selection
is a group; otherwise they go into the SVG root.

Canvas selection uses rendered hit testing and source offsets, so the outline
selection and inspector target the same element. Move, resize, and rotate keep
authored geometry and unknown attributes intact by adding or prepending one SVG
`transform` attribute. Each release creates one undo command and undo restores
exact prior bytes. The selection overlay is clipped to the canvas and draws a
bounding box, eight resize handles, and a rotate handle.

## Current limits

The current interaction layer supports one selected element at a time and only
rect, ellipse, and line creation. Polygon/polyline and path pen creation, path
node editing, snapping, multi-selection and group transforms, SVG clipboard
exchange, and PNG export are not implemented. Canvas transforms use rendered
axis-aligned bounds and do not rewrite geometry attributes such as `x`, `y`, or
`d`. Existing transforms can be extended by another transform edit.

Save As, dirty-file confirmation, clipboard, and IME marked text still depend on
platform services. Run `./dev editor-smoke` from an interactive macOS desktop
to exercise click-select, move, resize, rectangle creation, undo, and save.
