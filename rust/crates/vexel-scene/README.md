# Owned scene data

`vexel-scene` owns the renderer's immutable input. It contains data and checked
construction, not an SVG decoder, renderer, native media provider or desktop app.

- Source and parsed animation share the same `Arc<Vec<u 8>>`. Equal bytes held by
  another owner are rejected; there is no source copy or self-reference.
- `SceneBuilder::new` creates a root; `new_scene` creates independent children.
  `seal` validates a child before a mask can link it. Sealed scenes cannot be
  edited, so mask links follow seal order and cannot form ownership cycles.
- `append_shape` moves parser output and exact per-edge colors through one
  transaction, preserving source spans, implicit command markers, subpaths and
  stroke flags. Related tables are reserved before publication; cancellation
  while copying rolls all logical table lengths back. In-flight input buffers
  are included in the transient memory budget.
- Typed handles contain private graph/scene/slot/insertion identity. Rollback
  does not reuse insertion identity. A handle removed by rollback cannot access
  a later replacement in the same slot. Paint/image references are scene-local
  sparse indices checked by the builder and on freeze.
- Marks cover all tables/resources in one scene. They do not rewind independent
  child scenes or global ignored notices. Rollback invalidates that scene's
  earlier marks and preserves earlier handles. Work and allocation history are
  never rewound. Cleanup completes even after cancellation.
- `freeze_controlled` validates every retained scene before publishing a
  `SceneGraph`. Views borrow all tables, source and animation. Child lookup
  checks graph identity and sealing; sparse paint/dash access checks the shape.
  `SceneGraph` is immutable, Rust-owned, `Send + Sync` data.

Memory statistics count **actual Vec capacities**, including spare source bytes
once through `OwnedAnimation::storage_bytes`, scene/table storage, insertion
serials, padded image buffers, clip buffers and pattern tiles. They exclude
allocator metadata, Arc bookkeeping and RSS. The budget includes old/new buffer
overlap during growth; geometric growth falls back to required capacity when
that fits the explicit caller budget. The history records builder reserves and
owned payload admission, not the unknown allocation history of providers or
parsers. Logical rollback retains table capacity and releases removed payloads.

Native allocation and bulk Vec transfer/drop remain indivisible operations.
Controlled validation/copy loops poll per entry; this model does not promise a
main-thread latency bound. Desktop scheduling and performance measurements are
later integration gates. No-stop convenience `freeze` runs the same validator.

Reference input caps are retained, and larger overrides are clamped. The caller
can select smaller table caps and a retained/transient byte budget. Defaults do
not add a smaller byte-policy envelope. Token identity on a `Pattern` is checked
by the future decoder against its retained XML stream; this graph does not own
that stream. The decoder must deduplicate ignored-token identities before
recording aggregate notices. Text request/outline validation and the real media
provider interface are defined; native implementations remain required later.

Focused checks: `cargo +stable check -p vexel-scene --locked --offline` and
`cargo +stable test -p vexel-scene --locked --offline`. This is model evidence,
not complete rendering parity or application acceptance.

## Shared decoder and scalar contracts

The remaining shared data lives in `decode_contract`, `render_contract`, `frame`,
`accounting`, `pattern_ticket`, `source_context` and their validators. Concrete
SVG traversal/definition/media/paint/scalar/effect/animation engines and native
providers remain mandatory separate full ports; trait declarations are not those
implementations. The concrete decoder type belongs to its full engine owner and
implements `DecodeSession`; no unfinished `Decoder` is registered as a backend.

Adaptations from the signature proposal:

- `LexicalDocument` owns one source Arc and the token Vec moved from the accepted
  XML stream. `DecodeSource` borrows this external owner and privately creates
  the actual accepted CSS/reference engines. `ElementView` source/token fields
  are private and obtained only from that canonical owner; media consumers use
  its immutable accessors. This prevents pairing another document's tokens and
  source without copying or reparsing them. XML parsing is controlled one-shot
  over the accepted resumable engine, with per-poll work/cancellation accounting;
  source-read charges here count stream polls, not a claimed byte-I/O metric.
- `ResolveContext` and `DefinitionContext` borrow canonical query state instead
  of replaceable raw CSS/reference fields. Source lifetime is separate from
  temporary context/control/cache/provider lifetimes. Returned plans retain only
  source borrows. Context validation checks job/builder source owner identity.
- `FlatKernel` explicitly receives a painter range and the same mutable job
  ledger. Nested grouped/mask/filter work shares one reservation policy instead
  of constructing independent budgets. `KernelResources` variable-length clip/
  stroke operations are fallible and receive WorkControl so preflight and actual
  traversal can report stop/overflow without silent false samples.
- `RenderMemoryStats` and `RenderWorkStats` avoid colliding with accepted scene
  and base stats. `ChargeLedger` is explicit admission over caller-owned buffers;
  `admit_vec` measures actual capacity, aggregate admission must supply actual
  nested capacity, and leases are explicitly released with resource lifetime.
  Its registry capacity is counted too. It is not an allocator hook or RSS
  measurement. Opaque leases reject wrong, stale and duplicate release.
- Evaluated frames copy tables on change. Effective Shape records own changed
  strokes/inverses/clip references so consumers see changes through the same
  SceneView, while unchanged metadata/media/children/source remain borrowed.
  Clip copies include their nested buffers and keep the 64MiB reference bound.
  Frame stats count additional owned buffers, not borrowed base/source again.
- Stroke-grid references retain the original high-bit join tag and **global**
  segment/join indices; range/cell/reference validation polls through WorkControl.
  Pattern tickets bind one outstanding continuation, dimensions and sealed child
  identity; invalid/stale/wrong-owner completions cannot advance it. Engine errors
  still abort the job rather than silently rendering a fallback.

## Decoder draft finalization

`begin_draft(scene)` produces an opaque value token. It borrows no builder, so a
state machine can own both builder and token. Drafts form a bounded 64-level LIFO
stack; a child scope may only target a scene created within its active parent.
Parent mutation waits until the child closes. Only records created after the
scope checkpoint can be finalized; existing source/range/parent structure stays
fixed while deferred styles/clip/filter/mask/group details are attached.

