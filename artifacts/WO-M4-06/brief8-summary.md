# WO-M4-06 review brief 8

Status: fixes and requested checks are recorded in the uncommitted worktree.

## Changes

- Boundary-map preflight now charges three document-shape scans, clip-chain
  entries, and every clip child-shape visit across both geometry passes. It
  rejects over-budget work before calling the clip edge visitor. The regression
  uses 32,768 painted rects, 60 nested group clips, and 64 zero-width rects per
  clip; it asserts the exact 257,654,784 preflight units and a `limit` render
  diagnostic. See `tests/svg/shape/test_clip_work_budget.c`.
- `svg_paint_edge_span` forms a pointer only for a nonempty, in-range span. Its
  test seam asserts the empty case returns NULL with `formed == false`, and the
  nonempty case forms the expected pointer. The zero-area gradient decode test
  also asserts the edge array remains NULL.
- Removed the extra blank line from `paint_clip_generator.py`.
- Added a `has_clips` fast path so unclipped documents skip clip-map preflight.

## Checks

- Debug and Release builds pass. `shape_test` passes. The focused Python render
  suite passes 21/21.
- Seed `20261016`: 1,000,000 ASan+UBSan fuzz cases, zero crashes, invariant
  failures, or sanitizer reports. Full report: `fuzz-report-brief8-final.json`.
- `./dev asan` and `./dev ubsan`: shape-render passes; CTest is 18/20 in each.
  The remaining failures are `native-viewer-smoke` (LaunchServices presentation
  unavailable) and `index-replay` (offscreen Metal device/queue unavailable).
- Corpus run/check passes for 58 files and 116 trials: 2 supported previews,
  56 unsupported, 0 limit, 0 crash. All 58 input hashes, manifest hash, and
  supplied Inkscape baseline values match the previous report. Evidence:
  `corpus-run-brief8-final.txt`, `corpus-check-brief8-final.txt`, and
  `corpus-hashes-brief8.json`.
- Four paired solid workloads have identical output hashes and are within
  measured noise. Base/candidate medians: circle 4.316/4.355 ms; many shapes
  46.755/46.802 ms; dashed strokes 280.255/284.293 ms; simple 4096×2048
  106.939/108.523 ms. Raw trials: `solid-workloads-round8-final2.json`.
- UBSan did not report the old `out->edges + before` expression when restored
  temporarily in the zero-edge gradient path. A minimal C17 `NULL + 0` probe
  also produced no report under Apple Clang 21 with `-fsanitize=undefined` or
  `-fsanitize=pointer-overflow`. The deterministic empty-span seam remains the
  test. Evidence: `ubsan-null-pointer-brief8.json` and its build/run logs.
- `git diff --check HEAD` and `git diff --check 9bae63e` are clean. The pure
  committed range `9bae63e..HEAD` still reports the blank line in the existing
  round-6 commit; the worktree correction is uncommitted as requested. No index,
  branch, or HEAD changes were made.

All touched C/test source files remain below 800 lines. Oracle, golden, manifest,
and threshold files are unchanged.
