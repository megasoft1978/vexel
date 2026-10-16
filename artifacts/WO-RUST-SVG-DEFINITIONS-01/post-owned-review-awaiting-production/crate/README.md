# vexel-svg-definitions

This crate owns the complete typed planning and safe scene-domain behavior for
SVG clip, mask, pattern, marker, and filter definitions. Plans borrow the
canonical source for computed CSS, store checked token-table ranges for later
engine traversal, and do not retain temporary mutable context borrows. The
planner never calls back into the decoder.

Clip queries and mutation adapters operate on real `SceneView`, `SceneBuilder`,
and draft handles. Clip graph copies deduplicate shared IDs and own their
geometry; Rust ownership replaces the legacy raw retain/release interface.
Marker planning clears only inherited marker properties using the accepted
CSS context API. Filter primitives retain typed parameters, result references,
named unsupported errors, and original source offsets.

`PatternMapping` owns the tile origin and user-to-tile inverse, including
`patternTransform`. The current shared `PatternDescriptor` carries dimensions,
tile size, and content transform but has no fields for that paint mapping; the
later paint adapter must carry this mapping to the scene paint record.

This is a work-order candidate. Later coordinator adapters still need to execute
the returned expansions through the real decoder and kernel/provider
interfaces. The full renderer, application, runtime, release, and authentic
MCP demonstration gates remain outside this crate candidate. Candidate status
and exact evidence are recorded in the work-order artifacts.
