---
id: WO-RUST-PAINT-CHANNEL-CONTRACT-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 6eae6e9f218a067c8b29bfa58713f8cd6a6efd0bc4718ae49c72b9d0d364663e
parallel_safe: false
network: false
---
# Explicit paint channel for production pattern planning
The accepted DefinitionPlanner::plan_owned_definition has no PaintChannel, yet
PatternMapping.opacity is one resolved instance value. Full definitions adapter
currently refuses unequal fill-opacity/stroke-opacity, including omitted initial1.
Close this precise shared interface gap on accepted Scene153 without losing
original error priority, source ownership, complete mapping or scene publication.
Provide an additive publicly consumable REQUIRED channel-aware production planner
operation and validated dispatch accepting Fill/Stroke explicitly. There must be
no implicit fill/default conversion/equality restriction or lossy production
fallback. Existing153 tests, old literals/traits and explicit compatibility
interfaces remain byte/API compatible. Document how decoder and definitions
adopt the new production route; any needed typed combined operations/session
interface must be additive and genuinely usable. Nonpaint filter/clip/mask/marker
owners retain their existing semantics; no fake shape for group filter.

Validate canonical source/context/typed owner, element, bounds, transform, style,
returned kind/mapping/post just as accepted production planner does; both routes
must share validation behavior without copied drifting validator implementations.
No unchecked lower planner call, no dropped channel across suspended pattern
request/ticket/result/publication. Mapping already carries resolved opacity;
new fixture planner explicitly reads selected computed style channel, including
initial1. Prove fill .25/stroke .75 and opposite/default1 through actual production
request and completed PatternId fill/stroke publication on real builder/job;
source-span errors, stale/foreign/wrong-target, bad mapping/kind, cancellation,
rollback and original153 assertions preserved. Full lower engine implementation
and app runtime remain downstream, do not invent a backend or claim them here.

# Allowed paths
- rust/crates/vexel-scene/src/** (this additive shared route/refactoring only)
- rust/crates/vexel-scene/tests/paint_channel_contracts.rs (new only)
- rust/crates/vexel-scene/README.md
- artifacts/WO-RUST-PAINT-CHANNEL-CONTRACT-01/** (except coordinator baseline)
Coordinator alone freezes order, registers modules/manifests, integrates main,
seeds peers and updates ledgers/state. A source module needed by existing lib may
be registered within lib.rs under this accepted source-only scope.

# Protected paths
All existing tests/literals, other crates, manifests/lock/config, original603/
157 suites/fixtures/oracles, source C, old evidence/orders, HEAD/index, state/docs/
ledgers. One isolated shared writer only. No unsafe/newdeps/network/install/GPL/
Cshim/unsupportedOS/recursive agents/GUI/broadUI/stage/commit/reset/delete/billing.
Source/tests under800lines; preserve bounded checked fallible ownership/accounting.

# Verification
cargo +stable check -p vexel-scene --locked --offline
cargo +stable test -p vexel-scene --locked --offline
python3 scripts/wo_validate.py WO-RUST-PAINT-CHANNEL-CONTRACT-01 --ready
LunaMedium Standard existing subscription only; stop quota/rate errors, no retry
loop or purchased credits/APIbilling. Candidate only until independent LunaHigh
review, protected audit and fresh main checks. Exact command logs/verification,
APIcoverage.md including downstream adoption and result.md <=350words required.
