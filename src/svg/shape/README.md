# Bounded SVG shape subset

`svg_shape_decode` borrows exact source bytes and returns the source slice plus
owned flattened edges, per-edge RGBA colors and edge-to-element IDs. The caller
keeps source bytes alive until the document is destroyed. The source is never
modified. Diagnostics are stable
categories plus a byte offset into that source; XML scanner failures retain the
scanner offset. Root `svg`, `rect`, and `path` are supported. Root viewport
dimensions are finite unitless or px lengths (positive, at most 4096); tile
extents round up to integer pixels while preserving the viewport scale, with an
optional finite positive `viewBox` using the default centered meet mapping. Rectangles accept x/y/width/height; omitted
x/y are zero and negative dimensions fail. Fill supports none, #RGB, #RRGGBB,
black, white, red, green and blue. `fill-rule` is nonzero or evenodd,
independently for each rect/path element. Elements paint in source order;
subpaths of one path share its fill rule, and each subpath closes at a later
moveto or the end of path. Unsupported elements, colors and syntax fail
without changing source bytes.

Path commands are M/m, L/l, H/h, V/v, C/c, Q/q, A/a and Z/z with repeated
groups. Coordinates are limited to absolute value 1,000,000; total path commands
across the document are limited to 65,536; flattened edges to 262,144. Cubics
and quadratics use deterministic De Casteljau subdivision until every control
point is at most 0.25 user units from the finite endpoint segment; recursion depth is capped at 16.
Elliptical arcs use endpoint-to-center conversion and a sagitta tolerance of
0.25 user units, with total edges bounded by the document cap. These are
user-space flatness bounds, not device-pixel guarantees. Arc flags must be
exactly zero or one. Fill implicitly closes open
subpaths. Output uses scalar pixel-center point-in-path tests and transparent
black outside fills. The combined per-pixel edge-plus-shape tests are capped at
200,000,000 per render.

No CSS/style, stroke, transform, gradients, filters, external resource,
unsupported unit, smooth command, or unknown shape semantics are interpreted.
This module has no platform or third-party dependency.

The subset grammar and arc endpoint semantics reference the W3C SVG 2
[Paths](https://www.w3.org/TR/SVG2/paths.html) and
[Shapes](https://www.w3.org/TR/SVG2/shapes.html) specifications (consulted
2026-10-09). The C implementation is original project code; no external code
or SVG editor source was copied.
