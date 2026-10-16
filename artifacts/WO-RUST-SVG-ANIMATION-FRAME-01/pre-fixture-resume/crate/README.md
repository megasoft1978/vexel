# SVG animation frame

This crate evaluates the accepted `vexel-svg-animation` overlay against an
immutable `SceneView` and builds a copy-on-change `EvaluatedFrame`. Its public
entry point is `evaluate_frame`; it borrows the existing source, parsed tracks,
and base tables and applies active color, opacity, stroke, geometry, transform,
and motion values in overlay order. Invalid animation values and stopped work
return scene diagnostics without publishing a partial frame.

Clip-owner updates use the seeded all-record handle APIs and atomic
`EvaluatedFrame::replace_clips`. The implementation preserves table identity
through clip IDs, updates parent chains with owner-before-suffix matrices,
refreshes object-bounding-box bounds, and publishes one validated replacement
after all requested shapes succeed.

Legacy raster dispatch, editor authoring, and CLI/fuzz composition remain later
owning-engine gates. Real shape/clip integration fixtures are blocked in this
crate's tests because `ShapeInput` requires `vexel_svg_path::PathDocument`
while `vexel-svg-path` is not a declared direct dependency; its manifest is
protected. Focused owned scene/evaluator tests and pure transform/clip math
checks are included. Original cases and remaining proof gates are recorded in
`artifacts/WO-RUST-SVG-ANIMATION-FRAME-01/`.

Focused commands:

```sh
cargo +stable check -p vexel-svg-animation-frame --locked --offline
cargo +stable test -p vexel-svg-animation-frame --locked --offline
```
