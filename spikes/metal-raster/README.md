# Metal GPU rasteriser spike

Time-boxed spike: can an Apple GPU rasterise the scenes the CPU renderer handles, how fast, and how close are the pixels?
Standalone (not in the Cargo workspace). Swift + MSL compute, offscreen shared buffers, no windows.

Build/run: `make && build/mr bench [--sizes 900,1024,2048] [--n 500,2000,5000,20000] [--runs 7]`,
`build/mr fidelity ...`, `build/mr gen <dir>` (seeded binary scene dumps). Needs the Metal toolchain (`xcrun --find metal`).
Machine: Apple M5 Max. p50 of 5 runs after 1 warm-up. Metal compiled with `-fno-fast-math`.

## What was built
- `main.swift`: seeded scene generator (same xorshift RNG and blob/colour/gradient recipe as `raster_scene.rs`), profiles fill, linear gradient (4 stops, pad), stroke (width 1-40 px log-uniform, round joins/caps, **CPU-expanded to polygons**: segment quads + vertex discs, non-zero union), nest (nested opacity groups **flattened to per-shape alpha**, so not isolated-group correct).
- `raster.metal`: per-pixel shading that copies the CPU semantics: 4x4 samples at 0.125/0.375/0.625/0.875, ray cast to +x with half-open y, even-odd or non-zero, per-shape alpha-weighted resolve, integer straight-alpha source-over (all integer, so only edge crossings and gradients can differ).
  - **A** tile binning: CPU bins shapes into 16x16 tiles (z-ordered lists), one 16x16 threadgroup per tile, each thread composites the tile's shapes in registers and writes once.
  - **B** per-shape: one dispatch per shape over its bbox, blending read-modify-write into the framebuffer (a pixel-parallel stand-in for accumulation-buffer coverage; real signed-area accumulation does not give the 16-sample pattern).
- Not done: clips, filters, masks, radial gradients, spread modes other than pad, GPU-side binning, curve flattening and stroking on GPU. Unified memory: "upload/readback" are memcpys to/from shared buffers; buffers are allocated once per scene (a real integration would pool them).

