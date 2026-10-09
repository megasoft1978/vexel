# Scalar shape rasterizer

`raster_shape_render` allocates a caller-owned transparent RGBA tile and fills
the flattened edges from `svg_shape_document`. Geometry remains in user space;
the viewBox maps it to the tile with SVG default `xMidYMid meet` sizing and centered letterboxing. Each pixel uses a deterministic 4x4 grid at offsets
`{0.125, 0.375, 0.625, 0.875}` on each axis. Coverage alpha is the rounded
fraction of covered samples; RGB is averaged over covered samples, so fully
covered pixels retain the exact fill color and empty pixels stay transparent
black. Edges are grouped
by source element, fill rules are evaluated per element, and later painted
elements replace earlier opaque fills at each subpixel before coverage is
resolved. A `fill="none"` element is parsed but
does not erase earlier pixels. The output tile must be empty when passed in.

After each element's fill, bounded strokes are tested against the same 4x4
sample grid. The stroke decoder stores each element's ordered edge range,
literal RGBA color, local width, inverse element transform, cap/join policy,
and bounded dash state.
The renderer computes a conservative device-space box for each shape's fill and
stroke. Stroke boxes include the transformed half-width, miter-limit reach,
square-cap reach, and a small floating-point margin. The box maps to every tile
pixel that can contain one of the 4x4 sample points. Empty and zero-area
geometry stays harmless; invalid or non-finite bounds conservatively select
the full tile. Pixel coverage near edge segments is marked separately, with
stroke reach included, so a pixel outside the mark can use the original center
sample fast path while marked pixels retain all 16 samples.

Shapes are visited in source paint order and only over their clipped boxes.
A y-interval event set yields the shapes whose bounds intersect the current
row, avoiding a full shape-list scan for every tile row. A scanline event table
maintains each shape's fill edges whose half-open y-range contains each sample
row. The active set is emitted in original edge order;
the crossing arithmetic, winding/parity updates, color replacement, stroke
tests, sample locations, and final RGBA rounding keep the reference operation
order. This is why the culling and scanline shortcuts can be byte-identical to
the unculled renderer. The test-only `raster_shape_render_reference` retains
the unculled pixel/edge traversal as a differential oracle.

The renderer rejects invalid viewport data, malformed edge/group arrays, and
actual post-culling work above the existing 200,000,000-operation cap. The
checked work estimate includes tile pixels, summed shape-box pixels, edge-boundary
mark work, scanline events and active-edge visits, fill crossings, and stroke
edge/dash work for each relevant sample. Checked arithmetic rejects overflow
and excessive work before rendering; quality is never reduced to satisfy the
cap. Temporary storage is bounded by shape/edge counts, tile dimensions, and
the output tile: the boundary map is one bit per tile pixel, sample buffers are
one row wide, and scanline events are at most two per edge. Rendering remains
single-threaded and deterministic. Output ownership passes to the caller only
on success, who releases it with `raster_tile_destroy`.

## Tile cache

`raster_tile_cache` is mutable from one owner thread and has no platform or shape-decoding dependencies. Its explicit byte cap is nonzero and at most 256 MiB. Accounted bytes include the complete preallocated entry array and every occupied tile's RGBA payload; creation rejects a slot array larger than the cap. LRU ties use stable slot order; a two-part monotonic tick preserves recency across tick rollover. Cache keys are caller supplied. Changing generation or integer scale bucket through `raster_tile_cache_set_epoch` clears all entries. A successful put transfers the tile's malloc-owned RGBA allocation and clears the caller's tile; a failed put leaves the caller's tile owned and unchanged. Lookup returns a borrowed tile valid until the next cache call that mutates LRU/cache state or destruction.
