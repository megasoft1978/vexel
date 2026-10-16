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
