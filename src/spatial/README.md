# Immutable spatial index

`spatial_index_create` copies at most 1,000,000 records into an immutable Morton-ordered bounding-volume tree. Creation rejects invalid bounds and duplicate IDs before publishing an index. The complete owned index allocation budget, including temporary duplicate-ID validation storage, is capped at 192 MiB. Bounds are finite axis-aligned rectangles with nonnegative extents; endpoint overflow is invalid.

The index may be queried concurrently after creation. A query treats touching edges as intersections and returns caller record positions in their original input order. It first gathers and sorts matches in private storage. When the caller's capacity is too small, it writes no result elements and returns the required count with `SPATIAL_STATUS_LIMIT`. Query scratch memory is released before return. The caller owns and destroys the index.
