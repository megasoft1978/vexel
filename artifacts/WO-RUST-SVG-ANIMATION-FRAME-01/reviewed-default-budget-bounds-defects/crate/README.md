# SVG animation frame

This crate evaluates the accepted `vexel-svg-animation` overlay against an
immutable `SceneView` and returns a copy-on-change `EvaluatedFrame`. The public
`evaluate_frame` and `update_animation_clips` operations preserve checked scene
handles, transform ancestry, active overlay order, owner-before-suffix clip
matrices, and object-bounding-box updates. Clip replacement validates and
publishes atomically; failures leave the input frame unpublished.

`evaluate_frame` uses a named 64 MiB default, matching the original animation
clip-copy cap.
Composition with a configured job budget should call `evaluate_frame_with_budget`;
it rejects the unbounded `usize::MAX` sentinel and carries the selected budget
into the finished frame. Later clip replacement admits actual old-plus-new
capacity overlap against that same stored budget.

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
