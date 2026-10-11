# 02 — Rasterization and text (Q3, Q4)

## Renderer decision

**Recommendation:** own an SVG-aware scene compiler and tiled CPU rasterizer first; use Metal as a presentation/upload path. Keep `RasterBackend` swappable. A C CPU rasterizer offers deterministic output, easy test capture, and no GPU driver dependency; it is a good first viewport renderer, but it is not automatically fastest for every zoom level. Add Metal acceleration only when measurement shows rasterization, not parse/scene work, dominates. Do not begin by implementing general GPU path rendering.

### CPU design

1. Convert SVG path commands to canonical cubic/line geometry; flatten curves adaptively to screen-space tolerance (e.g. 0.25 px), preserve exact source commands for edit/save.
2. For each path, clip transformed bounds to visible tiles; build monotone edges and scan-convert with analytic area/coverage accumulation per pixel. Use winding counts for nonzero/even-odd fill. Avoid 4x supersampling as the default; reserve it for quality/debug comparisons.
3. Rasterize into 128–256 px tiles; parallelize independent tiles with bounded worker pool and immutable display-list snapshots. Keep tile cache keyed by transform/style/scale and invalidate changed bounds plus filter expansion.
4. Compose solid/linear/radial gradients, image patterns, opacity, clip paths, masks and Porter-Duff blend modes. Use premultiplied linear-light working colors where appropriate; explicitly test SVG color/compositing semantics.
5. Vectorize row coverage, span fills, color conversion and compositing using scalar baseline + dispatchable SSE2/AVX2/NEON. Keep intrinsics behind a small `simd.h`; compare exact/allowed output against scalar. Do not use SIMD where branch-heavy curve/clip setup dominates.
6. Filters are offscreen graph nodes with declared region, resolution and dependency expansion. Implement blur/offset/color matrix first; cap offscreen dimensions and expose fallback diagnostics.

### What is known from published comparisons

