# Publication API and ownership

The publication gap is implemented in the current scene source. This is an
isolated candidate, not root acceptance or a claim that PAINT/DECODE engines or
the application are complete.

| API | Actual contract |
| --- | --- |
| `DecodeSource::gradient_template_id(ElementId, [f 64;2], control)` | Validate canonical gradient element/source spans, finite positive viewport; retain source Arc in an opaque witness. |
| `GradientTemplateId::same_template` | Compare exact retained source allocation, definition token and viewport. |
| `ResolvedPaint::template` | Explicit resolver-selected optional witness; required for Linear/Radial, absent for other kinds. |
| `SceneBuilder::append_resolved_fill/stroke(scene, &ResolvedPaint, control)` | Validate local payload; publish stops plus instance paint atomically. |
| `append_template_fill/stroke(scene, &identity, Paint, &[PaintStop], control)` | Borrow real cached template stops without per-instance owned stop-vector duplication. |
| `publish_template_stops(scene, &identity, &[PaintStop], control)` | Publish once or compare/reuse existing contents; return opaque insertion witness. |
| `published_stop_range(scene, &PublishedStops, control)` | Revalidate builder/scene/insertion before exposing current graph range. |

`PaintCache` and `GradientCacheEntry` fields remain exactly unchanged, including
literal compatibility and cache-local `stop_range == paint.stops`. A complete
PAINT resolver constructs the identity from its actual selected definition and
actual viewport and returns it in `ResolvedPaint`. DECODE needs no URL resolution
or parser substitute. Existing ResolvedPaint literal callers must explicitly set
`template`; none of the original 66 scene tests constructs that literal. Existing
trait signatures, finished-frame and checked-handle contracts are unchanged.

The resolver can borrow an entry's local stops for the template handoff. It must
continue to distinguish template coordinates from per-instance inverse/bounds,
compute those with the real edges, and propagate errors before a successful
handoff. The publication layer supplies no parser/provider/empty decode engine.

# Identity, transactions and ownership

The retained Arc blocks mutation of the original allocation; Arc::make_mut creates
a different source. Per-scene records store only the definition/viewport key,
insertion serial, append order and graph range: source is implicit in the owning
builder. Stops are copied once into its owned graph vector. No cache entry owns
or authorizes a graph range. The owning scene retains records and spare capacity
through freeze, with source bytes counted only through existing animation/source
ownership. External borrowed cache/ResolvedPaint capacity remains job-ledger
owned; the scene never claims to count that caller-owned allocation twice.

Sorted Vec lookup is worst-case O(log n), with no hostile-key hash collision
behavior or infallible map allocation. New insertion is O(n) bounded movement.
Actual transfer, suffix shift and selected stop bytes are precharged in at-most 256
byte batches; lookup/comparison loops poll control. All reserve and post-reserve
checkpoints precede logical publication. Atomic handoff failure cleans its suffix
and preserves earlier publication witnesses. Capacity/history may remain after
failure. Cleanup after cancellation is unconditional, as in accepted draft APIs.
Plain TableRange snapshots are not persistent rollback authorization; retain a
PublishedStops witness or call atomic handoff each time. Reused slots mint fresh
serials; a stale witness cannot retrieve a replacement range.

BuildMark records publication count separately from table lengths. Sorted records
have append ordinals, allowing ordinary rollback to retain precisely the prefix
even for empty stop publications. Independent children survive ordinary rollback.
Nested LIFO draft rollback first cleans its scene and then drops every newly
created descendant (including committed children), releasing nested metadata
capacity. A child retry requires a new active child scope while the parent is
active. Existing parent guards, monotonic identity, ignored notices and older
stable graph prefixes remain intact.

# Source/parity map

| Reference source | Current implementation or remaining owner |
| --- | --- |
| `src/svg/paint/paint.c:53`, `:87`, `:129` template cache key/find/insert | Source-coherent opaque identity; scene-owned sorted publication lookup. Key retains actual definition and viewport, not instance inverse/bounds. |
| `src/svg/paint/paint.c:650` single template stop admission | One graph copy per template/scene, with 65536 stop and 32768 gradient bounds unchanged. |
| `src/svg/paint/paint.c:681` instance result | Instance Paint payload preserved while only local stop range becomes graph range. |
| `src/svg/paint/paint.c:153`–`:531` private attribute/source/chain/stop/coordinate helpers | Unmodified; actual algorithms remain PAINT-owned. Source spans are validated at identity construction through accepted canonical ElementView. |
| `src/svg/paint/paint.c:691`–`:711` empty edges/bounds errors; gradient transform inverse | Still required in actual PAINT resolver; no geometry/resolver replacement or false parity claim. |
| Accepted R2 cache/finished-frame/handle consumers | Original 66 tests and their assertions byte-identical; all pass. |

Current changed source: `paint_publication.rs` (witnesses, keyed publication and
handoff), `paint_publication_tests.rs` (private memory/fallible-reserve proofs),
`builder.rs` (mark/rollback metadata), `model.rs` (owned metadata capacity),
`decode_contract.rs` (returned identity), `validate.rs` (publication validation),
`lib.rs` (exports). New public proofs are in `tests/paint_publication.rs`. Every
scene source and test file remains below 800 lines. No CSS marker adoption,
manifest/dependency/platform/peer changes or external code were introduced.

# Evidence and limits

Final exact command output, return codes, hashes and scope audit are recorded in
verification.json and adjacent logs. Tests cover 40,000 instances sharing 2 stops;
32768 templates/65536 stops with reuse still admitted; source/viewport/definition/
scene separation; conflicts; reverse insertion and bounded lookup; stale slot
reuse; ordinary/nested rollback; every charged cancellation position; checkpoint
cancellation after publication; invalid local ranges/stops/identity; byte/work
limits; actual spare capacity/history/old-new overlap; source-once and freeze.

The deterministic allocation failure proof calls the real Vec fallible reserve
boundary with impossible capacity (CapacityOverflow mapped to OutOfMemory). It
does not induce process-wide allocator exhaustion or claim fault injection into
every allocator call. Budget rejection after one actual metadata reserve proves
retained unused capacity and logical cleanup. Full allocation hooks, performance,
complete parser/resolver parity and product release gates are not claimed here.

The seed catalog contains 246 files and no C/source603 inventory. Its protected
entries are rehashed without pretending that catalog covers all C files. The
ledger and worktree index differ from seed fingerprints; neither was written by
this worker. Root must resolve/audit these shared integration inputs. No stage,
commit/reset/delete/agent/GUI/network/install/billing operation was performed.
Root independent Luna review and fresh main checks remain external acceptance
gates before consumers resume. Full app/tested release/live MCP recording remain
incomplete.
