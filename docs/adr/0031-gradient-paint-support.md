# ADR-0030: gradient paint support (linear and radial)

- Status: Proposed (owner approved the scope amendment; ADR awaiting merge)
- Date: 2026-10-18
- Decision owner: project owner
- Amends: ADR-0029 (gradient exclusion only)

## Context

ADR-0029 limits the V1 SVG/Inkscape subset to solid fills and strokes, so
gradient paint is reported as unsupported. Real Inkscape drawings, including
the target character workflow, commonly use linear and radial gradients, and
Inkscape emits them as a stop-carrying gradient referenced by a second
gradient through `href`, annotated with `inkscape:collect`. Rejecting them
makes ordinary Inkscape files fail disclosure for a very common feature.

Verified current state:

- `vexel-svg-paint` already parses linear and radial gradients (coordinates,
  `gradientUnits`, `spreadMethod`, `gradientTransform`, same-kind `href`
  chains, stop sharing through `PaintCache`) and already ships
  `sample(paint, stops, x, y, rgba)` in `src/paint.rs`. `vexel-scene::Paint`
  already carries kind, spread, units, coordinates, inverses and stops.
- Two resolver rules reject real Inkscape output: `attributes_supported`
  (`src/resolver.rs`) has no entry for `inkscape:collect`, and `collect_chain`
  rejects any `href` chain that crosses linearGradient/radialGradient.
- `vexel-render` (`src/adapter.rs`, `classify` and the `paint_server` stroke
  check) returns `Unsupported(GradientPaint)` for `Linear`/`Radial` and does
  not call the sampler.
- Tests pin the rejection in `vexel-render/tests/errors.rs` and
  `vexel-render/tests/pattern_units.rs`.
- Unknown, to be verified by slice (c): whether production
  `vexel-svg-decode` publishes gradient paints end to end. Only a test
  support implementation (`tests/support/production.rs`) was seen.

## Decision

V1 supports linear and radial gradient paint on fills and strokes:

- `stop` children with offset, `stop-color`, `stop-opacity` (existing parse).
- `gradientUnits` both `userSpaceOnUse` and `objectBoundingBox`.
- `spreadMethod`: `pad` is required. `reflect` and `repeat` are included
  because the existing sampler and `Spread` enum already implement them; no
  new code is added for them beyond tests.
- `gradientTransform`.
- `href` / `xlink:href` inheritance of stops and attributes. A gradient with
  no stops of its own takes the stops of the nearest ancestor in the chain.
  Cross-kind chains are allowed per the SVG rule: only the attributes common
  to both kinds (`gradientUnits`, `spreadMethod`, `gradientTransform`) and
  stops are inherited across kinds; kind-specific geometry (`x1..y2`,
  `cx cy r fx fy`) is inherited only from a same-kind ancestor.
- `inkscape:collect` (bound `inkscape` namespace) is accepted and ignored on
  gradients. It is preserved in untouched source like any other unknown
  attribute. Other unknown prefixed attributes remain rejected.

The renderer reuses `vexel_svg_paint::sample`; it does not get a second
gradient implementation. Pixel results must match the frozen sampler.

## Non-goals

- Mesh gradients, gradient meshes, hatches, and `<pattern>` paint (patterns
  stay unsupported).
- SVG2 and CSS gradient features: `<meshgradient>`, `fr` focal radius,
  `gradientTransform` via CSS `transform`, CSS `linear-gradient()` values,
  `color-interpolation`, non-sRGB interpolation.
- External gradient references (`url(https://...)`), unchanged.
- Dithering or higher-precision gradient output.
- Any change to the limits (256 stops per gradient, 65,536 stops, 32,768
  templates, 4,194,304 token visits) or to error offsets.

## Frozen-oracle changes

`vexel-svg-paint/tests/resolver_parity.rs` does not currently assert either
resolver rejection (verified by search: it has no cross-kind `href` and no
`inkscape:` case), so no existing assertion in that file changes. The new
behaviour is added as new tests. All other `Unsupported` assertions there
(malformed span, href cycle, bad stop order, missing reference, unbound
`xlink:href`, external reference, singular `gradientTransform`, stop limits)
are unchanged and must keep passing byte for byte.

Assertions that change, each needing owner sign-off recorded by approving
this ADR:

1. `vexel-render/tests/errors.rs`, the table entries
   `(PaintKind::Linear, GradientPaint)` and `(PaintKind::Radial, GradientPaint)`
   (about lines 63 and 64). Reason: these kinds become supported. Replacement:
   `linear_and_radial_paints_convert_to_render_shapes`, asserting `Ok` and the
   carried paint kind. `PaintKind::Pattern` and `Image` entries stay.
2. `vexel-render/tests/errors.rs` (about line 93): `paint_server` stroke with
   no `stroke_paint` still returns `Unsupported(GradientPaint)`. Meaning is
   kept (a paint server flag with no resolved paint is a decode defect, not a
   supported gradient). Expected: unchanged. If the slice finds it must
   change, stop and escalate; do not edit silently.
3. `vexel-render/tests/pattern_units.rs` (about lines 536-537): a gradient
   `Paint` converted by `convert` is no longer `Unsupported`. Replacement:
   `gradient_paint_converts_and_samples_expected_colour`, asserting `Ok` and a
   sampled colour at a known point.

Not an assertion change but a doc change: `vexel-svg-paint/README.md` keeps
the frozen-behaviour statement and gains one sentence noting two accepted
extensions (cross-kind `href` and ignored `inkscape:collect`).

## Consequences

- Inkscape files that use gradients import and render instead of being
  disclosed as unsupported. Solid-only behaviour is unchanged.
- `vexel-render` gains a dependency on `vexel-svg-paint` (manifest change;
  coordinator only, per project rule) or the sampler moves to a shared crate.
  Decision: add the dependency; moving code is out of scope.
- The frozen resolver oracle is deliberately relaxed in two places; every
  other frozen outcome is preserved and guarded by the existing tests.
- Per-pixel gradient sampling costs more than solid fill. No performance
  target is claimed; the full-product performance gates remain deferred.
- Gradient edits through the editor (creating or editing stops) are not part
  of this decision; only import, preservation, and rendering are.

## Relationship to ADR-0029

This ADR amends ADR-0029 only where it excludes gradients: "solid fills and
strokes" in the SVG subset becomes "solid and linear/radial gradient fills
and strokes". All other ADR-0029 text, the cutline, deferrals, and the
estimate are unchanged. The estimate is not re-derived here; gradient work is
expected to be small because the resolver and sampler already exist.
