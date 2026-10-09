# SVG shape decoding

The shape module turns a borrowed SVG source slice into owned flattened edges,
per-edge solid fallback colors, per-shape fill/stroke state, gradients, and clip
records. `traverse.c` owns token traversal, inherited style/transform contexts,
and live `<use>` instances. `transform.c` parses the bounded affine syntax.
`shape.c` owns supported geometry parsing and flattening. `document.c` owns
public document cleanup.

`<defs>` is non-rendering and `<symbol>` is non-rendering unless reached by a
live `<use>`. Local `href="#id"` and root-bound `xlink:href="#id"` references
traverse the original token array at the use position. The use path allocates
no source subtree copy, rewritten document, or source map. A fixed-capacity ID table
supports up to 65,536 unique IDs. Use reference depth is capped at 16, instances
at 65,536, each traversed reference span at 64 KiB, aggregate reference-span
visits at 1 MiB, active element visits at 4,194,304, and transform operations
at 4,096. Diagnostics keep offsets into the original borrowed source.

Linear and centered radial gradients support 2–256 literal-color stops,
monotone finite offsets, stop opacity, object-bounding-box or user-space units,
and pad spread. Stop color and opacity use the CSS cascade for attributes,
inline styles, and supported class selectors. Gradient transforms and inherited
gradient references are unsupported. Both paint units sample in the
referencing element's user space and use its untransformed local bounds where
needed; singular transforms fail with `unsupported`. Clip paths support local
`url(#id)`, `userSpaceOnUse` (the default) or `objectBoundingBox`, and union of
up to 64 basic shape/path children. Group/use clips use the whole subtree AABB,
resolve once per declaration, and intersect with descendant clips. Nested
clip declarations inside clip geometry and external references fail closed.
Raster work is bounded by the document edge and clip-edge budgets.

CSS retains source spans and supports the nine fill/stroke paint properties,
`clip-path`, `stop-color`, and `stop-opacity` through presentation attributes,
stylesheets, and inline styles. Fill/stroke paint is inherited; `clip-path` is
applied at its declaring element. The stylesheet is
capped at 1 MiB, 4,096 rules, and 16,384 declarations. Unsupported SVG or CSS
semantics return stable diagnostics rather than fetching external content.

The renderer uses a deterministic scalar 4×4 coverage rasterizer. Its behavior
is compared with librsvg using small fixtures; this is selected-feature
differential evidence, not full SVG parity. SVG semantics follow W3C SVG 2
Shapes, Paths, CSS, Paint Servers, Document Structure, and Clipping specs
(consulted 2026-10-09). No SVG editor source is used.
