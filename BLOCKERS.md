# Blockers

## WO-M1-04 shutdown lifetime contract

- Candidate: `wo/WO-M1-04` at `181b62743e3373ba511c6732463eff283b5eeba2` in
  `.worktrees/WO-M1-04/`. It is unmerged.
- Evidence: fresh verifier run
  `artifacts/runs/WO-M1-04/verify-449d6bf3-29f1-4faf-8103-b4bc9e8a50fd/`.
  All nine frozen acceptance commands passed; the focused suite reported 21
  tests, `./dev verify` reported 144 tests, and the native queue smoke completed
  1,000 cycles with a maximum join of 0.001592 seconds.
- The read-only reviewer still rejects the queue shutdown lifetime contract.
  With a raw queue pointer, an API call paused before registering itself can
  resume after shutdown observes zero active operations and frees the queue.
  Three repair approaches were reviewed: internal active-call counting,
  deterministic admission gating, and a documented owner-quiescence contract.
  The reviewer requires either a lifetime protocol that protects such callers
  or an acceptance/API contract that excludes concurrent entry and a matching
  deterministic check.
- Do not merge this candidate. The owner/planner must choose whether to extend
  the queue API with an externally retained handle/reference protocol, or
  amend the frozen shutdown contract to require caller-side quiescence and
  narrow its race acceptance criterion. Keep `work/WO-M1-04.md` immutable until
  that decision is recorded.

The repeated review findings and machine evidence are preserved under
`artifacts/runs/WO-M1-04/` and `artifacts/WO-M1-04/`.
