# SVG clip paths

The clip module resolves a local `clip-path: url(#id)` to a bounded set of
basic shapes or paths, decodes those shapes into an owned scalar geometry
document, and tests point membership as a union. The rasterizer intersects that
union with each covered fill sample. The decoded clip and its temporary source
are owned by the parent shape document and released with it.

Before copying clip-child source, the resolver computes each definition and
child style through the document CSS cascade. Nested `clip-path` declarations
and other matched paint properties are rejected with a stable `unsupported`
diagnostic, including declarations supplied by class selectors.

`clipPathUnits` supports `userSpaceOnUse` (the SVG default) and
`objectBoundingBox`. User-space clips are sampled in the declaring element's
user space, including ancestor and `<use>` transforms. Object bounding boxes
come from the declaring element's untransformed local geometry and are then
mapped through its transform. A group or `<use>` clip uses the AABB of its
whole rendered subtree and is resolved once for that declaration. Child clips
intersect with enclosing group/use clips; `clip-path` does not pass down as an
inherited style property.

Each clip is limited to 64 direct children, 65,536 flattened edges per clip,
64 KiB per referenced child span, and 512 KiB of clip-child scratch copies per
decode job. The wrapper prefix is formatted into a 192-byte stack buffer; its
actual bytes, the six-byte `</svg>` suffix, the referenced span, and the NUL
terminator count toward the scratch budget. The copied child span is never
modified in the original document. Oversized references return a stable
`limit` diagnostic. Nested `clip-path` declarations inside clip geometry,
empty clips, unsupported child syntax, singular transforms, and external
references fail with stable diagnostics. The raster work budget includes clip
edge tests.

The module also exposes an owned four-corner rectangle clip used by nested SVG
viewports. Shape traversal maps that rectangle through the active viewport
transform and attaches it to every descendant shape in the viewport range;
this clip intersects with the shape's other clips during raster sampling.

Each resolved clip records its rendered transform-frame owner rather than only
the source byte offset. This distinguishes separate `<use>` instances of the
same source element. Animation overlays deep-copy shared clip chains within a
64 MiB per-frame cap, update the owning inverse transform, and refresh
object-box bounds after local geometry changes. Nested viewport rectangles use
the owning frame before its viewBox suffix so their clip stays in the viewport
coordinate system.

The 512 KiB budget allows about 1,700 standalone shape clips when each
generated copy is about 300 bytes. Group clips consume one copy per declaring
group regardless of its child shape count. Scratch is still charged once per
clip declaration, including declarations inside separate `<use>` instances.

No external resources or renderer source are loaded or copied. Clip-child
scratch allocation obeys ADR-0014; the original document is never modified.
`clipPathUnits` defaults and clip geometry follow the W3C
[CSS Masking Module Level 1](https://www.w3.org/TR/css-masking-1/) section 6.1
(consulted 2026-10-09); the prior SVG 2 masking chapter now points to this
specification.
