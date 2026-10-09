# Blockers

Active: WO-M4-06 environment-limited native CTests and full `test-fast` suite.

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

## WO-M4-06 revision 2

- Candidate: `fb09369fa77475707d8a1e9cde0736f033bf7130`; branch
  `wo/WO-M4-06`; not merged.
- Owner/planner amendment: ADR-0013 and WO-M4-06 revision 2 allow only
  compile-time-bounded synthetic buffers <=4 KiB, containing generated
  constants/syntax only. Source document bytes may not be copied/rewritten.
- Independent review `artifacts/WO-M4-06/manual-review-r2/verdict.json` fails
  AC-9: use expansion copies source spans into a dynamically-sized generated
  document (16 MiB cap); clip parsing copies source child intervals into a
  second generated document (16 MiB cap). Reference source maps also grow to
  1,048,576 entries with expansion. A safe fix requires replacing generated
  document construction with original-source/token traversal, not changing the
  cap.
- Review also finds incorrect userSpaceOnUse gradient defaults and missing
  exact endpoint/interpolation and independent-reference evidence.
- All 18 frozen commands passed in a clean runner worktree; Debug and Release
  fresh builds plus 199 Python tests passed in the independent review worktree.
  The first verification attempt stopped at `./dev verify` because revision 1
  lacked a terminal ledger event; ledger now records r1 blocked and r2 ready.
  Fuzz rerun: 1,000,000 ASan+UBSan cases in 73.008 s, zero crashes, invariants,
  or reports.
- Do not merge or start M4-06b until source traversal, gradient defaults and
  evidence gaps are fixed and a fresh verifier passes.

## WO-M4-06 merge integration review brief 6 — current status

The three librsvg threshold regressions are repaired. Round-4
`mark_varying_shape_bounds` promoted every pixel in a gradient or clip shape
bound to 4×4 sampling; removing it restores center sampling away from the
existing shape/clip boundary map. Scratch `0732b8e`, `9bae63e`, and current
outputs are byte-identical on the three affected fixtures. The updated scalar
reference remains uncullled and now uses the same boundary-map sampling choice.
All 47 librsvg comparisons pass unchanged thresholds, and the added
culled-versus-reference fixture checks and 120 seeded paint/clip differential
cases pass.

Debug and Release builds, `shape_test`, the focused Python renderer suite
(20/20), the 1,000,000-case ASan+UBSan fuzz run, and corpus run/check pass.
Debug, ASan, UBSan, and `./dev verify` each report 18/20 CTests: the two
sandbox-only failures are `native-viewer-smoke` (LaunchServices connection)
and `index-replay` (offscreen Metal device/queue unavailable). `./dev test-fast`
reports 202/212 passing, with 8 failures and 2 errors in benchmark
process-policy/timeout and platform-driver tests. Detailed evidence is in
`artifacts/WO-M4-06/` and the current `HANDOFF.md`.

The repaired four solid-workload outputs are identical to `9bae63e` and their
paired median changes are within observed noise. The 58-file corpus still has
two successful previews and 56 unsupported files. Git index, branch, and commit
state remain unchanged per the user's instruction. The post-merge runner path
guard remains unverified.
