# WO-RUST-SVG-MASK-01 result

Candidate only; independent review pending. App wiring and live use are unexecuted.

## Done
- `vexel-render`: new `group.rs`, `mask.rs`, `clip.rs`; additive `render_groups`,
  `RenderNode/Group/Mask/Clip`. `render.rs` gains `render_shapes_clipped`
  (per-sample clip chain; `render_shapes` delegates). `lib.rs` additive lines only.
- Parity vs C goldens (9 fixtures, rust/fixtures/render-mask): all max_delta=0,
  differing=0, outside_dilation=0 (byte-exact). AC3: nested opacity-1 groups equal
  `render_shapes` on every existing parity case.
- Engine hunk: none in `vexel-svg-engine/src/lib.rs` (its Mask path already delegates).
  The block was in `vexel-svg-definitions/src/planner.rs`: a `Group` + `Mask` arm in
  `plan_owned_production`, proven necessary by the failing `tests/mask_group.rs`.

## C constants mirrored
GROUP_SCRATCH_LIMIT 64 MiB, SHAPE_MASK_MAX_INSTANCES 1024, mask depth 8, group
recursion 64, 4x4 sample clip test, 1e-9 quad test, sRGB-linear luminance weights.

## Conflicts / gaps (need coordinator)
- AC4 vs AC6: pinned `vexel-svg-engine/tests/deferred_features.rs`
  `mask_referenced_fail_closed_unsupported_at_mask_element` asserts the behaviour AC4
  removes; it now fails. `vexel-app` tests `compose::tests::
  filter_mask_and_text_fixtures_yield_unsupported` and `compose::tests_open::
  view_failures_are_typed_status_text_and_keep_the_last_document` now report
  "opacity group" instead of "mask" (the deferral moves to the render features check).
  Update these pins in the integration/WIRE order.
- Zero-area mask region renders nothing (C's degenerate-quad test would accept all);
  unreachable from decode, which rejects width/height <= 0.
- Blend modes not modelled. Git was unavailable in the worker sandbox: no commit, no `git diff --check`
  (trailing-whitespace grep clean); tag commit read from the ref file.
