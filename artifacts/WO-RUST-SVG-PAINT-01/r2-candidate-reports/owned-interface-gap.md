# R2 exact shared integration gap

The actual cached engine is implemented and publicly usable:
`resolve_cached_context(CachedResolveContext { shared, cache, ledger }, ...)`
returns `CachedPaint<'cache>`, borrowing only the retained shared cache stops.
`CachedPaint::append_fill/stroke` directly invokes the accepted SceneBuilder
borrowed publication methods. Neither cache hits nor these handoffs allocate a
per-instance stop vector. A standalone harness proves the real graph flow with
40,000 instances/two stops, source/scene separation, cancellation and rollback.

The frozen shared `DecodeOperations::resolve_paint` signature returns
`Result<ResolvedPaint>`. The shared payload contains `stops: Vec<PaintStop>` and
an optional template identity; it has no borrowed-stop alternative or lease
field. Shared `ResolveContext` also gives source/cache/ledger references one
context lifetime. Returning an empty Vec as a cache-hit convention would fail
the actual append_resolved validation and require an unimplemented producer
convention. This candidate does not use that convention.

`resolve_fill` remains a compatible complete owned producer. It makes its
copy explicitly through `CachedPaint::to_owned`, charges work and actual Vec
capacity, and retains that admission for the job lifetime required by the
owned-only return signature. For independently managed payload lifetimes,
`OwnedPaint::as_resolved`, `release` and `into_parts` expose the actual owned
payload and releasable lease. Both compatible append_resolved publication and
cached append_template publication are exercised with the real SceneBuilder.

Consequently the owned-only trait itself cannot deliver the required no-copy
per-instance path. Root must wire the decoder to the public cached handoff or
approve a shared operation that can publish a borrowed template directly (or
return a lifetime-correct borrowed payload). This is a specific shared adoption
gate, not a missing parser/resolver or a blanket Unsupported engine. Shared
interfaces/manifests, pending raster contracts, old sources/oracles, root state
and all prior candidate snapshots remain protected.
