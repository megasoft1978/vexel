# Scalar shape rasterizer

`raster_shape_render` allocates a caller-owned transparent RGBA tile and fills
the flattened edges from `svg_shape_document`. Geometry remains in user space;
the root viewBox maps it to the tile with the parsed `preserveAspectRatio`
alignment and `meet`, `slice`, or `none` behavior. The default is centered
`xMidYMid meet`. Nested `<svg>` viewport clips are carried by the shape
document. Each pixel uses a deterministic 4x4 grid at offsets
`{0.125, 0.375, 0.625, 0.875}` on each axis. For each shape, a sample is
covered only when it is inside the shape and inside every clip in its chain.
Pixel coverage is the rounded fraction of those 16 covered samples. A gradient
is evaluated once at the pixel center, and that color is used for every
covered sample; translucent gradient samples keep source-over compositing.
Final RGB is averaged over nontransparent sample colors and alpha is averaged
across all 16 samples. Fully covered opaque pixels retain the exact paint color
and empty pixels stay transparent black. Edges are grouped by source element,
fill rules are evaluated per element, and later painted elements replace
earlier opaque fills at each subpixel before coverage is resolved. A
`fill="none"` element is parsed but does not erase earlier pixels. The output
tile must be empty when passed in.

Embedded PNG/JPEG image fills sample provider-owned premultiplied RGBA8 with
bilinear interpolation in inverse-transformed local coordinates. Sampling is
bounded by the image viewport, source pixel dimensions, and clip chain; image
opacity is applied through the same isolated opacity groups as vector content.
Text reaches this renderer as flattened CoreText outline edges, so fill/stroke,
clips, transforms, and opacity follow the regular shape path.

After each element's fill, bounded strokes are tested against the same 4x4
sample grid. The stroke decoder stores each element's ordered edge range,
literal RGBA color, local width, inverse element transform, cap/join policy,
and bounded dash state.
Before rasterization, stroke intervals and joins are materialized once. A
conservative local-space grid narrows each sample to nearby stroke pieces;
segments include cap reach and joins include the miter-limit reach. Grid cells
are at least 1 user unit wide, each axis is at most 128 cells, and all shapes
share caps of 65,536 cells, 1,000,000 references, and 16 MiB of cache storage.
The dash builder is capped at 65,536 pieces per array and 1,000,000 build steps.
If a shape cannot be indexed within those bounds, it uses the bounded linear
stroke scan and the work preflight accounts for every piece tested. The
preflight also charges grid lookups and candidate pieces per sample, so sparse
paths no longer pay for every edge inside the path's full bounding rectangle.
The renderer computes a conservative device-space box for each shape's fill and
stroke. Stroke boxes include the transformed half-width, miter-limit reach,
square-cap reach, and a small floating-point margin. The box maps to every tile
pixel that can contain one of the 4x4 sample points. Empty and zero-area
geometry stays harmless; invalid or non-finite bounds conservatively select
the full tile. The one-bit boundary map marks pixels near every painted-shape
edge, with stroke reach included, and every clip child edge in each clipped
shape's full parent chain. Clip geometry is mapped through its element
transform into device space; object-bounding-box clip edges are first mapped
through the painted shape bounds. Edge boxes expand beyond the 4x4 sample
footprint to stay conservative. The center-sample fast path is used only
outside this map and outside partial viewport pixels, where no shape or clip
edge can cross the pixel. Gradient interior variation does not require marking
because gradient color is sampled at the pixel center.

Shapes are visited in source paint order and only over their clipped boxes.
A y-interval event set yields the shapes whose bounds intersect the current
row, avoiding a full shape-list scan for every tile row. A scanline event table
maintains each shape's fill edges whose half-open y-range contains each sample
row. The active set is emitted in original edge order;
the crossing arithmetic, winding/parity updates, color replacement, stroke
tests, sample locations, and final RGBA rounding keep the reference operation
order. The test-only `raster_shape_render_reference` has no boundary map or
center shortcut: it tests all 16 sample positions for every pixel against
every shape and clip, while evaluating gradient paint at the pixel center. It
serves as the byte-comparison oracle for production culling and scanline paths.

The renderer rejects invalid viewport data, malformed edge/group arrays, and
actual post-culling work above the existing 200,000,000-operation cap. The
checked work estimate includes tile pixels, summed shape-box pixels, painted
and clip edge boundary-map visits and pixel marks, scanline events and
active-edge visits, fill crossings, gradient-stop and clip-test work, and
stroke edge/dash work for each relevant sample. Checked arithmetic rejects overflow
and excessive work before rendering; quality is never reduced to satisfy the
cap. Temporary storage is bounded by shape/edge counts, tile dimensions, and
the output tile: the boundary map is one bit per tile pixel, sample buffers are
one row wide, and scanline events are at most two per edge. Rendering remains
single-threaded and deterministic. Output ownership
passes to the caller only on success, who releases it with `raster_tile_destroy`.

Element/group `opacity` is composited in isolated RGBA layers after the group
contents have painted. Nested layers composite source-over in document order.
Scratch tile bytes are checked and capped at 64 MiB; a request above the cap
returns a limit diagnostic instead of painting a partial frame.

## Raster modules

- `shape_render.c` validates the document, builds bounds and scanline state,
  enforces the existing checked work cap, and coordinates deterministic sample
  traversal and RGBA output.
- `shape_render_sample.c` evaluates one fill sample, applies clip membership and
  paint-server sampling, then composites the fill and stroke with the existing
  integer source-over operation. It preserves the test-only scalar alpha mode.
- `shape_render_stroke.c` materializes local stroke segments and tests caps,
  joins, miter limits, dash intervals, and closed-path dash seams.
- `shape_render_stroke_grid.c` builds and queries the bounded local-space stroke
  grid; the scalar reference remains an unindexed comparison path.
- `shape_render_group.c` renders isolated opacity subtrees using bounded tile
  scratch and composites each layer with source-over alpha.
- `shape_bounds.c` owns conservative device-space shape boxes and the boundary
  mask; `shape_scanline.c` owns row events and active-edge ordering.
- `shape_render_internal.h` carries private measurement and helper contracts;
  the public `shape_render.h` API is unchanged.

## Tile cache

`raster_tile_cache` is mutable from one owner thread and has no platform or shape-decoding dependencies. Its explicit byte cap is nonzero and at most 256 MiB. Accounted bytes include the complete preallocated entry array and every occupied tile's RGBA payload; creation rejects a slot array larger than the cap. LRU ties use stable slot order; a two-part monotonic tick preserves recency across tick rollover. Cache keys are caller supplied. Changing generation or integer scale bucket through `raster_tile_cache_set_epoch` clears all entries. A successful put transfers the tile's malloc-owned RGBA allocation and clears the caller's tile; a failed put leaves the caller's tile owned and unchanged. Lookup returns a borrowed tile valid until the next cache call that mutates LRU/cache state or destruction.