Ordinary marks/rollback are unavailable during a draft. Internal append
checkpoints still work and do not invalidate draft authorization after a canceled
append. `commit_draft` validates actual data and retains the token on failure for
retry/rollback. Successful commit makes that token stale. `rollback_draft` removes
scope-created tables/media/descendants and restores global ignored-notice counts
and first offsets. Notices belong to the innermost active scope: child rollback
removes its notices, child commit retains them in the parent, and parent rollback
also removes committed child notices. Capacities/history and monotonic identity
are retained honestly. Seal/freeze reject pending scopes. A lost token requires
discarding the unfinished builder; it cannot publish a partial graph. The token
has no automatic destructor rollback and never owns foreign objects.

Finalizers validate and observe cancellation before publication. Color/filter
bulk copies and Vec cleanup are bounded indivisible operations; no owner-thread
latency claim follows. Nested replacement payloads charge actual spare capacities
and transient old/new overlap. Root scope rollback preserves all earlier records.

## Complete consumer identity, cache and finished clip interfaces

Immutable views provide checked `*_id(index)` enumeration for every serial table,
including zero-edge shapes; IDs preserve owner/scene/insertion identity through
frame overlays. `SceneGraph::scene_id` enumerates existing graph identities.

Gradient entries now retain the immutable source witness, viewport/inverse/bounds
context and owned stops. Original three-field `PaintCache` construction remains
compatible. `ResolveContext::validate_cache` checks canonical source identity,
context, stop/slot ranges and actual in-loop cancellation. Cache storage counts
nested stop capacities without counting source bytes again; production jobs use
controlled measurement. Compatibility measurement is a bounded indivisible
wrapper. The PAINT owner must preserve original template-sharing/global stop
semantics, not duplicate stops for every painted instance; these APIs do not
claim an implemented gradient resolver or accept its future parity yet.

Finished frames support complete `replace_clips` transactions. All clip values,
parent structure and actual old/new capacity overlap validate before publication;
the configured frame-owned budget survives `finish`. Invalid/canceled/over-budget
replacement preserves effective tables, base data and allocation statistics.
Actual clip-owner propagation and animation composition remain ANIMATION-owned.

## Template stop publication

`DecodeSource::gradient_template_id(definition, viewport, control)` creates an
opaque witness from the actual canonical gradient token and positive finite
viewport. It retains the immutable source allocation; equal bytes in another
allocation are a different source. Definition/viewport determine the template;
instance bounds/inverses do not. There is no public unchecked identity constructor.

The complete PAINT resolver now sets `ResolvedPaint::template` to this witness
for linear/radial results and to `None` for other kinds. `paint.stops` remains a
range into the returned local `stops` vector. DECODE passes that result directly
to `SceneBuilder::append_resolved_fill` or `append_resolved_stroke`. These validate
and atomically publish graph stops plus the instance paint. They reject missing,
wrong-kind, wrong-source identities, invalid local ranges and conflicting payloads.
Existing `PaintCache` and `GradientCacheEntry` literal fields are unchanged.

Resolvers that borrow their cache use `append_template_fill`/`stroke` with the
witness, instance paint and borrowed local cache stop slice. This avoids cloning
cache stops per instance. Lower-level `publish_template_stops` returns opaque
`PublishedStops`; `published_stop_range` rechecks scene/insertion identity. A plain
returned `TableRange` is a snapshot, not authorization across rollback: retain
and revalidate the witness, or use the atomic handoff. Never store graph ranges
in `GradientCacheEntry::stop_range` or its local `paint.stops`.

Each scene owns a sorted publication Vec keyed by definition/viewport, implicitly
bound to its builder source. Lookup is worst-case logarithmic; insertion moves a
bounded suffix, charged in advance. The global reference bounds remain 32768
publications,65536 scene stops and 256 local stops. A cache hit compares stop
contents and adds no graph stops. Unrelated ordinary stop appends do not alter
existing ranges. Marks save publication order independently of stop count;
rollback removes its publication suffix, including empty publications, while
preserving earlier witnesses and independent scenes. Draft rollback also drops
all newly created descendants and their metadata; retry mints fresh insertion
identity. Active parent/child draft guards remain mandatory.

Publication metadata's actual spare capacity participates in scene capacity,
old/new reserve overlap, history, rollback and freeze measurement. Source is
still counted once. Borrowed resolver/cache inputs remain owned and accounted by
the caller's job ledger; this API copies only the selected stops once per
scene/template. Stop visits and comparisons charge `PaintVisit`, binary lookup
charges `ReferenceProbe`, and bounded 256-byte batches of reserve transfer/copy/
insertion work charge `SourceRead`. Cancellation is checked before publication.
Native Vec reserve/transfer/insert/drop are indivisible; no latency benchmark is
claimed. Fallible reserve failures may retain unused capacity/history, never
partial logical records. Seal/commit/freeze also validate publication metadata.

Parsing gradient attributes/stops/inheritance and validating singular transforms,
empty geometry and object-bounding-box instances remain the complete PAINT
resolver's responsibility. Publication does not replace those algorithms or
turn their errors into successful paints. Root independent review and fresh main
checks precede consumer resumption; application/release/live MCP gates remain open.

## Referenced media instance source mapping

Text/image decode requests carry an explicit SourceMap separately from canonical
referenced ElementView tokens. A use-host annotation preserves its own source
span and instance transform/transform-node identity; token/provider input still
comes from the referenced media. validate_source_map checks canonical source/job
ledger identity, writable target scope, finite transforms, bounded host span and
live same-scene transform handles/cancellation before provider or append work.
The annotation transform need not equal the geometry transform. Actual MEDIA/
DECODE adapters must pass these mappings and use them at append time; this API
is not a media/native-provider or renderer completion claim.

## Owning raster jobs and controlled lower engines

`RasterJob::new(ChargeLedger)` moves the coordinator's source/job ledger into the
allocation owner. It rejects outstanding external leases; `JobStartError` returns
the unchanged ledger so the caller can release those resources and retry. There
is one source charge, one ledger, and no borrowed ledger, self-reference, Rc,
Box payload, global allocator hook or source copy. Multiple child scratch sets,
nested stroke grids and output buffers coexist in this job.

