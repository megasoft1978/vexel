# Prioritized feature-parity matrix (Q9)

Estimates are **engineering effort for one experienced graphics engineer** after document/rendering infrastructure exists: S = days, M = 2–6 weeks, L = 1–3 months, XL = 3+ months. They exclude design, QA, platform-specific packaging and maintenance; integration of multiple rows is not additive. A small team shortens elapsed time, not total complexity. Inkscape’s scope is visible in its [release history](https://wiki.inkscape.org/wiki/Release_notes) and [CLI/actions](https://inkscape.org/doc/inkscape-man.html).

| Feature area | Priority | Effort | Notes / exit condition |
|---|---:|---:|---|
| Open/save SVG + SVGZ, namespaces, IDs, metadata, unknown-node preservation | **MVP** | XL | Core prerequisite; round-trip corpus must show no unknown-node loss. |
| Canvas: pan, zoom, rotate view, selection, bounding boxes, hit testing | **MVP** | L | Include HiDPI, color-managed display where possible, large canvas tiles. |
| Select/move/scale/rotate/skew, snapping, guides, rulers | **MVP** | XL | Snapping and selection are core workflows, but advanced snap options later. |
| Rectangle, ellipse, star/polygon, spiral, freehand, Bezier pen, pencil, node edit | **MVP subset** | XL | Ship common shapes + Bezier + node tool first; pressure pencil and special shapes later. |
| Fill/stroke, solid color, opacity, linear/radial gradient, gradient editor | **MVP** | XL | Include CSS cascade/import semantics in scope; broad color spaces later. |
| Text tool, text editing, font selection, basic shaping | **MVP subset** | XL | Unicode shaping, multiline, font fallback and text-on-path parity are hard. Advanced typographic features later. |
| Layers/groups/objects panel, lock/hide/reorder, labels | **MVP** | L | Understand `inkscape:groupmode`, labels and IDs; preserve unknown layer metadata. |
| Align/distribute, duplicate, group/ungroup, z-order, transform panel | **MVP** | M/L | Essential utility; multi-page alignment and advanced selectors later. |
| Undo/redo, clipboard, copy/paste style, keyboard shortcuts | **MVP** | XL | Undo model must cover XML, selection, tool state and async operations coherently. |
| Import raster; embedded/linked images; PNG export | **MVP** | M/L | Decode safety and URI policy required. Export selected/page/drawing bounds. |
| Path operations: union/difference/intersection/exclusion/division, object-to-path, stroke-to-path | **MVP subset** | XL | Stable cubic curve booleans are complex; first ship well-tested subset and disclose edge-case handling. |
| Clones (`<use>`), markers, clipping and masks | **MVP render; edit later** | XL | Must render common constructs before claiming compatibility; editing UI for all details can follow. |
| CSS stylesheets, cascade, selectors, inherited values, variables | **MVP render subset; full later** | XL | SVG/CSS is a web platform; never silently rewrite external CSS. |
| Filters | **Later** | XL | Large semantic and performance surface; start rendering common primitives; unsupported filters preserved. |
| Patterns, symbols, pattern editor | **Later** | L/XL | Render common patterns in MVP, editing panel later. |
| Mesh gradients | **Later** | L | Inkscape-specific/de facto feature with uncertain broad browser support; preserve and approximate/export explicitly. |
| Inkscape LPE: read/display/preserve | **MVP** | L | Need preserve source path + effect metadata, render flattened visible result when effect unknown. |
| Common LPE author/edit (offset, corners, powerstroke, mirror/symmetry) | **Later** | XL each | Each effect is its own geometry/UI/test project. Do not promise full LPE parity early. |
| Text-on-path, flowed text, vertical text, font collections | **Later** | XL | Text layout and legacy flowed text conversion are high-risk. |
| XML Editor | **Later** | M | High value for expert escape hatches; expose namespace-aware tree and exact attribute editing. |
| Trace bitmap / centerline tracing | **Later** | L/XL | Integrate Potrace or another license-reviewed backend; async and cancellable. |
| PDF import/export | **Later (export high priority)** | XL | Fonts, clipping, transparency, blend modes, ICC and PDF/X output make this large. |
| EPS / PS import/export | **Later** | XL | Usually depends on external converters/Ghostscript; security and licensing/deployment concerns. |
| DXF import/export | **Later** | L | Unit and path approximation behavior must be specified. |
| EMF/WMF import/export | **Later** | L/XL | Primarily Windows interoperability; defer until target audience evidence. |
| Extensions / Python API | **Later; plugin contract first** | XL | Inkscape extensions are an ecosystem. Do not bundle Python runtime until plugin demand proven. [CLI/extensions docs](https://inkscape.org/doc/inkscape-man.html) |
| CLI, batch export, action API | **MVP basic; parity later** | L | Open/render/export/inspect actions first; command compatibility can be incremental. |
| Templates, palettes, symbols libraries | **Later** | M | Templates/palettes have content maintenance costs and licensing. |
| Multi-page documents | **Later** | L | Inkscape added this in 1.2; SVG interoperability varies. [1.2 notes](https://wiki.inkscape.org/wiki/index.php/Release_notes/1.2) |
| Animation timeline: transform/opacity/color to SMIL/CSS | **Phase 2** | XL | Distinct product milestone after stable edit model; portability tests in browsers. |
| Animation path morphing, events, scripts, Lottie | **Skip in initial Phase 2** | XL+ | Support only after basic tracks; preserve unsupported input and emit diagnostics. |
| Keyboard shortcut parity / customization | **Later** | L | Start with familiar defaults; exact Inkscape map invites user expectation and conflict. |
| Extension ecosystem + action/verb API parity | **Skip/full parity not planned** | XL+ | This is a platform, not a feature. Add a narrow documented plugin/API system. |
| Clone tiling, 3D box, spray, calligraphy, connector, mesh editing, all specialized tools | **Later / selective** | XL each | Prioritize from actual user cohort; do not attempt all on a solo schedule. |
| EPS/PS/DXF/EMF complete parity | **Skip until validated demand** | XL | Delegate to optional external tools only with secure subprocess handling and license review. |

## MVP commitment suggested

MVP means: open/save SVG/SVGZ without loss of unknown data; common objects, path and node editing; layers; solid/gradient fill and stroke; transforms and basic snapping; basic text; undo/redo; common `<use>`, markers, clip/mask rendering; PNG export; a limited but reliable CLI. Implement reading/preservation of unsupported effects before authoring them. This is a usable small editor, **not feature parity**.

## Effort and team realism

- A polished viewer and common editing subset is a multi-year effort for one person if compatibility/security/testing/packaging are real requirements.
- Full Inkscape parity includes years of accumulated tool behavior, extension contracts, import/export quirks, keyboard muscle memory, and file edge cases. **A solo developer + LLMs should plan parity as an open-ended program, not a 12-month deliverable.**
- Even an experienced 4–6 person team should expect multiple years for credible broad parity, especially while maintaining Windows, macOS and Linux builds. **Confidence: medium; estimate depends on reusing a renderer/toolkit and available SVG expertise.**
- Do not reuse Inkscape source while claiming a clean-room proprietary rewrite. Either participate upstream under compatible open-source terms or implement independently and keep provenance records.
