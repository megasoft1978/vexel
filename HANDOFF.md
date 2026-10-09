# Handoff

WO-M4-04 revision 1 is complete and fast-forward merged to `main` at
`dac63d6e8f8cd08787f3ee7e71099d0655ee258e`. Final candidate
`f48c19f017ddf1cbc756e4ff79a9c26d674335fd` passed fresh read-only review
`verify-3c3ca13c-0382-4d62-8823-d11408b97fa4` with no findings in 89.518 s;
all 18 frozen commands pass. Debug/ASan/UBSan CTest 11/11, Python 192/192,
native smoke/replay, 9 focused tests, 26 strict librsvg fixtures, and 1M
ASan+UBSan fuzz pass. The final raster benchmark records 2.138/4.301/5.331 ms
simple/curved/multi-shape fill and 2.501/10.735/78.994 ms line/curved/dashed
strokes. The corpus has 2/58 previews and 56 unsupported; the 100 MiB preview
gate fails, while max-step (0.199 ms) and RSS gates pass. This is open/index and
selected-shape evidence, not full SVG parity. Detailed results:
`artifacts/WO-M4-04/`, `reports/M4/head-to-head.md`, and `docs/PERF.md`.

Next: prepare and freeze WO-M4-05 for the CSS/style subset, following the M4
feature priority inventory at `reports/M4/corpus-feature-frequency.md`.

## WO-M4-05 closeout

WO-M4-05 is fast-forward merged to `main` at `81f2008311e886d3b8dc8da262fe3f40f2f9b14b`; reviewed candidate `9209ae3b670911470100e97b9538daf83dfa509d` implements the bounded CSS paint cascade and passes a fresh read-only review with no findings. The review fixed comment delimiter parsing, added an integration regression proving inline `!important` beats stylesheet `!important`, and replaced stale failure text. In a fresh clean worktree, `./dev build`, Release configure/build, and all 19 frozen commands passed: Debug/ASan/UBSan CTest 12/12, Python 199/199, native smoke/replay, 16 focused reference tests, 1,000,000 ASan+UBSan fuzz cases, 58-file corpus run/check, and Release builds.

The corpus remains 2/58 previewable, 56 unsupported, with no limit/crash results. The 100 MiB first-preview gate fails (0/12 eligible files), while max main-thread step is 0.199 ms and RSS gate passes. CSS workloads have 21 fresh-process trials, median/p95 27.890/29.662 ms stylesheet and 27.483/29.013 ms inline; peak RSS 2,113,536/2,146,304 B and stable shared output hash. There is no comparable pre-WO CSS workload. Evidence is in `artifacts/WO-M4-05/`; this is open/index and selected-shape evidence, not full-render parity.

The standard verifier completed its frozen commands but its fixed 250 KB prompt cap rejected the complete artifact diff. The follow-up used a clean worktree, reran every frozen command, and performed a fresh read-only review of the focused implementation/test diff.

WO-M4-06 revision 1 is frozen READY (spec `e3a8b6dc33c470f4306abc227ba2af985b075c551503a79efbc83ac8a1434e56`) for bounded local references, gradients, and clipping. Next: execute on `wo/WO-M4-06`.

## Current handoff: WO-M4-06 blocked

WO-M4-06 revision 2 remains unmerged on `wo/WO-M4-06` at
`fb09369fa77475707d8a1e9cde0736f033bf7130`. All 18 frozen commands pass,
but fresh read-only review fails AC-9 and raises gradient-default and evidence
gaps. The managed verifier rejected the full diff because it exceeded its
250 KiB input cap; the focused read-only fallback reviewed the complete
108,559-byte renderer/test diff.

Next: replace source-copy-generated SVG paths in `src/svg/reference/expand.c`
and `src/svg/clip/clip.c` with traversal over original borrowed source/token
slices. Add the deterministic <=4 KiB generated-syntax bound and before/after
source hash test, fix SVG gradient coordinate defaults and percentage handling,
then rebuild the AC-2/AC-3 evidence matrix and rerun all frozen checks. Do not
start M4-06b until M4-06 is accepted; afterward wire partial rendering into
`svgworkbench` and `./dev bench-corpus`. External corpus remains read-only.
