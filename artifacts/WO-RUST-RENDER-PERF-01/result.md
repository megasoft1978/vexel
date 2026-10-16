# WO-RUST-RENDER-PERF-01 result

Hot spots (timing counters in `vexel-render/src/scene_tests.rs`, debug): per-sample
work in `render_core` (to_user, in-view test, edge rescan per sample), per-row
rescan of all edges, and linear scan of all stroke segments/joins per sample.
Of `compose_2000_rects_512` (~3.1 s) the render is ~1.3 s; the rest is decode/compose
in `vexel-app` (out of scope).

Kept (all exact; skip work only):
- Active-edge sweep (`scan.rs`): edges binned once by row range via binary search on
  a precomputed row-boundary table; no per-row rescan.
- Per sample-row sorted crossings with suffix windings (`fill.rs`): same integer
  winding/parity as the per-sample loop, O(log n) per sample.
- Per-axis tables for sample x/y and in-view flags (same expressions as `ViewMap`).
- Fill-only shapes: skip rows/spans without crossings; fully covered pixels use the
  once-resolved 16-sample colour, uncovered pixels are skipped.
- Stroke candidate grid (`stroke_grid.rs`): primitive boxes use the hit-test's own
  arithmetic, one monotone cell function; "any primitive hits" is order independent.

Dropped: nothing. Not ported from the legacy grid: adaptive sizing, stat counters and
the per-sample work probe (no pixel effect); the grid is a plain uniform grid with a
reference budget and falls back to the linear scan.

Before/after (debug, Apple M5 Max):

| case | before ms | after ms |
|---|---|---|
| compose_2000_rects_512 (perf_gate, median of 5 best-of-3) | 3113 (base build, same host; 3178.29 in file) | 1957 (1.62x vs file) |
| rects 2000, render only | 1295 | 152 |
| path fill 100k seg, 512 | 3344 | 87 |
| path stroke 5k seg, 512 | 175160 | 745 |
| walk fill 3k, 256 | 6277 | 451 |
| path fill 20k, 256 | 455 | 21 |
| stroke 600, 128 | 1248 | 34 |
| fill+stroke 500, 128 | 978 | 981 |
| overlap strokes+clips, 160 | 969 | 908 |

No case slower than 5 %. RSS (`/usr/bin/time -l`, whole perf_gate process, 3 runs):
base 17.1-18.7 MB, new 17.0-18.7 MB: unchanged within noise.

Differential tests: 6 scenes (2000 rects, overlapping strokes with clips and a
transform, large fills, large strokes, letterbox/slice/none views) against an
in-test naive reference, byte equal. All scene hashes equal pre-change output.

Notes: `-p vexel-svg-parity` in the frozen test command names no package.
`cargo fmt --check` already fails at base (654 diffs); new files use the repo's
compact style.
