# Round 3 verdict — 8 October 2026

## Decision

**GO with a narrow wedge, conditional on a 6–8 week proof.** Build a Mac-first, C17, Inkscape-SVG-preserving **large-document viewer/editor**, then decide whether to deepen editing or pivot. Do not sell “the first native fast SVG editor”: **Donner** is an active, close native SVG2 editor and renderer project, and several products cover neighboring pieces. The differentiated claim to test is: *predictable progressive opening and low-latency editing of very large, real-world SVGs, while preserving unknown and Inkscape-specific XML*. No source found establishes that any incumbent already delivers this combination with measured performance and complete preservation, but no head-to-head evidence exists either. [Donner](https://github.com/jwmcglynn/donner) · [Inkscape architecture](https://wiki.inkscape.org/wiki/Architectural_overview)

## Exists?

| Capability | Verdict | Evidence / gap |
|---|---|---|
| Mature Inkscape-compatible editor | **Exists** | Inkscape is the parity baseline. It has broad native SVG/editor metadata support; 1.3 added async tiled canvas rendering. |
| From-scratch native SVG2/CSS3 editor and renderer | **Partially exists** | Donner: C++20, editor + XML view, GPU and CPU renderers, resvg tests, active. Not C17; its own docs describe it as under active development. Its large-file measurements and Inkscape-private metadata round-trip are unverified. |
| Lightweight C SVG renderer | **Exists, partial** | PlutoVG is MIT C, about 19k Debian source SLOC, not a document editor or full SVG engine. NanoSVG is smaller but deliberately limited. |
| Open editor with custom-drawn cross-platform UI | **Exists as frameworks/apps, not this product** | Makepad is an active Rust UI runtime, not an SVG-native editor. Graphite, Xilem and Vello are Rust projects with different scope/maturity. |
| Large-SVG fast editor with benchmarked edge | **Does not yet exist as verified evidence** | Forums document pain, not independent reproducible benchmarks. Donner is the closest contender; test both before claiming an advantage. |
| Inkscape-grade editor plus SVG animation timeline | **Does not exist in the reviewed mature alternatives** | Glaxnimate authors SVG/Lottie animation; Inkscape says it has no animation editor. Donner roadmap mentions animation, not a released parity feature. |

## Wedge, users, and economics

Start with map/cartography, GIS export, scientific plots, laser/plotter and textile workflows that generate many paths/nodes. The product should open and navigate a large document immediately, keep distant geometry cheap, select/edit the visible region, and preserve original XML when saving. Forum reports include 6 MB paths whose selection/deselection can take 5–10 minutes, a 90 MB many-node SVG making even text edits take about a minute, an 8 GB machine struggling with a 100 MB SVG, and an 780 MB SVG freezing save/tab changes. These are self-reported, not controlled tests. [Aug 2026 large files](https://www.reddit.com/r/Inkscape/comments/1w0ue90/methods_for_working_with_large_svgs/) · [90 MB file](https://www.reddit.com/r/Inkscape/comments/1ls879d/) · [100 MB on 8 GB RAM](https://www.reddit.com/r/Inkscape/comments/1tpv3tk/) · [780 MB file](https://www.reddit.com/r/flatpak/comments/1vps3o0/)

Users today use Inkscape, Illustrator, Affinity, CAD/GIS tools, LightBurn/Cricut Design Space, browsers, or split/optimize files. Complaints include long parse/save, huge path node sets, snapping/selection and layer-tree stalls, filters/meshes, embedded raster payloads, export previews, and toolchain-specific SVG imports. CNC users often need simpler paths and manufacturing controls more than a general editor; animation buyers need timeline/keyframe workflows more than large-file navigation. That argues against combining the two wedges at MVP. Switching willingness is **unknown**: forum frustration does not establish paid demand. Validate with 12–20 interviews and 5 real corpora from target users before building a broad editor.

**Recommendation:** lead with “fast, faithful large-SVG workbench”; make animation a later optional differentiator only after the viewer/editor is useful. Funding can be OSS sponsorship plus paid support, or a dual-license/commercial distribution model after counsel reviews actual dependencies. Target a solo 18–30 month cross-platform basic editor only if the 6–8 week gate is strong; 2–3 experienced engineers: roughly 12–20 months, plus ongoing compatibility/support. These are planning estimates, ±50%.

## Gate / kill criteria

Proceed past M0 only if, on a named Mac and pinned files, the prototype demonstrates (a) progressive first visible content <2 s for a 100 MB simple/map-like SVG, (b) no UI stall >100 ms during parsing, (c) visible-tile pan/zoom p95 <33 ms on the target corpus, (d) unchanged-open/save preserves unknown XML byte-for-byte outside documented serialization changes, and (e) at least 5 interviewees rank this pain high enough to pilot. Treat these as proposed gates, not published facts. Pause or pivot to upstream Inkscape contribution / a focused viewer if Donner or Inkscape matches the measured workflow, if likely users don't prioritize it, or if preserving real files forces a full parser/renderer scope before a demo.

## Main risks and schedule

Hard work is SVG semantics/round-trip, stroke correctness, text, filters and C memory safety—not drawing a toolbar. Custom UI accessibility, IME and platform services are a second product. The detailed sequence is M0 benchmark → M1 macOS/Metal → M2 base/safety → M3 progressive index/viewer → M4 raster → M5 styles/use → M6 UI → M7 selection → M8 editing panels → M9 minimal-diff save/undo → M10 go/no-go; then M11 Linux, M12 Windows, M13 animation, M14 packaging. The proof should take 6–8 weeks; a credible Mac beta is estimated 12–18 months for one senior developer, with broad uncertainty. Full cross-platform beta is 18–30 months solo or 12–20 months with 2–3 engineers.

Confidence: **high** that neighboring products exist and demand anecdotes are real; **medium** that the proposed wedge is underserved; **low** on willingness to pay, relative speed, final binary size, and solo schedule until measured.
