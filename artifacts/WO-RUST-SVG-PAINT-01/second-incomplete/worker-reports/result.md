# PAINT candidate result

**Status: blocked; the frozen order is not complete.** The seeded R2 shared
cache contract resolves the previous viewport/source cache-key blocker. This
candidate retains only checked edge-span access and safe linear/radial
sampling; it does not contain a crate-local cache or resolver substitute.

The remaining exact contract gap is publication of template stops into a scene
graph. `DecodeOperations::resolve_paint` returns `ResolvedPaint { paint,
stops }`; `ResolveContext` provides `DecodeSource`, `PaintCache`, and
`ChargeLedger`, but no `SceneBuilder`, `SceneId`, or `DraftToken`. A consumer
publishes stops through `SceneBuilder::append_stops`, which returns a scene
table range and charges every appended stop. Shared `GradientCacheEntry`
already owns template stops and a `stop_range` into that vector; cache
validation requires `stop_range == paint.stops` and validates the slice there.
Thus this field cannot also carry the graph range. The API provides no stable
template/publication identity or draft-scoped publication table for the
consumer to append once, reuse across instances, and invalidate on rollback.
Appending every returned vector would duplicate global stops per painted
instance; reusing a range across rollback can point at a later stop.

Per the frozen order, no substitute publication semantics are implemented.
Gradient parsing, inheritance, resolver/cache routines, and five assigned mixed
renderer cases remain unimplemented/unexecuted. The three focused tests prove
edge-span predicates, interpolation rounding, and invalid stop-range/order
no-write behavior. This is not an accepted work-order result.
