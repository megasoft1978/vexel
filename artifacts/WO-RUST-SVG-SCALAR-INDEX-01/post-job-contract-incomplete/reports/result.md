# Candidate status: incomplete

The accepted `RasterJob` API is now used by job-owned bounds, ordinary boundary-mask marking, and scanline event/index construction. These entrypoints validate source and scratch ownership, reserve typed capacities through the same `ChargeLedger`, charge boundary work in its actual loop, and roll back partial logical output on errors. A real `SceneBuilder`/parsed-path fixture covers these paths, including failed growth and cancellation.

The original safe Rust algorithm port remains in place for all 73 catalog symbols, including the 200M boundary cap, >4096-edge fallback, five scanline samples, event ties, fill winding/interval logic, and exact work-budget ramp. Eleven pure unit assertions pass; the full set of assigned pure assertions across all six original suites is not yet ported.

This is not complete owner scope: clip-boundary traversal, scanline rows, fill buffers/runs, and exact preflight still use the older Vec-based interfaces and cannot be registered as production engines. Some original mixed assertions remain for later composition. The animation fixture currently has empty tokens because `XmlToken` is unavailable through an accepted direct dependency without a protected manifest change.

Scoped locked offline check, test, formatting, and work-order validation pass. Independent review, protected-path audit, and fresh main checks remain coordinator gates; this candidate is not self-accepted.
