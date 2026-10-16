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
limits. Fill rows retain the center-only eligible-pixel path and exact 4×4
lattice at 0.125, 0.375, 0.625, and 0.875. `FrozenOracleOnly` is explicit;
normal fill rows use `SourceOver`.

Tests include real `SceneBuilder` drafts, parsed path and animation data,
same-source RasterJob buffers, canonical dash/grid stroke queries, controlled
fixture resources, row sample proofs, rollback, arithmetic, work-control, the original 70,000-segment dash/cache-limit
case, paint-order, fill-rule, and cap/join cases. These proofs are scoped to this
crate. Frozen mixed renderer cases, librsvg comparisons, the full 603/157
source/oracle audit, and final app/UI acceptance remain later integration gates.

The accepted scratch API currently lacks a typed fill-row cache descriptor with
per-row eligibility, five interval counts, interval stride, and pixel-run count.
`fill_row` therefore performs the scalar scan using admitted buffers; it does not
reproduce the original cached interval/run traversal or exact work profile. That
shared contract must be added by the coordinator before this optimization and
its work accounting can be called complete.
