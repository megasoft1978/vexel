# Executive summary

**Research checked 8 October 2026.** Confidence: high on Inkscape direction and SVG semantics; medium on competitor capabilities; low on cross-product binary/RSS comparisons because comparable public measurements were not found.

## Verdict

No mature product found combines Inkscape-level editing parity, faithful Inkscape-SVG round-tripping, small native binaries, large-document performance, and a first-class SVG animation timeline. Existing tools cover parts: Inkscape is the parity target; Glaxnimate authors animated SVG/Lottie; Graphite is an alpha Rust graphics suite; SVG-Edit and Boxy are SVG-focused; Synfig, Rive and commercial tools focus on motion or other vector workflows. [Inkscape SVG/animation status](https://inkscape.org/en/develop/about-svg/) · [Glaxnimate formats](https://docs.glaxnimate.org/en/formats.html) · [Graphite](https://github.com/GraphiteEditor/Graphite)

A rewrite is technically feasible, but **C is not inherently faster**. Inkscape already renders asynchronously across threads and tiles; data model, invalidation, filters, text, path complexity and compatibility are the hard work. Inkscape 1.5 development includes GTK 4, CapyPDF, a new color system and a renderer effort, so first prove a measurable advantage over current development builds. [Inkscape 1.3 performance](https://wiki.inkscape.org/wiki/Release_notes/1.3) · [1.5 notes](https://wiki.inkscape.org/wiki/Release_notes/1.5) · [June 2026 developer meeting](https://lists.inkscape.org/hyperkitty/list/inkscape-devel%40lists.inkscape.org/2026/6/)

**Recommendation:** do a 6–8 week feasibility spike only if the owner accepts a focused initial scope. Prefer C++20 or Rust for the core with a stable C ABI; if C is fixed, use C17, opaque APIs, strict ownership rules, sanitizers, fuzzing and static analysis. Keep original SVG XML authoritative and derive editable/render indexes incrementally. Benchmark `resvg`, ThorVG and Blend2D before selecting a renderer; none is a full Inkscape-compatible engine. [resvg](https://github.com/linebender/resvg) · [ThorVG](https://github.com/thorvg/thorvg) · [Blend2D](https://github.com/blend2d/blend2d)

## Top 10 decisions

1. Prove a wedge: large-document responsiveness plus safe SVG animation authoring; defer full parity.
2. Use a clean-room implementation with provenance records; copying Inkscape source changes the license analysis. [license](https://inkscape.org/en/about/license/) · [repo license notes](https://github.com/inkscape/inkscape/blob/master/COPYING)
3. Keep SVG XML the source of truth; preserve unknown nodes and namespace data.
4. Select C++20/Rust+C ABI by default; choose C only as a firm constraint, not a speed claim.
5. Use HarfBuzz with FreeType or platform font backends. [HarfBuzz](https://github.com/harfbuzz/harfbuzz)
6. Use an accessible retained-mode desktop UI; spike Qt 6 packaging/licensing against GTK 4.
7. Separate SVG semantics, style/layout, display list and replaceable raster backends.
8. Store timeline metadata privately but compile supported tracks to SMIL/CSS with a static fallback.
9. Ship viewer → editing core → prioritized parity → animation.
10. Measure package size, RSS and p95 interaction latency on named Mac-first workloads before setting firm budgets.

## Top 5 risks

- **Parity scale:** tools, extensions, filters, text, export and edge cases represent many person-years; solo full parity is not realistic on a short schedule.
- **Semantic fidelity:** SVG combines XML, CSS, filters, fonts, scripting and browser behavior; Inkscape adds editor metadata and LPEs. [SVG 2](https://www.w3.org/TR/SVG/) · [LPE behavior](https://wiki.inkscape.org/wiki/LivePathEffects)
- **Silent data loss:** serialization, unsupported elements, styles and path effects need dedicated round-trip guarantees.
- **Performance tradeoffs:** GPU/tiles can improve viewport response but use cache memory; huge paths, text and filters remain expensive.
- **License/dependency cost:** C does not shrink UI, text, codec and PDF dependencies; GPL-derived work can constrain distribution.

**M0 gate:** deliver a real-document compatibility corpus, render comparisons, edit/round-trip prototype, Mac universal package size, RSS and latency numbers. If the advantage is not material, contribute upstream or build an animation-focused companion.
