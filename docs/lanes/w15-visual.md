# W15 visual lane

## Changes

- Read the current macOS control accent and adapt selection/focus tokens to
  preserve contrast in light, dark, and increased-contrast appearances.
- Added persisted 100%, 110%, and 120% interface sizes, exposed through View,
  and persisted Layers and Inspector widths with bounded drag, keyboard, and
  double-click-reset splitters.
- Simplified toolbar grouping and tool glyphs. The Inspector now leads with the
  selected object, collapses Geometry and Appearance independently, and shows
  document dimensions and viewBox controls when nothing is selected.
- Added group scope in Layers with a breadcrumb and All Layers reset. Hovering
  or focusing a truncated layer reveals its full name and ID; the full ID can
  be copied without truncation.
- Added a toolbar-only native material view, enabled on macOS 26 and later when
  Reduce Transparency and Increase Contrast are off. Earlier macOS versions
  retain the opaque toolbar surface.
- Added interruptible sidebar and control-state transitions. Reduce Motion
  finishes sidebar changes immediately and disables control transitions.
- Added light/dark editor screenshots at 100%, 110%, and 120% in 1× and 2×
  rendering. Refreshed the editor chrome and selection overlay fixtures for the
  updated Inspector hierarchy and accent treatment.

## Verification

- The new `viewer-chrome-style` regression failed before its implementation.
- `./dev build` — passed.
- `viewer_chrome_test` — passed, including inspector hierarchy, focused Layers,
  full-ID reveal/copy, UI scale, splitter bounds/keyboard/reset, and interruptible
  motion checks.
- `viewer_chrome_layout_fuzz` — passed 200,000 cases with seed `0x6d2b79f5`.
- `tests/ui/verify_style_goldens.py` — passed: 28 dark/light screenshots, 1×/2×,
  seven states including 110% and 120% scale.
- `tests/editor/verify_chrome.py` and
  `tests/editor/verify_selection_goldens.py` — passed after reviewing and
  refreshing both appearances and both selection scales. `tests/ui/verify_chrome.py`
  also passed unchanged.
- `./dev shot tests/svg/shape/fixtures/rect.svg --size 1280x800` — passed.
  Two serial renders compared with `./dev pixdiff` were identical at tolerance 0.
- `./dev t --all` — 69/69 Python tests and 55/57 CTests passed. The remaining
  CTests were `index-replay` (reports that an offscreen Metal device or queue is
  unavailable) and `native-viewer-smoke` (failed in this environment). All lane
  editor, UI, preference, contrast, and golden tests passed.
- `python3 scripts/ledger.py validate ledger.jsonl` — 406 entries valid.
- `git diff --check` passed. All changed C, header, and Objective-C files are
  under 800 lines. No commit was made.

## Maintainer real-window checks

No live window, mouse/keyboard automation, or screencapture was used in this
lane. On a macOS desktop, verify:

- System accent updates in both appearances and remains legible with Increase
  Contrast. Check 100%, 110%, and 120% menu selection and persistence.
- Drag both splitters, adjust them with arrow keys, reset with a double-click,
  and confirm widths persist. Collapse, reopen, and quickly reverse each
  sidebar transition. Repeat with Reduce Motion enabled.
- Confirm inspector sections, document summary, group focus breadcrumb, full
  name/ID reveal, and Copy Full ID in the actual window.
- On macOS 26+, confirm native material is restricted to the toolbar and turns
  off under Reduce Transparency or Increase Contrast. On macOS 13–25, confirm
  the toolbar remains opaque.
- Check keyboard focus and the new slider, group-scope, and copy semantics with
  VoiceOver or Accessibility Inspector.

The requested W15 work orders were supplied from the external research scratchpad;
there is no frozen W15 order under `work/`. No lifecycle event was added to
`ledger.jsonl`.
