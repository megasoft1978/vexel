# Revision 2 API coverage — candidate only

The only production repair is the canonical `owner.element() == element` guard
in `validate_production_plan`. It runs after the existing owner, style and source
element checks, preserving their priority and diagnostic offsets, and before
transform/bounds checks and any lower planner invocation. Existing public
signatures, literal layouts and required trait methods are unchanged.

| Consumer / contract | Route and evidence |
| --- | --- |
| Explicit compatibility | `DefinitionPlanner` → `plan_production_definition` → shared validation → `plan_owned_definition`. New guard fixture proves valid canonical mismatch rejection, zero provider calls, unchanged scene/ledger, cancellation and negative bounds, then matching-element completed Fill publication. The fixture explicitly chooses Fill only inside its compatibility implementation. |
| Explicit production pattern | Required `PaintChannelDefinitionPlanner::plan_owned_pattern_definition`, through `plan_production_pattern`; identical shared validation. New guard fixture proves the corresponding mismatch/invalid/cancel behavior and completed matching Stroke publication. |
| Decoder combined operations | `ChannelAwareProductionDecodeOperations` retains required `DefinitionPlanner` and channel planner. `ChannelAwareProductionDecodeSession::step` accepts that combined trait object. Existing new test uses one dynamic object for Stroke pattern and typed group Filter dispatch and real group publication. |
| Nonpaint definitions | `plan_channel_aware_definition` forwards nonpattern definitions to the same shared validator and required compatibility operation. Pattern is rejected on this entry point. No fabricated shape owner for a group filter. |
| Opacity / initial values | Preserved eight-case test explicitly reads the selected computed property, resolves absence to 1, and completes real mapped request/ticket/job publication to the correct Fill/Stroke paint and PatternId. |
| Mapping / suspension / publication | Existing request_mapped/complete_mapped production path retained. Mapping already holds resolved opacity; explicit channel travels with the request to publication. This repair does not change those APIs or ticket guards. |
| Validation / original suites | Mapping, returned kind, descriptor element, source-map/bounds, post/expansion validation retained byte-for-byte. Full no-fail-fast execution: 155 passed, 1 failed including doctests. The frozen original153 contribution is 152 passed, 1 failed; new test target is 3 passed. |

Downstream definitions must implement the required channel-aware operation and
read the specified computed FillOpacity or StrokeOpacity including initial 1;
they must enforce canonical owner/element identity too. Downstream decoder
sessions must accept the additive combined operations object and call
`plan_channel_aware_pattern` with the selected channel, then carry its returned
mapping and that channel through mapped request/completion. Nonpattern dispatch
uses `plan_channel_aware_definition`. No implicit compatibility conversion,
equality restriction on the two opacity values, or backend is added here.
Full lower engine and app adoption remain downstream.

Acceptance is blocked by the frozen original test
`actual_mask_planner_shared_validation_nonempty_expansion_wrong_flavor_and_source_coherence`.
It unwraps InvalidArgument at render_integration_contracts.rs:616:57 with the
required guard and passes in the guard-only removal diagnostic. No protected
source/test was read or edited to resolve this conflict. Coordinator resolution
must preserve the required guard and reconcile frozen acceptance; no exception
or source gate is proposed.