`JobBuffer<T>` is a copyable opaque locator, not an allocation owner. The sealed
`JobElement` set contains only allocation-free Copy scalar records. Buffer data,
capacity, leases and both metadata registries remain private. `slice_mut` exposes
only existing elements. `reserve`, `extend` and `shrink_to` check arithmetic,
engine element caps, local pool caps, registry capacity and old/new overlap before
allocation. An active ledger entry reserves requested bytes before the real
`Vec::try_reserve_exact`. The remaining ledger budget is held exclusively until
Vec-reported actual capacity is validated. Excess reported capacity is checked
against both the global budget and the buffer's actual element cap before copy
or publication, and is rejected/dropped if it does not fit. This follows the
existing safe-std builder reserve pattern; it cannot constrain malloc/RSS or
predict allocator excess capacity before allocation returns.

Growth and physical shrink copy into a separately admitted replacement, polling
the caller's actual `WorkControl` in the copy loop and at publication. Failed
allocation or cancellation preserves old contents, capacity and lease. A failed
in-capacity append removes its unpublished suffix. Allocation history and peak
record successful physical reserves even when later canceled; failed native
reserves do not invent payload allocations. Registry growth may retain unused
capacity/history on failure. Native allocate/drop and metadata Vec transfer are
indivisible operations; this is not a latency guarantee.

`truncate` changes logical length and retains capacity. `clear_capacity` releases
storage while preserving the token and empty lease. `release` destroys storage
and invalidates every token copy. Wrong-job, wrong-source, stale and double
release fail without mutation. Dropping a descriptor does **not** release its
buffers: the job still owns them. `release_all` reclaims even discarded handles;
`finish` destroys payloads and both registries and returns final history with zero
job-live bytes. Ordinary job Drop also destroys everything without allocation.
A source Arc retained elsewhere can outlive the ended job charge. Nested grids
are independent job resources; releasing a scratch descriptor does not recursively
release shared grids. Release them explicitly, call `release_all`, or finish/drop
the job. `JobRasterScratch::release` prevalidates all tokens and uniqueness before
any release. `validate` additionally checks roles and nested grid validity.

`JobRasterScratch` maps every original scratch Vec to a typed buffer, including
bounds, boundary bits, AVL events/nodes/roots, row offsets/active indices, fill
crossings/intervals/runs, local edges/seams, dash records, samples and stroke cost.
Allocation-free scalar cursors/counts/flags remain engine state. `JobDashRange`
replaces nested grid Vecs with checked handles; `JobDashView::grid` returns the
original `GridCell`/`u32` slices. Segment/join types, global high-bit join encoding,
coordinates, five interval slots and scalar 4x4 sampling data need no math change.
The engine passes its original derived element bounds to `buffer`/`reserve`.
Scalar scratch adds no smaller global default. Dash+grid share the original
16 MiB retained limit; grid capacities retain 65536 cells/1000000 references,
validation retains 128 axes and 1000000 build steps, RasterJob::charge_boundary retains the
200M marking-work limit without inventing a boundary byte cap, group scratch retains 64 MiB, and output retains Tile's 4096 axes
and 64 MiB capacity with exact packed RGBA layout. The job budget additionally
admits transient overlap and registry capacity; metadata/Arc/allocator/RSS costs
outside Vec-reported storage are not claimed as measured.

`ControlledKernelResources` is a separate complete paint/clip/stroke capability.
Every method returns Result and receives the actual mutable WorkControl; paint
has no default, no legacy conversion, no stop-limit override, and no Unsupported
placeholder. The coordinator's eventual real adapter implements this trait.
`ControlledSampleInput` preserves the original sample data fields with a borrowed
job dash view. Its paint call passes control directly into the provider loop.
`ControlledSampleKernel`, `ControlledIndexKernel` and `ControlledFlatKernel` are
typed lower entrypoints. Controlled output remains a `JobRenderOutput` handle;
returning an unleased Tile Vec is not hidden in this interface. Legacy
`KernelResources`, `SampleInput`, `FlatKernel`, scratch structs and literals remain
usable for the original fixtures and cannot implicitly become controlled engines.

The new `raster_job_contracts` integration fixture demonstrates real row-buffer
writes, same-job child scratch/output, nested grids, controlled paint interruption
inside a 17-stop loop, and original 4x4 coordinates/rounding. Its explicit test
provider is not registered as a production backend. Existing scalar engine
crates in this tree remain registration-only; their full algorithms, coordinator
adapters, full oracle rendering and native application are later scopes.

Example allocation sequence (the integration fixture exercises this publicly):

```rust,ignore
let ledger = ChargeLedger::new(graph.source_owner(), byte_budget, max_leases)?;
let mut job = RasterJob::new(ledger).map_err(|e| e.diagnostic)?;
let scratch = JobRasterScratch::new(&mut job, control)?;
job.reserve(scratch.active_edges, edge_count, control)?;
job.extend(scratch.active_edges, &[edge_index], control)?;
let child = JobRasterScratch::new(&mut job, control)?;
let pixels = job.buffer::<u8>(RasterPool::Output, packed_bytes, control)?;
job.reserve(pixels, packed_bytes, control)?;
// Index/sample providers read job.slice(handle); variable loops poll control.
child.release(&mut job)?;
scratch.release(&mut job)?;
job.release(pixels)?;
let final_history = job.finish()?;
```

## Decoder publication repair (R2)

`begin_request(&DraftToken, control)` opens **one** request in the same active
scene/draft. `RequestToken` is an opaque value, with builder/scene identity, draft
nonce, generation and monotonic insertion identity; it contains no builder borrow.
`commit_request` validates the actual scene and keeps the prefix and request
suffix. Validation/cancellation leaves it active. `rollback_request` restores
logical table lengths, image-pixel admission, publications, ignored counts and
first offsets, and removes request-created child scenes. Prefix ID insertion
serials/publication witnesses survive; removed serials never revive on slot reuse.
Actual unused Vec capacities and physical allocation history are retained;
removed nested image/clip/tile/child allocations leave live accounting.

