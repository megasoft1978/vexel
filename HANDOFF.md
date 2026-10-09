# WO-M4-06 handoff

All five P2 review findings are repaired on branch `wo/WO-M4-06` from
`0732b8e`: symbol `clip-path` returns a stable unsupported diagnostic; plain
and bound `xlink:href` gradient references are rejected; clipPath children are
checked through the stylesheet cascade; gradient stop opacity composites over
prior samples; and parsed gradient definitions are cached with token work
counting. Tests cover the regressions, including a 512-shape/large-comment
work-count case. The gradient overpaint fixture passes independently against
`rsvg-convert`.

The `main` raster-culling changes are in the worktree from merge commit
`10a9878` (second parent `ae2b673`). The renderer retains shape culling,
scanline bucketing, paint/clip sampling, and clip-aware boundary refinement.
The scalar oracle and frozen thresholds were not changed. The seeded
culling differential test and 47 librsvg fixtures pass. The five-workload Release
paint benchmark was rerun with 21 trials per workload; stable hashes and all
per-workload/frozen-base outcomes are in `paint-benchmark.json` and
`bench/results/WO-M4-06.json`; no performance claim is made.

Verification: Debug/Release configure and build pass; focused Python tests are
18/18; `shape_test` passes; sanitized fuzz passes 1,000,000 cases with zero
crashes, invariant failures, or sanitizer reports; corpus run/check passes for
58 files and 116 trials. Debug, ASan, UBSan, and `./dev verify` CTest each pass
18/20. `native-viewer-smoke` cannot connect to LaunchServices/Hiservices and
presents zero frames; `index-replay` reports that no offscreen Metal device or
queue is available. JSON, work-order, ledger, and diff checks are recorded in
`artifacts/WO-M4-06/`.

The conflict resolutions are present in the worktree, but the merge is not
staged or finalized: Git cannot create `.git/worktrees/WO-M4-06/index.lock` in
this sandbox. The post-merge path guard is not verified. The maintainer must
stage and complete the merge; no commit was created. `shape.c` remains 799
lines. No nested `codex exec` review was run.
