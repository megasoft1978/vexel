Revision 3 candidate complete. Removed only blanket owner/definition element equality from shared production validation; no kind-specific equality was introduced. Documented lexical host/planning anchor versus separately resolved canonical definition.

Replaced R2 equality expectations with both-route distinct path-owner/pattern-definition planning and completed real PatternId publication. Added pre-provider invalid/foreign/stale owner and out-of-range/foreign-source proofs, unchanged scene/accounting snapshots, and retained cancellation/invalid-bounds checks. All eight opacity/default publications and combined dyn group-filter proof remain unchanged.

Fresh named verification passed (exit 0 each):
- `cargo +stable check -p vexel-scene --locked --offline`
- `cargo +stable test -p vexel-scene --locked --offline` (156 passing including doctests)
- `python3 scripts/wo_validate.py WO-RUST-PAINT-CHANNEL-CONTRACT-01 --ready`

Exact logs: check.log, test.log, wo.log; structured verification: verification.json. 10 old scene test/support files byte unchanged from revision input; retained new-test prefix hash matches. No existing153 test/assertion/literal edited. Frozen mask integration passes.

Only production_decode.rs, the new paint_channel_contracts.rs and scene README changed outside revision3 evidence. No dependencies, manifests, peer sources, GUI, network or billing; no agents/review processes launched. APIcoverage.md records adoption requirements and limits. Candidate only: coordinator owns independent review, protected audit and fresh main acceptance. No worker source acceptance claimed.