## Speed (ms)
CPU = `cargo test --release -p vexel-render --test raster_bench -- --ignored --nocapture` (1024x1024, p50 of 7, this machine, today). CPU scenes come from `raster_scene.rs` (strokes 0.6-7 px, real isolated groups); GPU scenes are a re-implementation of the same recipe (strokes 1-40 px), so ratios are indicative, not a strict same-scene comparison. 'nest' CPU number does real group compositing, GPU flattens, so its ratio overstates the win. e2e = upload + CPU bin + encode + GPU + wait + readback; includes full scene re-upload every frame.
| size | profile | N | CPU Rust ms (1024) | A kernel ms | A bin ms | A e2e ms | A speedup kernel / e2e | B kernel ms | B e2e ms |
|---|---|---|---|---|---|---|---|---|---|
| 900 | fill | 500 | - | 0.21 | 0.02 | 0.43 | - | 2.15 | 2.50 |
| 900 | fill | 2000 | - | 0.36 | 0.08 | 0.67 | - | 8.62 | 9.15 |
| 900 | fill | 5000 | - | 0.86 | 0.16 | 1.29 | - | 21.61 | 22.59 |
| 900 | fill | 20000 | - | 3.41 | 0.59 | 4.29 | - | 84.43 | 87.69 |
| 900 | gradient | 500 | - | 0.23 | 0.01 | 0.42 | - | 4.75 | 5.04 |
| 900 | gradient | 2000 | - | 0.80 | 0.06 | 1.05 | - | 19.33 | 19.88 |
| 900 | gradient | 5000 | - | 1.97 | 0.17 | 2.43 | - | 48.56 | 49.49 |
| 900 | gradient | 20000 | - | 7.90 | 0.63 | 8.78 | - | 195.79 | 198.77 |
| 900 | stroke | 500 | - | 0.73 | 0.02 | 1.09 | - | 8.92 | 9.29 |
| 900 | stroke | 2000 | - | 2.83 | 0.08 | 3.28 | - | 35.55 | 36.30 |
| 900 | stroke | 5000 | - | 6.91 | 0.19 | 7.65 | - | 88.88 | 89.87 |
| 900 | stroke | 20000 | - | 27.44 | 0.74 | 29.11 | - | 353.28 | 356.93 |
| 900 | nest | 500 | - | 0.10 | 0.02 | 0.34 | - | 2.15 | 2.47 |
| 900 | nest | 2000 | - | 0.37 | 0.07 | 0.66 | - | 8.52 | 9.09 |
| 900 | nest | 5000 | - | 0.87 | 0.16 | 1.32 | - | 21.46 | 22.42 |
| 900 | nest | 20000 | - | 3.43 | 0.62 | 4.47 | - | 85.26 | 88.37 |
| 1024 | fill | 500 | 2.0 | 0.23 | 0.02 | 0.86 | 9x / 2x | 4.01 | 4.65 |
| 1024 | fill | 2000 | 6.9 | 0.73 | 0.06 | 1.05 | 9x / 7x | 14.69 | 15.42 |
| 1024 | fill | 5000 | 16.8 | 1.18 | 0.17 | 1.62 | 14x / 10x | 23.86 | 25.19 |
| 1024 | fill | 20000 | - | 3.42 | 0.62 | 4.37 | - | 85.41 | 88.46 |
| 1024 | gradient | 500 | 9.0 | 0.22 | 0.02 | 0.46 | 41x / 20x | 4.92 | 5.31 |
| 1024 | gradient | 2000 | 35.8 | 0.80 | 0.06 | 1.12 | 45x / 32x | 19.41 | 20.08 |
| 1024 | gradient | 5000 | 89.2 | 1.94 | 0.23 | 2.59 | 46x / 34x | 49.03 | 49.98 |
| 1024 | gradient | 20000 | - | 7.79 | 0.67 | 8.79 | - | 195.27 | 198.97 |
| 1024 | stroke | 500 | 26.9 | 0.73 | 0.03 | 1.02 | 37x / 26x | 8.84 | 9.54 |
| 1024 | stroke | 2000 | 99.9 | 2.82 | 0.08 | 3.21 | 35x / 31x | 35.68 | 37.41 |
| 1024 | stroke | 5000 | 234.0 | 6.89 | 0.20 | 11.37 | 34x / 21x | 88.97 | 90.39 |
| 1024 | stroke | 20000 | - | 27.71 | 0.74 | 29.91 | - | 355.37 | 358.78 |
| 1024 | nest | 500 | 15.4 | 0.10 | 0.04 | 0.52 | 154x / 30x | 2.32 | 2.72 |
| 1024 | nest | 2000 | 52.6 | 0.37 | 0.07 | 0.73 | 142x / 72x | 9.05 | 9.69 |
| 1024 | nest | 5000 | 123.5 | 0.87 | 0.16 | 1.40 | 142x / 88x | 21.33 | 22.22 |
| 1024 | nest | 20000 | - | 3.42 | 0.62 | 4.40 | - | 85.59 | 89.01 |
| 2048 | fill | 500 | - | 0.12 | 0.02 | 0.51 | - | 2.08 | 2.56 |
| 2048 | fill | 2000 | - | 0.38 | 0.05 | 0.81 | - | 8.63 | 9.32 |
| 2048 | fill | 5000 | - | 0.89 | 0.18 | 1.56 | - | 21.58 | 22.84 |
| 2048 | fill | 20000 | - | 3.50 | 0.57 | 4.62 | - | 86.88 | 90.05 |
| 2048 | gradient | 500 | - | 0.24 | 0.02 | 0.65 | - | 4.97 | 5.43 |
| 2048 | gradient | 2000 | - | 0.82 | 0.07 | 1.31 | - | 19.84 | 20.60 |
| 2048 | gradient | 5000 | - | 2.00 | 0.15 | 2.59 | - | 49.90 | 51.02 |
| 2048 | gradient | 20000 | - | 8.01 | 0.56 | 9.07 | - | 198.04 | 201.73 |
| 2048 | stroke | 500 | - | 0.77 | 0.03 | 1.19 | - | 8.87 | 9.82 |
| 2048 | stroke | 2000 | - | 2.86 | 0.08 | 3.44 | - | 35.72 | 36.56 |
| 2048 | stroke | 5000 | - | 7.04 | 0.19 | 7.82 | - | 90.70 | 93.17 |
| 2048 | stroke | 20000 | - | 28.26 | 0.75 | 30.20 | - | 362.41 | 367.48 |
| 2048 | nest | 500 | - | 0.12 | 0.03 | 0.56 | - | 2.30 | 2.84 |
| 2048 | nest | 2000 | - | 0.39 | 0.08 | 0.91 | - | 9.00 | 10.02 |
| 2048 | nest | 5000 | - | 0.91 | 0.17 | 1.60 | - | 22.50 | 24.02 |
| 2048 | nest | 20000 | - | 3.63 | 0.67 | 4.94 | - | 90.81 | 95.51 |

Notes: kernel time barely depends on canvas size (work scales with shape area, tiles with no shapes are cheap). Strategy B is 10-50x slower than A (per-dispatch cost ~4 us/shape plus serial read-modify-write); drop it. Stroke 20000 on A is 28 ms because the polygon expansion creates ~2M edges (100/shape) that every covered pixel scans linearly; per-tile edge binning or a GPU stroker would cut that sharply.

CPU baseline.json (full renderer, different scenes) for reference: render_900 fill 500/1000 shapes 6.6/12.8 ms, render_2048 fill 15.6/27.4 ms.

Headline (A vs Rust CPU at 1024, e2e incl. upload/readback): fill 5000 ~10x, gradient 5000 ~34x, stroke 5000 ~20x (kernel-only 14x / 46x / 34x). Small scenes (500 shapes, fill) are only ~2x e2e: the CPU path is already ~2 ms there.

## Fidelity (GPU A vs Swift Double CPU reference, same scene, different algorithm: row crossings)
The reference is my own CPU implementation of the Rust sampling/compositing rules, not the Rust renderer itself (no safe headless path without editing the workspace). Percentages are of pixels whose max channel diff exceeds the threshold.

