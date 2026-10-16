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
accepted generation-checked handles. Query storage uses the accepted
`ChargeLedger::allocate_buffer` and opaque `LedgerBuffer`, wrapped in a
scope-owned `TrackedBuffer`; no Vec or lease escapes. `LedgerIdentity` and
`require_owner` bind each operation to the same source and exact ledger.
`reserve_table` uses the accepted replacement operation, which admits requested
bytes before physical reservation and reconciles actual capacity before
publication. Old and new lease records overlap until the old storage is dropped.
`track_filled_buffer` applies typed zero initialization after admission without
allocating an unaccounted temporary buffer.
Cancellation releases temporary live capacity while retaining successful
physical allocation history and honest ledger metadata. The adapter applies the
legacy 65,536 record bound to initial buffers and replacement overlap. It does
not claim raw allocator hooks, allocator metadata, or RSS.

The mixed original measurement fixture still requires coordinator composition
with the concrete decoder/reference index, clip-copy producer, and raster
output. Those integration assertions are retained in the work-order coverage
record; they are not claimed as passed here.
