# Rust renderer contract and port plan

WO-RUST-RENDER-CONTRACT-PLAN-01, revision 1. **Proposal for independent review;
implementation scopes and registrations must be frozen by root afterward.**
This plan changes no application code and claims no renderer, application,
fidelity, memory, or performance acceptance.

Owner directives, AGENTS, ADR0022–25, the coding standard, accepted Rust package
checkpoints, named C public/internal headers and signatures, and the existing
rendering research govern this proposal. Legacy README limits are stale where
current headers/signatures differ. Research directions remain testable
inferences; the C kernels remain the operation-order reference. No external
renderer, dependency, native binding candidate, or source-editor candidate is
adopted. Initial support remains Apple Silicon macOS13+, MPL2 and owned SVG/XML.

## Complete ownership and evidence

Evidence is in [the work-order artifact directory](../../artifacts/WO-RUST-RENDER-CONTRACT-PLAN-01/).
The machine files are normative proposal details, beyond this concise report:

- `source-api-inventory.json`: every file in `src/svg/shape`, `svg/paint`,
  `svg/clip`, `raster/shape_*`, and injected `platform/media`; signatures,
  include edges, original macros and source hashes. Native animation export is
  explicitly accounted downstream.
- `ownership-matrix.json`: **69 files and 678 signature locations**, including
  private helpers, header declarations and forward declarations. Each location
  has one owner, target module and disposition. Locations are not 678 distinct
  public APIs. Whole-file scope includes cleanup, custom geometry, source
  selection, allocation hooks, metadata, unsupported errors and diagnostics.
- `accepted-api-signatures.json`: main-tree accepted public API evidence.
  `accepted-contract-gaps.json` separates proposed additions from accepted APIs.
- `shared-contracts.json`: shared data, typed operations, actual ownership,
  read/write boundaries, continuation requests and failure behavior.
- `limits-features.json`: 140 original macro definitions, 90 resolved limits,
  scalar/math/error contracts and explicit supported/unsupported feature scope.
- `dependency-dag.json`: partition prerequisites, disjoint writers and launch
  order. `assertion-assignments.json` binds 53 original suite/support files,
  1,458 assertion/condition locations, 104 fixtures and all 24 authored goldens.

The scope preserves all path commands, basic/rounded geometry, source-number
and command/subpath provenance, CSS/currentColor, root/nested/symbol viewports,
use instances, switch language conditionals, inherited styles, complete paint,
clip, marker, pattern, mask, filter, media and explicit-time composition.
Bounds, scanline/event trees, uniform fill intervals/runs, dash/seam/join caches,
stroke grids, conservative boundary marking, work preflight, scalar sampling,
flat/grouped rendering and original measurement/oracle adapters remain full
ports. A helper's privacy never removes its behavior or assertions.

## Shared ownership and cycles

Four new internal crate roles suffice: `vexel-scene` for shared owned data and
contracts, `vexel-svg-decode` for decoding/resolution, `vexel-svg-scalar` for the
full scalar kernels, and `vexel-svg-render` for coordinator composition.
These are proposed registrations, not edits to Cargo.

A graph owns the accepted `Arc<Vec<u8>>` source lease, scene tables, masks,
materialized patterns, clips and animation data. Source bytes are never copied
for nested definitions. Checked base/XML spans retain original offsets.
Instance-specific transform frames keep attribute, suffix, motion and parent;
path tables retain authored case, implicit commands and explicit stroke closure.
Sparse fill/stroke paints, shared dash values and per-edge colors/shape IDs remain.
Masks refer to completed child scene IDs; clips use validated arena IDs and
parent links. The graph admits no ownership back edges. Active definition
identity/depth/instance checks reject recursive expansion normally.

