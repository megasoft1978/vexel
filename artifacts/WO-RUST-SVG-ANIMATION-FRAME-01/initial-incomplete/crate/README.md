# SVG animation frame

This crate evaluates the accepted `vexel-svg-animation` overlay against an
immutable `SceneView` and builds a copy-on-change `EvaluatedFrame`. Its public
entry point is `evaluate_frame`; it borrows the existing source, parsed tracks,
and base tables and applies active color, opacity, stroke, geometry, transform,
and motion values in overlay order. Invalid animation values and stopped work
return scene diagnostics without publishing a partial frame.

The frame path is a candidate, not complete legacy parity. The accepted scene
API does not expose clip-handle enumeration or mutation after frame finish, so
`update_animation_clips` is explicitly blocked pending a shared API extension.
The accepted renderer contracts also have no concrete flat-kernel/native
provider adapter yet. `raster_shape_render_at` therefore has no real raster
dispatch here. Authoring and CLI/fuzz harnesses likewise need their owning Rust
engines and later integration proofs. These gaps and every original catalog
entry are recorded in `artifacts/WO-RUST-SVG-ANIMATION-FRAME-01/`.

Focused commands:

```sh
cargo +stable check -p vexel-svg-animation-frame --locked --offline
cargo +stable test -p vexel-svg-animation-frame --locked --offline
```
