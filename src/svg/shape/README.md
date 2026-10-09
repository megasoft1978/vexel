# SVG shape decoding

The shape module turns a borrowed SVG source slice into owned flattened edges,
per-edge solid fallback colors, per-shape fill/stroke state, gradients, and clip
records. It supports the bounded root/shape/CSS/transform subset implemented in
`shape.c`. `document.c` owns the public decode wrapper and frees all document
allocations, including recursively owned clip geometry.

Local `href="#id"` and namespace-bound `xlink:href="#id"` references support
basic shapes, groups, and symbols through bounded expansion. IDs are indexed
with a 65,536-entry cap; expansion depth is capped at 16, and instances at
65,536. Source bytes are borrowed and remain unchanged. Expansion diagnostics
for copied child content may currently refer to generated-source offsets.

Linear and centered radial gradients support 2–256 literal-color stops,
monotone finite offsets, stop opacity, object-bounding-box or user-space units,
and pad spread. Gradient transforms and inherited gradient references are
unsupported. Clip paths support local `url(#id)`, `userSpaceOnUse` (the default)
or `objectBoundingBox`, and union of up to 64 basic shape/path children. Nested
clips and external references fail closed. Raster work is bounded by the
document edge and clip-edge budgets.

CSS retains source spans and supports the nine fill/stroke paint properties
plus `clip-path` through presentation attributes, stylesheets, inline styles,
and inherited group paint where the property applies. The stylesheet is capped
at 1 MiB, 4,096 rules, and 16,384 declarations. Unsupported SVG or CSS
semantics return stable diagnostics rather than fetching external content.

The renderer uses a deterministic scalar 4×4 coverage rasterizer. Its behavior
is compared with librsvg using small fixtures; this is selected-feature
differential evidence, not full SVG parity. SVG semantics follow W3C SVG 2
Shapes, Paths, CSS, Paint Servers, Document Structure, and Clipping specs
(consulted 2026-10-09). No SVG editor source is used.
