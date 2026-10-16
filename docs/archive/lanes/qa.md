# W9 Real-Machine QA

## Environment and result

- Host: Apple Silicon macOS 27.0.
- No dependencies were added. No commit was created.
- `./dev final` passed in 240 seconds after fixing the shape-fuzz invocation.
- The final gate passed Debug and Release verification, ASan and UBSan CTest
  (56/56 each), 293 Python tests, and ledger validation (406 entries).
- ASan+UBSan shape fuzz passed 1,000,000 cases with seed `20261010`, zero
  crashes, invariant failures, or sanitizer reports. MCP fuzz passed 200,000
  cases with seed `12648430`. E2E passed 256 documents, 674 renders, and 162
  mutated inputs with no crashes, timeouts, or failures. The one-minute soak
  passed 10,000 iterations with 606,208 bytes RSS growth.
- Native editor and viewer smoke passed in the final gate.

## Commands run

- `./dev final`
- `./dev editor-smoke` — 10 consecutive runs, all passed.
- `./dev viewer-smoke` — 10 consecutive runs, all passed.
- `./dev build`
- `ctest --test-dir build/mac-debug -R '^editor-(canvas|clipboard-ids)$' --output-on-failure`
- `git diff --check`

The smoke loops used the native app and `VEXEL_TEST_OPEN` path. Each run
finished cleanly; the loop checked for and cleaned up any app process left
behind. No stray process needed termination, and no `svgworkbench` process
remained afterward.

## Bugs found and fixed

### Final gate omitted shape-fuzz seed

The first `./dev final` stopped at shape fuzz because the final target invoked
`shape-fuzz` without its required `--seed`. Added a deterministic seed and
repo-local report path to `dev`. The new
`tests/e2e/test_dev_final.py` regression failed before the change and passed
afterward. The complete final gate then passed.

### Window had no route for arrange shortcuts

The alignment operation existed in the editor but the real window did not
dispatch alignment shortcuts. Added Command+Option+Arrow for edge alignment
and Command+Option+H/V for center alignment. The new
`tests/e2e/test_editor_alignment_shortcuts.py` regression failed before the
change and passed after. In the real window, aligning a multi-selection,
undoing, and redoing all worked.

### Clipboard paste duplicated SVG IDs

Pasting a copied animated element kept its original ID. The document then had
duplicate IDs, so preview decoding failed. System clipboard paste now assigns
unique IDs to collisions and updates matching `href="#id"` and `url(#id)`
references in XML attribute values. The existing raw-fragment paste API is
unchanged. The new `editor-clipboard-ids` C regression covers an animated
element and a grouped clip-path reference; it failed before the fix and passed
after. The live copy/paste, preview, save, and reopen flow also passed.

## Real-window walkthrough

Used `tests/editor/mouse.py`, System Events, and `screencapture -l <CGWindowID>`
against the native window. The walkthrough covered opening an SVG; selecting,
moving, and resizing; drawing ellipse, line, polygon, polyline, pen, and text;
editing a path node; grouping and ungrouping; aligning; undo and redo;
copy/paste; PNG export; save and reopen; and playing an animated SVG on the
timeline. No crash, stuck UI, or incorrect hit-testing was found during this
pass. The PNG export is valid RGBA at 640×480 pixels.

Screenshots and run logs are in the ignored repo-local `.cache/qa/` directory.
The walkthrough captures run from `00-open.png` through
`44-timeline-midpoint.png`, with `qa-export.png` for the exported PNG. The
timeline captures show playback advancing at the midpoint and stopping at
the animation endpoint.

## Remaining

- One immediate ellipse-tool switch followed by a drag produced no ellipse;
  repeating after a short pause worked. This was not reproduced or isolated,
  so rapid tool-switch drawing should be retested.
- Clipboard ID rewriting accepts safe ASCII IDs and updates fragment
  references in `href` and `url(...)` attribute values. References in CSS text
  nodes or other SVG reference forms, and IDs outside that accepted character
  set, are not rewritten by this helper.
