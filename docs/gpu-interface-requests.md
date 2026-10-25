# GPU rasterizer interface requests

Changes the lanes need in the frozen `vexel-gpu-scene` layout
(`src/layout.rs`, `shaders/layout.h`). None is applied; each lane works
around it as described.

## L3 (encoder, binning, reference)

1. **Per-tile edge ranges.** `layout.rs` says edges are tile-local and split
   at tile borders, but a `GpuShape` has one `edge_start/edge_count` and the
   tile list holds bare shape indices, so a tile cannot address its own
   slice of a shape's edges. Workaround in force: edges are stored once per
   shape, unsplit, in canvas coordinates (tile (0,0) origin; the kernel
   subtracts its tile origin), and each tile visits every edge of every
   shape in its list. Work is `sum(edges * covered tiles)`, budgeted by
   `classify::MAX_EDGE_TILE_WORK`. Request: make a tile list entry a
   `{shape, edge_start, edge_count}` triple (12 bytes) with edges clipped per
   tile (left of the tile collapsed to a winding carry), if large shapes
   (maps, big paths) show up in profiles.
2. **`EDGE_FIXED_SHIFT` 8 -> 12.** At 1/256 px the encoder's vertex rounding
   alone moves edge samples, which breaks the ADR-0035 bounds (the encoding
   is otherwise exact: scenes with vertices on the 1/256 grid match the CPU
   within 1 level, max diff 1, 0.0005% of pixels over 2/255). Measured on
   seeded unsnapped fill and gradient scenes, fraction of pixels over
   2/255 (ADR max 0.02%), over 16/255 (ADR max 0.002%):
   - shift 8: 0.07% to 0.18%; 0.001% to 0.008%
   - shift 10: 0.026% to 0.055%
   - shift 12: 0.004% to 0.014%; 0 to 0.002% (one pixel over on one seed)
   Shift 12 keeps `i32` coordinates exact up to +-524288 px; products in the
   crossing test need `i64` (MSL `long`) and stay inside `i64` while
   coordinates are at most 262144 px, so `classify::MAX_COORDINATE` would
   drop from 1e6 to 262144 (the CPU accepts 1e6). The test
   `unsnapped_scenes_meet_adr_bounds` (ignored) asserts the ADR bounds and
   should be un-ignored after the change.
3. **Content rectangle.** The CPU excludes samples outside the `ViewMap`
   content rectangle (meet letterboxing). The header has no field for it, so
   `classify` returns `Unsupported(Letterbox)` when it is smaller than the
   canvas. Request: two header words, or let the adapter clip shapes to it.
4. **Clip union.** `GpuClip` carries one rule per edge range, but a CPU clip is
   a union of child shapes with their own rules and a chain intersects.
   Needed for the clip lane: a `GpuClip` per child and a flag/size to union
   children of one clip.
5. `UnsupportedReason` (error.rs) gained `Strokes`, `Groups`, `Clips`,
   `Letterbox`, `InvalidScene` (append only).
