# ADR-0035: GPU display rasterizer

- Status: accepted (provenance in docs/dependencies/objc2-metal-compute.lock.json)
- Date: 2026-10-25
- Related: ADR-0026, ADR-0027, ADR-0034; spike `spikes/metal-raster`

## Decision

Draw the on-screen canvas with a Metal compute rasterizer (tile-binned, one
16x16 threadgroup per tile, the spike's variant A) when the scene qualifies,
and fall back to the CPU rasterizer automatically otherwise.

- Metal FFI lives in `vexel-platform-apple` through the already pinned
  `objc2-metal =0.3.2`. Three more cargo features of that same audited
  package are enabled: `MTLBuffer`, `MTLComputeCommandEncoder`,
  `MTLComputePipeline`. No new package, version or download; `Cargo.lock` is
  unchanged by them. `dispatch2` stays inactive (`cargo tree -p
  vexel-platform-apple -i dispatch2` is empty), so no `DispatchData`.
- MSL is compiled at runtime from `include_str!` source with
  `newLibraryWithSource_options_error`. No Swift shim, no `metal-rs`, no
  offline metallib build step.
- New safe crate `vexel-gpu-scene` (`#![forbid(unsafe_code)]`, no external
  dependencies) holds the frozen Rust/MSL layout (`layout.rs`,
  `shaders/layout.h`), scene capability classification and the error and
  fallback policy. It never depends on `vexel-app`; the app lowers its render
  tree into this crate's plain-data types. `vexel-render` is a dev-dependency
  only, to assert the sample pattern matches.

## CPU stays the reference

The CPU rasterizer stays byte-identical and authoritative for export,
`render_png`, goldens and MCP. The GPU route is display only and is never used
to produce files or test oracles. Unsupported features (filters, masks,
patterns, images, text, groups deeper than 4, clip chains longer than 4,
scenes over the layout limits) classify as `Unsupported(reason)` and render on
the CPU.

## Parity tolerance (GPU vs CPU, whole frame, per channel)

- pixels differing by more than 2/255: at most 0.02%
- pixels differing by more than 16/255: at most 0.002%
- mean absolute error: at most 0.001 (0..1 units)

These bounds may be tightened, never loosened. GPU display becomes the default
only after the parity suite passes against them.

## Failure handling

- Fallback latch: after 3 runtime GPU errors in a session the GPU route is
  off for the rest of the session; each failing frame renders on the CPU.
- `VEXEL_GPU=0` disables the GPU route at start.

## Owner decisions accepted

- The on-screen canvas may differ from export within the tolerance above.
- GPU display is default-on once parity passes.
- No CPU re-render when interaction stops; the GPU frame is the final display.

## Addendum 2026-10-25: frozen layout v5

Interface changes decided by the coordinator after the encoder lane's
measurements (`docs/gpu-interface-requests.md`); the header and
`shaders/layout.h` carry layout version 5.

- **Edge precision: integer fixed point, `EDGE_FIXED_SHIFT = 12`**
  (1/4096 px), `MAX_COORDINATE = 262144` px (`classify` rejects larger
  coordinates with `Unsupported(InvalidScene)`, so the app renders them on the
  CPU). Crossing products use `i64` (MSL `long`). On 12 seeded unsnapped fill
  and gradient scenes at 256, 900 and 1024 px (`unsnapped_scenes_meet_adr_bounds`)
  the worst frame is 0.0112% over 2/255 (ADR 0.02%), 0.00033% over 16/255
  (ADR 0.002%) and mean error 3e-6 (ADR 0.001); shift 8 was 0.07 to 0.18% and
  failed. Alternatives measured by `tests/precision.rs` (mismatching samples
  next to an edge crossing, per million, radius 40 / 400 / 4000 px shapes):
  shift 12: 122 / 185 / 155; shift 14: 0 / 39 / 41 (needs
  `MAX_COORDINATE <= 32768` for `i32`); `f32` relative to a per-shape origin:
  24 / 44 / 398. The `f32` form is better only for small shapes, degrades with
  shape size, and breaks the shared-vertex consistency that exact integers
  give, so it is not used. Shift 14 is the fallback if a future scene class
  needs more margin.
- **Content rectangle**: four header words, a half-open range of sample
  indices (`content_x0..content_x1`, `content_y0..content_y1`, sample `i` at
  `(i + 0.5) / 4` px), exactly the CPU `offset <= sample < offset + size`
  test. Letterboxed frames match the CPU exactly (max diff 0). `Letterbox`
  stays in `UnsupportedReason` but is no longer produced.
- **Per-tile edge ranges**: the tile list is `GpuTileEntry {shape, edge_start,
  edge_count}` over a per-tile edge pool in tile-local coordinates. Edges left
  of a tile are dropped, straddling edges are kept verbatim, edges right of
  it collapse into vertical unit edges on its right border (the CPU counts
  crossings strictly right of a sample, so the carry is on the right). The
  result equals the unclipped one byte for byte; edge visits fall 35x to
  1700x on large shapes.
- **Clips**: `GpuClip` is a child record with its own fill rule and
  `union_size`; a shape's chain of up to `MAX_CLIP_CHAIN` links intersects,
  each link is the union of its children. Clip edges stay canvas-space and
  unclipped per tile; `classify` budgets them (`MAX_CLIP_EDGE_WORK`,
  `MAX_CLIP_RECORDS`). Rendering them is the clip lane's work; until then
  clipped scenes classify as `Unsupported(Clips)`.
