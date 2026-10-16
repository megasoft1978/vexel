# WO-RUST-V1-RENDER-DASH-01 result

Dashed strokes render in `vexel-render` through `render_dashed_shapes`,
`DashedShape` and `from_scene_shape_with_dashes`; `render_shapes` and
`from_scene_shape` (still `Unsupported(Dash)`) are unchanged.

Decoder: no change. `vexel-svg-decode/src/style.rs` already parses
`stroke-dasharray`/`stroke-dashoffset`; no failing test showed a dropped attribute.

C constants mirrored: `SVG_SHAPE_MAX_DASH_VALUES` 16, `RASTER_STROKE_DASH_EPSILON`
1e-12, `RASTER_DASH_CACHE_MAX_STEPS` 1_000_000 (document-wide step budget),
`RASTER_DASH_CACHE_BYTE_LIMIT` 16 MiB (segments x 40 + joins x 48 + shapes x 96).
Ported: odd repeat, offset modulo, per-subpath restart, `dash_range_joins_seam`,
per-dash caps with butt caps at joins/seam.

Parity (5 fixtures, C goldens): max_delta 0, differing 0, outside_dilation 0 on all.
Tests: 70 in vexel-render (49 pre-existing unchanged + 21 new); vexel-app 203 pass.

Deviation: the legacy parser and cache reject zero dash entries (`0 6` fails with
"number at byte 201"), so the C binary cannot produce a golden for a literal
`0 6`. The project-authored `dash-dots.svg` uses `0.01 6` (round caps, negative
offset, odd array on circle and rect). True zero-length dots (round draws, butt
draws nothing) are covered by unit and render tests only; this is a Rust
extension over C. Mixed zero entries never join across a seam.

The legacy binary was prebuilt by the coordinator (no build in this run);
`legacy-build.log` records its provenance. App wiring (`vexel-app` compose and
animation) and live use are unexecuted.
