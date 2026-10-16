# PAINT revision 2 candidate

**Cache-copy repair implemented; independent acceptance and shared decoder
adoption remain pending.** The entire resolver/sampler scope is preserved,
including 23 symbols, 27 algorithms, original math, limits, errors, source
spans and the previous 16 meaningful test cases. Prior code and reports are
preserved in `r1-preserved/`.

`resolve_cached_fill` and `resolve_cached_context` return `CachedPaint` with
an actual immutable cache stop slice and source/definition/viewport witness.
The public context separates cache lifetime from temporary source/ledger
borrows. Cache hits allocate no stop vector. Cold parsing admits one actual
stop-vector capacity and transfers that vector into the accepted shared cache.
Metadata replacements account for retained capacity and old/new overlap.
Instance inverse and bounds still rebind independently.

`CachedPaint::append_fill/stroke` calls actual SceneBuilder borrowed-template
publication. Public graph proofs cover 40,000 fill/stroke instances sharing two
stops, source/scene separation, ordinary and draft rollback, stale publication
witnesses and every handoff cancellation position. Cached-pointer, capacity,
allocation-count, ledger, cold-budget and cancellation proofs are added.

Owned compatibility is explicit: `to_owned` copies and charges its actual
capacity/work, returning a releasable `OwnedPaint` lease. The existing
`resolve_fill` producer returns a complete nonempty owned payload with admission
retained for its job. The shared `DecodeOperations::resolve_paint` return type
requires `ResolvedPaint.stops: Vec<PaintStop>` and carries neither a cache borrow
nor a releasable lease. It cannot itself be the no-copy per-instance path. Root
must wire the real cached handoff or approve a shared operation repair; the
exact gap is in `owned-interface-gap.md`. No shared source or manifest was
changed and no empty-payload convention was introduced.

The malformed fill-span parity repair now returns Unsupported at the original fill offset; exact adversarial and valid-span evidence covers classify, borrowed cached, and owned resolve flows. Fresh locked/offline checks pass 23 crate tests and 3 graph tests; formatting and WO validation pass. Exact command output is in `verification.json`.
Root independent review, protected audit and fresh main checks precede
acceptance. Original mixed renderer/compositor, native/app/MCP, release and
demo gates remain later.
