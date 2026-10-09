# SVG clip paths

The clip module resolves a local `clip-path: url(#id)` to a bounded set of
basic shapes or paths, decodes those shapes into an owned scalar geometry
document, and tests point membership as a union. The rasterizer intersects that
union with each covered fill sample. The decoded clip and its temporary source
are owned by the parent shape document and released with it.

`clipPathUnits` supports `userSpaceOnUse` (the SVG default) and
`objectBoundingBox`. Each clip is limited to 64 direct children, 65,536
flattened edges per clip, 64 KiB per referenced child span, and 512 KiB of
clip-child scratch copies per render job. A bounded wrapper is generated in a
192-byte stack buffer; the copied child span is never modified in the original
document. Oversized references return a stable `limit` diagnostic. Nested
clip paths, empty clips, unsupported child syntax, and external references
fail with stable diagnostics. The raster work budget includes clip edge tests.

No external resources or renderer source are loaded or copied. Clip-child
scratch allocation obeys ADR-0014; the original document is never modified.
`clipPathUnits` defaults and clip geometry follow the W3C
[CSS Masking Module Level 1](https://www.w3.org/TR/css-masking-1/) section 6.1
(consulted 2026-10-09); the prior SVG 2 masking chapter now points to this
specification.
