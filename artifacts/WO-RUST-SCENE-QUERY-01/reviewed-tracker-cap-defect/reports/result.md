# Candidate result

Implemented the complete owned selection and allocation-query algorithms against accepted scene APIs. Query handle resolution now uses `SceneView::shape_id(index)`, including empty records, and preserves generation, scene, and owner guards. Bounds/source queries and hit testing have controlled variants; variable shape/edge scans charge shared work control and propagate cancellation. Hit testing preserves reverse painter order, edge-color/image visibility, nonzero/even-odd fill, clip-provider calls, inverse-space stroke distance, open-subpath edge exclusion, and tolerance scaling.

Source assignment is an append-time `ShapeInput` operation because published scene data is immutable. Allocation tracking uses actual Vec capacities and shared `ChargeLedger` leases. Replacement admission requires an active old-capacity lease, charges the new capacity while overlap exists, and returns its lease for caller-managed lifetime release. No raw allocator or RSS claim is made.

Tests build a real sealed graph using the seeded lexical, SVG path, and animation APIs. They cover zero-edge handles, stale/foreign IDs, bounds, source specificity, reverse painter order, clip gating, even-odd fill, stroke closure/inverse/tolerance, invalid coordinates, cancellation, capacity overlap/release, budget rejection, arithmetic overflow, and the original fixture source bytes.

The original `measure_allocation_source` integration assertions remain preserved but unexecuted until concrete decoder/reference-index, clip-copy, and raster-output composition is available. They are not reported as passed. Full app/runtime/release/authentic MCP demo and product QA remain later gates.
