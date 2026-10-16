# Candidate result

Implemented the catalogued RASTER_SAMPLE symbols in owned Rust modules: dash and seam construction, scalar stroke hit testing, bounded stroke grids and fallback, image/pattern sampling, source-over and paint order, clipping through the real resource seam, and cached plus 4x4 fill-row sampling. Original dash, cap, join, alpha-rounding, fill-lattice, and work-limit predicates have focused Rust assertions. See `parity-map.json` and `coverage.json` for symbol-by-symbol and original-case status.

Scoped offline check and test pass; the frozen work-order validator passes. This is a candidate, not an accepted completion. Two protected shared contracts block the full gate: `KernelResources::paint_sample` cannot receive/charge `WorkControl`; `DashCache`/`RasterScratch` have no retained `ChargeLedger` lease path, and the cache builder has no ledger argument. The crate enforces the original actual-capacity 16 MiB cache cap locally. The coordinator must repair these shared contracts and run preserved mixed renderer fixtures through real providers and `FlatKernel` before acceptance.

## Shared-contract implementation in this isolated tree

`JobDashCache`/`JobDashRange` describe same-job segments/joins/nested grid buffers;
`JobDashView` resolves original element slices without copying the cache.
`ControlledKernelResources` is a separate complete Result-returning capability
whose paint method receives the actual WorkControl. No conversion/default from
legacy unmetered paint exists. `ControlledSampleInput` and lower typed traits
let SAMPLE consume these resources without changing scalar math. The explicit
fixture proves cancellation during stop traversal and exact 4x4 input/rounding.

No production provider is faked or registered. Complete SAMPLE algorithms,
root adapters and mixed oracle parity remain later owned scopes. See
APIcoverage.md and verification.json for lifecycle and exact evidence.