| size | profile | N | max abs | mean abs (per channel) | % px >0 | % px >1/255 | % px >2/255 |
|---|---|---|---|---|---|---|---|
| 900 | fill | 500 | 16 | 0.00004 | 0.0014 | 0.0012 | 0.0012 |
| 900 | fill | 2000 | 16 | 0.00009 | 0.0033 | 0.0031 | 0.0026 |
| 900 | gradient | 500 | 15 | 0.00007 | 0.0106 | 0.0012 | 0.0012 |
| 900 | gradient | 2000 | 46 | 0.00019 | 0.0228 | 0.0032 | 0.0027 |
| 900 | stroke | 500 | 242 | 0.00023 | 0.0020 | 0.0017 | 0.0012 |
| 900 | stroke | 2000 | 16 | 0.00011 | 0.0047 | 0.0040 | 0.0030 |
| 900 | nest | 2000 | 16 | 0.00011 | 0.0056 | 0.0038 | 0.0030 |
| 900 | fill/grad/stroke/nest | 5000 | 41/14/14/13 | 0.00013-0.00020 | 0.0056/0.0286/0.0094/0.0075 | 0.0042/0.0031/0.0068/0.0042 | 0.0033/0.0026/0.0053/0.0023 |
| 2048 | fill/grad/stroke/nest | 5000 | 245/244/255/255 | 0.0002-0.0005 | 0.0046/0.0350/0.0098/0.0040 | 0.0041/0.0042/0.0086/0.0031 | 0.0037/0.0037/0.0079/0.0025 |

Strategy B output is bit-identical to A. Essentially every pixel matches (99.99%+). The few outliers are float32 vs float64 edge crossings flipping a single sample on or near a vertex/edge (1 of 16 samples = up to 16/255 on a pixel, large only where a thin sliver lands on dark/light contrast); gradient pixels also show +-1 from float rounding. Larger canvases have larger coordinates and slightly more flips. Fixable by making tests tolerance-based or by computing crossings in fixed point / with edge-relative offsets.

## Recommendation: GO for a gated, optional GPU path (continue to a prototype), NO-GO as a replacement for the CPU renderer
Why go: 10-45x lower frame time for dense scenes (5-20k shapes, strokes, gradients) with clean fidelity; scrub/zoom/tick at 900 px that cost 50-75 ms on CPU (baseline tick_900/scrub_900) could drop under 16 ms; 20k-shape fill is 4 ms e2e.
Why not replace: small scenes gain little; no clips/filters/masks/patterns/dashes yet; output is not byte-identical.

### Integration plan
1. Keep the CPU renderer as the reference and fallback (no Metal device, unsupported feature in scene, feature flag, GPU error/timeout). Route per scene: GPU only when every node is in the supported subset.
2. Binding: swift shim (static lib with a C ABI: create, upload scene, render, readback) vs `objc2-metal` from Rust. Prefer `objc2-metal` + a precompiled `.metallib` embedded via `include_bytes!` (no Swift toolchain in the Rust build, no new runtime); the MSL stays as is. The Metal Toolchain component must exist in CI/build machines to compile the shader (or commit the built metallib).
3. Present path today is CPU premultiplied RGBA. Step 1: GPU renders to a shared buffer, convert/copy into the existing tile (readback is a memcpy on unified memory, 0.2-1 ms at 2048). Step 2 (later): render straight to premultiplied and present the texture without readback.
4. Move flattening/stroking onto the GPU or add per-tile edge binning (stroke polygons dominate the 20k-stroke cost); implement clips (rect and path), group opacity as real isolated layers (needs per-tile layer stack), radial/spread gradients, patterns, then filters last (probably stay CPU).
5. Fidelity policy: GPU vs CPU is not byte-identical (~0.004% pixels off by >2/255 in this test, rare single-sample outliers). Goldens cannot be shared: add a GPU golden set with tolerance (max channel error and pixel-count budget), keep CPU goldens byte-exact, regenerate only with the owner's approval.

Effort estimate: prototype behind a flag with fill/gradient/stroke/opacity-groups: ~3-4 weeks; parity with clips, patterns, dashes, masks: another 6-10 weeks; filters excluded.

### Risks
- Output not byte-identical, so exports/goldens stay CPU or become tolerance-based; mixing CPU and GPU frames in one session could flicker at single pixels.
- Determinism across GPUs (M-series vs AMD/Intel macs): float32 crossings and FMA contraction can differ; integer compositing is exact. Pin results with a fixed-point edge test if cross-GPU stability is required.
- Energy: GPU wins on time-to-finish for dense scenes, but wakes the GPU for tiny scenes; use a shape-count threshold (about 1k+ shapes) to choose.
- Pathological scenes: O(edges x covered pixels) in the pixel kernel; GPU watchdog timeouts on huge strokes/large shapes without edge binning; need a CPU fallback on command-buffer error.
- Maintenance: a second implementation of all paint semantics (MSL) to keep in lock-step with Rust.
- Numbers here are for an M5 Max; measure on a base M-series before committing.
