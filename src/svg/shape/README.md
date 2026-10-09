# Bounded SVG shape subset

`svg_shape_decode` borrows exact source bytes and returns the source slice plus
owned flattened edges, per-edge RGBA colors, edge-to-element IDs, and bounded
per-element stroke state. The caller
keeps source bytes alive until the document is destroyed. The source is never
modified. Diagnostics are stable
categories plus a byte offset into that source; XML scanner failures retain the
scanner offset. Root `svg`, `rect`, `path`, `circle`, `ellipse`, `line`, `polygon`, and `polyline` are supported; fill implicitly closes paths and point lists, while `line` remains invisible without a stroke. Ellipses use deterministic polygonal flattening with at most 0.25 transformed user-unit sagitta and a 4096-segment cap. The bound scales the unit-circle chord error by the larger semiaxis, then by the active affine matrix norm; this remains conservative for eccentric ellipses without treating angular parameter as arc length. Nested groups and shapes support bounded `matrix`, `translate`, `scale`, and `rotate` transforms; skew forms are rejected. Group nesting is capped at 64 and transform operations at 4096 per document. Root viewport
dimensions are finite unitless or px lengths (positive, at most 4096); tile
extents round up to integer pixels while preserving the viewport scale, with an
optional finite positive `viewBox` using the default centered meet mapping. Rectangles accept x/y/width/height; omitted
x/y are zero and negative dimensions fail. Fill supports none, #RGB, #RRGGBB,
black, white, red, green and blue. `fill-rule` is nonzero or evenodd,
independently for each filled element. Elements paint in source order;
subpaths of one path share its fill rule, and each subpath closes at a later
moveto or the end of path. Unsupported elements, colors and syntax fail
without changing source bytes.

Path commands are M/m, L/l, H/h, V/v, C/c, S/s, Q/q, T/t, A/a and Z/z with repeated
groups. S/s reflect the previous second cubic control point only after C/c or
S/s; T/t reflect the previous quadratic control point only after Q/q or T/t.
Every other command, including closepath, resets both reflection contexts.
Coordinates are limited to absolute value 1,000,000; total path commands
and point-list vertices across the document are limited to 65,536; flattened edges to 262,144. Cubics
and quadratics use deterministic De Casteljau subdivision until every control
point is at most 0.25 transformed user units from the finite endpoint segment;
recursion depth is capped at 16. Elliptical arcs use endpoint-to-center
conversion and the same transformed-space sagitta tolerance, with total edges
bounded by the document cap. Flattening tolerance is conservatively adjusted
for the active affine matrix. These are user-space flatness bounds, not
device-pixel guarantees. Arc flags must be
exactly zero or one. Fill implicitly closes open
subpaths. Output uses scalar pixel-center point-in-path tests and transparent
black outside fills. The combined per-pixel edge-plus-shape tests are capped at
200,000,000 per render.

Direct presentation attributes implement a bounded first stroke subset on
line, polyline, polygon, rect, circle, and ellipse: literal stroke color, width,
butt/square/round caps, miter/bevel/round joins with miter-limit fallback, and
positive dash lists of at most 16 entries with phase continuity along each
ordered outline. Strokes use local user units and inverse-map samples through
the element transform, so nonuniform scaling changes stroke thickness correctly.
Width defaults to 1 user unit; stroke defaults to none, cap to
butt, join to miter, miter limit to 4, and dashing to solid. Stroke paints
after fill for each element. Painted paths are explicitly unsupported because
their subpath boundaries are not represented in the raster document. Widths
are capped at 1024 user units; miter limits at 64; dash values at 1,000,000.
Paint style resolves through root `svg`, nested `g`, and supported basic
shapes. Internal `<style>` blocks and `style` attributes accept the nine paint
properties listed above. Simple compound selectors support an optional
element, one ID, and up to 16 class terms; comma lists, importance, specificity,
source order, and inherited paint are applied deterministically. The stylesheet
is capped at 1 MiB, 4096 rules, and 16,384 declarations; a selector can match
at most 256 class tokens. Unsupported selectors, declarations, values,
gradients, filters, external resources, unsupported units, or unknown shape
semantics are not interpreted. Source bytes remain borrowed and unchanged.
This module has no platform or third-party dependency. Transform and shape syntax follows the W3C SVG 2 [Shapes](https://www.w3.org/TR/SVG2/shapes.html) and [Coordinate Systems and Transformations](https://www.w3.org/TR/SVG2/coords.html) specifications (consulted 2026-10-09).

The subset grammar and arc endpoint semantics reference the W3C SVG 2
[Paths](https://www.w3.org/TR/SVG2/paths.html) and
[Shapes](https://www.w3.org/TR/SVG2/shapes.html) specifications (consulted
2026-10-09). The C implementation is original project code; no external code
or SVG editor source was copied.
