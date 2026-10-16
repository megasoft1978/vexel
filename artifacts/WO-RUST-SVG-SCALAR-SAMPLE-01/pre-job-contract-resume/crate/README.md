# vexel-svg-scalar-sample

Owned scalar sampling and stroke cache implementation for the RASTER_SAMPLE
partition. The public surface in `src/lib.rs` uses the accepted `SceneView`,
`SampleInput`, `KernelResources`, and `WorkControl` contracts. Dash generation
keeps source edge order, closed seams, odd-pattern repetition, caps, joins, and
miter limits. Stroke grids retain global segment/join indices and abandon to
the scalar path when the cell, reference, or shared 16 MiB cache limit is hit.

Sampling implements image and wrapped pattern bilinear interpolation,
premultiplied pattern interpolation, source-over integer rounding, clips through
the caller's resource provider, and fill/stroke paint order. `fill_row` uses all
sixteen original sample positions and stores its row result in
`RasterScratch::center_samples`; subpixel colors are retained in
`subpixel_samples`. All variable geometry and cache loops receive the actual
job work controller. Allocation failures map to the scene OOM diagnostic.

Two shared integration seams remain blockers for acceptance. `KernelResources::paint_sample`
has no `WorkControl` parameter, so a real gradient provider cannot report or
charge variable stop traversal. `DashCache` and `RasterScratch` expose owned
vectors but no `ChargeLedger` lease slot, and the cache builder has no ledger
argument; this crate enforces the original actual-capacity 16 MiB dash cap but
cannot register those capacities against the job's shared reservation. The
coordinator must repair these shared contracts before native composition.

The direct Rust tests cover the original scalar cap, join, seam, source-over,
and 4x4 lattice predicates. Decoder-backed source fixtures and whole-renderer
comparisons stay in the original protected C/Python files and are recorded as
later composition gates in the work-order coverage artifact.
