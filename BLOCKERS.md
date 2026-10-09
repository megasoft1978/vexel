# Blockers

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
  exact endpoint/interpolation and independent-reference evidence. See full
  read-only review artifact and the reference matrix.
- All 18 frozen commands passed in clean runner worktree; Debug and Release
  fresh builds plus 199 Python tests passed in independent clean review worktree.
  The first verification attempt stopped at `./dev verify` because the blocked
  revision-1 ledger event lacked a terminal transition; ledger now records r1
  blocked and r2 ready. Fuzz rerun: 1,000,000 ASan+UBSan cases in 73.008 s,
  zero crashes/invariants/reports.
- Do not merge or start M4-06b until source traversal and gradient/evidence
  gaps are fixed and a fresh verifier passes.
