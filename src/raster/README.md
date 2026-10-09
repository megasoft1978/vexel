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

The operation rejects invalid viewport data, malformed edge/group arrays, and
work above 200,000,000 edge and shape evaluations across all 16 subpixel
samples plus the conservative per-pixel edge-boundary scan. Checked arithmetic
accounts for that scan and rejects excessive work before the pixel loop;
quality is never reduced to satisfy the cap. Temporary winding,
parity, and color storage is bounded by the shape count. Output ownership passes to the caller
only on success, who releases it with `raster_tile_destroy`.

## Tile cache

`raster_tile_cache` is mutable from one owner thread and has no platform or shape-decoding dependencies. Its explicit byte cap is nonzero and at most 256 MiB. Accounted bytes include the complete preallocated entry array and every occupied tile's RGBA payload; creation rejects a slot array larger than the cap. LRU ties use stable slot order; a two-part monotonic tick preserves recency across tick rollover. Cache keys are caller supplied. Changing generation or integer scale bucket through `raster_tile_cache_set_epoch` clears all entries. A successful put transfers the tile's malloc-owned RGBA allocation and clears the caller's tile; a failed put leaves the caller's tile owned and unchanged. Lookup returns a borrowed tile valid until the next cache call that mutates LRU/cache state or destruction.
