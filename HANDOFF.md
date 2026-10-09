# WO-M4-06 handoff — round 7

The missing clip-boundary edges caused the reported transparent pixel. The old
boundary map covered painted-shape edges but omitted child edges from the clip
chain, so a pixel whose center landed on the clip boundary bypassed the 4×4
path. The map now visits clip children and parent clips, transforms their edges
into device space with the element mapping, and conservatively marks the 4×4
sample footprint. Work accounting includes edge transforms/visits and map
marks.

Coverage is the fraction of 16 subsamples inside both shape and clip. Gradient
paint is evaluated once at the pixel center. The unculled reference always
evaluates all 16 shape-and-clip samples for every shape and pixel, with no map
or center shortcut. The new linear/radial/rotated clip fixture's pixel (20,4)
now matches librsvg exactly: `(124,0,131,128)`. Source-over gradient overpaint
still produces `(127,0,128,255)`; opaque cases stay unchanged. These semantics
are recorded in `src/raster/README.md`.

The seeded paint/clip differential covers 160 generated documents and passes
byte-for-byte against the shortcut-free reference. The current librsvg report
contains 48 fixtures: 47 pass, while `clip-union` exceeds the unchanged raw
straight-RGB threshold by 3 (delta 4, limit 1). At its partially covered edge
pixel (8,4), Vexel gives `(48,160,96,16)` and librsvg `(44,158,97,29)`. The
defined 4×4 coverage has one covered sample and retains the solid paint color;
librsvg's analytic antialiasing differs at this low-alpha edge. No threshold or
oracle was changed. This is the remaining acceptance blocker.

Verification:

- Debug and Release builds pass; `build/mac-debug/bin/shape_test` passes.
- `python3 -m unittest tests.test_shape_render`: 21 tests run, 20 pass, one
  fails on the `clip-union` frozen threshold described above.
- `./dev shape-fuzz --duration 60 --seed 20261015`: 1,000,000 ASan+UBSan cases
  pass in 29.890 s; zero crashes, invariant failures, and sanitizer reports.
- Debug CTest and `./dev asan`, `./dev ubsan`, and `./dev verify` each report
  18/20 CTests. The only failures are `native-viewer-smoke` (LaunchServices
  unavailable) and `index-replay` (offscreen Metal device/queue unavailable in
  this sandbox).
- Corpus run/check: 58 files, 116 fresh trials, 2 previews and 56 unsupported;
  no limit/crash outcomes and input hashes unchanged.
- All changed source/test files are under 800 lines. Current six-workload
  benchmark medians and paired solid comparisons are recorded in
  `docs/PERF.md`, `artifacts/WO-M4-06/paint-benchmark.json`,
  `artifacts/WO-M4-06/solid-workloads-round7.json`, and
  `bench/results/WO-M4-06.json`.

The work remains uncommitted. No branch, index, or commit state was changed.
Evidence for this round is under `artifacts/WO-M4-06/`, including
`verify-round7.txt`.
