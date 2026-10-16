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
accepted generation-checked handles. The safe allocation adapter records
caller-owned capacity with shared `ChargeLedger` leases stored beside their
buffers; the shared ledger is the sole lease registry. It does not claim raw
allocator hooks, allocator metadata, or RSS. `reserve_table` builds a charged
replacement while old storage remains live and transfers values only after
admission succeeds. The old Vec capacity must already have a live ledger lease;
keep that lease until replacement returns, then release it. The returned lease
owns the new capacity until that buffer leaves the job.
The adapter enforces the legacy 65,536 active-record bound and rejects leases
from another tracking scope.

The mixed original measurement fixture still requires coordinator composition
with the concrete decoder/reference index, clip-copy producer, and raster
output. Those integration assertions are retained in the work-order coverage
record; they are not claimed as passed here.
