# WO-RUST-SVG-DEFINITIONS-01 candidate result

Status: **incomplete candidate; not accepted**.

Implemented typed filter parsing with source offsets and named unsupported-filter errors; safe clip geometry, boundary mapping/accounting, deduplicated owned overlays and draft-scoped attachment; definition source/style planning for clip, mask, pattern, marker and filter; and marker orientation/transform/path-placement helpers. The local CSS color parser uses the existing project-owned color implementation. The crate remains std-only and no shared files, manifests, source fixtures or original assertions were changed.

All 79 catalog symbols are listed in `parity-map.json`; several mask/pattern/marker traversal and execution symbols remain partial or unimplemented. Focused Rust tests cover the filter primitives/errors/colors, clip point/bounds math and marker math. The ten assigned original suites and all their input bytes remain unchanged; coverage records them as unrun because renderer/provider composition is not available in this crate candidate.

A concrete accepted-contract blocker prevents exact marker child-style planning: `CssPaint` has immutable getters but no operation to clear the four inherited marker properties while preserving other computed properties, as the legacy marker path requires. The root-owned CSS contract needs repair before that behavior can be implemented. Full definition expansion execution and original mixed renderer/oracle proofs remain open. This result does not claim complete parity or work-order acceptance.
