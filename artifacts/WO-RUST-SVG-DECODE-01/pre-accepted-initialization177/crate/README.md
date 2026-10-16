# vexel-svg-decode

This is the same frozen WO-RUST-SVG-DECODE-01 revision 2 candidate, continued with
the accepted Scene153 integration seed. It is not complete or accepted.

`Decoder::from_source` borrows the canonical LexicalDocument/DecodeSource and
owns its SceneBuilder, draft stack and source-only traversal tasks. It implements
inherited style, currentColor, marker clearing, transforms/viewports, switches,
local use/symbol expansion, geometry/path/custom publication, source host maps,
groups, ignored diagnostics, typed lower plans, sealed child scenes and pattern
suspension. Numeric loops and owned buffer transfers use actual work controls;
DecodeQuantum is a soft boundary between bounded tasks, with actual overshoot
reported. It makes no main-thread latency guarantee.

## Production composition

Use `step_production(quantum, operations, &job, control)` with actual
ProductionDecodeOperations and the same caller-owned RasterJob across steps.
CachedPaintResolver output is published directly with its borrowed template/stops.
DefinitionPlanner receives real typed Shape/Group owners. Mapped pattern requests
carry sealed actual children; end the request-view borrow, render through the
actual coordinator kernel, then call `resume_pattern_production(ticket, &mut job,
result, control)`. Completion transfers job-owned pixels once and attaches the
exact channel mapping. Errors invalidate the job's decode transaction and ticket.
Finish requires Complete and validates/commits/freezes the real graph.

The older `step`/DecodeOperations and raw Tile resume are explicit legacy fixture
interfaces. They are not a blanket production adapter. Tests retain their seven
original assertions unchanged. No default media/provider/resource implementation
is supplied; native composition remains mandatory downstream.

Owned vectors use fallible reserves and actual-capacity leases, including old/new
stack overlap, child drafts, active stacks, plans and temporary clip copies.
Attribute lists use fixed bounded stack arrays. No source clone/reparse, unsafe
code or dependency change was introduced.

## Remaining gates

The planner lacks an explicit requested paint channel. Shared canonical validation of
post-close overlaps conflates authored target spans with use-host annotations. Builder initialization
cannot represent original hidden-root/deferred-animation semantics. Acceptance
assertions expose these gaps; see artifacts/WO-RUST-SVG-DECODE-01/integration-resume/
accepted-contract-blockers.md and the top-level parity/coverage/verification reports.
Original mixed inputs remain intact, including later real cache, media command-use,
renderer/CLI/fuzz/oracle proofs. Independent review and root main checks remain.
