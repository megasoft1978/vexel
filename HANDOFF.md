# Handoff

WO-M1-04 is blocked before merge. The implementation candidate is preserved
on `wo/WO-M1-04` at `181b62743e3373ba511c6732463eff283b5eeba2` in
`.worktrees/WO-M1-04/`. Its frozen machine checks pass, but the fresh read-only
review rejects shutdown safety for a caller paused before operation admission.

Read `BLOCKERS.md` and the final verifier evidence at
`artifacts/runs/WO-M1-04/verify-449d6bf3-29f1-4faf-8103-b4bc9e8a50fd/`.
Next, get the owner/planner decision on the queue lifetime protocol versus the
frozen shutdown contract. Keep the candidate unmerged and the frozen work order
unchanged until that decision is recorded.