All draft finalizers reject pre-request records during the request. Parent draft
exit and reentrant requests are denied. Existing bounded (64) child-draft
semantics remain available for newly created children, with checked top-of-stack
exit; pre-request children cannot be reopened/sealed. End a child draft before
rolling back its parent request. No token Drop silently commits/rolls back. If a
request token is discarded, `abort_request(&outer_draft)` recovers it. Discarding
an outer draft prevents freeze, as before; abandoning the entire builder drops
all owned storage. Cleanup has no WorkControl argument, polls no canceled control,
and allocates nothing. The fixed single request checkpoint copies only 26 lengths,
4 diagnostic records and scalar identity/context; no prefix/source snapshot or
heap registry is allocated. Fixed builder stack metadata is outside the existing
Vec-capacity statistics, like the draft stack. Caller-owned text cursors/caches
remain caller-owned: restore those alongside this builder scope on request failure.

```rust,ignore
let request = builder.begin_request(&draft, control)?;
match publish_media(&mut builder, control) {
    Ok(()) => match builder.commit_request(&draft, request, control) {
        Ok(()) => {},
        Err(error) => { builder.rollback_request(&draft, request)?; return Err(error); }
    },
    Err(error) => { builder.rollback_request(&draft, request)?; return Err(error); }
}
```

`definition_owner` mints a checked source-token/active-draft `DefinitionOwner`
for `DefinitionTarget::Shape` or `Group`. Empty groups need no shape. The original
filter algorithm needs a group, so no unrelated source-only fake owner is added.
`plan_production_definition` checks actual source/job/owner/style/ranges and
requires the full mapping for patterns. `publish_group_filter` atomically appends
primitives/filter and updates the real group's filter; later `finalize_group`
sets its actual child shape range. The lower planner is responsible for admitting
and retaining/releasing its plan allocations through its context ledger.

Production adapters implement `CachedPaintResolver`, `DefinitionPlanner`,
`MediaDecodeOperations`, and explicitly `ProductionDecodeOperations`; sessions
implement `ProductionDecodeSession`. Every underlying operation is required;
there is no blanket legacy adapter, default Unsupported or successful empty
backend. `resolve_publish_cached` calls the real resolver and immediately uses
its borrowed `CachedPaint` in the builder. Cache lifetime is independent of the
temporary source/ledger borrows. Stops pass directly to the accepted opaque
`GradientTemplateId`/`PublishedStops` publication; only the first scene publication
copies stops. Cache-owning adapters must admit/retain their actual cache capacity.
Legacy `DecodeOperations`, `ResolvedPaint`, structs and literals remain explicit
fixture surfaces; production does not convert to their owned stop Vec path.

`PatternMapping` carries computed tile origin, user-to-base inverse, pattern
transform, instance user inverse, viewport, bounds (x/y/width/height), source map
and opacity. The unchanged `PatternDescriptor` carries texture dimensions, tile
size and content transform. `request_mapped` verifies actual source/pattern token,
same-source/job identity, mutable target and sealed child, then retains the entire
mapping under the original single-use `PatternTicket`. `complete_mapped` consumes
an actual `JobRenderOutput` from that exact job/generation, moves its pixel Vec
internally (no public raw buffer/Tile escape and no pixel copy), appends `PatternId`,
computes pixel-scale × user-to-base, publishes fill/stroke paint and attaches it
to the actual shape. Job charge stays live through the handoff, then builder-owned
capacity accounting takes over; these are distinct existing job/scene budgets,
not a claim of one combined allocator/RSS budget. Errors after transfer remove
publication suffixes and reclaim output; matching stale scopes reclaim their own
output. Foreign tickets/jobs/builders and wrong dimensions preserve the pending
continuation for correction. Lower errors/cancellation invalidate the continuation;
`cancel` abandons an outstanding ticket before any output is handed in. Legacy
`complete` cannot bypass a mapped continuation.

```rust,ignore
let request = tickets.request_mapped(&builder, &draft, &source, sealed_child,
    descriptor, computed_mapping, shape, PaintChannel::Stroke, &job, control)?;
let ticket = request.ticket;
// Actual lower controlled renderer writes output into this same job.
let (pattern, paint_index) = tickets.complete_mapped(ticket, &mut builder,
    &draft, &mut job, rendered_output, control)?;
```

Production pattern planners must use `PaintChannelDefinitionPlanner` and
`plan_production_pattern`, which pass `PaintChannel::Fill` or
`PaintChannel::Stroke` explicitly. The planner resolves the matching computed
`FillOpacity` or `StrokeOpacity` (including the initial value `1`) into the
single opacity carried by `PatternMapping`; the channel and mapping remain bound
through `request_mapped` and `complete_mapped` to the final `PatternId` paint.
Both public planner routes share the same owner, source, style, bounds, kind,
mapping, expansion and post validation. Existing `DefinitionPlanner` and
`plan_production_definition` remain available for compatibility and other
definition kinds. Decoder and SVG definitions adapters should route pattern
planning through the explicit channel operation; there is no default fill route
or equality-based opacity fallback. `ChannelAwareProductionDecodeOperations`
requires both retained `DefinitionPlanner` dispatch and explicit-channel pattern
planning, in addition to resolver and media operations. The additive
`plan_channel_aware_definition` and `plan_channel_aware_pattern` functions accept
that exact combined trait object and share the same production validator, so a
session's `step` operation set can plan patterns and typed clip/mask/marker/filter
definitions. Group filters retain the normal `DefinitionTarget::Group` owner and
`publish_group_filter` path; they do not use a fabricated shape owner.
`ChannelAwareProductionDecodeSession` is the matching additive session contract;
its `step` receives only that explicit-channel operation set, while resume keeps
the existing job-owned mapped completion protocol.

`ChargeLedger::owner_identity` and `require_owner` provide opaque, unforgeable
same-ledger/Arc-source checks without a zero-byte identity probe.
`allocate_buffer<T: Copy>` invokes the accepted owning allocator: requested
reservation before physical allocation, actual capacity admission, honest registry
retention and physical reserve history. `LedgerBuffer` exposes checked slices,
capacity, controlled in-capacity append, truncate, atomic `replace_capacity`, and
unconditional explicit `release`. It exposes neither Vec nor lease. Copy elements
cannot own nested heap allocations; any referenced external allocations remain
separately owned/accounted. Replacement keeps old+new charges and active records
until controlled copy and publication, drops old physical storage before release,
and preserves old values on every error. Configure the query ledger with **65536**
records: replacement overlap uses a second active record, not an auxiliary
uncharged registry. Arithmetic rejection differs from real native Vec reserve
failure; physical successes remain history even after later cancellation.

