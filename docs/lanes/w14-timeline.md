# W14 timeline lane

## Changes

- Added the source-preserving animation timeline contract and layout sketches
  in `docs/ui/animation-timeline.md`.
- Replaced the floating timeline overlay with a dock above the status bar. It
  opens for documents with supported SMIL tracks, starts at 220 points, resizes
  from 184–320 points, collapses to 32 points, and remembers the user's choice
  as documents change. Short windows collapse the dock before reducing the
  canvas below its minimum.
- Added virtualized track rows, a fixed target gutter, time ruler and grid,
  playhead, source timing bars, key hit targets, scrubbing, command-scroll
  zoom, horizontal pan, Fit, One Shot/Loop/Ping-Pong, and 0.25×–2× preview
  speeds. Compact layouts retain every transport control.
- Kept source bytes untouched during preview. Lane identity uses source spans;
  repeated attributes receive ordinal labels, and unsupported rows announce
  their parser diagnostic. Canvas/editor selection synchronizes with the
  selected timeline row.
- Passed the system Reduce Motion preference through the editor and timeline.
  Layout and redraw remain immediate, and explicit SVG Play remains available.
- Added light and dark headless timeline goldens at 1× and 2×. Split existing
  display-name and test-layout-dump helpers out of `viewer.c` so the edited
  file stays below 800 lines.

## Verification

- Test-first check: the initial `./dev t -k viewer-timeline` run failed because
  the new timeline APIs/layout were not yet implemented. After implementation,
  `./dev t -k viewer-timeline` passed (1/1 CTest), and
  `./dev t -k viewer-chrome` passed (2/2 CTests).
- `./dev build`: passed Debug configure and build.
- `python3 tests/ui/verify_timeline_goldens.py`: passed for dark/light at 1×/2×.
- `./dev shot tests/anim/fixtures/pulse-radius.svg --size 640x480` rendered
  successfully. `./dev pixdiff` against the earlier render reported identical
  pixels at tolerance 0.
- `./dev shape-fuzz --duration 10 --seed 20261011 --cases 10000000`: passed
  230,400 cases under ASan/UBSan, with zero crashes, sanitizer reports, or
  invariant failures; all required feature render classes were covered.
- `python3 scripts/ledger.py validate ledger.jsonl`: 406 entries valid.
- `git diff --check`: passed.
- `./dev t --all`: 69/69 Python tests passed; 56/58 CTests passed. The two
  failures were `index-replay` (offscreen Metal device/queue unavailable) and
  `native-viewer-smoke`. That CTest invokes native event injection; it was not
  rerun separately.
- `./dev fuzz --duration 3 --seed 20261011` did not complete. The existing CLI
  smoke runner treated the CLI's standard usage exit 2 as unexpected; after a
  temporary diagnostic adjustment, a later child timed out with LaunchServices
  XPC errors. No changes to that unrelated runner were retained. The bounded
  shape sanitizer campaign above passed.

## Maintainer real-window verification

No manual window driving or screencapture was done for this lane. Please check
these behaviors in the native app:

- Open an animated SVG and confirm the dock appears above the status bar; use
  the toolbar and dock buttons to collapse/reopen it and confirm the choice
  persists when switching documents.
- Resize the dock, then make the window short and confirm it collapses while
  the canvas and status bar remain usable. Review the compact transport at a
  narrow window width.
- Scrub and click keys, use command-scroll zoom and horizontal pan, select a
  track and its target in both directions, then exercise each mode and speed.
- Toggle system Reduce Motion while the app is open. Confirm the layout changes
  immediately and explicit SVG preview playback still works.
- Review the dock in light and dark appearances, including VoiceOver names for
  the time readout, transport, selected track, and unsupported-track diagnostic.

These are report-derived W14 proposals rather than frozen work orders under
`work/`; no lifecycle ledger entry was added.
