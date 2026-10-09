# Handoff: WO-M4-06 blocked

WO-M4-06 revision 2 remains unmerged on `wo/WO-M4-06` at
`fb09369fa77475707d8a1e9cde0736f033bf7130`. All 18 frozen commands pass,
but fresh read-only review fails AC-9 and raises additional gradient-default
and evidence gaps. The managed verifier rejected the full diff because it
exceeded its 250 KiB input cap; the required focused read-only fallback ran
against the full renderer/test diff.

Next: replace source-copy-generated SVG paths in `src/svg/reference/expand.c`
and `src/svg/clip/clip.c` with traversal over the original borrowed token/source
slices. Add the deterministic <=4 KiB generated-syntax bound and before/after
source hash test, fix SVG gradient coordinate defaults and percentage handling,
then rebuild the full AC-2/AC-3 matrix and rerun the frozen checks. Do not start
M4-06b until M4-06 is accepted; afterward M4-06b must connect partial rendering
to `svgworkbench` and `./dev bench-corpus`. External corpus remains read-only.
