# Blockers

No implementation blocker remains for AC-9. Full machine acceptance is still
open because this host session cannot provide the offscreen Metal device or
queue required by the existing `index-replay` CTest.

## Resolved: WO-M4-06 revision 2 acceptance block

Revision 2's source-copy restriction was resolved by the planner amendment and
later frozen revisions. The current AC-9 repair replaces the former bounded
copy path with direct traversal of original tokens; 64 KiB per-reference and
1 MiB per-job source-span limits remain as traversal-work bounds. It also
retains depth 16, a 65,536-use cap, and fixed ID-index allocation. The source
and allocation evidence is in `artifacts/WO-M4-06/reference-matrix.json`.
Fresh read-only review remains pending.
