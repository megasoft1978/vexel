# Blockers

No active blockers.

## Resolved: WO-M4-06 revision 2 acceptance and revision 4 review findings

Planner authority recorded in ADR-0014 and WO-M4-06 revision 4 allows copied
reference subtrees up to 64 KiB each and 1 MiB total per render job, with 512
KiB sub-budgets for `<use>` and clipping. Revision 4 also fixes omitted SVG
`stop-color` to its initial black value. Candidate-specific Debug/Release,
ASan/UBSan, 1,000,000-case fuzz, 35 librsvg fixture, 58-file corpus, and
21-trial benchmark evidence is under `artifacts/WO-M4-06/`. Independent fresh
review and merge are pending.
