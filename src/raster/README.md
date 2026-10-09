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
