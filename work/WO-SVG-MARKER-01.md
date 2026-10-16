---
id: WO-SVG-MARKER-01
revision: 2
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 11035d1a6d75265eb1fbb6eb8ed7b9171c20f28d2293c31d17a8b975d6fe798e
parallel_safe: false
network: false
---
# Goal
Correct marker paint-style inheritance so referenced marker contents inherit from their lexical marker-definition ancestors, not from the referencing path; add a focused regression and verify the T1 endpoint improvement against independent renderers.

# Context
- WO-CORPUS-16 found that removing `marker-end` reduces endpoint-ROI pixels above delta 10 from 71 to 5 and lowers the max delta from 149 to 13. Removing blur or replacing the path gradient does not remove the hotspot.
- `src/svg/shape/marker.c::make_marker` currently computes marker style using `parent_style`, which is the referencing graphic's computed style.
- W3C SVG 2, §13.7.4 states: “Properties do not inherit from the element referencing the marker into the contents of the marker.” Marker contents inherit from the marker element and its ancestors; the referencing element's stroke width still controls `markerUnits="strokeWidth"`.
- Reference: https://www.w3.org/TR/SVG/painting.html#RenderingMarkers
- The SVG 2 rule is consistent with the observed Inkscape 1.4.4 marker fixture behavior; preserve marker definition ancestry, CSS cascade, marker opacity, and existing placement/units behavior.

# Files allowed to touch
- `work/WO-SVG-MARKER-01.md`
- `src/svg/shape/marker.c`
- `src/svg/shape/traverse_internal.h`
- `tests/svg/shape/test_w5_render.py`
- `artifacts/WO-SVG-MARKER-01/**`
- `.cache/WO-SVG-MARKER-01/**`
- `build/mac-debug/**`, `build/mac-release/**` (ignored generated outputs only)
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- No changes to gradient interpolation, filters, marker geometry/orientation/reference placement, corpus sources, goldens, or thresholds unless evidence proves required for this contract; stop and draft a follow-up order if so.
- Do not read/fetch/copy GPL editor source. Inkscape and librsvg/rsvg-convert may be executed only as renderer binaries.
- Preserve all unrelated working-tree changes. Leave edits uncommitted.
- No broad UI/E2E, full corpus sweep, fuzz, sanitizer, or performance suite.

# Required implementation
1. Add a byte-stable renderer regression with a marker child that has no explicit fill, a root/definition-ancestor fill, and a referencing path with a distinct gradient fill. Verify the marker uses its definition-tree fill and matches `rsvg-convert` at a fully covered marker pixel.
2. Derive the marker's computed parent style by walking the lexical ancestor stack from bounded XML tokens, then call the existing CSS cascade for the marker. Retain CSS selector handling and inherited styles from `<svg>`, `<defs>`, and nested definition ancestors.
3. Bound token visits and poll the existing cancellation callback during ancestor resolution. Reuse an existing traversal work limit; do not add an unbounded allocation or unbounded rescanning. Keep files under 800 lines.
4. Continue using the referencing shape's stroke width and source transform for marker units/placement; do not pass referencing paint properties into marker content.
5. Build Debug and Release `shape_render_cli`; run only the new focused Python regression and existing marker-specific test methods in `tests/svg/shape/test_w5_render.py`.
6. Render the W13 filtered-path-only fixture with Vexel and Inkscape at 1600x1000; record full-page tolerance-2 and endpoint ROI x580..619/y80..119 metrics against the saved W13 Inkscape reference. The exact T1 SVG is outside the workspace and protected from access, so a fresh full-T1 render is explicitly deferred until its hash-pinned source is available within the workspace. Do not claim corpus-wide parity.

# Verification
- Run `python3 -m unittest` only for the named new and existing marker-specific methods.
- Validate work-order SHA, JSON/report artifacts, ledger, exact changed-path allowlist, source/test line counts, and `git diff --check`.
- Do not run comprehensive UI/E2E, whole Python/CTest suites, broad corpus, fuzz, sanitizers, or performance tests.

# Output
Write `artifacts/WO-SVG-MARKER-01/verification.txt`, update `STATE.md` and both ledgers, leave changes uncommitted, and keep the full product goal active.