| Evidence | Published result | How to read it |
|---|---|---|
| **Wikimedia 2021 SVG suites** compared librsvg 2.50, resvg 0.14, Inkscape 1.0 and Batik on 337+1338+141 files. | W3C-suite CPU totals: resvg 1m04s, librsvg 1m46s, Inkscape 7m57s, Batik 6m51s. On resvg suite: resvg 2m30s vs Inkscape 46m22s. Wikimedia featured picture set: resvg 4m46s vs librsvg 5m17s vs Inkscape 15m28s. Reported visual correctness differs by suite; resvg scores 0.831 W3C and 0.956 on its own suite, Inkscape 0.745/0.729. [MediaWiki report](https://www.mediawiki.org/wiki/SVG_benchmarks) | Old versions and machine/setup; whole-file CLI render tests, not editor interaction; suite ownership/feature support affects correctness. Strong evidence a focused renderer can be much faster on these corpora, not a guaranteed 3–18× product win. |
| **Blend2D** | Its public `bl_bench` compares small shapes/path fills/strokes against AGG, Cairo, JUCE and Skia, with 8–256 px geometries; published setup notes differ by dependency/compiler. [Benchmark](https://blend2d.com/performance.html) | Good algorithmic microbench/reference and implementation target. Vendor reports performance and JIT/multithread benefits; take benchmark results as project-reported and run the tool locally on target Mac. |
| **Vello** | GPU compute-centric renderer in the Xilem stack. No directly comparable, primary-source SVG editor benchmark against Skia/resvg/Inkscape was found in this pass. | Unknown. Treat as research option, not proven Mac editor speed. GPU startup/shader compilation and upload can outweigh simple geometry. |
| **Skia** | Mature CPU/GPU 2D engine, but no single apples-to-apples public SVG benchmark found that predicts this project’s large-scene interaction. Skia has `skpbench` support for SVG input in source. [skpbench](https://github.com/google/skia/blob/main/tools/skpbench/skpbench.cpp) | Benchmark own scene/cache path and same machine; “Skia is fast” does not answer parse/open/selection latency. |
| **pugixml / XML** | Not rasterization. See parser doc: public benchmark uses 1–20 MB files and documents markup-heavy memory overhead. | Do not conflate parser speed with rendered first-pixel latency. |

### GPU path options

| Approach | Upside | Cost / recommendation |
|---|---|---|
| GPU compute path rasterization (Vello-like) | Scales coverage work across GPU, can merge many paths and compose in tiles | Complex shader/compiler, GPU resource lifetime, synchronization, driver variation, readback/testing, Metal-specific implementation; prototype after CPU baseline. |
| Stencil-and-cover | Mature path technique; GPU handles coverage and fill | Stencil complexity, path complexity and overdraw; API availability/platform correctness. Do not make first renderer. |
| Tessellate paths to triangles | Reuse conventional GPU pipelines; simple shapes fast | Robust curves/intersections/self-overlap are hard; triangle count and antialias seams; unsuitable as a universal fill engine. |
| MSDF / Slug-style text | Fast scalable glyph rendering with texture/analytic approaches | Text-only path; specialized implementation and licensing/evaluation required. Not SVG path renderer. |
| CPU tiles → GPU texture | Portable visual semantics, easy deterministic baseline, cheap Metal integration | Full viewport tile upload can be bandwidth-heavy; partial dirty rect uploads and double buffering. **Recommended M0/M1.** |

**From-scratch LOC estimate (confidence low–medium):** competent solid-fill/stroke CPU raster core 8k–15k C LOC; production gradients/clips/images and blend modes 15k–25k; masks/patterns/common filters and robust SVG semantics push the renderer path to 25k–45k, excluding parser/editor/UI. Test corpus may be as large as implementation. At 1k LOC/week effective solo throughput, this is 6–12+ months before broad fidelity. These are planning estimates, not published project counts. Vendor permissive PlutoVG/ThorVG/Blend2D only after license + semantics review; they may save time but violate the ideal own-rasterizer constraint. [PlutoVG](https://github.com/sammycage/plutovg) · [ThorVG](https://github.com/thorvg/thorvg)

## Text strategy

“Own text” should mean owning editor layout/state and converting glyph outlines to the same display pipeline where possible, not implementing OpenType shaping from scratch.

| Option | What it gives | Cost / gaps |
|---|---|---|
| Own TrueType subset (`glyf`, `loca`, `cmap`, `head`, `hhea`, `hmtx`, `maxp`, `name`, basic variation later) | No runtime dependency; extract outlines/advances; small simple Latin subset possible | Font format security/parser burden, CFF/CFF2, variations, hinting, color fonts, TTC and licensing/document embedding issues. `stb_truetype` is a permissive single-header example; check current license/version. [stb](https://github.com/nothings/stb) |
| Own glyph outlines + own raster | One geometry path for SVG and text; consistent zoom/edit | No shaping: Arabic/Indic/SEA joining/reordering, ligatures, mark positioning, contextual forms and OpenType GPOS/GSUB fail; kerning only covers simple legacy `kern` if implemented. |
| System shaping: CoreText / DirectWrite / Linux font stack | Correct font fallback, shaping, metrics, IME integration; avoids embedding fonts | OS-dependent output and font metrics; need backend adapters and deterministic test font fixture. CoreText is Apple’s text framework [CoreText](https://developer.apple.com/documentation/coretext); DirectWrite offers font/layout APIs [guide](https://learn.microsoft.com/en-us/windows/win32/directwrite/programming-guide). Linux usually needs fontconfig + FreeType + HarfBuzz, which are system deps or must be bundled. |
| HarfBuzz + FreeType bundled | Consistent shaping and broad formats on all OSes | Adds dependencies, build/size/license work; contrary to “own everything,” but cheapest parity path. [HarfBuzz](https://github.com/harfbuzz/harfbuzz) |

### Text MVP

- UI chrome: system shaping/text APIs, platform font fallback, baseline metrics and selection/caret/IME.
- SVG text read-only in M0: basic `font-family`, size, weight, anchor, x/y/dx/dy, spans and whitespace; preserve unknown text attributes. Draw shaped runs from OS API or cache glyph outlines where available.
- M2 editing: plain text and basic per-span style edits, OS-backed shaping, caret/selection, SVG `<text>/<tspan>` round-trip.
- Defer vertical writing, complex text-on-path fidelity, advanced OpenType features UI, variable/color fonts editing, embedded fonts, exact cross-platform pixel identity.

**System dependency recommendation:** Mac CoreText, Windows DirectWrite; Linux provisionally use `fontconfig + FreeType + HarfBuzz` (distro libraries) because there is no universal Linux system shaper equivalent. A bundled FreeType/HarfBuzz build may be required for reproducible Linux package behavior. This is a deliberate exception to strict zero-dependency.
