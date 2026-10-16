# WO-RUST-SVG-PATTERN-01 result

Pattern paint renders in `vexel-render` with exact pixel parity against the
legacy C renderer on all five fixtures (max delta 0, 0 differing pixels each).

Mirrored constants: `PATTERN_MAX_INSTANCES` 1024, `PATTERN_MAX_PIXELS`
1048576, `PATTERN_MAX_AXIS` 4096 (all `LimitExceeded`), tile extent 65536,
pixel size `ceil(extent * density)` min 1. Sampling is C `pattern_sample`
(half-pixel bilinear, wrapping, opacity). C also paints pixels untouched by any
edge (boundary mask, `shape_bounds.c`) with one centre sample; this is ported
in `pattern_boundary.rs` and applied only to pattern-painted pixels, so solid
output is byte-identical (verified on every existing parity case).

Fixtures: the five legacy pattern fixtures (`tests/realworld/fixtures/w5/
pattern-*.svg`; none exist under `tests/svg/shape/fixtures`) are all
solid-colour children, none excluded. They cover userSpaceOnUse, objectBoundingBox
dots, viewBox, patternTransform rotate and stroke paint. C geometry and tile
content were dumped with a read-only hook copy of `pattern.c` outside the repo;
the Rust tile equals the C tile byte for byte on all five.

Additive API: `PatternPaint`, `PatternPaintShape`, `render_pattern_shapes`,
`render_pattern_tile(s)`, `pattern_pixel_size`, `from_scene_shape_with_pattern`.
`from_scene_shape` still returns `Unsupported(PatternPaint)`.

Limits of this order: `from_scene_shape_with_pattern` success path needs a
scene `PatternId`, which only the scene builder can mint, so it is exercised
only for error paths. vexel-app wiring (completing `NeedPattern`) is
unexecuted and belongs to WO-RUST-SVG-WIRE-01. The tag commit in the manifest is
the raw `.git/refs/tags/legacy-c-final` value (git on the tag was unavailable).
