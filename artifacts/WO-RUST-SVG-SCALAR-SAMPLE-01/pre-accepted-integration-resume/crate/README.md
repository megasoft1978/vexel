# vexel-svg-scalar-sample

This crate owns scalar fill sampling, stroke geometry, dash generation, stroke
grid construction, and paint compositing for the RASTER_SAMPLE partition. The
sampling surface now accepts `ControlledSampleInput`, whose paint and clip calls
receive the caller's actual `WorkControl`; stroke queries read validated
`JobDashView` ranges and same-job grid buffers.

The current candidate is not complete against the frozen owner order. Dash and
grid construction still use the earlier standalone `DashCache` vectors, and
`fill_row` still writes the legacy `RasterScratch` vectors rather than
`JobRasterScratch` buffers. Those construction paths therefore are not suitable
as controlled production composition entrypoints. The accepted APIs are present in `vexel-scene`, but their current borrow surface
also blocks a lower engine from retaining `JobDashView` while mutating disjoint
same-job output/scratch buffers: the view borrows `RasterJob` immutably and all
job writes require `&mut RasterJob`. Recreating the view for each sample repeats
charged validation work and changes the frozen work limits. The coordinator must
provide a safe split-borrow seam before this crate can port those loops without
changing semantics. The full engine port must then retain pre-allocation leases,
old/new overlap, append-time source assignment, per-iteration work charging, and
failure atomicity.

The tests include original pure geometry/rounding/lattice predicates and a
canonical scene built through `SceneBuilder`, a committed `DraftToken`, parsed
path geometry, and parsed animation. They also verify same-source `RasterJob`
scratch accounting and cancellation rollback. These tests do not establish
whole renderer parity. Mixed decoded rendering and fidelity cases remain later
composition gates and their original protected fixtures are unchanged.