Source and token owners live outside the decoder. Its temporary CSS/reference
objects borrow those owners; neither an owned scene nor decoder embeds a
self-reference. Endpoint/attribute/paint/text caches are bounded and die with
decode. Persisted tables contain checked spans/IDs and owned Rust buffers.
Builder transactions reserve before mutation and roll back every coupled table;
work already performed is not refunded. Published scenes expose immutable
views. Mutable access exists only in builder transactions and owned evaluated
frame tables.

The decoder suspends at the original pattern-render point and emits a ticket
plus a completed child view. Composition invokes the actual full scalar kernel,
then returns the owned tile through `resume_pattern`. Nested patterns complete
first. The ticket is consumed once; failure aborts the draft. This preserves
error order and avoids a decoder-to-raster crate cycle, duplicate parsing,
self-referencing providers, placeholder tiles or omitted pattern strokes.

Definition planners return `DefinitionPlan<'src>` with source-borrowed
`Expansion<'src>` styles. `DefinitionContext<'src, 'ctx>` stores source and tokens
under `'src`; accepted `CssSheet<'src>` and `ReferenceIndex<'src>` keep their
source borrows, while mutable CSS/reference, active-definition, counter and
builder borrows use the independent temporary `'ctx`. The object-safe
`DecodeOperations::plan_definition<'src, 'ctx>` method returns a plan tied only
to `'src`, allowing all context and control borrows to end on return.
Only the traversal engine walks expansions/postoperations. Shared
`DecodeOperations` signatures permit independent
compilation; coordinator bridges delegate complete paint/definition/media
implementations. The real flat kernel similarly implements `FlatKernel` for
full grouped rendering. Neither interface permits production scope to be
replaced with a convenient smaller renderer. There are no temporary stub bodies
or unfinished peer implementation dependencies.

Clip decoding uses original token ranges with restricted clip flavor and
clip-rule inheritance, replacing synthetic wrapper decoding. Original admission
limits and diagnostic offsets remain; actual allocation counters measure actual
buffers, rather than invented wrapper allocations. Queries retain child union,
parent intersection and conservative boundary behavior when inversion fails.

## Contracts that must land first

Four additive gates require independent review and root-owned implementation:

1. **Owned animation/view:** accepted `Document` borrows source and owns track
   vectors. Move parsed vectors into an owned source lease and expose borrowed
   slices to the same evaluator. No cloned tracks, reparsing or parallel parser.
2. **Large values adapter:** accepted containing-source checks are 16MiB and
   attribute views cap at256. Add policy-bearing entry points up to1GiB/4096
   attributes without changing existing defaults, grammar or absolute offsets.
   No source-slice shortcut disguises a smaller decoder limit.
3. **Existing parser/flatten access:** expose the accepted number/length cursor
   and curve/edge operations needed for primitive/text/custom geometry. Reuse
   accepted path output with checked table rebasing; do not port shadow readers.
4. **Controlled work/capacity:** shared base control and additive CSS/reference/
   path/animation APIs expose cancellation, actual reserved bytes and real
   resumable cursor state. Existing one-shot entry points use the same engine.
   `Pending` reports actual charged work and cannot certify success.

The task queue's scalar completion API remains accepted as-is: an owned job
context uses a separately bounded result slot/channel keyed by task/generation;
`TaskOutput` signals completion. No assumed typed tile-return API is invented.
The accepted document model supplies the source lease and source ranges;
composition does not depend on unfinished editor snapshot interfaces.
Accepted spatial queries and tile cache ownership/epoch/capacity APIs remain the
integration seams for visible work and invalidation. Published results carry a
generation; stale/canceled results are dropped. Drain/join precedes lease release.

## Media, limits and fidelity

Media injection remains synchronous and real. Requests borrow UTF8/encoded
bytes only until return; outputs own Rust text outlines or premultiplied RGBA
buffers. The provider need not cross threads. Framework handles must be
released on the invoking thread before owned Rust outputs are transferred;
no foreign pointer/release token survives in the graph. Later native orders
must audit actual CoreText/CoreGraphics/ImageIO ABI, ownership, availability,
threading and failure/drop paths. Unaccepted binding research is not a usable
implementation. Missing text/image service reports original unsupported errors,
never successful fake media. Original deterministic injected tests remain tests.

