# vexel-svg-scalar-index

This crate contains the safe Rust port of the assigned bounds, boundary,
scanline, uniform-fill and preflight algorithms. The Vec-based scalar entrypoints
preserve the source algorithms and remain useful for pure parity fixtures; they
are not the controlled production composition API.

The current job-owned entrypoints are `build_bounds_job`,
`mark_boundaries_job`, and `create_scanlines_job`. They validate source/job
identity, reserve typed `JobRasterScratch` buffers through the same `RasterJob`
and `ChargeLedger`, poll the caller's `WorkControl` in variable loops, and keep
partial results unpublished on errors. Boundary marking retains the original
200,000,000 work cap. Scanline construction retains five samples, original
event ordering, and AVL/node sizing.

This is an incomplete work-order candidate. Fill-cache/interval/run storage,
row construction, controlled clip-boundary traversal and exact preflight have
not yet been converted to job-owned storage. Do not register the Vec-based
entrypoints as production engines. Full mixed renderer and decoder assertions,
fixture hash proofs, and native providers remain later composition gates.
