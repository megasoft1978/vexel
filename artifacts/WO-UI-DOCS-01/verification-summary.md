# WO-UI-DOCS-01 revision 3 candidate evidence

Pinned base: `c43fa826c0fbc8910e8dc0c09b4139c8aa615995`. Initial tracked/untracked changes were only coordinator-prepared ledger events and the ready work order. The external source directory was read-only; exactly three files were read/copied for artwork import.

Revision 1 is preserved in `spec-r1.md`. Revision 2 is preserved in `spec-r2.md`. Revision 2 deferred shared compact glyph and layer ID/hover rendering because those changes require protected screenshot families. Revision 3 permits only the exact proven pre-existing native-shell test failure; it does not declare a clean all-tests result. Final frozen hash is in `spec-validation.txt`.

## Implementation and relevant checks

Vertical track scrolling works over gutter and grid. Command-wheel zoom and horizontal panning retain their precedence. Layout reports actual row capacity and clamps after viewport/dock/document changes. Selection sync marks a reveal request only when the selected track changes; paint reveals through actual row capacity, avoiding a repeated snap while the user scrolls. Additional tracks are described in accessibility as total/visible/above/below counts.

Timeline controls use local project-authored media geometry and the existing Fit/Redo paths, with unchanged button targets. Compact glyphs are 14 logical points in existing 26-point controls (six-point vertical inset), capped at 16 when geometry allows. Tooltips are localized via the platform lookup/fallback and are bounded to the viewport. Speed and elapsed/duration stay readable text. Named tracks use stable animation IDs with attribute and transform type; anonymous fallback and duplicate disambiguation remain. The transform display checks animation kind because the decoder uses an internal motion attribute sentinel for transform tracks; no parser/evaluator code changed.

- `pre-fix-timeline.txt`: regression fails on the original grid scroll early return.
- `focused-timeline.txt`: focused timeline and keyframe targets pass, 2/2.
- `headless/`: six deterministic dark/light/increased-contrast renders at 1×/2×, including tooltip and accessibility snapshots.
- Four directly affected timeline goldens: originals and manifest retained in `baseline-before/timeline-golden/`; hashes, changed bounds and reasons in `baseline-comparison.json`; before/after sheet in `comparison-sheet.png`. No thresholds or oracle assertions changed.
- `build.txt`: exact `./dev build` output, exit 0.
- `all-tests-first.txt`: initial gate had the named-track regression plus native-shell mismatch. The label was corrected by animation kind.
- `all-tests.txt`: final exact `./dev t --all`, exit 1, 81/82 CTests and 70/70 Python tests. Sole failure: `native-shell-goldens`, `welcome-empty-light.png differs from its native UI golden`.
- `pristine-base-build.txt` and `pristine-base-native-shell.txt`: pristine detached c43fa82 configures/builds the native target and reproduces that same sole focused failure (CTest exit 8).
- `native-shell-baseline-comparison.json`: candidate and pristine base render bytes are identical (SHA-256 `011d21b2a99f4ba6a7cbd7d2dc666f11344e8dfc3f55ba3a3824e3d7b1c07fab`); protected fixture/manifest unchanged. This is a pre-existing external gate blocker, not a regression.
- `ledger-validation.txt` and `diff-check.txt`: exact validator and whitespace checks.

## Documentation and asset checks

`asset-verification.json` records source/destination hashes/bytes, SVG track metadata, GIF frames/timing and dimensions. SVG is 1034×1521 with 12 named spline tracks, 11.01s scene; GIF is 348×512, 88×120ms = 10.56s, infinite loop; contact sheet is 624×654. The three copied outputs are byte-identical. No original artwork, audio or per-frame source captures were copied.

`local-links.json` checks 19 local README/Pages/provenance links/downloads. Browser screenshots and DOM checks are retained as `pages-desktop.*` (1280px, light) and `pages-narrow.*` (390px, dark), with no horizontal overflow and loaded preview. The first window-resize call did not actually set the viewport, so evidence was regenerated using supported DevTools viewport emulation.

README and Pages claims are grounded in pinned main's app/animation/MCP source contracts, with explicit subset/resource limits. The separate MCP candidate was not used. Demo artwork rights are not established by MPL-2.0. The handoff's frame-render success/full-page-export failure is reported as a handoff observation, not a newly rerun MCP result or fixed exporter.

## Review and integration

Fresh independent Sol 6.1 review: FAIL. One P2 acceptance blocker: Fit and Loop call the fixed-size 24-point shared icon atlas with 14/16-point bounds, clipping the glyphs (Fit appears as a corner). Exact locations: `src/app/viewer_timeline_ui.c:447,449`; painter behavior: `src/ui/ui_icons.c:391-415`. The comparison is `timeline-detail-comparison.png`. Fix with complete local geometry in a successor bounded work order, then regenerate/review the affected timeline evidence. No implementation commit, push, merge or publication is claimed in this work-order revision. GitHub Pages is not checked as a candidate publication; that check belongs after authorized integration. Shared compact glyph sizing and layer ID/hover rendering remain deferred. The known native-shell baseline mismatch is separately proven pre-existing and is not this review finding.
