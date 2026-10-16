# WO-RUST-SVG-FILTER-01 result (candidate)

Contract decision: the planner keeps `FinishGroup`. The Decoder's filter path
(`definition_work.rs:181`, a protected file) rejects any post with
`InvalidArgument` and publishes the group itself via `publish_group_filter`, so
the definition_finish/plan_validation edit named in the order cannot help.
Fixed in the engine adapter (filter arm of `plan_owned_definition`): the
redundant post is cleared after planning. Before the fix the referenced-filter
test failed with `InvalidArgument` (offset 134). Proper one-line fix for the
coordinator: accept `FinishGroup` at definition_work.rs:181.

C algorithms mirrored (`shape_filter.c`, f64 maths, straight RGBA8, always
linearRGB with sRGB LUTs): separable true Gaussian, radius ceil(3 sigma),
f32 intermediate, edge taps skipped; offset by llround; flood; blend
normal/multiply/screen/darken/lighten; composite over/in/out/atop/xor/lighter/
arithmetic; merge by source-over; colorMatrix matrix/saturate/hueRotate/
luminanceToAlpha; drop shadow; region clip by pixel centre (bbox or user space);
work 50M / scratch 64 MiB limits.

Parity: 29 C goldens, all bit-exact (max delta 0, 0 differing pixels).
Deviations: the existing engine test `filter_referenced_fail_closed_*` asserted
the behaviour this order lifts and was removed (superseded by
`tests/filter_decode.rs`). Three vexel-app tests expect the label "filter" and
now see "opacity group" (group machinery unwired): left for WIRE-01.

App wiring and live use are unexecuted.
