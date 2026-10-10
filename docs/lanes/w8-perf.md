# W8 huge-file raster performance

## Changes

Added an eligibility-gated scanline fill path in `src/raster/shape_scanline_fill.c`.
It reuses bounded crossing, interval, and row-run storage; it handles filled
shapes with one RGBA color across their edges, no active stroke, and at most
4096 edges. Pixel-run scratch is capped at 8 MiB. Other shapes retain the
existing per-sample path.

`src/raster/shape_render_fill.c` paints samples only within scanline intervals.
It keeps the existing center-sample behavior for interior pixels, the 4×4
antialiasing path for boundary pixels, and the existing gradient, clip, and
source-over paint code for covered samples. `src/raster/shape_render_preflight.c`
accounts for interval setup and range traversal before accepting a render. The
sample compositor exposes a shared post-membership paint operation so the fast
path avoids repeating edge membership work.

Added `shape-render-large-work`, which checks the frozen 512×512 map pixel hash
and renders the same real-world fixture at 4096×4096 under the 200,000,000 work
limit. The test was run before the implementation and failed at 4K with
`limit:0; work=0`.

## Evidence

- Final Release `./dev bench-corpus`: 58 files, 116 fresh trials, 2 previewable
  (`natural-earth-world-map-blank` and `natural-earth-world-map-countries`), 56
  unsupported, 0 limit, 0 crash. The count is for index/open preview coverage,
  so raster changes do not increase it; the reported count remains 2/58.
- At 3072×3072, estimated work fell from 191,961,695 to 118,891,388 units.
  Fill-edge visits fell from 109,871,984 to 949,140, while shape samples fell
  from 58,721,155 to 28,774,119; the new path recorded 51,886,613 interval/range
  tests.
- At 4096×4096, the test reports 189,245,433 work units, below the 200,000,000
  cap.
- `./dev pixdiff` compared pre-change and optimized Release 3072×3072 map
  images with tolerance 0: identical pixels. Their raw RGBA buffers are also
  byte-equal with SHA-256
  `9692e96270ed3c227f901cc2f2207475327d2cc3118a75ddf5e66fac085c3b65`. The
  regression test checks frozen 512×512 output SHA-256
  `9e0af1a5d014af710fabb39fa152733f50c662ae1f2dc4bfe4a1d7cb3a3433a4`.
- `shape-render` and `shape-render-large-work` pass. `./dev t`, `./dev t --all`,
  and `./dev t --full` each run all raster checks but report the same two
  sandbox-dependent CTest failures: `index-replay` cannot get an offscreen Metal
  device/queue, and `native-viewer-smoke` cannot use LaunchServices. The full run
  also passes all 41 `realworld.test_corpus` tests.

## Limits

No decoder or index-preview support was changed. The corpus preview count stays
2/58 because the unsupported outcomes occur before full rasterization. Decode
costs were not part of the raster work counters. The sandbox has no UDS-backed
native UI/service path, so that integration is unverified. Metal preview and
the LaunchServices viewer smoke are also unverified here. No output golden,
acceptance threshold, dependency, or platform support was changed.
