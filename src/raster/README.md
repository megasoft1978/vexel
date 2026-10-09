# Scalar shape rasterizer

`raster_shape_render` allocates a caller-owned transparent RGBA tile and fills
the flattened edges from `svg_shape_document`. Geometry remains in user space;
the viewBox maps it to the tile with SVG default `xMidYMid meet` sizing and centered letterboxing. Sampling uses pixel centers. Edges are grouped
by source element, fill rules are evaluated per element, and later painted
elements replace earlier opaque fills. A `fill="none"` element is parsed but
does not erase earlier pixels. The output tile must be empty when passed in.

The operation rejects invalid viewport data, malformed edge/group arrays, and
work above 200,000,000 combined edge and shape tests. Temporary coverage
storage is bounded by the shape count. Output ownership passes to the caller
only on success, who releases it with `raster_tile_destroy`.

## Tile cache

`raster_tile_cache` is mutable from one owner thread and has no platform or shape-decoding dependencies. Its explicit byte cap is nonzero and at most 256 MiB. Accounted bytes include the complete preallocated entry array and every occupied tile's RGBA payload; creation rejects a slot array larger than the cap. LRU ties use stable slot order; a two-part monotonic tick preserves recency across tick rollover. Cache keys are caller supplied. Changing generation or integer scale bucket through `raster_tile_cache_set_epoch` clears all entries. A successful put transfers the tile's malloc-owned RGBA allocation and clears the caller's tile; a failed put leaves the caller's tile owned and unchanged. Lookup returns a borrowed tile valid until the next cache call that mutates LRU/cache state or destruction.
