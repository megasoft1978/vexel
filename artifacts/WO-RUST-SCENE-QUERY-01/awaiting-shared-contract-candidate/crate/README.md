# Scene queries and allocation admission

This crate ports safe scene selection and explicit memory-capacity accounting.
`bounds_query` and `hit_test` consume immutable accepted `SceneView` data;
hit testing requires the real `KernelResources` clip implementation and a
caller work controller. Controlled bounds/source variants and hit testing
charge visited edges/shapes and observe cancellation. `element_at_offset`
selects the narrowest half-open source span, with later painter order winning
ties. Slot enumeration uses accepted `SceneView::shape_id`, including
zero-edge shapes, and inherits its generation/scene/owner validation.

Scene publication assigns source spans on `ShapeInput` before
`SceneBuilder::append_shape`. `source_assign` therefore modifies only an
unpublished input; published spans remain immutable. Shapes are reached with
accepted generation-checked handles. A Vec transfers ownership into an opaque
`TrackedBuffer` with its shared `ChargeLedger` lease attached; mutable Vecs and
bare leases do not escape. Only its tracker can replace or release it.
The tracker probes exact ledger ownership through the ledger's opaque zero-byte
lease while checking source identity.
`reserve_table` counts old and new records during
overlap, checks available budget before physical reservation, then admits actual
capacity before publishing. It releases the old lease only after old storage is
dropped. A failed physical reservation leaves original values and ledger history
intact. The adapter applies the legacy 65,536 active-record bound to buffers,
explicit capacity leases, and replacements. It does not claim raw allocator
hooks, allocator metadata, or RSS.

The mixed original measurement fixture still requires coordinator composition
with the concrete decoder/reference index, clip-copy producer, and raster
output. Those integration assertions are retained in the work-order coverage
record; they are not claimed as passed here.
