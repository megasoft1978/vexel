# vexel-svg-scalar-sample

Scalar SVG paint and stroke sampling for the `RASTER_SAMPLE` partition. Public
entrypoints use `RasterJob`, `JobRasterScratch`, `ControlledKernelResources`,
and accepted `RasterJob::compose` / `RasterComposition` APIs. Dash records,
grid cells/references, and row buffers are admitted to the same job ledger;
cache reads are validated once per composition, with controlled work charged
inside dash, grid, row, paint, and stroke loops.

Dash rebuilds stage new job buffers and replace the old cache only after success.
Grid admission preserves the frozen scalar fallback and 16 MiB shared dash/grid
capacity, 65,536 cell, 1,000,000 reference, 128 axis, and 1,000,000 build-step
limits. Fill rows take a fixed-size `FillRowLayout` from the lower index owner;
the five interval counts, stride, max-edge count, and run count describe records
held in admitted same-job buffers. The row executes the frozen interval binary
search, center-only run pass, and active-slot/run 4×4 pass at 0.125, 0.375,
0.625, and 0.875. Actual comparison, interval, run, pixel, and sample work is
charged through the composition meter. `FrozenOracleOnly` remains explicit in
sample APIs; normal fill rows use `SourceOver`.

Tests include real `SceneBuilder` drafts, parsed path and animation data,
same-source RasterJob buffers, canonical dash/grid stroke queries, controlled
fixture resources, exact row-work and 4×4 sample proofs, malformed-layout
rejection, dash-cache cancellation rollback, arithmetic, limits, the original
70,000-segment dash/cache-limit case, paint order, fill rules, fractional tile
rounding, and cap/join cases. These are owned scalar proofs. Frozen mixed
renderer cases, librsvg comparisons, the full 603/157 source/oracle audit, and
final app/UI acceptance remain later integration gates.
