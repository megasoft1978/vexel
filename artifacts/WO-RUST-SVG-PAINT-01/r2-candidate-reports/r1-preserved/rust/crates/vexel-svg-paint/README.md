# vexel-svg-paint

This crate ports the PAINT partition's safe edge access, linear/radial fill
resolution, template cache, and paint sampling. `classify_fill` resolves local
paint-server references and returns patterns explicitly for the definition
planner. `resolve_fill` handles gradient href inheritance, CSS stop values,
`currentColor`, units, spread, focal coordinates, transform inversion, and
per-instance inverse/bounds. It returns owned stop data and the opaque template
identity selected from the real `DecodeSource` and viewport.

Pass a resolved result to `SceneBuilder::append_resolved_fill` or
`append_resolved_stroke` for atomic graph publication. The builder shares stops
once per template per scene and scopes publication to its rollback lifecycle.
For an adapter that already holds a borrowed template entry, the seeded
`append_template_fill` and `append_template_stroke` calls accept that entry's
local stop slice without cloning it for each painted instance. Cache-local
ranges are never treated as graph ranges.

The resolver cache uses the accepted shared `PaintCache` and
`GradientCacheEntry`, including source ownership and normalized template
context. It keeps C's 32,768-entry, 65,536-slot, 65,536-stop, 256-stop-per-
gradient, and 4,194,304-token-visit limits. Cache growth is fallible and its
actual retained capacities are admitted to the job ledger. Any caller-provided
cache capacity and the returned `ResolvedPaint` stop-vector capacity remain
owned by the caller's job ledger. Source spans never escape their retained
source owner.

`sample` validates untrusted ranges, stop order, and geometry before indexing
or writing. It preserves the C transform order, linear and radial equations,
spread rules, endpoint selection, and `lround`-equivalent byte rounding.

Focused resolver and sampler tests port pure behavior and adversarial cases.
Renderer-dependent original assertions remain named in
`artifacts/WO-RUST-SVG-PAINT-01/coverage.json`; they require later real decoder,
native provider, raster, and application composition.