Charge actual reserved capacity, including source lease capacity once, token
and open-name storage, lookup/cache metadata, all scene/overlay/media buffers
and overlapping nested group/mask/filter scratch. All range/size arithmetic is
checked; fallible reserve occurs before commit. Bounds and admission caps keep
the original per-scene versus job-wide semantics and nested shared counters.
Native synchronous calls are cancellation boundaries with bounded inputs;
Rust work checks cancellation at bounded intervals. Unsafe belongs only to a
later separately audited native boundary; domain code adds none.

Current source/token bounds are1GiB/256MiB, edges4,194,304, shapes65,536,
commands4,194,304. The native-resolution preflight remains
`min(16,000,000,000, 200,000,000 + max(0,pixels-1,048,576)*24,576)`.
Group/filter scratch remains64MiB with shared occupancy, and filters retain50M
work. Grid/fill ineligibility keeps original scalar fallbacks, not refusal.
The machine matrix preserves the remaining exact caps, including differing SVG
and native per-run/document text budgets.

Preserve f64 operation order, right-first transforms, clipping inequalities,
sort ties, alpha numerators/rounding, gradient/image/pattern interpolation,
4x4 subpixel locations, blur passes and linear/sRGB conversions. Group order is
children, filter, mask, opacity/blend. Oracle-only synthetic alpha stays test-only.
No FMA reassociation, SIMD/GPU rewrite, source normalization or changed goldens.

Unsupported filter categories remain distinct. External resources/scripts,
textPath, color-only glyphs, animated image input and original restricted
CSS/clip/pattern forms retain their reports. Current animated geometry/effect/
pattern and active path-composition restrictions remain explicit; broader
product authoring workflows are downstream work, not claimed implemented.

## Execution and acceptance

Root first reviews/freezes shared interfaces and additive gates, then registers
modules. Disjoint decoder, paint, definition, SVG media, scene-query/accounting,
raster-index, raster-sample and animation writers can proceed against frozen
contracts. Definitions alone own lower clip queries; paint alone owns lower
paint sampling. Raster effects and audited native media follow, then coordinator
composition and complete native export. One writer owns each module; root alone
changes shared interfaces/manifests/locks/orders/ledgers. No worker was launched.

Each partition receives its entire named C/API scope and original meaningful
suite assertions. Full scalar integration owns mixed suites, random byte
comparisons, exact budget/error counters and all24 frozen authored goldens;
actual native media is mandatory at that stage. No weaker tolerances, reduced
fixtures or re-generated baselines are authorized. Future orders must freeze
focused Rust proof commands after registration. No hypothetical command is
reported as executed here.

Full100MB editing/rendering/save, first content<2s, owner pauses<100ms and cached
pan/zoom p95<33ms remain product requirements. Resumable/visible work is required;
a partial preview cannot certify final validation. Module limits are not proof
of those budgets. Broad UI/E2E, full corpus/fuzz/fidelity/performance, native
accessibility/recovery, packaging/release and authentic live MCP vector/cartoon
recordings remain final gates.

Executed proof commands are the work order's ready validator, ledger validator
and `git diff --check`; exact results are in `validators.json`. Worker observation
found HEAD/index unchanged and one protected source-editor candidate delta
relative to the existing launch baseline. No attribution or original603/root
preservation acceptance is asserted. Independent review, root preservation
audit, implementation freeze and registration remain pending.

The original focused review blocker is resolved in this proposal: `DefinitionPlan<'src>`
carries `Vec<Expansion<'src>>`, and its trait/function signatures consistently
tie `CssPaint<'src>` and the returned plan to source/paint lifetime `'src`. The
recheck-v1 context borrow is also independent: mutable context fields use `'ctx`,
while accepted `CssSheet<'src>` and `ReferenceIndex<'src>` retain source `'src`.
