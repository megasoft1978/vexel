# W14 Nodes

## Changes

- Node editing now enters only for a selected path with supported path commands,
  through `N`, the Node tool, or Return. Unsupported commands and non-path
  selections stay out of node mode.
- Anchors use distinct square, circle, and diamond states for unselected,
  hovered, and selected points. Selecting a curve anchor reveals its adjacent
  handles. The options row exposes Corner/Smooth only when the selected command
  can be converted safely.
- Tab and Shift-Tab traverse anchors in path order; arrows nudge by one unit or
  ten with Shift. Focus leaves the canvas only after traversal passes an end.
  Escape cancels a handle gesture or exits node mode. The initial move-to anchor
  cannot be deleted, including through the toolbar; toolbar Delete removes the
  selected node while node mode is active.
- Segment hover explains double-click insertion. Corner expands shorthand
  S/T commands while preserving their current geometry; Smooth updates supported
  cubic/quadratic controls. Edits remain span-local and undoable.
- Added the `node-edit` headless style state and four light/dark 1x/2x goldens.
  The render shows the path, selected anchor, adjacent handles, and hovered
  segment state. The chrome semantics test covers the Corner/Smooth options and
  Delete enablement.

## Verification

- The non-path node-mode regression failed before the fix at the assertion that
  node editing must remain inactive; it passes after the change.
- `./dev t -k editor`: 10/10 CTests and 2/2 mapped Python tests passed.
- `./dev t -k viewer-chrome-style`: 1/1 CTest passed, including node option
  semantics and node-aware toolbar Delete behavior.
- Final `./dev t --all`: 55/57 CTests and 69/69 Python tests passed. The two
  environment failures are `index-replay` (no offscreen Metal device/queue) and
  `native-viewer-smoke` (the sandbox LaunchServices failure already recorded in
  `STATE.md`). Slow Python tests are reported as skipped by this command.
- `./dev shape-fuzz --duration 60 --seed 20261011 --cases 1000000 --output
  .cache/w14-nodes/shape-fuzz.json`: 1,000,000 ASan+UBSan cases passed in
  42.409 seconds with zero crashes, sanitizer reports, or invariant failures.
- `python3 tests/ui/verify_style_goldens.py`: all 24 light/dark 1x/2x screenshots
  matched. `./dev pixdiff` reported exact matches for the new light and dark
  1280x800 node-edit images. `./dev shot` rendered
  `tests/svg/shape/fixtures/smooth-cubic.svg` to
  `/private/tmp/w14-nodes-smooth-cubic.png`.
- `python3 scripts/ledger.py validate ledger.jsonl`: 406 entries valid;
  `git diff --check` passed. No ledger lifecycle entry was added because these
  report proposals are not frozen work orders under `work/`.

## WO-UX-03 gate

WO-UX-03 remains unstarted because the research report requires acceptance of
the active gradient renderer first. The current WO-M4-06 round 7 status in
`STATE.md` is blocked: 47/48 reference fixtures pass, with `clip-union` still
over the raw RGB limit (4 vs 1 at pixel (8,4)). Direct gradient and stroke
editing must wait for that acceptance so handles use the accepted renderer
semantics.

## Maintainer real-window verification

No real-window automation or screen capture was run for this lane. Please check
in both light and dark appearances that:

- `N` and Return enter node mode on a selected supported path; unsupported paths
  remain read-only and show a clear status.
- Hover, selection, anchor shapes, adjacent handles, segment hover text, and
  double-click line insertion are visible and hit targets feel usable.
- Tab/Shift-Tab follow path order, focus exits at either end, and arrow nudges
  use one/ten-unit steps.
- Corner/Smooth appear only for supported anchors, the initial M anchor has
  disabled Delete, and Delete on other anchors removes only the node.
- Escape cancels a drag without changing source; release creates one undo step,
  and undo/redo restores exact source bytes.
