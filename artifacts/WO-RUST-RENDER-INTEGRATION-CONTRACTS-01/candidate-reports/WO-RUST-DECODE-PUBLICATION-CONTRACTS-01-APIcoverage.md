# R2 shared owning API coverage

All examples run in the sole new integration file, `decode_publication_contracts.rs`.
They are explicit fixtures; no lower product engine is imported or registered.
README examples show coordinator integration without a self-reference.

| Required owning operation | Implemented surface | Meaningful public evidence |
| --- | --- | --- |
| Same-draft request checkpoint/commit/rollback | `begin_request`, `commit_request`, `rollback_request`, `abort_request` | Late shape/image/child failure retains prefix IDs, stops witness, source map and outer usability; successful commit; foreign/double/stale/reentrant/freeze denial |
| Guarded prefix updates | Request-aware `draft_slot`; protected pre-request child entry/seal; parent exit guards | Every existing finalizer is exercised, plus real nested child draft commit and out-of-order rollback denial |
| Retained capacity/removed lifetime | Existing actual scene capacities, nested payload measurement, request scene/diagnostic checkpoints | Removed image/child bytes decrease live; spare table capacities and history remain; reused shape/child slots reject saved IDs |
| Empty filter owner | `DefinitionTarget`, opaque `DefinitionOwner`, `definition_owner`, `publish_group_filter` | Group exists before any geometry; filter/primitives are actually published and its later child shape range finalized; wrong/stale/prefix/canceled owners rejected |
| Full typed planner handoff | Required `DefinitionPlanner`; checked `plan_production_definition`; `ProductionDefinitionPlan` | Explicit nonempty filter planner fixture produces a source expansion and primitives which enter the real builder; kind/source/style/range/control validation |
| Actual no-copy cached delegate | Required `CachedPaintResolver`; `resolve_publish_cached`, `append_cached_paint` | 40000 fill AND stroke instances use one preallocated cache stop buffer and one scene stop publication; both pointers/capacities/history checked; temporary source/ledger borrows end before publication; in-loop cancel/limit propagates |
| Entire computed pattern mapping | `PatternMapping`, `request_mapped`, `complete_mapped` | Nonidentity transform/origin/scale, explicit pixel-inverse literals, bounds/user inverse/opacity and content transform reach actual PatternId paints attached to fill and stroke |
| Owning pattern output transfer | Private `RasterJob::publish_pattern_output`, no public Vec/Tile extraction | Same job/output/source/generation checked; pixel pointer equality proves move, output handle invalidates, output charge releases only after builder admission; cancellation swept across publication boundaries |
| Ticket security/cleanup | Existing ticket owner/serial, mapped draft/request/job binding | Sealed child required; wrong ticket/job/dimensions and legacy-completion bypass denied; matching stale scope reclaims output; lower error/cancel consumes continuation; double use denied |
| Public client preallocation | `allocate_buffer`, opaque `LedgerBuffer<T: Copy>` | Budget rejects payload before native reserve; actual capacity tags; native CapacityOverflow vs checked multiplication overflow; after-allocation cancel retains physical history |
| Same-ledger/source identity | `LedgerIdentity`, `owner_identity`, `require_owner`; private buffer lease | No probe lease/history; same-source wrong ledger, different empty source Arc, stale/double/reused client identities rejected |
| Overlap/lifetime/metadata | Controlled append, `replace_capacity`, truncate, explicit release | Old+new capacity peak, unchanged old values after budget/copy failure, real shrink/reuse, exact 65536 active-record limit including replacement; retained registry capacity stays honest |

## Ownership boundaries

Request cleanup is allocation-free and unconditional after cancellation. Only one
request checkpoint exists; fixed metadata snapshots lengths/diagnostics/scalars,
not prefix records. Child draft nesting retains the accepted depth-64 semantics.
External media cursor/cache state remains its owning adapter's responsibility.

Cached results borrow the cache, not ephemeral mutable source/ledger contexts.
Production delegates have required methods and no implicit legacy conversion.
The lower adapter must meter its actual plan/cache capacities in its supplied
ledger; shared fixtures do not claim to implement its parser/resolver algorithms.

Mapped output remains owned by the job through handoff; the builder then owns
and measures the same allocation. Scene and raster job retain their existing
separate budgets; no combined allocator/RSS guarantee is asserted. Query clients
retain buffers and explicitly release them before teardown. Discarding a client
buffer without release conservatively leaves its abandoned charge until ledger
Drop, rather than erasing history or exporting an unchecked lease. RasterJob's
private arena remains independently owning and releases all payloads on Drop.

Original fixture traits/struct literals and all old test files are byte preserved.
Protected audit and exact offline command outcomes are in `verification.json`.
Independent root review, fresh main checks and terminal peer seeding remain
coordinator gates. No full lower engine, 603/157 render-oracle execution, app,
performance, native runtime, GUI, live MCP, release or demo acceptance is claimed.
