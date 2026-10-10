# UI lane

See [the w4-ui style implementation](../lanes/ui-style.md) for the production
editor palette, layout, CoreText path, accessibility bridge, computed contrast
report, new 1×/2× style goldens, and remaining integration limits.

See [editor canvas interaction](editor-interaction.md) for opening fit, live
canvas status, pan/zoom input, selection, transform handles, shape creation,
keyboard shortcuts, and current feature limits. The editor selection overlay
has separate dark/light 1×/2× fixtures under `tests/editor/selection-golden/`;
the existing style and chrome fixtures remain unchanged.

The retained tree keeps stable node and parent IDs, full UTF-8 semantic text,
and focus/selection/invalid state. Legacy demo fixtures remain on the scalar
path and are kept separate from the modern macOS editor rendering path.

M6-02 adds widget constructors for buttons, checkboxes, text fields, sliders, and
rows. Tab/Shift-Tab move focus through enabled interactive controls; Enter/Space
activate commands, and Space toggles checkboxes. Stable semantic snapshots copy
node IDs, parent IDs, roles, labels, values, and disabled/checked/focused states.
Semantic group and list labels stay in the accessibility tree without being
painted over their child titles. Hover and pressed states flow through the
viewer event bridge and request a frame redraw.

The editor composes its toolbar, virtualized structure outline, inspector
attribute rows, text field, and status bar from the retained tree. The existing
`tests/editor/chrome-golden/` screenshots remain unchanged; new style fixtures
live under `tests/ui/style-golden/`. Canvas interaction tests are headless and
inject events directly; the separate maintainer smoke script needs a macOS
desktop session.
