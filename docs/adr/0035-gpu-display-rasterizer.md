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
