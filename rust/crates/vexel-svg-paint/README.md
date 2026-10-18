# vexel-svg-paint

The full PAINT resolver and sampler retain the frozen href/CSS/currentColor,
linear/radial coordinate, units, spread, transform, limits, error offsets and
rounding behavior. Gradient href chains may mix linear and radial kinds: stops,
units, spread and transform inherit across kinds, coordinates inherit only from
same-kind gradients, and `inkscape:collect` is accepted when its prefix is bound
to the Inkscape namespace. All original catalog symbols and algorithms remain mapped.

For normal per-instance graph composition use `resolve_cached_fill`, or consume
`CachedResolveContext` with `resolve_cached_context`. Both return `CachedPaint`
whose `stops` is the actual immutable slice in the accepted shared `PaintCache`.
The temporary source, ledger and control borrows end on return; only the cache
borrow remains. The context has independent cache and temporary-field lifetimes.
The caller can reuse its ledger while the result lives, and Rust prevents cache
mutation while its stop slice is live. `CachedPaint::append_fill` and
`append_stroke` call the actual `SceneBuilder::append_template_fill/stroke` APIs.
They publish once per template per scene and delegate rollback to the builder.
Cache-local ranges never serve as persistent graph authorization.

Cold parsing allocates one stop vector, admits its actual capacity, and moves it
into `GradientCacheEntry`. Cache metadata grows through fallible replacements,
with actual old/new overlap admitted before transfer and retained growth charged
to the same source job. The job owns these leases until its cache and ledger end.
Any pre-existing caller-provided cache capacity must already be admitted to that
same job. Cache hits do not allocate, copy stops, or admit another stop buffer.
No source bytes are copied; opaque witnesses retain the original source Arc.
The 32,768-template, 65,536-slot, 65,536-stop, 256-stop-per-gradient and
4,194,304-token-visit limits are preserved.

`CachedPaint::to_owned` explicitly copies into a separate complete shared
`ResolvedPaint` and returns `OwnedPaint` with its actual-capacity lease. Use
`as_resolved` with `append_resolved_fill/stroke`, then `release`, or transfer
both payload and lease with `into_parts`. Copy work is charged and cancellation
releases the temporary allocation. `resolve_fill` preserves the existing owned
producer signature: it deliberately converts to a complete nonempty owned
payload and retains its admission for the entire job. That compatibility route
is not the no-copy cached flow. The shared `DecodeOperations::resolve_paint`
return type requires an owned Vec and cannot carry this borrowed result or its
releasable lease. Root must adopt the public cached handoff in the real decoder,
or repair the shared operation contract. See `owned-interface-gap.md` in the
work-order artifacts; no shared source or manifest was changed.

`sample` checks untrusted stop ranges/order and geometry before indexing. It
retains the C equations, transform order, finite fallbacks and byte rounding.
Patterns classify explicitly for the real definition planner.

All 16 prior tests remain, with repeated-cache assertions strengthened to check
pointer reuse and unchanged memory statistics. New focused tests prove cold
capacity/overlap, independently reusable temporary context borrows, 40,000 cache
hits, owned compatibility and cancellation. The standalone
`tests/harness/run_graph.py` builds with locked offline Cargo artifacts and uses
the already accepted transitive animation parser only to create real graph test
fixtures. Its three tests exercise actual SceneBuilder publication, 40,000
instances/two stops, fill/stroke compatibility, source/scene separation,
rollback and every publication cancellation position. It adds no dependency.
These graph proofs do not execute the later mixed renderer or full application.