Clients must retain their buffer until `release(&mut ledger)` and release before
ending/discarding their tracker. Double/stale/foreign use fails. Dropping a buffer
without release frees its storage but conservatively leaves its abandoned charge
in the ledger until ledger destruction; it never silently forgives reservations.
This explicit cleanup contract supports owned query trackers without selfrefs;
RasterJob retains its separate private arena and owning Drop invariants.

```rust,ignore
let identity = ledger.owner_identity();
ledger.require_owner(identity, source)?;
let mut buffer = ledger.allocate_buffer::<u32>(AllocationTag::ReferenceIndex,
    required_capacity, control)?;
buffer.append(&ledger, records, control)?;
buffer.replace_capacity(&mut ledger, AllocationTag::ReferenceIndex,
    replacement_capacity, control)?;
buffer.release(&mut ledger)?;
```

`decode_publication_contracts.rs` contains explicit public test adapters, including
40000 cache-backed fill/stroke instances, full nonempty filter planning, mapped
pattern suspension/resume and variable-loop interruption. These fixtures have no
product registration and import no unfinished lower engines. Full lower parsers,
resolvers/planners/decoder/root adapters, original renderer oracles and product
acceptance remain their separate scopes.

## Render integration contracts (frozen R1)

`SceneBuilder::admit_text_commands` admits **uses** of a successfully validated
layout, including cache hits, against the original cumulative 4 Mi document
command limit. `TextState` and its literals remain unchanged; the separate
262144-command layout-cache limit remains the MEDIA owner's policy. Admit after
provider/layout and existing cumulative glyph validation, at the original token
value offset, before publishing outline geometry. It requires the current draft
and an active request. A huge count produces `TextCommands` at that offset before
control cancellation, matching the original subtraction test. Counter addition
is checked; the counter is privately bounded. All nested child drafts share this
same document counter, corresponding to the original mask/pattern `sync_work`
propagation. Child rollback, outer request rollback/abort, and draft rollback
restore their logical checkpoints. Commit retains admissions. Ordinary scene-local
marks do not rewind independent document admissions. Work and physical allocation
history are retained. MEDIA still restores its own cursor/glyph/cache state and
releases provider results on failure; this counter does not spend cache admission.

`RasterJob::compose(source, cache, control)` validates the source, roles, dash
records, nested grids and original limits exactly once. It returns an exclusive
`RasterComposition`, with no cache copy, Arc clone, self-reference, unsafe code
or metadata/payload allocation. `with_buffers(a, b, control, engine)` checks both
handles and distinct slots before splitting the private arena safely into cache
reads and two mutable admitted element slices. Scratch, boundary, group scratch
and output writes are allowed; Dash/Grid writes are denied throughout the scope.
Call it repeatedly for rows, fill intervals/runs, samples and packed output using
the engine's preallocated buffers. Grow/admit these buffers before entering the
scope. No Vec, lease, logical length, or capacity escapes. `ValidatedDashView`
provides immutable `segments()`, `joins()`, `shapes()` and constant-time `grid_at`
access; clients cannot replace validated slice fields. Cache/grid source/job/
serial/pool checks remain in force. The actual callback's WorkControl updates the
same job work ledger and forwards the caller's policy, including inside providers.
`charge_work` covers actual scalar work outside callbacks, once per operation.

Legacy `JobDashView`, `ControlledSampleInput`, and their explicit validation remain
compatible. Their replaceable fields still require validation; the production
composition path uses the immutable validated view directly and must adapt the
full lower sample algorithm to that read capability. It does not clone a cache,
revalidate every sample, or convert to unmetered legacy paint. Scope ownership
statically prevents release, release_all, finish, teardown or cache modification
while reads can exist. Callback errors/cancellation/panic retain partial scratch
writes and actual work; discard that unfinished output. Scope exit restores normal
job access without detach/restore machinery. Allocation ownership/capacity/leases
are unchanged; discarded handles remain job-owned until explicit release,
release_all, finish or Drop. All existing grid/dash/boundary/group/output caps hold.

Post-close path compatibility checks the original overlapping record events
against retained command/segment boundaries only when overlap occurs. Both input
and sealed-scene validation require canonical M/Z events, explicit close provenance,
monotonic in-element source spans, contiguous segment edges, exact record counts,
closed endpoints and original implicit closing-edge allowance. Scene-offset
publication and shape identity are preserved. No ranges, authored numbers,
implicit commands, geometry, source bytes or subpaths are rewritten/dropped.
Ordinary ordered topology retains its previous validator and work. Frames borrow
this frozen metadata and preserve range identity during edge/color overlays.

`mask_post_target` mints an opaque target from a canonical `DefinitionOwner`, the
actual current-draft GroupId, its exact existing shape range and a real MaskId
whose child is sealed. Nonempty ranges must lie inside the owner's canonical XML
subtree. It retains draft/request/generation and insertion identity, without a
source clone or fake shape. `AttachTargetedMask` carries this target through the
actual production plan. `plan_production_definition` validates source/scope/post
kind and mask flavor before returning it; `MaskPostTarget::validate` is also
available to lower planners. `apply_mask_post` validates it again and atomically
finalizes the intended group. Wrong scope, changed range, stale request/IDs,
foreign source, wrong post kind and cancellation leave the group unchanged.
Legacy `AttachMask(MaskId)` remains an explicit fixture surface, but cannot enter
the production mask-post operation without a real target.

### Atomic quantum semantics

`DecodeQuantum` implements a **soft** scheduling boundary between synchronous
atomic operations. Maximum atomic scope is one complete existing shared call:
one style computation, one reference lookup, one shape append (including its
validation/reservations/copies), or one entire current scene request/draft commit
validation. `atomic` supports other existing bounded shared calls under that same
one-call obligation; it must not wrap a whole traversal/subtree or an unbounded
loop. Its closure must borrow pending engine state. Use the typed `append_shape`
wrapper for owned input: `Boundary(input)` returns it unchanged without calling
or dropping the pending operation.

