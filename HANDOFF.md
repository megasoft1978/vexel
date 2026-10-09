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
