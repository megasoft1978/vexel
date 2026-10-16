# Accepted contract blockers (R2, before substituting behavior)

## Definition owners before geometry

`vexel-scene::DecodeOperations::plan_definition` requires `owner: ShapeId`.
`src/svg/shape/traverse.c::svg_shape_traverse_begin_opacity_group` parses the
filter before direct geometry or child traversal. A valid empty group with a
filter therefore has no ShapeId; all ShapeId fields are private and enumeration
only returns already-published real shapes. Adding a placeholder shape changes
shape counts, source maps, groups and original error priority and is forbidden.
A coordinator-owned accepted source/group owner contract is needed for this
planning stage, including empty groups. Deferring filter parsing until a first
shape exists also changes the original filter-before-geometry error priority.

## Pattern paint handoff

`PatternDescriptor` returns tile dimensions, content transform, definition and
error offset, but no resolved instance paint/mapping. The original pattern
resolver computes x/y tile origin, patternTransform inverse, user-to-base,
pixel scaling, pattern_inverse and opacity separately from content_transform
(`src/svg/shape/pattern.c:463-633`). `resolve_paint` receives only DecodeSource,
gradient PaintCache and ChargeLedger; it receives no builder or notification of
the scene-owned PatternId created after tile completion. A source-only typed
pattern paint template/handoff is needed so DECODE can insert the single-use
completion PatternId into the exact fill/stroke mapping. Content transform is
not an equivalent replacement for the missing paint mapping.

## Work quantum across accepted atomic APIs

The current DecodeSource exposes synchronous `compute_style`/`find_reference`,
and SceneBuilder exposes synchronous `append_shape`/`commit_draft` validation.
These APIs charge variable loops internally but return no resumable source-only
state. Interrupting them by WorkControl::Stop aborts or rolls back instead of
yielding; storing CssComputeSession borrowing the decoder-owned DecodeSource
would introduce a self reference. A strict total-work <= work_quantum guarantee
needs accepted resumable state, or an explicit accepted contract defining bounded
atomic operations across quantum boundaries. Reporting one token visit while
these APIs perform substantial work would be false accounting.

The worker will not change shared contracts or fabricate IDs/mappings/providers.
These blockers do not excuse missing independent style/geometry/traversal
algorithms or original pure assertions; those remain the worker's scope.

## Original close/smooth path metadata and accepted validator disagree

`tests/svg/shape/test_shape.c::main` requires successful equivalent decoding of
`M1 1 C2 2 3 3 4 4 Z S6 6 7 7` and its explicit cubic form. The accepted Rust
path parser retains the overlapping subpath records after close, matching the
original parser structure. The accepted SceneBuilder rejects overlapping
subpath ranges with InvalidArgument:0. `original_shapes.rs` preserves this
assertion and currently fails. Coordinator must reconcile accepted parser/scene
metadata contracts; decoder must not discard, rewrite or hide source metadata.

## Mask expansion post target

`PostOperation::AttachMask(MaskId)` supplies neither target GroupId nor target
shape range. Mask ownership is a group field. An Expansion contains source range,
transform, viewport, style, flavor and post only. The engine cannot choose a
real target group for this post without guessing. Direct MaskDescriptor plans
can use the known owning Finish group; the standalone post requires an accepted
explicit owner contract. The decoder reports MaskPostTarget rather than success.

## Independent unfinished parity and evidence

These interface issues do not account for all remaining work. Definition plan
expansion allocations/active stacks/child draft lists and temporary clip copying
still require comprehensive actual-capacity leases. Lower-plan flavor/post
validation and nested pattern/media/marker/gradient tests are incomplete. Root
hidden/display:none and eager animation error priority need original-order
review. Full original fuzz command-cap, cache/accounting, gradient lookup and
remaining pure predicates have not all been ported. Whole original mixed inputs
are preserved, but preserving inputs is not execution proof. Do not accept this
candidate or infer that repairing the named contracts alone completes it.