Atomic bounds are the original accepted source/context/table bounds, not the
requested quantum: source up to 1 GiB, lexical token storage up to 256 MiB,
CSS's 1 MiB/4096-rule/16384-declaration and existing match-work bounds, reference
index's 65536 IDs and original probe controls, and the builder's effective
`Limits` (up to 65536 shapes, 4 Mi edges/commands, 28 Mi numbers, plus its original
bounded media/effect tables and depth-64 ownership). A commit may validate that
entire scene. Source scans, registry moves, allocation/free and bounded native
bulk operations can be indivisible. None of these bounds promises small latency.
No smaller SVG/source/capacity limits have been invented to make a quantum hard.

Every actual `charge` call is recorded by kind and forwarded to the original
policy. Work includes the stopping charge site on error/cancellation; counter
overflow rejects before forwarding unreportable work. `work().total_work` is the
sum of these charge reports, **not tokens visited or a byte-I/O metric**.
`overshoot()` reports the exact amount above the requested quantum. Once reached,
the next operation returns Boundary and does no work. Completed operations stay
completed in caller-owned source/builder/engine state; a new quantum continues
there without source reparse or replay. Failure propagates the original diagnostic;
request/draft cleanup remains explicit and unconditional. Never reinterpret
cancellation as a successful resumable yield. `DecodeEvent::Progress` in a real
session must report this entire step's actual work, including overshoot.

Run these atomic operations on a worker thread; schedule only the bounded
publication/notification required by the eventual UI on its owner thread. This
contract does not establish main-thread or 100 MB responsiveness. Those product
targets still require the full engine integration and measurements.

The sole new `render_integration_contracts.rs` fixture exercises real lexical,
path, animation, scene, provider, planner, request, frame and owning job seams.
It is not a product renderer. Existing 135 assertions/literals remain byte intact;
full lower algorithms/oracles/native application acceptance remains downstream.

Revision 3 owner and definition roles: `DefinitionOwner::element()` is the
canonical lexical host/planning anchor for a real shape or group target in the
active draft. The dispatch `element` is the independently canonical resolved
definition; URL references and mask child expansion can legitimately differ
from the anchor. Neither pattern nor nonpaint dispatch requires equality. Both
production routes retain source/job/draft/target checks; returned pattern
descriptors must name the requested definition, and mappings must name the
actual target shape's source map and requested bounds.

## Source-owned decode initialization

The ordered decoder path starts with `SceneBuilder::provisional(source,
generation, limits)`. It retains that exact source Arc, charges its actual spare
capacity once, and allocates the existing scene storage. There is no viewport or
animation value yet. `initialization()` reports `Unresolved`; no shape writes,
child allocation, draft, seal, ready `scene`, or successful freeze are available.
The existing `new(source, animation, generation, viewport, limits)` remains the
already-initialized constructor with its original argument/error ordering.

The downstream decoder must retain the canonical `LexicalDocument` and
`DecodeSource`, perform original supported-attribute/style/display validation,
and preserve original branching: a hidden root skips transform, viewport and
children; a visible root validates transform before parsing its viewport. Use
`initialize_hidden_root(&mut source, root, control)` or
`initialize_root(&mut source, root, control)` at that point.
These methods check canonical source ownership, an actual depth-zero SVG root,
once-only initialization and cancellation. They recheck root display through the
accepted canonical CSS query (including stylesheet rules and invalid-display
errors). This is a guarded root query, not another source/token parse. Hidden
initialization accepts no viewport and requires empty storage; normal dimensions
remain strictly positive and finite. The ordered initializer now owns canonical
root viewport parsing through the accepted values engine. Supported-attribute and
transform validation remain decoder/lower-engine obligations before that call.

During visible traversal use `traversal_scene(id)` for reads. Its
`TraversalView` uses the same `SceneView` table algorithms and underlying scene
Vecs, with `Option<Viewport>` and `Option<AnimationView>` that honestly represent
unresolved values. `parsed_animation()` is `None` until installation. This read
capability cannot convert to the default ready `SceneView`, enter `FrameBuilder`,
or enter the scalar render interfaces. Existing append, text-command admission,
media source-map validation, typed definition owners/planners, draft/request
commit/rollback, child seal, paint publication and mask guards operate over this
same storage before animation exists. External lower planners that read builder
tables must use `traversal_scene` while decoding. Ready consumers keep `scene`.

`PatternTickets::request_mapped_traversal` returns the same generic
`PatternRequest` with an explicitly provisional child read. It uses the same
request validation/continuation storage as `request_mapped`; wrong/stale child,
shape, draft, request, source and job identities still reject. Actual rendering
requires finalized input; a downstream scheduler must not reinterpret that
provisional request as render-ready. The existing ready request API and literals
remain compatible. No native media provider or production decoder is supplied.

Only after traversal succeeds and root_seen is established, invoke the accepted
`parse_owned_controlled` on the **same lexical source and tokens**, propagate its
actual error, then call `finalize_decode(actual_animation, generation, control)`.
Finalization rejects unresolved roots, mismatched source allocations/generations,
duplicates and cancellation. Active drafts/requests retain their exact tokens
through installation so deferred child rendering/publication can finish; freeze
still rejects them until their explicit commits/rollbacks. On failure,
`FinalizationError` returns the original owned animation and diagnostic; the
builder's scene identities, logical contents and admission statistics stay
unchanged. Retry after cancellation is permitted;
otherwise dropping the builder and returned owner aborts cleanly. A traversal or
parser error never implicitly finalizes: discard/rollback unfinished work.

Admission measures actual animation capacities, including nested key/motion/
spline capacity, charges controlled track visits, and checks live/peak/history/
count arithmetic before publication. Existing source bytes are subtracted once;
there is no old animation replacement or clone/rebuild overlap. Animation parser
allocations are caller-owned until admission, so the caller must account for
that external work/storage in its own job policy. This is admission accounting,
not allocator interception, global OOM/RSS evidence, or a latency bound. Source
Arc bookkeeping and pre-admission parser allocation history remain excluded.

Visible completion uses the unchanged `freeze[_controlled]`, or
`freeze_decode(control)` returning `DecodedScene::Drawable`. A hidden root can
only complete with actual installed animation via
`freeze_decode -> DecodedScene::Hidden(HiddenScene)`. This immutable empty owner
retains exact source, generation, memory statistics, actual animation and its
diagnostics. Its `view()`/`frame().view()` have no viewport, no drawable records,
and checked empty handle queries. It has no conversion into a ready graph or
normal frame/render input; ordinary freeze and ready scene access reject hidden
roots. No zero/default dimensions, empty animation backend or empty successful
normal render stand in for hidden state.

