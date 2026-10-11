# w3-editor lane

## Built

- Wired the existing source-preserving edit engine into the SVG viewer.
- Added retained toolbar, structure outline, attribute/text inspector, status
  bar, direct scalar chrome painting, and stable editor semantics.
- Added key and pointer event injection, structural edits, bounded inline field
  editing, undo/redo, atomic save, dirty-title state, and generation-cancelled
  worker preview rendering that retains the last good canvas on failure.
- Added state-machine, randomized-event, dark/light chrome golden, and native
  desktop smoke coverage.

## Limits and platform requests

The platform boundary does not yet expose native Save As, dirty-close/open
confirmation, system appearance notifications, clipboard, or IME marked text.
Dirty navigation is therefore blocked with a status message, external changes
block overwrite after a pre-save timestamp check, Save As reports that the
native panel is missing, the viewer uses the dark theme, and clipboard/IME
composition are unavailable. Basic single-line caret, range selection,
deletion, and keyboard movement work.

The renderer does not expose per-shape source offsets or bounding boxes. Canvas
selection remains a named source-offset hook, and no canvas outline is drawn
until bounding boxes are available. Current decoder/rasterizer SVG support
limits previews; invalid or unsupported edited source keeps the last good
frame and reports the diagnostic.

Platform API requests: native Save As panel; dirty open/close/quit decision
prompt; system appearance subscription; clipboard read/write; marked-text
composition events and ranges. A canvas shape hit-test/source-offset and
bounds API is also needed from the decode/render lane.

## Verification

- `./dev build` — passed on the final source tree.
- `ctest --test-dir build/mac-debug -R '^(editor-state|editor-chrome)$'
  --output-on-failure` — 2/2 passed. The state test includes 200,000 seeded
  event calls, well-formedness checks after each event, and undo-all byte
  restoration.
- The editor state and chrome golden tests were rebuilt and run directly in
  `build/mac-asan` and `build/mac-ubsan`; both passed in each configuration.
- `./dev verify` and full Debug CTest — 26/28 passed. `native-viewer-smoke`
  could not present a frame because Metal was unavailable; `index-replay`
  could not create an offscreen Metal device/queue.
- `./dev asan` and `./dev ubsan` — builds succeeded; each full CTest run had
  the same two Metal-dependent failures above, with the editor tests passing.
- `python3 -m unittest discover -s tests -t .` and `./dev test-fast` —
  238/248 passed, 8 failed, 2 errored. Seven failures are benchmark-harness
  process-policy/output cases; the other three platform-driver cases require
  an unbuilt Release executable (two errors) or native Metal (one failure).
- `python3 scripts/ledger.py validate ledger.jsonl` — passed, 406 entries.
- `sh -n tests/editor/native_smoke.sh` and `git diff --check` — passed.

The native editor smoke is intended for the maintainer to run from an
interactive macOS desktop. It was not run in this headless session; Python
platform-driver tests that expect `build/mac-release/bin/svgworkbench` also
remain unverified because the Release preset was not built.
