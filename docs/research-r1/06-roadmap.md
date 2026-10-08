# Roadmap and owner decisions (Q12)

Numbers below are **proposed engineering gates**, not measured current product performance or promises. Baselines must be captured in M0 on named Apple Silicon and Intel Macs, Windows and Linux. “Compressed bundle” means installed distribution artifact excluding user content. RSS includes app and caches; benchmark files defined by object/path/image counts, not only source MB.

## Milestones

| Phase | Scope | Exit criteria |
|---|---|---|
| **M0 — 6–8 week feasibility spike** | Build clean-room prototype with C17, C++20 or Rust+C ABI candidate; parse/write SVG, render via resvg/ThorVG/Cairo reference adapters; UI toolkit and Mac packaging spikes. Assemble corpus and license inventory. | **Decision gate:** render ≥90% of selected SVG 1.1/WPT static fixtures within agreed per-fixture tolerance; no crash/data-loss on ≥500 hand-picked Inkscape files; round-trip preserves all unknown nodes and metadata in automated semantic comparison; Mac universal build package target ≤150 MB compressed, idle RSS ≤250 MB, 10 MB SVG / 10k simple elements first paint ≤2 s on M-series baseline. If no renderer matches or package/engineering risk unacceptable, stop/redirect. |
| **M1 — static viewer/renderer** | Secure parser, XML token/source preservation, SVG common subset, resource limits, styles, text baseline, clip/mask/use/markers; tiled viewport; CLI render oracle. | 100% of the project’s selected WPT+resvg pass subset (publish manifest); ≥95% match on curated real SVG visual corpus with documented exceptions; 100% round-trip unknown-node retention; 10 MB/100k simple objects: cold open ≤5 s, first paint ≤2 s, steady RSS ≤600 MB, pan/zoom p95 ≤50 ms on baseline Mac; compressed app ≤180 MB. These are product targets, not facts. |
| **M2 — editing core** | Selection, transforms, shapes, Bezier/node edit, fill/stroke/gradients, layers, undo/redo, snapping, PNG, basic CLI. | 500 user-action scripts pass undo/redo invariants; all MVP operations save/reopen; no XML loss in corpus; 100k-object selection/edit p95 ≤100 ms; 1M-node path remains responsive for view/selection with editing operations either ≤250 ms or visibly cancellable; peak RSS ≤1.2 GB for defined large-file fixture; app ≤220 MB compressed. |
| **M3 — parity subset / beta** | Text quality, common path operations, common LPE read/write/edit, filters/patterns, PDF export, multi-page, extensions/plugins, Windows/Linux polish. Prioritize based on beta telemetry/opt-in reports. | Feature matrix reflects shipped/tested subset; reference render matrix documented; zero P0 data-loss defects in release candidate corpus; Windows/macOS/Linux signed/packaged; large-file limits published; 90% of selected must-have workflows pass cross-platform end-to-end runs. Full parity is not an exit criterion. |
| **M4 — animation phase 2** | Time-based timeline; transform/opacity/color tracks; undoable keyframes; SMIL and CSS output, static fallback; metadata namespace; browser preview/export. | 100 animation fixtures round-trip through our editor; exported SMIL/CSS renders in current Safari, Firefox, Chromium without private runtime; 100% of unsupported feature cases show explicit diagnostics and preserve source; playback at 60 fps on 10k static-object / 100 animated-track fixture at 1080p on Mac baseline; editing animation adds ≤25% idle RSS; stock Inkscape open/save test matrix documents exactly what survives. |
| **M5 — growth / parity program (optional)** | Advanced LPE, filters, PDF/print, plugin ecosystem, accessibility refinements, resource library; additional animation tracks and Lottie export. | Each feature has user evidence, documented semantics and regressions; no “complete parity” marketing until a maintained feature-by-feature matrix is proven. |

### Performance budget interpretation

- The proposed file-specific gates must be validated against at least 3 machine classes and reported with mean, p95, peak RSS, file source size, object count, path-node count, image decoded pixels, and whether caches were warm.
- For a **10 MB SVG** with 100k simple elements, a first-render goal of 2 seconds and RSS ≤600 MB are ambitious but measurable; deep filters/images may exceed them. Publish exceptions rather than redefining the fixture.
- Keep two memory limits: normal interactive target and graceful handling ceiling. At ceiling, drop caches and prompt before refusing work; never corrupt source or OOM due to one filter surface.
- Binary targets are intentionally provisional. Qt/GTK, codecs, fonts, renderer, architecture slices and notarized universal binaries may make these impossible; M0 is the reset point.

## Open decisions for the owner

1. **Language:** Is native C a hard requirement, or is C++20/Rust core with stable C ABI acceptable? Recommendation: allow C++20 or Rust for core safety/ergonomics.
2. **Product scope:** Is the goal a commercial clean-room competitor, an open-source Inkscape fork, or a focused fast editor/animation companion? A fork has an existing feature base but inherits GPL and architecture; clean-room requires years.
3. **License:** Apache-2.0/MIT permissive clean-room product, or GPL-compatible community product? Obtain counsel review before source access/implementation if clean-room is required.
4. **Parity definition:** Which 20 workflows define MVP for the target user (illustration, icon/UI, print/prepress, technical drawing, animation)?
5. **Interoperability:** Is source round-trip priority above pixel-perfect render parity? Recommended: unknown-node preservation and editable Inkscape metadata first; disclose renderer differences.
6. **Animation target:** Browser SVG playback (SMIL/CSS), video/GIF, app/game Lottie, or all? These require different semantics and dependency budgets.
7. **Mac support:** Universal Intel+Apple Silicon or Apple Silicon only? Which minimum macOS version? Current release requirements require live verification before committing.
8. **Bundle budget:** Is ≤150 MB compressed app a hard target or provisional? Is an external dependency install acceptable?
9. **Project staffing:** Solo, small funded team, or community? Full parity is not credible as a short solo project.
10. **Compatibility corpus:** Can owner provide legally shareable real-world SVG/Inkscape documents, including huge files and extension-generated files?

## Recommendation

Do M0 only after choosing the language flexibility, legal posture, and target workflow. The highest-value proof is not a custom renderer demo: it is opening a corpus of real Inkscape documents, editing a few useful objects, preserving unknown data, and outperforming current Inkscape on an agreed large-file interaction while producing a smaller signed Mac bundle. If it cannot do those together, narrow to a viewer/animation companion or upstream targeted performance work.
