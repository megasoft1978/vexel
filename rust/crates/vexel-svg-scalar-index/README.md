# vexel-svg-scalar-index

`ScalarIndex::build` constructs the complete conservative bounds, boundary/clip
mask, event/AVL index, row workspace and uniform-fill cache from an accepted
`SceneView`. It takes the caller's owning `RasterJob`, mapping, explicit
`ControlledKernelResources` and `WorkControl`. It retains only typed handles,
scene identity and scalar state. The same job owns every payload and backup;
source bytes are retained and charged once by its ledger.

Use `ScalarIndex::build_row`, `build_fill_row`, `build_fill_runs`, and
`preflight_job` for production composition. The standalone job functions are
also public. The actual stroke engine supplies the admitted `stroke_cost` and
`JobDashCache` records before preflight; missing setup is an error. Providers are
explicit and receive the actual control inside clip/stroke traversal. This crate
registers no paint, clip, stroke or native provider.

Every buffer reserve/append uses the accepted job's pre-allocation admission,
actual capacity checks, old/new overlap and lease lifecycle. Row/fill and whole
preflight rollback storage is admitted at build time and reused; no row call
allocates. Errors restore payload/scalar state and leave physical allocation/work
history recorded. Restoration after a stop is bounded, allocation-free cleanup.
Clip errors restore bits while retaining incurred boundary work. Build failures
release all created payloads. External shared dash grids remain independently
job-owned and require caller release or job finish/Drop.

The engine retains five row samples, event ties, ascending painter order, winding
intervals, crossing heapsort, binary lookup comparisons, pixel run transitions,
the >4096-edge fallback, conservative invalid clip inverses, the 200M marking
work cap, and the exact 200M-to-16B preflight ramp. Fixed stack state is not a
heap allocation. Work control polls actual variable loops; native allocation and
failure cleanup are indivisible. Capacity evidence measures Vec storage, not RSS.

Historical Vec entry points are compiled only under `cfg(test)` for the original
11 unit fixtures. They are private and absent from normal builds. Production
callers use admitted owning-job APIs and the allocation-free `fill_color` scene
lookup. Shared allocation-free
geometry, heapsort, AVL and budget helpers remain available internally.

Canonical fixtures tokenize real SVG through `LexicalDocument.tokens()`, parse
animation/path data, use `SceneBuilder` drafts and evaluated frames, and exercise
all admitted algorithms. The original 1000-tooth cost is exactly 365,760,771;
1100 teeth fail with work limit at offset zero. Mixed decoder/renderer bytes,
W8 pixel SHA, native providers and app acceptance remain composition gates.
The frozen numeric-separator rejection passes through both accepted path parser
APIs after the coordinator seeded the accepted peer repair. All 37 focused tests
pass; independent review and main integration checks remain required.
