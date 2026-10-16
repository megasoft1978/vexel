# vexel-svg-definitions

This crate owns the SVG definition-domain slice: filter parsing, definition
planning, clip geometry/query helpers, clip-chain overlays, and marker math.
The code is safe Rust and uses the crate's existing pinned dependencies only.
Plans borrow CSS values from the source lifetime and own their expansion ranges;
they do not retain a borrow of the temporary mutable `DefinitionContext`.

`DefinitionPlanner` consumes the accepted `DefinitionContext`, scene builder,
active-definition state, and `WorkControl`. Downstream code can use its
`DefinitionPlan` with the real decoder and scene APIs. Filter plans preserve
supported primitive parameters, result references, the six named unsupported
primitive errors, and source offsets. Clip overlays own copied tables and
preserve shared clip IDs without raw pointers or reference counts. `apply_group`
uses the caller's real `DraftToken` and `SceneBuilder::finalize_shape`.

This is an implementation candidate, not a completed renderer port. Marker
expansion is not wired through all authored marker properties and nested
scenes. The shared `CssPaint` API exposes immutable getters only; it has no way
to clear inherited `marker`, `marker-start`, `marker-mid`, and `marker-end`
values while preserving other inherited paint values, which the legacy marker
path explicitly does before expanding marker children. The shared
`DefinitionPlan` also supplies no final execution adapter. Full clip/mask/
pattern/marker child decoding, provider execution, and renderer-level parity
remain coordinator integration gates. See the work-order artifacts for the
symbol map, original-case status, and exact verification results.
