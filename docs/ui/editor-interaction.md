# Editor canvas interaction

The editor canvas has its own viewport. A new document opens fitted to the
canvas, and window resizes keep fitting until the user changes zoom. After a
user zoom, a resize preserves the document point at the canvas center.

The HUD reports zoom percentage, pointer position in rendered document pixels,
decoded document dimensions, and visible sharp-tile progress. Moving the
pointer over a toolbar control updates hover state; pointer down and up update
pressed state and request a redraw.

The zoom value is a button with 25%, 50%, 100%, 200%, 400%, and 800% presets.
Presets zoom around the viewport center. View → Zoom to Selection centers and
fits the current selection with padding; when there is no selection it fits the
document. Cmd+0 and Cmd+1 keep their Fit and Actual Size meanings.

Top and left rulers show SVG user units with adaptive 1/2/5-decade tick spacing.
Drag from a ruler to create a guide, drag a guide line to move it, then press
Delete to remove the selected guide. Right-click a ruler to clear guides on that
axis. View → Rulers and View → Guides control their visibility, while View →
Lock Guides prevents creating, moving, or deleting an individual guide. Guides
are editor state and survive visibility and panel changes without changing SVG
source bytes. Visible guides are snap targets; Shift temporarily bypasses
snapping. Holding Space pans and hides selection and snap overlays until release.

## Navigation and tools

| Input | Action |
|---|---|
| `Option-Command-L` | Show or hide the operator log |
| `Control-PageDown` | Next task workspace (Draw, Nodes, Animate, Inspect) |
| `Control-PageUp` | Previous task workspace |
| `Control-Tab` / `Control-Shift-Tab` | Aliases for next / previous workspace |
| `V` | Select tool |
| `H` or hold Space | Pan tool |
| `Z` | Zoom tool |
| `R` | Rectangle tool |
| `O` | Ellipse tool |
| `L` | Line tool |
| `Y` | Polygon tool |
| `U` | Polyline tool |
| `P` | Pen tool |
| `N` | Node edit tool |
| `T` | Text tool |
| `G` | Toggle the pixel grid (shown above 800% zoom) |
| Drag with Select | Move a hit-tested rendered shape |
| Drag a selection handle | Resize or rotate |
| Shift-click | Add or remove an element from the selection |
| Drag empty canvas | Select elements in a marquee |
| Cmd+A | Select all rendered elements |
| Arrow key | Nudge the selected shape by one document unit |
| Shift+Arrow | Nudge by ten document units |
| Cmd+[ / Cmd+] | Move the selected element backward or forward |
| Cmd+Shift+[ / Cmd+Shift+] | Send the selected element to the back or front |
| Cmd+' | Toggle grid and element snapping |
| Cmd+C / Cmd+X / Cmd+V | Copy / cut / paste SVG elements |
| Cmd+Shift+V | Paste in place |
| Cmd+Shift+E | Export PNG |
| Cmd+Option+C | Copy as PNG |
| Cmd+0 | Fit canvas |
| Cmd+1 | Actual size |
| Cmd++ / Cmd+- | Zoom in or out |
| Scroll over canvas | Pan |
| Cmd/Ctrl+Scroll or pinch | Zoom |
| Escape | Cancel an active canvas gesture |
| Return | Finish polygon, polyline, pen, or text input |

Rectangle, ellipse, and line creation begins on pointer down and commits on
pointer up. Polygon and polyline collect clicked points; the pen tool makes
straight and cubic segments and closes when the first anchor is clicked. Return
or double-click finishes a path. During a gesture, the overlay previews the
shape and source bytes stay unchanged. A finished drawing is one undo command.
Created elements go into the selected `<g>` when it is a group; otherwise they
go into the SVG root. Text creates an inline draft and commits an escaped
`<text>` element on Return, with editable `font-size` and `fill` attributes in
the ordinary inspector.

Canvas selection uses rendered hit testing and source offsets, so the outline
selection and inspector target the same element. Shift-click, marquee, and
Cmd+A select up to 128 elements. Group transforms, nudging, delete, duplicate,
and z-order operations apply to sibling elements with one parent, within a
64 KiB changed source region. Move, resize, and rotate preserve authored
geometry and unknown attributes by serializing one composed matrix while
retaining the original transform suffix. Each release creates one undo command
and undo restores exact prior bytes. The overlay is clipped to the canvas and
draws selection bounds, resize and rotate handles, path anchors/handles, and
snap guides.

Path node editing supports retained numeric commands `M`, `L`, `H`, `V`, `C`,
`S`, `Q`, `T`, and `A`. It replaces only edited number tokens. Double-click
adds a node to a line segment; Delete removes a noninitial node. Snapping is on
by default to a one-unit grid and other rendered element bounds and centers.
The clipboard uses SVG fragment text and `public.svg-image`; pasted content is
limited to one validated root element and 64 KiB. PNG export uses the existing
renderer and ImageIO, allows document or selection output at 1×/2×/4×, and is
bounded to 4096 pixels per axis and 16 million pixels.

## Workspaces and the status bar

The tab strip under the toolbar switches between four task workspaces. A workspace
selects pane visibility and the expanded inspector section only; the document and
the editing model are untouched, and a workspace switch never marks the document
dirty. `Control-PageDown` and `Control-PageUp` cycle with wrap, and the choice is
remembered when `rememberWorkspace` is enabled.

The status bar carries the active-tool hint on the left, the running operation and
its Cancel button in the centre, and the save state, selection count and zoom on
the right.

## Current limits

Canvas transforms use rendered axis-aligned bounds and do not rewrite geometry
attributes such as `x`, `y`, or `d`. Group operations require same-parent
siblings; unsupported selections are rejected. Path node insertion handles
line segments, and delete keeps the initial move-to node. PNG scale is constrained
by the output dimensions. Save As and dirty-file choices use native AppKit
dialogs. Run `./dev editor-smoke` from an interactive macOS desktop to exercise
pointer selection, move, resize, rectangle and polygon creation/undo, and save.
