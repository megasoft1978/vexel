# 04 — Benchmark protocol (Q6)

## Rule: build this before the editor

Benchmark decisions have to separate parse, scene compile, raster, UI and document model. Compare release builds on fixed machines and files; keep cold/warm cache runs distinct. Every run records OS build, CPU/GPU/RAM, power mode, app commit, compiler flags, fixture SHA-256, font set, display resolution/scale, renderer backend and cache state. Capture raw samples; publish median and p50/p95/p99, not only “best.” Randomize tool order and repeat at least 5–10 times after warm-up. Avoid screenshots/recording during timing unless all products get identical treatment.

## Products and versions

- Inkscape **1.4.x latest stable available at test date** plus **1.5 development build pinned to commit/hash**; command-line `inkscape --version` and build config stored.
- New editor release/dev; resvg CLI/library; `rsvg-convert` (librsvg); Chrome/Chromium SVG viewer; optional Affinity/Illustrator only if licensed locally and scripted actions are repeatable. Keep browser versions pinned. Results across dissimilar products are descriptive, not absolute ranking.
- Compare the same SVG and pixel viewport; note unsupported features and output differences. Renderer CLI tests isolate raster performance; GUI runs measure editor overhead.

## Corpus tiers

| Tier | Fixture profile | Purpose |
|---|---|---|
| 1 MB | Natural authored SVG; ~few thousand mixed primitives, text/gradient/clip; one image | Startup + normal editing baseline |
| 10 MB | 10k/100k object variants, separate high-node path and text-heavy fixture | Realistic larger file, parse/model overhead |
| 100 MB | GIS/map, long-path plot, embedded-image fixture as separate dimensions | memory growth / incremental first paint |
| 500 MB + 1 GB | Deterministically generated simple-element, long-path, base64-image and text fixtures; pin generator and SHA | Stress limits, progressive load, OOM behavior; do not claim representative of all SVGs |

Track object count/path segments/UTF-8 chars/image decoded pixels/depth/features separately from file bytes. Include W3C SVG tests and resvg tests for correctness but they are not size tiers. Existing public data include a 1.96 MB Natural Earth world map and Natural Earth GIS source downloads up to hundreds of MB; fixed 1 GB SVG corpus is unknown today.

## Measurements and protocol

| Metric | Exact definition |
|---|---|
| Startup | process launch to first interactive window; separately cold launch and warm launch |
| Open time | user open action to parsed document model ready; record UI responsiveness and errors |
| Time to first pixel (TTFP) | open action to first non-background canvas frame with correct visible content; capture via frame markers/screen probe and application signpost |
| Full settle | all visible tiles/render tasks done and no spinner/progress; report separately from TTFP |
| Peak/steady memory | peak RSS during open; settled RSS after 30s idle; include app child processes and GPU allocations when tool permits; report RSS and footprint separately |
| Pan/zoom | deterministic script: 20 pan steps, zoom in/out and rotation if supported; record input-to-present frame latency p50/p95/p99, dropped frames and stale frames |
| Selection/hit test | click visible path, select via layers, box select; input-to-selection overlay latency; repeat near dense geometry and offscreen items |
| Move/transform | drag selected object 100 px, scale/rotate; latency until updated canvas; commit/undo timing |
| Save | save original unchanged, save one attribute edit, save full structural edit; elapsed time, output bytes, semantic diff and source-preservation result |
| Binary/bundle | stripped release executable and installed `.app`/package compressed/uncompressed; report universal vs single architecture and resources |
| Idle CPU/power | idle with document open, 60 s sample; macOS Energy Impact/Instruments where available; screen refresh kept fixed |

Automation: use app CLI to open named document if supported; otherwise AppleScript/AX or scripted UI. Keep the same action script and verify each event actually occurred. For pure render, invoke renderer at fixed dimensions and compare PNG output/perceptual diff.

## macOS tools

- `/usr/bin/time -l app ...` for wall time/max resident set (CLI only; GUI launch wrappers need process-tree collection).
- `hyperfine` for repeated CLI startup/render runs; pin CPU/power and warm-up, avoid using only minimum time.
- Instruments: Time Profiler, Allocations, Points of Interest/signposts, System Trace; add `os_signpost` for parse, style, compile, tile, present, hit-test and save phases.
- `vmmap <pid>`, `footprint <pid>` or Activity Monitor for VM/physical footprint; distinguish mapped file pages from resident memory and note shared memory.
- Metal System Trace / Xcode GPU capture for GPU submissions, upload and wait costs; Metal HUD availability depends on build/environment. Record exact tool and version; do not infer “GPU time” from wall-clock alone.
- Instruments Energy Log / `powermetrics` where permissions allow for idle energy; keep laptop power mode and display identical.
- On Windows use WPR/WPA, ETW, GPUView, Process Explorer; Linux perf, `/usr/bin/time -v`, heaptrack/massif, perfetto where useful.

## Harness layout

```text
bench/
  README.md                 # protocol, machine image, tool versions
  corpus/manifest.toml      # pinned URLs, license, hashes, dimensions/features
  generators/               # deterministic SVG stress-fixture generator
  scripts/run_open.py       # launch/open + markers + process sampling
  scripts/run_actions.py    # pan/zoom/select/move interaction script
  scripts/render_matrix.py  # CLI render, image compare
  scripts/collect_macos.sh  # time, footprint, Instruments export
  baselines/<machine>/<commit>/results.jsonl
  reports/                  # generated charts and regression summaries
```

Do not commit licensed customer SVGs or personal documents; store private fixture manifests locally and publish synthetic reproductions.

## Continue/stop wedge gates

These are proposed product gates, not predicted results. Choose one Apple Silicon baseline Mac and one Intel/low-memory reference if supported. After M0 revise once based on actual corpus, then freeze.

| Gate | M0 target for clean/basic 10 MB fixture | Rationale |
|---|---:|---|
| TTFP | ≤1 s / ≥5× faster than Inkscape 1.4.x where baseline >1 s | Viewer feels immediate; 10× is a stretch, not prerequisite. For the specific large-file wedge require ≥5× TTFP or documented progressive value. |
| Peak RSS | ≤1.5× source bytes + 250 MB on simple path/object fixture; ≥2× lower than Inkscape for large test | RSS depends on expansion/content; target measurable on named fixture. |
| Pan/zoom | p95 ≤33 ms at 60 Hz for cached tiles; p99 ≤50 ms | Includes compositor and input delay; cold tile work reported separately. |
| Selection/move | p95 ≤50 ms for 100k simple visible objects; no whole-document freeze | Initial MVP target, not full complexity guarantee. |
| Binary | ≤10 MB stripped executable for viewer core is aspirational; app bundle ≤40 MB before fonts/assets, ≤80 MB with frameworks/resources | Metal/Obj-C/system APIs are system-linked; universal binary and bundled font libraries change result. Measure actual. |
| Open | Must show first content progressively on 100 MB simple map without allocating 3× source size | Better than monolithic parse even if complete compile takes longer. |

Continue beyond M0 if a material wedge appears in at least one user-valued dimension (e.g. ≥5× first paint and ≥2× lower peak RSS) without unacceptable fidelity/data loss. If the only win is “C binary is smaller” but no interaction advantage, stop renderer investment and reassess. Compare also against current 1.5 dev: 1.3 release notes already describe async multithreaded/tiled canvas work with reported 2–4× zoom/pan/transform gains on multicore systems; our target must be measured against contemporary builds, not an old baseline.
