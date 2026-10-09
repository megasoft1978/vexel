# WO-M4-06 handoff

Status: blocked; candidate is not merged. The worktree branch `wo/WO-M4-06` contains implementation and evidence through reviewed candidate `6691bac7a9e6955dbe3a841bad41a62f53a4250a` (code revision `c32a8195bc10cff7d66707dc2979d029f89c8157`).

## Completed

- Bounded local `<use>`/`<symbol>` handling, linear and radial gradients, clipping paths, CSS `clip-path`, diagnostics, and raster integration.
- Fixed the fresh verifier's depth-index out-of-bounds finding and nested unbound `xlink:href` validation gap.
- Refreshed candidate comparison, benchmark, corpus, fuzz, and verifier artifacts.
- `./dev verify`: 12/12 CTests and 199/199 Python tests pass.
- `./dev asan`: 12/12 CTests pass; `./dev ubsan`: 12/12 CTests pass.
- Release configure/build pass; 16/16 librsvg differential tests pass at unchanged thresholds.
- Seeded ASan+UBSan fuzz: 1,000,000 cases, zero crashes, invariant failures, or sanitizer reports.
- Raster benchmark: five 512x512 workloads, 21 trials each, stable hashes; basic-solid frozen-base comparison recorded.
- Corpus: 58 files, 116 fresh trials; 2 successful previews, 56 unsupported, 0 limit, 0 crash. 100 MiB preview gate remains fail; maximum owner step 0.873 ms; RSS gate passes 12/12.
- Three fresh focused verifier rounds recorded under `artifacts/WO-M4-06/focused-verifier-*.json`.

## Blocking acceptance issue

The frozen `work/WO-M4-06.md` requires “Avoid copying or rewriting source bytes.” `src/svg/reference/expand.c` builds a generated SVG for `<use>` expansion and `src/svg/clip/clip.c` builds a scratch SVG for clip geometry. The original caller buffer remains unchanged, but these derived source buffers violate the explicit work-order constraint. The third fresh verifier still fails on this issue. Do not merge this branch or mark WO-M4-06 complete.

## Resume

Resolve the design contract before continuing: implement reference/clip traversal without generated SVG source buffers, or obtain a recorded owner/planner amendment to the frozen work order. Then rerun the frozen acceptance commands, refresh evidence for the final candidate, run a fresh read-only verifier, and merge only if it passes. M4-07 and later work orders remain pending.

## 2026-10-09 gradient correction checkpoint

The candidate now resolves user-space gradient percentages/defaults against
the root viewBox dimensions, and omitted radial focal coordinates inherit the
resolved center. Added `user-space-gradient-defaults.svg`; the focused
`rsvg-convert` pixel comparison passes. Exact linear endpoint/interpolation
checks pass in `shape_test`; `python3 -m unittest tests.test_shape_render`
passes 16/16. Debug build and `git diff --check` pass.

Still outstanding and blocking: `reference/expand.c` and `clip/clip.c` generate
SVG buffers containing copied document bytes. The direct decoder currently
requires a contiguous source byte slice, so completion needs a token visitation
path that carries instance transforms/inherited paint and decodes clip geometry
from original token spans. Do not merge or start M4-06b until that traversal,
AC-9 source-hash/bound evidence, and the full frozen checks pass.

Verification after the gradient correction: `./dev verify` passes 12/12 CTests
and 199/199 Python tests; `./dev asan` and `./dev ubsan` each pass 12/12 CTests.
The focused shape-render suite passes 16/16 tests and the new independent
`rsvg-convert` viewBox/user-space gradient comparison. No refreshed corpus or
raster benchmark run is claimed for this unmerged correction.
