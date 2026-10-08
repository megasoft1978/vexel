# Blockers

No active blockers.

## Resolved: WO-M1-04 shutdown lifetime contract

WO-M1-04 revision 1's reviewer identified that a raw queue pointer cannot
protect a caller paused before API admission from a concurrent destroy.
Planner authority amended the contract to require owner quiescence, defined
concurrent entry during destroy as undefined behavior, and required a
debug-assert active-call counter with a deterministic death test. ADR-0006 and
WO-M1-04 revision 2 record that decision.

Revision 2 then exposed a pre-existing flaky native pointer-down test in both
the accepted M1-03 binary and its own candidate. The comparison is retained in
`artifacts/WO-M1-04/baseline-ui-smoke-comparison.json`. Revision 3 added a
test-only direct dispatch to the target view while preserving the event type,
coordinate, bounds, and button assertions; ADR-0007 records that change.

WO-M1-04 revision 3 passed all nine frozen commands and two fresh read-only
reviews, then merged at `91ff611782c843428fb5bdc4ba874e3ccdd37f94`. Final
verifier evidence is under
`artifacts/runs/WO-M1-04/verify-e71955ec-d0a3-4888-8fa2-4355f82ce469/`.
