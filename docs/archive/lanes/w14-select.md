# W14 Select and snapping

This lane implements WO-UX-01 and WO-UX-02 from the interaction research
report at `/private/tmp/claude-501/-Users-megasoft78-Desktop-Freelance-vexel/73fa8381-5851-4b85-bab5-8911cf2fe579/scratchpad/research2/r2-interaction.md`.
The requested work orders were not frozen under `work/` in this worktree.

## Changes

- Layer-pane and canvas selection share the clipped canvas outline and handles.
  Handles draw at 6 logical points with an 8 point hit diameter. Hovering a
  selected shape uses the open-hand move cursor; edge and corner handles use
  directional resize cursors, and the top handle uses the rotate cursor. Pan
  uses the open hand while idle and the closed hand only during a pan gesture.
- Added a four-action contextual strip for supported path and multi-selection
  actions. It positions above or below the selection, avoids the pointer, and
  falls back to the existing options row when the canvas is too small.
- Replaced independent axis snapping with one deterministic chosen candidate.
  The dominant drag axis is preferred, with nearest-candidate and stable
  tie-breaking on that axis; snapping falls back to the other axis when needed.
  Snapping uses a 6 logical point tolerance, fixed 128-shape candidate storage,
  an 8,192-shape scan bound, 128 guide slots, a 32-byte label, and no per-event
  heap allocation. The chosen alignment line draws at 1 logical point. Grid
  snapping uses whole SVG user units; object snapping uses visible bounds edges
  and centers; guides are independent candidates. Equal-gap labels are emitted
  only for an achieved equal spacing.
- Added a Snapping options-row button and popover describing the snap sources,
  the 6 point tolerance, and Shift bypass. Grid, Objects, and Guides toggle
  independently; the existing master shortcut remains available.
- Added focused headless regressions and dark/light selection, chrome, style,
  snapping-popover, and snapping-overlay goldens.

## Verification

- `./dev build` passed.
- `./dev t -k editor-ux-selection-snap` passed.
- `python3 tests/editor/verify_selection_goldens.py`,
  `python3 tests/editor/verify_chrome.py`,
  `python3 tests/ui/verify_style_goldens.py`,
  `python3 tests/editor/verify_snap_popover_goldens.py`, and
  `python3 tests/editor/verify_snap_overlay_goldens.py` passed. These include
  light and dark headless renders.
- `./dev t --all` completed with 58/60 CTests and 69/69 Python tests passing.
  `native-viewer-smoke` could not launch/present because LaunchServices XPC
  failed and no Metal device or queue was available. `index-replay` completed
  but its preview could not initialize because no offscreen Metal device or
  queue was available. Both failures reproduce when run directly with CTest.
- `python3 scripts/ledger.py validate ledger.jsonl` passed with 406 entries;
  `git diff --check` passed. No lifecycle event was added because these work
  orders are not frozen in `work/`.

## Maintainer real-window verification

This lane intentionally did not drive or capture the live window. The maintainer
should verify in macOS light and dark appearance that selection feedback matches
between canvas and Layers selection; all nine handles remain hittable at 25%,
100%, and 800%; move, directional resize, rotate, and pan cursors match their
targets; and the action strip stays clear of the pointer and selection while its
actions remain easy to hit. Check the snapping popover placement near window
edges, independent toggles, the master shortcut, Shift bypass, and the displayed
alignment line/measurement at 25%, 100%, and 800% zoom.

No lifecycle ledger event was added: these two work orders exist only in the
research scratchpad, and the ledger validator requires a frozen `work/WO-*.md`
file for every event.
