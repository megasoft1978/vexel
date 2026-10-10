# SVG shape decoding

The shape module turns borrowed SVG bytes and XML tokens into owned flattened
edges, per-shape fill and stroke state, gradients, clips, and opacity groups.
`traverse.c` owns bounded token traversal, inherited style/transform contexts,
and live `<use>` instances. `transform.c` parses affine transforms;
`shape_geometry.c` decodes the supported geometry; `viewport.c` resolves root,
nested `<svg>`, and symbol viewports; `document.c` owns public cleanup.

Supported geometry includes all SVG path commands (`M/L/H/V/C/S/Q/T/A/Z`,
absolute and relative), `<line>`, `<polyline>`, `<polygon>`, `<circle>`,
`<ellipse>`, and `<rect>`. Paths retain subpath and closed-stroke metadata so
strokes support caps, joins, miter limits, dash arrays, and dash offsets.
Rounded rectangles implement the `rx`/`ry` copy rule, radius clamping, and
elliptical corners. Curves are flattened with a transformed 0.25-unit tolerance
and a recursion cap of 16. Paths are limited to 65,536 commands and documents
to 262,144 edges.

Transforms include matrix, translate, scale, rotate (including a center point),
`skewX`, and `skewY`. The paint CSS cascade covers fill/stroke rules, color and
opacity properties, visibility, display, clips, and gradient stops. The shared
color parser accepts 148 named colors, `rgb()`/`rgba()`/`hsl()`/`hsla()`, 3/4/6/8
digit hex, `transparent`, and context-resolved `currentColor`. Selectors allow
up to four descendant terms and sixteen class terms per term. Child/sibling,
pseudo, attribute, namespace, at-rule, and resource selectors remain unsupported.

Root and nested `<svg>` viewports support viewBox transforms and
`preserveAspectRatio` alignment, `meet`, `slice`, and `none`. Nested viewports
clip their descendants. Shape lengths accept percentages and `px`, `pt`, `mm`,
`cm`, `in`, and `em`; `em` currently resolves to 16 user units and does not
compute inherited `font-size`. Fill/stroke opacity and isolated element/group
opacity are supported. Opacity group rendering uses bounded tile scratch and
returns `limit` if nested layers exceed 64 MiB.

`<defs>`, `<title>`, `<desc>`, `<metadata>`, and Inkscape's
`sodipodi:namedview` are non-rendering; metadata subtrees are structurally
tokenized and then skipped. Unknown prefixed export attributes are ignored
without changing their borrowed source bytes. `<symbol>` is
non-rendering unless reached by a live `<use>`. Local `href="#id"` and
root-bound `xlink:href="#id"` references
traverse the original token array at the use position. The ID table supports up
to 65,536 unique IDs; reference depth is capped at 16, instances at 65,536,
each referenced span at 64 KiB, and aggregate span visits at 1 MiB. Diagnostics
keep offsets into the original borrowed source.

Clip paths support local references, `userSpaceOnUse` or `objectBoundingBox`,
and unions of up to 64 basic shape/path children. Group/use clips use their
subtree AABB. Nested clip declarations inside clip geometry and external
references fail closed. Raster work, clip geometry, stylesheet data, and
references retain their fixed budgets; unsupported XML/CSS semantics return
stable diagnostics rather than dropping source or fetching resources.

The deterministic scalar 4×4 rasterizer is compared with librsvg using small
fixtures. This is selected-feature differential evidence, not full SVG parity.
SVG semantics follow W3C SVG 2 Shapes, Paths, CSS, Paint Servers, Document
Structure, and Clipping specs (consulted 2026-10-09). No SVG editor source is
used.
