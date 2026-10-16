# Continuation 4 — remaining API proof

## Custom-element clip and pattern publication

`rust/crates/vexel-svg-decode/src/publication.rs:249-310` exports both custom
helpers with `ResolveContext`, `SceneBuilder`, and `SceneId`, but neither takes
the active `DraftToken` or request token. Both parse `ResolvedStyle` and then
call the unscoped geometry publisher. `ResolvedStyle.clip` is populated in
`src/style.rs:331`, but these helper bodies do not read it. `append_geometry`
returns only `ShapeId`, so it cannot return a suspended static-pattern request.

The actual traversal is `Task` in `src/tasks.rs:31-67`; it has no custom
geometry task or completion event. Scoped pattern suspension is owned by
`ScopedProductionDecodeSession` and `ScopedDecodeEvent`, while mapped resume
completes against the decoder's private pending definition/request state
(`src/session.rs:152-185`). Consequently, directly adding a shape-level clip
mutation to the helper cannot resolve `url(#id)`, preserve the parent request,
or route a custom pattern through the real suspension path. Doing so would
require a decoder-owned custom publication entry/event that receives the
active draft/request and typed pattern/clip plan. Current Scene guarded
finalizers exist, but the decoder custom path does not expose that contract.

The original C helper resolves fill and stroke pattern servers and clip paths
before returning (`src/svg/shape/shape_custom.c:90-199`). The accepted Rust
helper is therefore still incomplete; this report does not mark the catalog
entry or custom behavior covered.

## Exact frozen cache/counter predicates

The frozen predicates remain visible and unchanged:

- `tests/svg/shape/test_reference.c:707-735` requires exactly 512 shape
  instances and `reference_lookup_probes >= 512 && <= 1024`.
- `test_reference.c:740-787` retains the 4,096-comment/512-shape input,
  two-stop output, and `paint_token_visits >= 4096 && <=
  8 * (4096 + 512 + 64)`.
- `test_reference.c:114-240` and `:200-365` contain exact clip span,
  aggregate span, scratch-size, and resolved-once allocation history checks.
- `tests/raster/cache/cache_contract.c:17-70` exercises ownership on success
  and failure, deterministic LRU, clock rollover, generation/scale invalidation,
  the byte/entry caps, and 10,000 insertions.

The accepted lower APIs do not expose the required decoder-path evidence:
`vexel-scene/src/source_context.rs:182-200` provides `find_reference` and
storage bytes, but no decoder-owned lookup probe count; the reference crate's
`lookup_probes` accessor belongs to a separately constructed index. The paint
crate is registration-only (`rust/crates/vexel-svg-paint/src/lib.rs:1-3`), so
there is no production paint-token scan or counter to exercise. Decoder
`Cargo.toml:9-20` does not depend on `vexel-tile-cache`; changing it or the
workspace lock is protected. The decoder's capacity journal measures its own
leases and is not the frozen aggregate allocation-history counter. No local
test substitute is presented as proof of those original predicates.

## Seed evidence

The named `artifacts/WO-RUST-SVG-DECODE-01/accepted-child-requests-source-seed.json`
is absent from this worker (`find . -name` returned no match). The five likely
Scene capability files' current SHA-256 values are recorded in
`edits-and-hashes.json`; no Scene file was edited. The expected prior-worker
hashes from the missing seed could not be compared, so seed hash identity is
not claimed.
