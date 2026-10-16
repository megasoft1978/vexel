# W14 rulers and zoom clarity

Implemented WO-UX-07 and WO-UX-06 from the supplied `r2-interaction.md` report.
There are no matching immutable UX work-order files under `work/`, so this lane
uses the report text as its scope and adds no ledger lifecycle row, following
the no-frozen-order precedent in `docs/lanes/polish2.md`.

## Changes

- Added per-editor horizontal and vertical guide arrays, each capped at 128
  finite user-unit coordinates. Guides stay in editor state; creating, moving,
  hiding, or clearing them does not change SVG source bytes.
- Added optional top/left rulers with adaptive 1/2/5-decade tick spacing and
  user-unit labels. Ruler bands reserve a 36-point inset at the active backing
  scale. Guides can still be selected and moved with rulers hidden.
- Added ruler drag creation, guide-line move/select, Delete removal, right-click
  axis clearing, visibility and lock controls in View, guide snapping, and
  Shift snap bypass. Space temporarily hides selection/snap overlays while
  panning and restores them on release.
- Made the zoom HUD value open 25%, 50%, 100%, 200%, 400%, and 800% presets.
  Presets zoom around the viewport center. View → Zoom to Selection centers and
  fits the selected rendered bounds with 32-point padding; an empty selection
  falls back to Fit. Cmd+0 Fit, Cmd+1 Actual Size, Cmd+plus/minus, wheel, and
  pinch handling keep their existing mappings.
- Added a navigation bounds-zoom helper with finite input checks and tests.
  Added deterministic light/dark guide renders at 1×/2× and refreshed the
  existing editor chrome goldens for the new HUD button and ruler inset.

The current branch has no Command Search surface, so Zoom to Selection is in the
View menu only. The initial guide-state regression failed against the stubbed
model (`rulers_visible && guides_visible && !locked`); it passed after the
bounded state implementation. The guide suite also runs 20,000 deterministic
state mutation cases.

## Verification

- `./dev t -k 'viewer-navigation|viewer-editor-guides'`: 2 CTests passed.
- Final `./dev t --all`: Debug build passed; 57/59 CTests and 69/69 Python tests
  passed. `index-replay` and `native-viewer-smoke` failed because this host could
  not provide an offscreen Metal device/queue. The existing `editor-chrome`
  failure from the intended visual change was resolved by refreshing both
  light/dark goldens; `python3 tests/editor/verify_chrome.py` then passed.
- `python3 tests/ui/verify_guides_goldens.py`: light/dark at 1×/2× passed.
  The new goldens use deterministic scalar text; production ruler labels use
  CoreText.
- `./dev shape-fuzz --duration 90 --seed 20261010`: 1,000,000 ASan+UBSan cases,
  0 crashes, 0 invariant failures, 0 sanitizer reports; 42.119 seconds. The
  report is `.cache/qa/w14-rulers-shape-fuzz.json`.
- `./dev shot tests/viewer/fixtures/simple-shape.svg --size 640x480`: rendered
  twice headlessly. `./dev pixdiff` reported identical images at tolerance 0.
  Comparing pre/post editor chrome images with `pixdiff` showed the intended
  HUD/inset change in both themes (1.484% of pixels, bounds x=280..587,
  y=76..139).
- `git diff --check` passed. The new C implementation and test files are below
  the 800-line cap (largest: `viewer_editor_guides.c`, 549 lines).

## Maintainer real-window checks

This run did not automate the real window or capture its screen. On a desktop,
manually check ruler drag creation and guide movement/deletion, right-click clear
for both axes, lock and visibility menu states, editing a guide with rulers
hidden, snapping and Shift bypass, HUD preset selection, Zoom to Selection with
and without a selection, Cmd+0/Cmd+1 behavior, Space pan/overlay restoration,
and ruler spacing on 1× and 2× displays. Also check center retention after a
zoomed window resize and ruler toggles. Command Search can include Zoom to
Selection when that surface is implemented.
