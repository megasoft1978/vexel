# SVG animation frame

This crate evaluates the accepted `vexel-svg-animation` overlay against an
immutable `SceneView` and returns a copy-on-change `EvaluatedFrame`. The public
`evaluate_frame` and `update_animation_clips` operations preserve checked scene
handles, transform ancestry, active overlay order, owner-before-suffix clip
matrices, and object-bounding-box updates. Clip replacement validates and
publishes atomically; failures leave the input frame unpublished.

The production entry point requires explicit policy:

```rust,ignore
evaluate_frame(view, time_seconds, frame_byte_budget, control)
```

The caller's finite budget passes unchanged to `FrameBuilder` and survives frame
publication, including actual old-plus-new clip capacity overlap. There is no
whole-frame default. The original 64 MiB clip-copy cap remains independent.
Actual scan iterations poll job control, while legacy local work retains its
50,000,000 ceiling, edge totals and 64-unit per-shape clip-owner accounting.

The tests build real owned scenes from `SourceOwner`, canonical
`LexicalDocument` tokens, accepted animation parsing, and the declared
`vexel-svg-path` test dependency. They cover original timing samples, colors,
opacity rounding, syncbase, direct motion and mpath, object paint bounds,
transform inverses, shared ClipId ownership, zero-edge handles, nested clip
owners, cancellation, and rollback. Renderer pixel composition, authoring/editor,
CLI/fuzz, and final application/runtime integrations remain later gates.

Focused commands:

```sh
cargo +stable check -p vexel-svg-animation-frame --locked --offline
cargo +stable test -p vexel-svg-animation-frame --locked --offline
```
