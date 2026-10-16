# WO-RUST-SVG-WIRE-01 result (candidate, uncommitted: git refused in the worker sandbox)

Compose and animation frames now route through one scene renderer
(`vexel-app/src/compose/render_scene.rs`) built on the landed `vexel-render` entry points:
`render_groups` (clips), `render_pattern_shapes`, `render_dashed_shapes`,
`filter::apply_filter_chain`; decode injects the CoreText provider and completes `NeedPattern`.
No protected crate changed.

## Lifted deferrals
mask, opacity group, clip group, pattern paint (fill and stroke, tiles via the scene renderer),
filter (group layer, C order: children, filter, mask, composite), text (macOS, `DecodeOptions.media`
gate plus `set_text_provider`), dashed strokes (compose only).

## Still deferred (each tested)
gradient paint (disclosure), image, unsupported filter primitive, `textPath` (typed errors),
crisp edges, paint order, group blend mode (typed error), clipped dashed/pattern shape
(disclosure), pattern-painted dashed shape (disclosure), dashes in animation frames.

## Deviations
- The render crate cannot filter, pattern or dash inside `render_groups`, so the group walk, layer
  compositing and mask application live in vexel-app (`layers.rs` ports C `composite_tile`/
  `apply_mask`; verified bit-exact below).
- Animation frames keep dashes deferred: `unsupported_or_invalid_inputs_are_typed_not_approximated`
  pins it and is not in AC2.
- Decoder (protected, untouched) fails with `InvalidArgument` for nested layered groups and for
  some pattern/mask geometries and orderings (e.g. pattern shape at x>=30 beside another pattern,
  group then pattern). Wire fixtures avoid these. Bare circles differ from C (pre-existing
  tessellation), so fixtures use polygons.

## Expectation changes (each deliberate)
1. `tests::filter_mask_and_text_fixtures_yield_unsupported` renamed `..._now_compose`: before Err
   Unsupported(text / opacity group x2); after Ok, no disclosures, pixel checks.
2. `tests::unsupported_error_text_names_feature_and_location`: before text fixture error; after
   inline `textPath` document, same message.
3. `tests::deferred_shapes_are_skipped...`: before 1 shape, [gradient, dashed], "2 deferred"; after
   2 shapes, [gradient], "1 deferred", dash pixels.
4. `tests_open::view_failures_...`: three fixtures replaced by inline turbulence/image/textPath.
5. `tests/walkthrough.rs` step_i: text fixture replaced by inline `textPath`.

## Evidence
Parity (9 fixtures, `parity-report.txt`): all max_delta 0, 0 differing, 0 outside dilation.
vexel-app 209 lib tests (was 191) plus integration all pass; vexel-render all pass; fmt, wo_validate,
ledger valid; no Cargo change, no unsafe, files < 800 lines.

## Not executed
Live window, Retina, MCP. `git diff --check` (git refused): replaced by a trailing-whitespace grep.
Legacy binary prebuilt, not rebuilt.
