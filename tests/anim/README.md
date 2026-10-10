# Animation tests

`anim_test.c` checks source-preserving decode, explicit-time rendering for
numeric, color, opacity, transform, and motion tracks, timing boundaries,
syncbase sequencing, unsupported timing/CSS diagnostics, and the timeline
bound. At least 15 authored SVG fixtures live under `fixtures/`.
`test_animation.py` compares timing/interpolation with an independent Python
evaluator, checks timed frame rendering and bounded PPM sequences, and verifies
the fixture inventory. `test_render_reference.py` compares sampled animated
pixels with librsvg renders of static SVGs carrying the equivalent values.
`animation_fuzz` exercises XML and SMIL timing syntax in direct, nested group,
`<use>`, `<symbol>`, and nested viewport trees. Successful parses are decoded
and sampled, and the borrowed source bytes are checked for mutation. Run it
under the combined ASan/UBSan build and the separate UBSan build for the seeded
million-case campaign.
