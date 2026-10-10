# UI

The UI module owns a fixed-capacity retained widget tree, deterministic layout,
design tokens, bounded labels and values, keyboard focus and command routing,
semantic snapshots, native text adapters, and virtualized inspector behavior.
`ui_widgets` provides constructors for buttons, checkboxes, text fields, sliders,
and rows. `ui_core` is platform-independent C17 and paints node backgrounds
through the public scalar SVG shape decoder and rasterizer. Platform input and
text services stay behind `src/platform/` interfaces.

`ui_tree_paint_into` draws retained fills, focus outlines, labels, and values
directly into caller-owned RGBA storage using a bounded scalar glyph painter.
The caller provides the viewport buffer; the UI renderer allocates no
viewport-sized scratch. Text is limited to the painter's built-in ASCII glyphs,
with input values still stored as bounded UTF-8 by the widget tree.
