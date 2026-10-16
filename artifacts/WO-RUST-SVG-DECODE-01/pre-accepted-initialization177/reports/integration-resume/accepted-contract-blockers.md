# Remaining accepted contract dependencies

The Scene153 seed is adopted, with no rollback. Typed group/shape owners,
production cached paint and definition operations, mapped pattern publication,
targeted masks and soft DecodeQuantum task boundaries are active. The original
direct smooth-after-Z assertion passes unchanged.

## Definition paint channel (coordinator-known gap)

`DefinitionPlanner::plan_owned_definition` receives kind, element, typed owner,
transform, bounds and the full CssPaint, but no `PaintChannel` or requested value
span. `ProductionDefinitionPlan::Pattern` contains a single PatternMapping,
including opacity. Two requested server channels can require different mapping
opacity even when they reference the same definition. The engine now records the
channel at task creation and never recovers it from span or value equality.
Single-channel mapped requests execute completely. Dual-server pattern planning
returns the named DefinitionPaintChannel contract failure before a guessed plan
is published. Pass the actual requested channel/value through the accepted
planner handoff; do not infer fill/stroke equivalence.

## Post-close canonical range versus instance-host span

`SceneBuilder::append_shape` calls `validate::path(source, path,
shape.source.element, control)`. When subpaths overlap after close,
`path_topology::canonical` uses that host element as its source_range. The graph
validator repeats the same host-range constraint for overlapping records.
Ordinary non-overlapping marker/use path metadata already passes.

For `<defs><path id="p" d="M... Z S..."/></defs><use href="#p"/>`, original C
assigns shape source to the use host and retains the target's authored command,
number and overlapping subpath metadata. Canonical record validation must use the
actual authored command range, independently of the instance annotation.
`original_metadata.rs` preserves this assertion and fails InvalidArgument:0.
Coordinator must repair this accepted validator range at append and freeze,
retaining source/shape association checks and every parser record. The engine
must not rewrite metadata or replace the use-host map to bypass validation.

## Hidden-root representation and deferred animation initialization

`SceneBuilder::new` requires a validated nonzero Viewport and an already parsed
OwnedAnimation; there is no accepted later animation replacement/initialization.
Original `traverse.c:433-436` exits display:none before authored transform and
viewport parsing, leaving a successful empty document with no established root
viewport. Original `decode.c:202-218` traverses before animation parsing.
`zz_contract_priority.rs` preserves both expectations and currently fails:
hidden-root bad transform is parsed, and 129 tracks replace an earlier Path
error with Animation limit. An empty fake animation or invented 1x1 viewport
would replace the required semantics. Coordinator must supply valid draft
initialization/finalization for deferred animation and an absent/hidden viewport.

## Later composition dependencies; no proof substitution

The production media interface receives no guarded DraftToken/request context,
but `SceneBuilder::admit_text_commands` requires both a DraftToken and active
request. Its actual command-use accounting gate still needs an accepted handoff
for the real downstream media adapter. The new media fixture proves provider
calls and canonical host maps only; it is not that admission proof.

The allowed raster dependency exposes Tile but no tile-cache API, so the original
spatial_cache_contract_tests pure LRU/epoch/ownership predicates remain an actual
cache integration dependency. Native providers, cached-gradient scan/probe
statistics, full decode-job memory metrics and all original renderer/CLI/fuzz/
pixel/oracle predicates require the actual coordinator adapters/resources. All
whole inputs, function hashes and individual pending reasons remain in coverage.
