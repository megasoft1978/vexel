# PAINT candidate result

**Status: complete candidate, pending root review.** The crate now implements
the full frozen PAINT scope: checked edge spans; linear/radial local references
and same-kind href inheritance; ancestor CSS/currentColor; stop defaults,
ordering, opacity, and limits; units, focal/radius defaults, spread and
transform inversion; source-owned viewport template caching; per-instance
bounds/inverse rebinding; and safe exact sampler math. Pattern URLs classify
explicitly for the definition planner; `resolve_fill` returns an error for a
pattern instead of reporting false success.

The implementation uses the seeded shared `PaintCache`, `GradientCacheEntry`,
`GradientTemplateId`, and `ResolvedPaint` contracts. Cache-local stop ranges
remain local. Resolved templates carry the opaque source/definition/viewport
witness used by the accepted SceneBuilder atomic append APIs, which share stops
once per template per scene and invalidate publication witnesses on rollback.
No crate-local replacement cache or fake provider was added.

All 23 catalog symbols and 27 algorithms are mapped in `parity-map.json`.
Sixteen focused tests pass, including exact frozen sampler pixels, CSS/href
resolution, 4,096 comments/512 repeated fills, cancellation/source offsets,
256-stop boundaries, and aggregate 65,536-stop admission. Original C/Python
oracle files remain preserved. Resolver-pure assertions are ported; original
mixed raster/compositing and full-tile render predicates remain integration
gates for the actual DECODE/native-provider/RENDER composition. App, MCP,
release, and demo gates remain incomplete.

Scoped locked/offline checks, tests, formatting, and work-order validation pass.
`scope.json` hash is unchanged. This is a worker candidate only; independent
root review, protected-path audit, and fresh main focused checks are still
required before acceptance.
