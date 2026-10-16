# W14 keyframes and SMIL easing

## Changes

- Added bounded, source-preserving authoring patches for numeric, color, and
  transform animation values, key timing, spline tuples, timing, and track
  creation/removal. Editor commands apply the patches through
  `document_edit`, preserving undo and redo source identity.
- Added selected-key markers and drag editing to the timeline, plus a custom
  easing graph, named numeric controls, interpolation modes, presets, and
  accessibility values for the selected outgoing interval.
- Added focused C tests for lifecycle edits, limits, stale-source rejection,
  undo/redo, exact source preservation, spline tuple isolation, fresh decode
  and render state, keyboard/pointer editing, accessibility, and light/dark/
  high-contrast raster output.

## Verification

- `./dev build`: passed (Debug, arm64, macOS 13 deployment target).
- `animation_authoring_test`: passed.
- `viewer_timeline_test`: passed; the three generated easing PNGs compare
  byte-for-byte with the new light, dark, and increased-contrast goldens.
- Focused CTest run for `svg-animation`, `viewer-timeline`, and
  `animation-authoring`: 3/3 passed.
- Both frozen READY work orders validate, `python3 scripts/ledger.py validate
  ledger.jsonl` reports 412 valid entries, and `git diff --check` passes.
- `./dev t --all`: 69/69 Python tests passed; 56/58 CTests passed. The two
  failures are `index-replay` (`offscreen Metal device or queue unavailable`)
  and `native-viewer-smoke`. The latter uses native event injection; it failed,
  and the first-failure reporter did not retain its diagnostic. It was not
  rerun. The full-suite acceptance gate remains incomplete in this environment.
  No separate manual review or screencapture was performed.
- Pre-fix authoring regression evidence is in
  `artifacts/WO-MOTION-03/pre-fix-regression.txt`. Test logs and the full
  suite output are under `artifacts/WO-MOTION-03/` and
  `artifacts/WO-MOTION-04/`.

## Maintainer real-window verification

Please inspect the timeline panel at the normal window size and at a compact
size, then confirm selected-key changes, spline mode selection, numeric field
focus and keyboard commit behavior, and VoiceOver labels/values in the actual
macOS window. This lane intentionally does not include a manual real-window
review or screenshot.
