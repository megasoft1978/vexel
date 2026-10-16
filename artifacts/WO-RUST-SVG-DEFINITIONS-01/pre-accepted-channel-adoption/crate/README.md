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

The production planner implements the accepted scene `DefinitionPlanner`
contract using typed `DefinitionOwner::Shape` and `DefinitionOwner::Group`
values. Empty filter groups are planned from their actual source and publish
through `SceneBuilder::publish_group_filter`; they do not require placeholder
shapes. Retained plan and bounded parsing scratch capacities are admitted to
the caller's job `ChargeLedger`, with temporary leases released when planning
fails.

The production pattern handoff returns the complete source-derived
`PatternMapping` (origin, both transforms, inverse, viewport, bounds, source
map, and opacity). The coordinator passes it to `PatternTickets::request_mapped`
and `complete_mapped`, which attach the resulting `PatternId` to fill or stroke.
The shared planner has no `PaintChannel` input even though legacy behavior
selects `fill-opacity` or `stroke-opacity`; this adapter rejects a pattern plan
when those effective values differ rather than publishing the wrong mapping.

This is a work-order candidate. Later coordinator adapters still need to execute
the returned expansions through the real decoder and kernel/provider
interfaces. The full renderer, application, runtime, release, and authentic
MCP demonstration gates remain outside this crate candidate. Candidate status
and exact evidence are recorded in the work-order artifacts.
