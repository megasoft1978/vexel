# Animation tests

`anim_test.c` checks source-preserving decode, explicit-time rendering for
numeric, color, opacity, transform, and motion tracks, timing boundaries,
syncbase sequencing, unsupported timing/CSS diagnostics, and the timeline
bound. At least 15 authored SVG fixtures live under `fixtures/`.
`test_animation.py` compares timing/interpolation with an independent Python
evaluator, checks timed frame rendering and bounded PPM sequences, and verifies
the fixture inventory. `test_render_reference.py` compares sampled animated
pixels with librsvg renders of static SVGs carrying the equivalent values.
`animation_fuzz` exercises XML and SMIL timing syntax with a deterministic seed;
run it under both sanitizers for the full million-case campaign.