`tests/decode_initialization.rs` contains ordered explicit fixtures using the
actual lexical/CSS/value/path/animation/builder/media/planner/request/child/frame
APIs, plus a compile-fail traversal-to-frame guard. These are contract checks,
not an implementation/acceptance of the downstream production decoder, renderer,
native app, performance targets or broad UI QA. Coordinator independent review,
protected audit and fresh main checks remain acceptance gates.


Viewport provenance correction after independent review: provisional normal root
initialization accepts no `Viewport` argument or external attribute list/proof.
After decoder transform validation, `initialize_root` reads the canonical root's
contiguous attribute tokens and calls the accepted `parse_root_viewport`. Only
unprefixed xmlns/width/height/viewBox/preserveAspectRatio spans enter a fixed
five-entry stack array; unrelated and child attributes cannot supply dimensions.
Every visited attribute token polls the actual WorkControl, and selected value
bytes receive bounded 256-byte source admission charges before the synchronous
values parser call. These charges describe admission, not internal numeric-parser
visits or a hard latency bound. Original mapping error kind/offset is preserved;
no builder state changes until parsing, validation and final checkpoint succeed.
Hidden initialization never enters this extraction/parser. Supported attributes,
style/display and visible transforms retain their original decoder ordering;
XML is never reparsed and the caller cannot convert arbitrary dimensions into a
root proof. The accepted already-initialized `new` constructor remains compatible.

## Cache generations and atomic replacement

`RasterJob::begin_cache(source, old, control)` returns an exclusive
`CacheTransaction`. Its typed Dash/Grid allocations share the original physical
`ChargeLedger`, including old/new buffer growth, both cache generations, actual
native capacities, slot metadata and lease registry growth. Each explicit
logical generation separately retains at most 16 MiB across all Dash/Grid
buffers, 65536 cells and 1000000 references, including spare capacities and
unreferenced buffers. Default unscoped pool APIs retain their original combined
limits; there is no doubled global policy or Scratch role transfer.

Construction uses the same job allocator and mutable element APIs. Transaction
handles cannot operate on another generation. `commit` consumes the transaction,
validates actual finite geometry, ranges, grid indices, roles, source/job/serial
and generation identity, then retires the old generation without copying staged
payloads. The opaque `CacheGeneration` is copyable metadata: replacement makes
all old copies stale. A copied `JobDashCache` remains compatible with existing
read APIs but supplies no publication authority. `cache_descriptor` checks the
current token; `cache_view` and `compose` validate actual payload and grids.
Legacy descriptors cannot mix scoped and unscoped buffers or different explicit
generations. Grid reads also enforce the view's generation in constant time.

Errors from commit, explicit abort and transaction Drop release staged payloads;
the old generation remains usable. A failed individual reserve keeps its prior
staged contents/capacity and may be retried or abandoned. Physical allocation
history and retained registry capacities do not rewind. Deliberately forgetting
a transaction leaves the job unresolved: new transactions and successful
`finish` reject it, even if no buffer was created. `release_all` explicitly aborts
all ownership. Ordinary job Drop destroys every payload. No destructor allocates.

A complete lower construction/replacement example follows. The actual lower
engine supplies its own geometry and original work counts at these same calls;
this small example demonstrates ownership, not scalar-rendering parity.

```rust
use vexel_scene::*;

fn lower(tx: &mut CacheTransaction<'_>, c: &mut dyn WorkControl) -> Result<JobDashCache> {
    let segments = tx.buffer(RasterPool::Dash, usize::MAX, c)?;
    let joins = tx.buffer(RasterPool::Dash, usize::MAX, c)?;
    let shapes = tx.buffer(RasterPool::Dash, 65536, c)?;
    let cells = tx.buffer(RasterPool::Grid, 65536, c)?;
    let refs = tx.buffer(RasterPool::Grid, 1_000_000, c)?;
    tx.reserve(segments, 1, c)?;
    tx.extend(segments, &[DashSegment {
        edge: Edge { a: Point { x: 0., y: 0. }, b: Point { x: 1., y: 1. } },
        start_cap: Linecap::Butt, end_cap: Linecap::Butt,
    }], c)?;
    tx.extend(joins, &[DashJoin {
        previous: Point { x: 0., y: 0. }, vertex: Point { x: 1., y: 1. },
        next: Point { x: 2., y: 0. },
    }], c)?;
    tx.extend(refs, &[0, STROKE_GRID_JOIN_BIT], c)?;
    tx.extend(cells, &[GridCell { first: 0, count: 2 }], c)?;
    tx.extend(shapes, &[JobDashRange {
        segments: TableRange { first: 0, count: 1 },
        joins: TableRange { first: 0, count: 1 }, grid_cells: cells, grid_refs: refs,
        grid_min_x: 0., grid_min_y: 0., grid_cell_size: 1.,
        grid_columns: 1, grid_rows: 1,
    }], c)?;
    tx.slice_mut(segments)?[0].end_cap = Linecap::Round;
    Ok(JobDashCache { segments, joins, shapes, build_steps: 1 })
}
fn replace(job: &mut RasterJob, source: &SourceOwner,
           old: Option<CacheGeneration>, c: &mut dyn WorkControl) -> Result<CacheGeneration> {
    let mut tx = job.begin_cache(source, old, c)?;
    let cache = lower(&mut tx, c)?; // any error drops tx and leaves old intact
    tx.commit(cache, c)
}
fn run(source: &SourceOwner, c: &mut dyn WorkControl) -> Result<()> {
    let mut job = RasterJob::new(ChargeLedger::new(source, 64 << 20, 1024)?)
        .map_err(|e| e.diagnostic)?;
    let first = replace(&mut job, source, None, c)?;
    let current = replace(&mut job, source, Some(first), c)?;
    assert!(job.cache_descriptor(first).is_err());
    let descriptor = job.cache_descriptor(current)?;
    let out = job.buffer(RasterPool::Output, 4, c)?;
    let count = job.buffer(RasterPool::Scratch, 1, c)?;
    job.extend(out, &[0u8; 4], c)?;
    job.extend(count, &[0usize], c)?;
    {
        let mut scope = job.compose(source, descriptor, c)?;
        scope.with_buffers(out, count, c, |cache, pixels, count, _| {
            count[0] = cache.grid_at(0)?.2.len();
            pixels.copy_from_slice(&[1, 2, 3, 255]);
            Ok(())
        })?;
    }
    assert_eq!(job.slice(count)?, &[2]);
    assert_eq!(job.finish()?.live_bytes, 0);
    Ok(())
}
```

