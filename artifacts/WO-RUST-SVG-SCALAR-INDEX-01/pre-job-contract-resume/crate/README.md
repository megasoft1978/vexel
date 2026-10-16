# vexel-svg-scalar-index

Safe Rust port of the scalar raster bounds, boundary mask, scanline event/AVL,
uniform-fill interval/run, and exact work-preflight partition. The public
functions consume accepted `vexel-scene` views and `KernelResources`; this crate
does not install a default clip, paint, or stroke provider. The coordinator
supplies the production adapter at composition time. Fixture providers belong
only in focused tests.

The port owns its vectors and uses fallible reservation, checked size math, and
`WorkControl` polling. Scanline row storage is explicitly prepared once before
repeated row construction. Shapes with more than 4096 edges stay on the scalar
fallback path. Boundary masks remain conservative when mappings are invalid.

`work_budget` preserves the 200,000,000 base, 1,048,576 pixel ramp start,
24,576 units per additional pixel, and 16,000,000,000 cap. Preflight accounts
for the full tile, bounds, boundary work, events, row traversal, stroke setup,
and either the cached fill route or the scalar sample route. Cancellation is
reported independently from the frozen work budget.

The original mixed renderer, fixture hash, decode accounting, native resource
provider, and application suites remain composition gates. The focused Rust
tests cover pure index math and data algorithms; they do not claim renderer
fidelity or product acceptance.
