# 06 — MVP, plan and prior art (Q9, Q10)

## Smallest useful editor

**MVP includes:** open SVG / SVGZ only if decompression is explicitly supported; progressive canvas render; pan/zoom/fit; select by click/layers/tree; move/scale/rotate; basic shape and path-node edit; fill/stroke/opacity/solid + linear/radial gradients; layers/groups panel; XML tree/attribute view; undo/redo; save/save-as/autosave recovery; OS file dialogs and clipboard; keyboard shortcuts; basic accessible names/focus for core controls.

**MVP excludes:** full Inkscape tool/LPE/extension parity, boolean/path ops beyond node edit, mesh gradients, markers parity, advanced filters/masks, scripting/foreignObject, animation timeline, print/PDF workflow, collaboration, browser-perfect text, color-managed print, full tablet pressure parity, plugins. Unsupported content remains source-preserved and is reported; never silently delete.

## Phases and gates

LOC are C LOC planning ranges, including production implementation but excluding vendor code; broad uncertainty ±50%. Calendar is for one experienced C/graphics developer with LLM assistance and no dedicated QA/designer. Parallel LLM tasks help boilerplate/tests/research, not replace integration and visual QA.

| Phase | Scope | LOC estimate | Solo calendar estimate | Exit criteria |
|---|---|---:|---:|---|
| **M0 — benchmark + viewer skeleton** | Harness/corpus; C17 build; mmap source, secure XML token index; Mac app shell; progressive canvas; CPU/GPU-blit backend; resvg/Inkscape references | 12k–20k | 8–12 weeks | Reproducible benchmark; 1/10/100 MB profile; 10 MB first visible pixel ≤1 s on simple fixture; 100 MB progressive; no data loss opening/saving unchanged; RSS/bundle measured and baseline Inkscape 1.4.x + 1.5-dev captured. |
| **M1 — renderer fidelity** | CSS subset, geometry, path parser, CPU AA tiles, gradients, clips, use, common images/text, test harness | 20k–35k incremental | 5–9 months | ≥90% selected W3C+resvg static fixtures pass documented visual tolerance; no crash on corpus; visible tile p95 ≤33 ms on target fixture; explicit unsupported-feature report; repeatable compare against resvg/librsvg/Inkscape. |
| **M2 — selection/edit** | Hit test/spatial index; model IDs; transforms, node editing, style controls, tree/layers, undo/op log, patch save/autosave | 18k–30k incremental | 5–8 months | MVP workflows work, undo/redo invariants; edits localized; 100k simple objects select/move p95 ≤50 ms; unknown XML preservation suite passes; crash recovery demonstrable. |
| **M3 — UI polish + Linux/Windows** | Menus/dialogs/IME/a11y/focus, platform backends, theming, keyboard, usability and packaging | 20k–40k incremental | 6–12 months | Mac beta accessible for core workflows; Windows native build; Linux X11+Wayland support tier declared; per-monitor DPI; signed/notarized Mac package; screen-reader/IME QA; sustained benchmark wedge vs Inkscape dev. |

**Whole basic editor:** roughly **18–30 months** for one senior developer to reach a credible cross-platform beta under this custom-UI/no-dependency constraint; 2–3 focused engineers might reach 12–20 months but add coordination and QA. A narrower macOS-only viewer/edit spike can be 2–4 months. Broad SVG fidelity adds years and remains open-ended.

### Performance targets (proposed)

- 1 MB simple file: TTFP <250 ms; steady RSS <150 MB; pan/zoom p95 <16.7 ms while cached.
- 10 MB / 100k simple shapes: TTFP <1 s, peak RSS <500 MB, pan/zoom p95 <33 ms; hit-test/move p95 <50 ms.
- 100 MB simple map: progressive first pixel <2 s, visible tiles usable while parse continues; no UI stalls >100 ms; bounded cache.
- 500 MB/1 GB stress: not necessarily fully editable or fully rendered immediately; no crash/data corruption, progress/cancel and memory-pressure behavior; first visible content target <5 s on simple generated fixture.
- Binary target ≤10 MB stripped executable is a stretch research KPI, not promise. Mac `.app` bundle depends on assets and architecture; measure and publish exact scope.

## Prior art refresh specific to C/custom UI direction

| Project | What it is / relevance | Sizes / speed evidence |
|---|---|---|
| **Rive renderer** | Proprietary editor/runtime plus open-source runtimes; fast vector animation focus, not SVG source round-trip. | No public comparable editor binary/RSS benchmark found. |
| **Vello** | Open GPU compute 2D renderer in Rust; renderer component, not editor. | No direct relevant end-to-end SVG large-file speed figure found. |
| **ThorVG** | C++ vector/motion renderer used in embedded/UI contexts; SVG/Lottie render, not a full editor. | No apples-to-apples published benchmark retrieved for these exact workloads. [repo](https://github.com/thorvg/thorvg) |
| **PlutoVG + PlutoSVG** | C 2D vector renderer and separate compact SVG loader/renderer; closest permissive C rendering prior art. | Tiny relative to full editors, but repo does not provide a dependable final stripped binary size; count current files after pinning commit. [PlutoVG](https://github.com/sammycage/plutovg) · [PlutoSVG](https://github.com/sammycage/plutosvg) |
| **NanoSVG** | Single-header parser/rasterizer for a restricted SVG subset, useful as a small code-size floor and parser experiment. | No robust AA/filter/text parity; project itself calls implementation simple; reported binary size varies by integration, so unknown. [repo](https://github.com/memononen/nanosvg) |
| **LunaSVG** | C++ SVG rendering library, based on SVG Tiny subset; not editor. | Fedora packages source/library separately; no fixed cross-platform app size/speed number used. [repo](https://github.com/sammycage/lunasvg) |
| **resvg** | Rust SVG rendering focused on correctness; CLI/library, not editable document UI. | Wikimedia 2021 corpus numbers show it faster than old Inkscape for those batch renders (see rendering doc), with version caveats. [repo](https://github.com/linebender/resvg) |
| **Rnote** | Rust note-taking/vector ink app, not SVG-native Inkscape editor; useful canvas interaction reference. | No comparable large-SVG editor benchmark found. |
| **Graphite** | Rust node-based graphics suite, currently different product architecture and maturity. | Not an established C/custom-UI precedent; current release state is volatile, see [repo](https://github.com/GraphiteEditor/Graphite). |
| **Penpot / Lunacy** | Product UX references, web/proprietary internal architectures; SVG used for exchange/output. | Their bundle/RSS figures do not isolate rendering and are not comparable. |
| **Sublime Text / Blender** | Custom-rendering and platform-abstraction evidence from mature apps. | Proprietary internals (Sublime) or large C++ product (Blender GHOST); neither is a small recipe. |

## Staffing reality

One person can ship a focused Mac-first basic editor if “basic” means a documented SVG subset, reliable source preservation, and measured large-file improvement. Cross-platform custom desktop UI is itself a second product. A realistic staffing split for a small team: one C/document/render lead, one platform/UI/accessibility engineer, one test/compatibility/QA engineer. LLM agents can draft isolated modules, fixture generators and API boilerplate, but a human must own SVG semantics, architecture and visual regression triage.