`tests/cache_publication.rs` runs populated exact-16-MiB replacement, verifies
unchanged payload address across commit, old/new physical overlap, insufficient
physical admission, aggregate/individual capacity caps, populated simultaneous
65536-cell/1M-reference generations, repeated cleanup, cancellation after real
allocation, strict identity and actual repeated composition reads/writes.
Composition validates once and borrows the job; rows allocate no metadata or
cache copies. Construction capacity admission scans the bounded slot registry;
allocation/free and registry moves remain indivisible native operations. This
establishes neither a latency bound nor renderer/application acceptance.

### Static pattern and scoped media contracts

`PatternTickets::request_mapped_static` is the production traversal-time tile
handoff. It requires a sealed child newly owned by the live request and parent
draft, the actual source owner, a matching positive tile viewport (including
origin and aspect), mapping, owner shape, channel and raster job. Its opaque
`StaticPatternView` contains the original tables and no animation. It borrows the
builder through rendering; mutation, rollback and freezing must wait until all
borrowed input is dropped. Publish the owned output with the same ticket/job via
`complete_mapped`, before parsing parent animation. Nested patterns publish
before the containing child seals.

`RasterInput` supplies identical immutable table getters for ready frames and
static tiles. It exposes checked handles, source annotations, images, path
metadata, transforms, paints/stops, patterns, clips, masks, groups and filters.
`mask_content` authorizes only the content of a checked local mask; no arbitrary
SceneId lookup or ready conversion is exposed. Static minting validates the
sealed mask closure with bounded depth/work. `CommonKernelResources`,
`CommonIndexKernel`, `CommonSampleKernel`, `CommonFlatKernel` and the explicitly
implemented `CommonRasterKernel` are required additive lower-engine interfaces.
`CommonSampleInput` uses the actual validated job cache and work control.
`render_common_pattern` validates source/job/output identities and dimensions;
failed same-job output validation releases the owned output buffer.

`ScopedMediaDecodeOperations` receives the decoder's actual `&DraftToken` for
both text and image calls. `ScopedProductionDecodeOperations` also requires the
cache resolver, explicit paint-channel planner and nonpaint planner.
`decode_scoped_text`/`decode_scoped_image` validate token, scene and source before
the adapter runs. The adapter uses that same token for real request transactions
and cumulative text admission. `ScopedProductionDecodeSession` receives the
actual raster job at step, emits a separate `ScopedDecodeEvent`, and requires
explicit same-job pattern resume. Old interfaces, events and literals remain
compatibility APIs with no automatic conversion or fallback backend.

`ordered_bounds_extent` defines finite ordered `[minx,miny,maxx,maxy]`, allowing
negative maxima and zero extent while rejecting inverted or overflowing extents.
Pattern planning, inverse mapping and paint publication share it. Existing clip
and effect-region semantics remain unchanged.

The accepted contract fixtures prove static failure priority over actual invalid
late animation, success/publication before that parse, nested patterns/masks and
effect metadata, same-job guards, scoped media rollback and cumulative budgets,
and combined capability dispatch. These are explicit typed fixtures, not a
production decoder, scalar renderer or native equivalence result. Lower-engine
adoption and native renderer integration remain required.

### Shape admission and canonical use geometry

`SceneBuilder::admit_shape` reserves the actual shape and insertion-serial
capacities inside an active request before geometry or provider work. The
one-use `ShapeAdmission` must be canceled or consumed by
`append_admitted_shape`; request rollback also clears abandoned reservations.
Pending reservations count against the shape limit and block request, draft,
scene and graph publication. Interleaved ordinary appends leave their reserved
slots available. Capacity growth stays in the scene memory ledger after a
cancel, as does its allocation history.

For referenced paths, `DecodeSource::canonical_path_source` mints provenance
from the same immutable source and token stream after finding the canonical
unprefixed `d` attribute on a path element. Pass that proof to
`admit_shape_with_source`. The `Shape::source` map continues to identify the
use host and its transform; `SceneView::shape_geometry_source` separately
returns the canonical element and path-data range. Seal, frame, static raster
and query views preserve both identities and validate path metadata against the
canonical attribute. Equal bytes owned by another source allocation do not
authorize the proof.
# Scene allocation and lookup observability

`ChargeLedger::stats()` reports current live bytes and total bytes admitted over
the ledger lifetime. Live bytes fall when leases are released; total allocated
bytes do not. `ChargeLedger::allocation_history()` is a fixed five-entry
cumulative payload counter indexed by `AllocationTag` discriminant order
(`None`, `TokenArray`, `ReferenceIndex`, `ClipCopy`, `Output`). Admission adds
nonzero caller payload bytes, and `allocate` adds the actual capacity after a
successful physical reserve. Real retained lease-registry Vec growth is also
recorded under `None`, using checked full replacement bytes minus previous
capacity bytes. `total_allocated_bytes` instead retains its established physical
allocation history: each registry replacement adds its full actual capacity.
Before registry reserve can move entries, the ledger proves `None` history can
represent every replacement capacity that could pass the physical overlap
budget (rounded down to whole entries), minus the previous capacity, plus any
caller `None` payload. Vec's actual excess capacity is unknown before reserve;
insufficient proof headroom returns `Arithmetic` and sets `overflow` without
changing counters, registry storage or leases. This conservative proof can reject
when the requested capacity alone would fit. Existing lease and physical budget
errors retain priority over this registry-history proof.
This metadata history is separate from caller payload history even when the
caller payload itself is tagged `None`. Reusing or releasing a registry slot
does not erase prior growth. Release, rollback after reserve, cancellation
after reserve, and replacement do not subtract history. `DecodeSource::reference_lookup_probes`
exposes the actual lookup-probe counter of its owned `ReferenceIndex`.
